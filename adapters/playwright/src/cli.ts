#!/usr/bin/env node

import { readFile } from "node:fs/promises";

import { canonicalJson } from "./canonical.js";
import { capture } from "./capture.js";
import { AdapterError, type JsonValue } from "./types.js";
import { parseCaptureRequest } from "./validate.js";

interface Arguments {
  request: string;
  repositoryRoot: string;
  artifactIrOut: string;
  screenshotOut: string;
}

function parseArguments(values: string[]): Arguments {
  const parsed = new Map<string, string>();
  for (let index = 0; index < values.length; index += 2) {
    const flag = values[index];
    const value = values[index + 1];
    if (flag === undefined || value === undefined || !flag.startsWith("--")) {
      throw new AdapterError("usage", "expected flag/value pairs");
    }
    if (!["--request", "--repository-root", "--artifact-ir-out", "--screenshot-out"].includes(flag)) {
      throw new AdapterError("usage", `unsupported argument ${flag}`);
    }
    if (parsed.has(flag)) {
      throw new AdapterError("usage", `duplicate argument ${flag}`);
    }
    parsed.set(flag, value);
  }
  const request = parsed.get("--request");
  const repositoryRoot = parsed.get("--repository-root");
  const artifactIrOut = parsed.get("--artifact-ir-out");
  const screenshotOut = parsed.get("--screenshot-out");
  if (request === undefined || repositoryRoot === undefined || artifactIrOut === undefined || screenshotOut === undefined) {
    throw new AdapterError(
      "usage",
      "required arguments: --request, --repository-root, --artifact-ir-out, --screenshot-out",
    );
  }
  if (artifactIrOut === screenshotOut) {
    throw new AdapterError("usage", "Artifact IR and screenshot outputs must be different files");
  }
  return { request, repositoryRoot, artifactIrOut, screenshotOut };
}

async function main(): Promise<void> {
  const args = parseArguments(process.argv.slice(2));
  const requestBytes = await readFile(args.request).catch(() => {
    throw new AdapterError("request-read", "failed to read capture request");
  });
  const request = parseCaptureRequest(requestBytes);
  const response = await capture(request, args.repositoryRoot, {
    artifactIrPath: args.artifactIrOut,
    screenshotPath: args.screenshotOut,
  });
  process.stdout.write(canonicalJson(response as unknown as JsonValue));
}

try {
  await main();
} catch (error) {
  const adapterError = error instanceof AdapterError
    ? error
    : new AdapterError("execution-error", "adapter capture failed");
  process.stderr.write(`sightlint-web: ${adapterError.code}: ${adapterError.message}\n`);
  process.exitCode = 2;
}
