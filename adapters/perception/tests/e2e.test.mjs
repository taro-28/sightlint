import assert from "node:assert/strict";
import { access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { canonicalJson } from "../src/canonical.mjs";
import { requestFor } from "./helpers.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const cli = resolve(repositoryRoot, "adapters/perception/src/cli.mjs");
const referenceWorker = resolve(repositoryRoot, "adapters/perception/src/reference-worker.mjs");
const sightlintBinary = process.env.SIGHTLINT_BINARY ?? resolve(repositoryRoot, "target/debug/sightlint");

function run(program, args) {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, { cwd: repositoryRoot, stdio: ["ignore", "pipe", "pipe"] });
    const stdout = [];
    const stderr = [];
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", reject);
    child.on("close", (code, signal) => resolveRun({ code: code ?? -1, signal, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) }));
  });
}

async function report(name) {
  return JSON.parse(await readFile(resolve(repositoryRoot, `fixtures/perception/${name}`), "utf8"));
}

function cliArguments(directory, requestPath, worker = referenceWorker) {
  return [
    cli,
    "--request", requestPath,
    "--worker-program", process.execPath,
    "--worker-argument", worker,
    "--worker-source", worker,
    "--sightlint-binary", sightlintBinary,
    "--response-out", join(directory, "response.json"),
    "--artifact-ir-out", join(directory, "artifact-ir.json"),
  ];
}

test("public wrapper runs the reference worker twice with byte-stable mapped IR", { timeout: 30_000 }, async () => {
  const firstDirectory = await mkdtemp(join(tmpdir(), "sightlint-perception-first-"));
  const secondDirectory = await mkdtemp(join(tmpdir(), "sightlint-perception-second-"));
  try {
    const request = requestFor(await report("observed-segmentation-report.json"));
    const firstRequest = join(firstDirectory, "request.json");
    const secondRequest = join(secondDirectory, "request.json");
    await writeFile(firstRequest, canonicalJson(request));
    await writeFile(secondRequest, canonicalJson(request));
    const first = await run(process.execPath, cliArguments(firstDirectory, firstRequest));
    const second = await run(process.execPath, cliArguments(secondDirectory, secondRequest));
    assert.equal(first.code, 0, first.stderr.toString("utf8"));
    assert.equal(first.signal, null);
    assert.equal(first.stderr.byteLength, 0);
    assert.deepEqual(second, first);
    const firstResponse = await readFile(join(firstDirectory, "response.json"));
    const secondResponse = await readFile(join(secondDirectory, "response.json"));
    const firstArtifact = await readFile(join(firstDirectory, "artifact-ir.json"));
    assert.deepEqual(secondResponse, firstResponse);
    assert.deepEqual(await readFile(join(secondDirectory, "artifact-ir.json")), firstArtifact);
    const response = JSON.parse(firstResponse.toString("utf8"));
    assert.equal(response.status, "partial");
    assert.equal(response.observations.length, 2);
    assert.deepEqual(response.observations.map((item) => item.id), [...response.observations.map((item) => item.id)].sort());
    const artifact = JSON.parse(firstArtifact.toString("utf8"));
    assert.equal(artifact.nodes.length, 2);
    assert.equal(artifact.relations, undefined);
    assert.equal(artifact.extensions["org.sightlint.perception"].mapping.coreSemanticPromotionCount, 0);
    const check = await run(sightlintBinary, ["check", join(firstDirectory, "artifact-ir.json"), "--format", "json"]);
    assert.equal(check.code, 0, check.stderr.toString("utf8"));
    const checkReport = JSON.parse(check.stdout.toString("utf8"));
    assert.equal(checkReport.summary.failed, 0);
  } finally {
    await rm(firstDirectory, { recursive: true, force: true });
    await rm(secondDirectory, { recursive: true, force: true });
  }
});

