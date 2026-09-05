import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type AnySchema } from "ajv/dist/2020.js";

import { canonicalJson, sha256 } from "../src/canonical.js";
import { LIMITS, type JsonValue } from "../src/types.js";
import { parseCaptureRequest } from "../src/validate.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

async function json(path: string): Promise<unknown> {
  return JSON.parse(await readFile(resolve(repositoryRoot, path), "utf8")) as unknown;
}

test("canonical JSON orders object keys, preserves array order, and normalizes negative zero", () => {
  const value = { z: -0, a: [{ z: 1, a: 2 }, "last"] } as unknown as JsonValue;
  const encoded = canonicalJson(value);
  assert.equal(encoded, '{"a":[{"a":2,"z":1},"last"],"z":0}\n');
  assert.equal(sha256(encoded), "sha256:4fac6adbd6cba80f1be71fb64d4aabb12b4734c8da4c294659eaab14a59d4238");
});

test("request parser accepts the reviewed capture request", async () => {
  const bytes = await readFile(resolve(repositoryRoot, "evaluation/web/requests/dashboard-browser-clean.json"));
  const request = parseCaptureRequest(bytes);
  assert.equal(request.protocolVersion, "0.1.0");
  assert.deepEqual(request.environment.viewport, { width: 1280, height: 800, unit: "cssPixel" });
  assert.equal(request.privacy.externalProcessing, false);
  assert.equal(request.network.mode, "deny");
});

test("request parser rejects unknown fields and path traversal", async () => {
  const base = await json("evaluation/web/requests/dashboard-browser-clean.json") as Record<string, unknown>;
  assert.throws(
    () => parseCaptureRequest(Buffer.from(JSON.stringify({ ...base, invented: true }))),
    /request fields must be exactly/u,
  );

  const escaped = structuredClone(base) as {
    fixture: { entrypoint: string };
  };
  escaped.fixture.entrypoint = "../outside.html";
  assert.throws(
    () => parseCaptureRequest(Buffer.from(JSON.stringify(escaped))),
    /fixture entrypoint must be a repository-relative path/u,
  );
});

test("request parser enforces byte, viewport, and closed-environment limits", async () => {
  assert.throws(
    () => parseCaptureRequest(Buffer.alloc(LIMITS.requestBytes + 1, 0x20)),
    /request exceeds 1048576 bytes/u,
  );

  const base = await json("evaluation/web/requests/dashboard-browser-clean.json") as {
    environment: { viewport: { width: number }; locale: string };
  };
  const oversizedViewport = structuredClone(base);
  oversizedViewport.environment.viewport.width = 4097;
  assert.throws(
    () => parseCaptureRequest(Buffer.from(JSON.stringify(oversizedViewport))),
    /viewport axes must be integers from 1 through 4096/u,
  );

  const unsupportedLocale = structuredClone(base);
  unsupportedLocale.environment.locale = "ja-JP";
  assert.throws(
    () => parseCaptureRequest(Buffer.from(JSON.stringify(unsupportedLocale))),
    /request.environment.locale must be "en-US"/u,
  );
});

test("all protocol and oracle examples satisfy their versioned JSON Schemas", async () => {
  const pairs = [
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-ambiguous.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-clean.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-intentional-grouping.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-mobile.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-out-of-viewport.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-spacing-mutant.json"],
    ["adapters/playwright/schemas/capture-request.schema.json", "evaluation/web/requests/dashboard-browser-text-scale.json"],
    ["evaluation/web/browser-acquisition.schema.json", "evaluation/web/annotations/browser-acquisition.json"],
    ["evaluation/web/browser-rule.schema.json", "evaluation/web/annotations/browser-rules.json"],
  ] as const;

  for (const [schemaPath, documentPath] of pairs) {
    const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
    const validate = ajv.compile(await json(schemaPath) as AnySchema);
    assert.equal(validate(await json(documentPath)), true, `${documentPath}: ${ajv.errorsText(validate.errors)}`);
  }
});

test("protocol schemas reject extension fields", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const validate = ajv.compile(await json("adapters/playwright/schemas/capture-request.schema.json") as AnySchema);
  const request = await json("evaluation/web/requests/dashboard-browser-clean.json") as Record<string, unknown>;
  request["futureField"] = true;
  assert.equal(validate(request), false);
  assert.match(ajv.errorsText(validate.errors), /additional properties/u);
});
