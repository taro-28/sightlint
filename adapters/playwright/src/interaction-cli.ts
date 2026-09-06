#!/usr/bin/env node

import { readFile } from "node:fs/promises";

import { canonicalJson } from "./canonical.js";
import { captureInteraction } from "./interaction-capture.js";
import { parseInteractionRequest } from "./interaction-validate.js";
import { AdapterError, type JsonValue } from "./types.js";

interface Arguments {
  request: string;
  repositoryRoot: string;
  artifactIrOut: string;
}

function parseArguments(values: string[]): Arguments {
  const parsed = new Map<string, string>();
  for (let index = 0; index < values.length; index += 2) {
    const flag = values[index];
    const value = values[index + 1];
    if (flag === undefined || value === undefined || !flag.startsWith("--")) {
      throw new AdapterError("usage", "expected flag/value pairs");
    }
    if (!["--request", "--repository-root", "--artifact-ir-out"].includes(flag)) {
      throw new AdapterError("usage", `unsupported argument ${flag}`);
    }
    if (parsed.has(flag)) throw new AdapterError("usage", `duplicate argument ${flag}`);
    parsed.set(flag, value);
  }
  const request = parsed.get("--request");
  const repositoryRoot = parsed.get("--repository-root");
  const artifactIrOut = parsed.get("--artifact-ir-out");
  if (request === undefined || repositoryRoot === undefined || artifactIrOut === undefined) {
    throw new AdapterError(
      "usage",
      "required arguments: --request, --repository-root, --artifact-ir-out",
    );
  }
  return { request, repositoryRoot, artifactIrOut };
}

async function main(): Promise<void> {
  const args = parseArguments(process.argv.slice(2));
  const requestBytes = await readFile(args.request).catch(() => {
    throw new AdapterError("request-read", "failed to read interaction request");
  });
  const request = parseInteractionRequest(requestBytes);
  const capture = await captureInteraction(request, args.repositoryRoot, args.artifactIrOut);
  process.stdout.write(canonicalJson(capture.response as unknown as JsonValue));
}

try {
  await main();
} catch (error) {
  const adapterError = error instanceof AdapterError
    ? error
    : new AdapterError("execution-error", "interaction capture failed");
  process.stderr.write(`sightlint-interaction: ${adapterError.code}: ${adapterError.message}\n`);
  process.exitCode = 2;
}