test("explicit unavailable acquisition remains nonblocking with no partial observations", { timeout: 30_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-perception-unavailable-"));
  try {
    const request = requestFor(await report("unavailable-segmentation-report.json"), { requestId: "reference-unavailable" });
    const requestPath = join(directory, "request.json");
    await writeFile(requestPath, canonicalJson(request));
    const result = await run(process.execPath, cliArguments(directory, requestPath));
    assert.equal(result.code, 0, result.stderr.toString("utf8"));
    const runReport = JSON.parse(result.stdout.toString("utf8"));
    assert.equal(runReport.status, "unsupported");
    assert.equal(runReport.blocking, false);
    assert.equal(runReport.ruleOutcome, "untested");
    const response = JSON.parse(await readFile(join(directory, "response.json"), "utf8"));
    assert.deepEqual(response.observations, []);
    const artifact = JSON.parse(await readFile(join(directory, "artifact-ir.json"), "utf8"));
    assert.deepEqual(artifact.nodes, []);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("public wrapper retains inferred families without semantic core promotion", { timeout: 30_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-perception-inferred-"));
  try {
    const model = { status: "selected", name: "fixture-model", version: "1", sha256: `sha256:${"b".repeat(64)}` };
    const request = requestFor(await report("observed-segmentation-report.json"), {
      requestId: "inferred-conformance",
      expectedName: "fixture-inferred-worker",
      model,
    });
    const requestPath = join(directory, "request.json");
    await writeFile(requestPath, canonicalJson(request));
    const worker = resolve(repositoryRoot, "fixtures/perception/workers/inferred-semantics.mjs");
    const result = await run(process.execPath, cliArguments(directory, requestPath, worker));
    assert.equal(result.code, 0, result.stderr.toString("utf8"));
    const response = JSON.parse(await readFile(join(directory, "response.json"), "utf8"));
    assert.deepEqual(new Set(response.observations.map((item) => item.family)), new Set(["hierarchy", "peerGroup", "region", "role", "text"]));
    assert.equal(response.observations.find((item) => item.id === "role:a").confidence.status, "calibratedProbability");
    assert.equal(response.observations.find((item) => item.id === "text:a").alternatives[0].probability, null);
    const artifact = JSON.parse(await readFile(join(directory, "artifact-ir.json"), "utf8"));
    assert.deepEqual(artifact.nodes, []);
    assert.equal(artifact.relations, undefined);
    assert.equal(artifact.extensions["org.sightlint.perception"].mapping.coreSemanticPromotionCount, 0);
    assert.equal(artifact.extensions["org.sightlint.perception"].mapping.unmappedObservationCount, 6);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("timeout, resource overflow, nonzero exit, malformed output, and identity mismatch emit stable errors without artifacts", { timeout: 30_000 }, async () => {
  const cases = [
    ["hang.mjs", "worker-timeout", { timeoutMs: 50 }],
    ["oversize.mjs", "worker-output-budget", { maxOutputBytes: 1024 }],
    ["oversize-stderr.mjs", "worker-stderr-budget", { maxStderrBytes: 1024 }],
    ["nonzero.mjs", "worker-exit", {}],
    ["malformed.mjs", "worker-json", {}],
    ["wrong-identity.mjs", "protocol-invalid", {}],
  ];
  for (const [workerName, code, options] of cases) {
    const directory = await mkdtemp(join(tmpdir(), `sightlint-perception-${workerName}-`));
    try {
      const request = requestFor(await report("observed-segmentation-report.json"), options);
      const requestPath = join(directory, "request.json");
      await writeFile(requestPath, canonicalJson(request));
      const worker = resolve(repositoryRoot, `fixtures/perception/workers/${workerName}`);
      const result = await run(process.execPath, cliArguments(directory, requestPath, worker));
      assert.equal(result.code, 2, `${workerName}: ${result.stderr}`);
      assert.equal(result.stdout.byteLength, 0);
      assert.match(result.stderr.toString("utf8"), new RegExp(`^sightlint-perception: ${code}:`));
      await assert.rejects(access(join(directory, "response.json")));
      await assert.rejects(access(join(directory, "artifact-ir.json")));
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  }
});

test("public wrapper refuses to overwrite caller-owned output and removes its partial pair", { timeout: 30_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-perception-output-exists-"));
  try {
    const request = requestFor(await report("observed-segmentation-report.json"), { requestId: "output-exists" });
    const requestPath = join(directory, "request.json");
    await writeFile(requestPath, canonicalJson(request));
    await writeFile(join(directory, "artifact-ir.json"), "owned-by-caller\n");
    const result = await run(process.execPath, cliArguments(directory, requestPath));
    assert.equal(result.code, 2);
    assert.equal(result.stdout.byteLength, 0);
    assert.match(result.stderr.toString("utf8"), /^sightlint-perception: output-exists:/);
    await assert.rejects(access(join(directory, "response.json")));
    assert.equal(await readFile(join(directory, "artifact-ir.json"), "utf8"), "owned-by-caller\n");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
