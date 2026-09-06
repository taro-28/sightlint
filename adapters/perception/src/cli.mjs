#!/usr/bin/env node

import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

import { canonicalJson, sha256 } from "./canonical.mjs";
import { PerceptionError } from "./errors.mjs";
import { mapResponseToArtifactIr } from "./map.mjs";
import { runBoundedProcess } from "./process.mjs";
import { parseRequest, parseWorkerResponse } from "./validate.mjs";

function parseArguments(values) {
  const singles = new Map();
  const workerArguments = [];
  for (let index = 0; index < values.length; index += 2) {
    const flag = values[index];
    const value = values[index + 1];
    if (value === undefined || !flag?.startsWith("--")) throw new PerceptionError("usage", "expected flag/value pairs");
    if (flag === "--worker-argument") {
      workerArguments.push(value);
    } else if (["--request", "--worker-program", "--worker-source", "--sightlint-binary", "--response-out", "--artifact-ir-out"].includes(flag)) {
      if (singles.has(flag)) throw new PerceptionError("usage", `duplicate argument ${flag}`);
      singles.set(flag, value);
    } else {
      throw new PerceptionError("usage", `unsupported argument ${flag}`);
    }
  }
  const required = ["--request", "--worker-program", "--worker-source", "--sightlint-binary", "--response-out", "--artifact-ir-out"];
  for (const flag of required) if (!singles.has(flag)) throw new PerceptionError("usage", `missing required argument ${flag}`);
  if (singles.get("--response-out") === singles.get("--artifact-ir-out")) throw new PerceptionError("usage", "response and Artifact IR outputs must differ");
  return {
    request: singles.get("--request"), workerProgram: singles.get("--worker-program"), workerSource: singles.get("--worker-source"),
    sightlintBinary: singles.get("--sightlint-binary"), responseOut: singles.get("--response-out"), artifactIrOut: singles.get("--artifact-ir-out"),
    workerArguments,
  };
}

async function writeExclusive(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes, { flag: "wx" }).catch((error) => {
    if (error.code === "EEXIST") throw new PerceptionError("output-exists", "refusing to overwrite an existing output file");
    throw new PerceptionError("output-write", "failed to write perception output");
  });
}

async function writeOutputPair(responsePath, responseBytes, artifactPath, artifactBytes) {
  const created = [];
  try {
    await writeExclusive(responsePath, responseBytes);
    created.push(responsePath);
    await writeExclusive(artifactPath, artifactBytes);
    created.push(artifactPath);
  } catch (error) {
    await Promise.all(created.map((path) => rm(path, { force: true })));
    throw error;
  }
}

async function main() {
  const args = parseArguments(process.argv.slice(2));
  const requestBytes = await readFile(args.request).catch(() => { throw new PerceptionError("request-read", "failed to read perception request"); });
  const request = parseRequest(requestBytes);
  const workerSource = await readFile(args.workerSource).catch(() => { throw new PerceptionError("worker-source-read", "failed to read worker provenance source"); });
  const workerResult = await runBoundedProcess(args.workerProgram, args.workerArguments, Buffer.from(canonicalJson(request)), {
    label: "perception worker", timeoutMs: request.execution.timeoutMs,
    maxStdoutBytes: request.execution.maxOutputBytes, maxStderrBytes: request.execution.maxStderrBytes,
    spawnCode: "worker-spawn", timeoutCode: "worker-timeout", stdoutCode: "worker-output-budget", stderrCode: "worker-stderr-budget", exitCode: "worker-exit",
  });
  const response = parseWorkerResponse(workerResult.stdout, request, sha256(workerSource));
  const responseBytes = Buffer.from(canonicalJson(response));
  const candidateBytes = Buffer.from(canonicalJson(mapResponseToArtifactIr(request, response)));
  const normalized = await runBoundedProcess(args.sightlintBinary, ["normalize", "-"], candidateBytes, {
    label: "SightLint normalizer", timeoutMs: 5000,
    maxStdoutBytes: 8 * 1024 * 1024, maxStderrBytes: 64 * 1024,
    spawnCode: "normalizer-spawn", timeoutCode: "normalizer-timeout", stdoutCode: "normalizer-output-budget", stderrCode: "normalizer-stderr-budget", exitCode: "normalizer-exit",
  });
  if (normalized.stderr.byteLength !== 0) throw new PerceptionError("normalizer-stderr", "SightLint normalizer wrote unexpected stderr");
  JSON.parse(normalized.stdout.toString("utf8"));

  await writeOutputPair(args.responseOut, responseBytes, args.artifactIrOut, normalized.stdout);
  const runReport = {
    runSchemaVersion: "0.1.0",
    protocolVersion: "0.1.0",
    requestId: request.requestId,
    status: response.status,
    blocking: false,
    ruleOutcome: "untested",
    requestSha256: sha256(Buffer.from(canonicalJson(request))),
    workerResponse: { reference: request.output.responseReference, sha256: sha256(responseBytes), byteLength: responseBytes.byteLength },
    artifactIr: { reference: request.output.artifactIrReference, sha256: sha256(normalized.stdout), byteLength: normalized.stdout.byteLength },
    familyStatus: response.familyStatus,
    resourceUse: {
      requestBytes: Buffer.byteLength(canonicalJson(request)), workerStdoutBytes: workerResult.stdoutBytes,
      workerStderrBytes: workerResult.stderrBytes, observationCount: response.observations.length, timeoutMs: request.execution.timeoutMs,
    },
    limitations: [
      "The run report is acquisition evidence and never a trusted rule verdict.",
      "Wall-clock duration is intentionally excluded from canonical output.",
      "Process isolation is not an operating-system sandbox.",
    ],
  };
  process.stdout.write(canonicalJson(runReport));
}

try {
  await main();
} catch (error) {
  const known = error instanceof PerceptionError ? error : new PerceptionError("execution-error", "perception wrapper failed");
  process.stderr.write(`sightlint-perception: ${known.code}: ${known.message}\n`);
  process.exitCode = 2;
}
