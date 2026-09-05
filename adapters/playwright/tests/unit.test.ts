import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type AnySchema } from "ajv/dist/2020.js";

import { canonicalJson, sha256 } from "../src/canonical.js";
import { parseAccessibilitySnapshot } from "../src/capture.js";
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

test("accessibility snapshot parsing is linear for escaped and malformed names", () => {
  const observed = parseAccessibilitySnapshot('- button "Save \\"draft\\"" [disabled focused]', true);
  assert.equal(observed.status, "observed");
  assert.equal(observed.role, "button");
  assert.equal(observed.name, 'Save "draft"');
  assert.deepEqual(observed.states, ["disabled", "focused"]);

  const adversarial = `- A "${"\\!".repeat(4_096)}`;
  const rejected = parseAccessibilitySnapshot(adversarial, true);
  assert.equal(rejected.status, "cantTell");
  assert.equal(rejected.role, null);
  assert.equal(rejected.name, null);
  assert.equal(rejected.rootLine, adversarial);
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
  const requestFiles = (await readdir(resolve(repositoryRoot, "evaluation/web/requests")))
    .filter((name) => name.endsWith(".json"))
    .sort();
  const pairs = [
    ...requestFiles.map((name) => ["adapters/playwright/schemas/capture-request.schema.json", `evaluation/web/requests/${name}`] as const),
    ["evaluation/web/browser-acquisition.schema.json", "evaluation/web/annotations/browser-acquisition.json"],
    ["evaluation/web/browser-rule.schema.json", "evaluation/web/annotations/browser-rules.json"],
    ["evaluation/web/agent-workflow.schema.json", "evaluation/web/annotations/agent-workflow.json"],
  ] as const;

  for (const [schemaPath, documentPath] of pairs) {
    const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
    const validate = ajv.compile(await json(schemaPath) as AnySchema);
    assert.equal(validate(await json(documentPath)), true, `${documentPath}: ${ajv.errorsText(validate.errors)}`);
  }
});

test("workflow report schema compiles with the capture response compatibility surface", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  ajv.addSchema(await json("adapters/playwright/schemas/capture-response.schema.json") as AnySchema);
  const validate = ajv.compile(await json("adapters/playwright/schemas/web-workflow-report.schema.json") as AnySchema);
  assert.equal(typeof validate, "function");
});

test("previous strict schemas remain available and reject current documents", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const previousAcquisition = ajv.compile(await json("evaluation/web/browser-acquisition-0.1.schema.json") as AnySchema);
  assert.equal(previousAcquisition(await json("evaluation/web/annotations/browser-acquisition.json")), false);
  assert.match(ajv.errorsText(previousAcquisition.errors), /schemaVersion|must be equal to constant/u);

  const previousExtension = await json("adapters/playwright/schemas/web-extension-0.1.schema.json") as Record<string, unknown>;
  assert.equal(previousExtension["$id"], "urn:sightlint:schema:web-extension:0.1.0");
  assert.deepEqual((previousExtension["properties"] as Record<string, unknown>)["extensionVersion"], { const: "0.1.0" });
});

test("protocol schemas reject extension fields", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const validate = ajv.compile(await json("adapters/playwright/schemas/capture-request.schema.json") as AnySchema);
  const request = await json("evaluation/web/requests/dashboard-browser-clean.json") as Record<string, unknown>;
  request["futureField"] = true;
  assert.equal(validate(request), false);
  assert.match(ajv.errorsText(validate.errors), /additional properties/u);
});
