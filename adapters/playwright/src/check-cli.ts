#!/usr/bin/env node

import { readFile } from "node:fs/promises";

import { canonicalJson } from "./canonical.js";
import { AdapterError, type JsonValue } from "./types.js";
import { parseCaptureRequest } from "./validate.js";
import { humanWorkflowReport, runWebCheck } from "./workflow.js";

interface Arguments {
  request: string;
  repositoryRoot: string;
  sightlintBinary: string;
  format: "json" | "human";
  allowServerCommand: boolean;
}

function parseArguments(values: string[]): Arguments {
  const parsed = new Map<string, string>();
  let allowServerCommand = false;
  for (let index = 0; index < values.length;) {
    const flag = values[index];
    if (flag === "--allow-server-command") {
      if (allowServerCommand) throw new AdapterError("usage", "duplicate argument --allow-server-command");
      allowServerCommand = true;
      index += 1;
      continue;
    }
    const value = values[index + 1];
    if (flag === undefined || value === undefined || !flag.startsWith("--")) {
      throw new AdapterError("usage", "expected flag/value pairs");
    }
    if (!["--request", "--repository-root", "--sightlint-binary", "--format"].includes(flag)) {
      throw new AdapterError("usage", `unsupported argument ${flag}`);
    }
    if (parsed.has(flag)) {
      throw new AdapterError("usage", `duplicate argument ${flag}`);
    }
    parsed.set(flag, value);
    index += 2;
  }
  const request = parsed.get("--request");
  const repositoryRoot = parsed.get("--repository-root");
  const sightlintBinary = parsed.get("--sightlint-binary");
  const format = parsed.get("--format") ?? "human";
  if (request === undefined || repositoryRoot === undefined || sightlintBinary === undefined) {
    throw new AdapterError(
      "usage",
      "required arguments: --request, --repository-root, --sightlint-binary; optional: --format human|json",
    );
  }
  if (format !== "human" && format !== "json") {
    throw new AdapterError("usage", "format must be human or json");
  }
  return { request, repositoryRoot, sightlintBinary, format, allowServerCommand };
}

async function main(): Promise<void> {
  const args = parseArguments(process.argv.slice(2));
  const requestBytes = await readFile(args.request).catch(() => {
    throw new AdapterError("request-read", "failed to read capture request");
  });
  const request = parseCaptureRequest(requestBytes);
  const execution = await runWebCheck(
    request,
    args.repositoryRoot,
    args.sightlintBinary,
    { allowServerCommand: args.allowServerCommand },
  );
  const output = args.format === "json"
    ? canonicalJson(execution.report as unknown as JsonValue)
    : humanWorkflowReport(execution.report);
  process.stdout.write(output);
  process.exitCode = execution.exitCode;
}

try {
  await main();
} catch (error) {
  const adapterError = error instanceof AdapterError
    ? error
    : new AdapterError("execution-error", "local Web check failed");
  process.stderr.write(`sightlint-web-check: ${adapterError.code}: ${adapterError.message}\n`);
  process.exitCode = 2;
}
