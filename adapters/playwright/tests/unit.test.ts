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

async function managedRequest(): Promise<Record<string, unknown>> {
  return await json("evaluation/web/managed-requests/dashboard-managed-clean.json") as Record<string, unknown>;
}

test("managed request parser accepts the strict loopback contract and Japanese locale", async () => {
  const base = await managedRequest();
  const environment = base["environment"] as { locale: string };
  environment.locale = "ja-JP";
  const request = parseCaptureRequest(Buffer.from(JSON.stringify(base)));
  assert.equal(request.protocolVersion, "0.2.0");
  assert.equal(request.environment.locale, "ja-JP");
  if (request.protocolVersion !== "0.2.0") assert.fail("managed request expected");
  assert.equal(request.target.kind, "managedLoopbackHttp");
  assert.equal(request.server.port, 43173);
});

test("managed request parser rejects unsafe paths, ports, timeouts, and unknown fields", async () => {
  for (const pathAndQuery of ["relative", "//example.invalid/path", "/path#fragment", "/path\n", "/path?x=%0a"]) {
    const candidate = await managedRequest() as { target: { pathAndQuery: string } };
    candidate.target.pathAndQuery = pathAndQuery;
    assert.throws(
      () => parseCaptureRequest(Buffer.from(JSON.stringify(candidate))),
      /pathAndQuery/u,
      pathAndQuery,
    );
  }
  for (const port of [1023, 65536, 4173.5]) {
    const candidate = await managedRequest() as { server: { port: number } };
    candidate.server.port = port;
    assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(candidate))), /port must be an integer/u);
  }
  for (const timeout of [0, 180001, 1.5]) {
    const candidate = await managedRequest() as { server: { startupTimeoutMs: number } };
    candidate.server.startupTimeoutMs = timeout;
    assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(candidate))), /startupTimeoutMs/u);
  }
  const unknown = await managedRequest();
  Object.assign(unknown, { invented: true });
  assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(unknown))), /request fields must be exactly/u);
});

test("managed request parser enforces argv placeholder and resource limits", async () => {
  for (const argv of [["node", "server.mjs"], ["node", "{port}", "{port}"], ["node", "{port}{port}"]]) {
    const candidate = await managedRequest() as { server: { argv: string[] } };
    candidate.server.argv = argv;
    assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(candidate))), /\{port\} exactly once/u);
  }
  const tooMany = await managedRequest() as { server: { argv: string[] } };
  tooMany.server.argv = ["{port}", ...Array.from({ length: 64 }, () => "x")];
  assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(tooMany))), /1 through 64 elements/u);

  const tooLarge = await managedRequest() as { server: { argv: string[] } };
  tooLarge.server.argv = ["{port}", "x".repeat(8192)];
  assert.throws(() => parseCaptureRequest(Buffer.from(JSON.stringify(tooLarge))), /exceeds 8192 UTF-8 bytes/u);
});

test("all protocol and oracle examples satisfy their versioned JSON Schemas", async () => {
  const requestFiles = (await readdir(resolve(repositoryRoot, "evaluation/web/requests")))
    .filter((name) => name.endsWith(".json"))
    .sort();
  const pairs = [
    ...requestFiles.map((name) => ["adapters/playwright/schemas/capture-request.schema.json", `evaluation/web/requests/${name}`] as const),
    ...["dashboard-managed-clean.json", "dashboard-managed-unnamed-control.json", "dashboard-managed-intentional-overlay.json"]
      .map((name) => ["adapters/playwright/schemas/capture-request-0.2.schema.json", `evaluation/web/managed-requests/${name}`] as const),
    ["evaluation/web/browser-acquisition.schema.json", "evaluation/web/annotations/browser-acquisition.json"],
    ["evaluation/web/browser-rule.schema.json", "evaluation/web/annotations/browser-rules.json"],
    ["evaluation/web/agent-workflow.schema.json", "evaluation/web/annotations/agent-workflow.json"],
    ["evaluation/web/managed-loopback-acquisition.schema.json", "evaluation/web/annotations/managed-loopback-acquisition.json"],
    ["evaluation/web/managed-loopback-rule.schema.json", "evaluation/web/annotations/managed-loopback-rules.json"],
    ["evaluation/image-alpha/corpus.schema.json", "evaluation/image-alpha/corpus.json"],
    ["evaluation/image-alpha/annotation.schema.json", "evaluation/image-alpha/annotations/acquisition.json"],
    ["evaluation/image-alpha/annotation.schema.json", "evaluation/image-alpha/annotations/rules.json"],
    ["evaluation/png-format-demand/assessment.schema.json", "evaluation/png-format-demand/assessment.json"],
    ["evaluation/perception/corpus.schema.json", "evaluation/perception/corpus.json"],
    ["evaluation/perception/annotation.schema.json", "evaluation/perception/annotations/acquisition.json"],
    ["evaluation/perception/annotation.schema.json", "evaluation/perception/annotations/rules.json"],
    ["adapters/perception/schemas/response.schema.json", "fixtures/perception/inferred-response.json"],
    ["evaluation/pptx/corpus.schema.json", "evaluation/pptx/corpus.json"],
    ["evaluation/pptx/acquisition-annotation.schema.json", "evaluation/pptx/annotations/acquisition.json"],
    ["evaluation/pptx/rule-annotation.schema.json", "evaluation/pptx/annotations/rules.json"],
    ["evaluation/pptx/metric-contract.schema.json", "evaluation/pptx/metric-contract.json"],
    ["adapters/pptx/schemas/request.schema.json", "evaluation/pptx/requests/atlas-clean.json"],
    ["adapters/pptx/schemas/request.schema.json", "evaluation/pptx/requests/atlas-off-slide-mutant.json"],
    ["adapters/pptx/schemas/request.schema.json", "evaluation/pptx/requests/atlas-asymmetric-hard-negative.json"],
    ["evaluation/pdf/corpus.schema.json", "evaluation/pdf/corpus.json"],
    ["evaluation/pdf/acquisition-annotation.schema.json", "evaluation/pdf/annotations/acquisition.json"],
    ["evaluation/pdf/rule-annotation.schema.json", "evaluation/pdf/annotations/rules.json"],
    ["evaluation/pdf/metric-contract.schema.json", "evaluation/pdf/metric-contract.json"],
    ["adapters/pdf/dependency-lock.schema.json", "adapters/pdf/dependency-lock.json"],
    ["adapters/pdf/schemas/request.schema.json", "evaluation/pdf/requests/atlas-clean.json"],
    ["adapters/pdf/schemas/request.schema.json", "evaluation/pdf/requests/atlas-off-page-mutant.json"],
    ["adapters/pdf/schemas/request.schema.json", "evaluation/pdf/requests/atlas-quadpoints-hard-negative.json"],
    ["evaluation/android/corpus.schema.json", "evaluation/android/corpus.json"],
    ["evaluation/android/acquisition-annotation.schema.json", "evaluation/android/annotations/acquisition.json"],
    ["evaluation/android/rule-annotation.schema.json", "evaluation/android/annotations/rules.json"],
    ["evaluation/android/metric-contract.schema.json", "evaluation/android/metric-contract.json"],
    ["adapters/android/schemas/capture.schema.json", "evaluation/android/captures/clean.capture.json"],
    ["adapters/android/schemas/capture.schema.json", "evaluation/android/captures/off-canvas-control-mutant.capture.json"],
    ["adapters/android/schemas/capture.schema.json", "evaluation/android/captures/scroll-offscreen-hard-negative.capture.json"],
    ["adapters/android/schemas/request.schema.json", "evaluation/android/requests/android-atlas-clean.json"],
    ["adapters/android/schemas/request.schema.json", "evaluation/android/requests/android-atlas-off-canvas-control-mutant.json"],
    ["adapters/android/schemas/request.schema.json", "evaluation/android/requests/android-atlas-scroll-offscreen-hard-negative.json"],
    ["evaluation/ios/corpus.schema.json", "evaluation/ios/corpus.json"],
    ["evaluation/ios/acquisition-annotation.schema.json", "evaluation/ios/annotations/acquisition.json"],
    ["evaluation/ios/rule-annotation.schema.json", "evaluation/ios/annotations/rules.json"],
    ["evaluation/ios/metric-contract.schema.json", "evaluation/ios/metric-contract.json"],
    ["adapters/ios/schemas/capture.schema.json", "evaluation/ios/captures/clean.capture.json"],
    ["adapters/ios/schemas/capture.schema.json", "evaluation/ios/captures/off-canvas-control-mutant.capture.json"],
    ["adapters/ios/schemas/capture.schema.json", "evaluation/ios/captures/scroll-offscreen-hard-negative.capture.json"],
    ["adapters/ios/schemas/request.schema.json", "evaluation/ios/requests/ios-atlas-clean.json"],
    ["adapters/ios/schemas/request.schema.json", "evaluation/ios/requests/ios-atlas-off-canvas-control-mutant.json"],
    ["adapters/ios/schemas/request.schema.json", "evaluation/ios/requests/ios-atlas-scroll-offscreen-hard-negative.json"],
  ] as const;

  for (const [schemaPath, documentPath] of pairs) {
    const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
    const validate = ajv.compile(await json(schemaPath) as AnySchema);
    assert.equal(validate(await json(documentPath)), true, `${documentPath}: ${ajv.errorsText(validate.errors)}`);
  }

  for (const schemaPath of [
    "adapters/perception/schemas/request.schema.json",
    "adapters/perception/schemas/response.schema.json",
    "adapters/perception/schemas/run-report.schema.json",
    "adapters/perception/schemas/perception-extension.schema.json",
    "adapters/pptx/schemas/response.schema.json",
    "adapters/pptx/schemas/pptx-extension.schema.json",
    "adapters/pdf/schemas/response.schema.json",
    "adapters/pdf/schemas/pdf-extension.schema.json",
    "adapters/android/schemas/response.schema.json",
    "adapters/android/schemas/android-extension.schema.json",
    "adapters/ios/schemas/response.schema.json",
    "adapters/ios/schemas/ios-extension.schema.json",
  ]) {
    const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
    ajv.compile(await json(schemaPath) as AnySchema);
  }
});

test("PNG extension 0.2 schema is strict and compiles", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const validate = ajv.compile(await json("crates/sightlint-adapter-png/schemas/png-extension-0.2.0.schema.json") as AnySchema);
  const annotations = await json("evaluation/image-alpha/annotations/acquisition.json") as {
    cases: Array<{ alpha: Record<string, unknown> }>;
  };
  const alpha = structuredClone(annotations.cases[0]!.alpha);
  delete alpha["expectedInkBox"];
  Object.assign(alpha, {
    version: "0.1.0",
    status: "available",
    sourceAlphaEncoding: "unassociatedPngEncodedAlpha8",
    visiblePredicate: "alphaGreaterThanZero",
    opaquePredicate: "alphaEquals255",
    coordinateSpaceId: "canvas",
  });
  const extension = {
    version: "0.2.0", bitDepth: 8, colorType: 6, compressionMethod: 0, filterMethod: 0,
    interlaceMethod: 0, chunkCount: 3, idatChunkCount: 1, idatBytes: 1,
    hasPalette: false, inflatedScanlineBytes: 9264, reconstructedPackedSampleBytes: 9216,
    nonEmptyPassCount: 1,
    encodedRgba8Raster: {
      version: "0.1.0", encoding: "pngEncodedRgba8", colorManagementApplied: false,
      evidenceId: "evidence:png-raster", status: "available", width: 48, height: 48,
      byteCount: 9216, byteCrc32: "35ac49cb",
    },
    alphaGeometry: alpha,
  };
  assert.equal(validate(extension), true, ajv.errorsText(validate.errors));
  Object.assign(extension, { unexpected: true });
  assert.equal(validate(extension), false);
  assert.match(ajv.errorsText(validate.errors), /additional properties/u);
});

test("workflow report schema compiles with the capture response compatibility surface", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  ajv.addSchema(await json("adapters/playwright/schemas/capture-response.schema.json") as AnySchema);
  const validate = ajv.compile(await json("adapters/playwright/schemas/web-workflow-report.schema.json") as AnySchema);
  assert.equal(typeof validate, "function");

  const managedAjv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  managedAjv.addSchema(await json("adapters/playwright/schemas/capture-response-0.2.schema.json") as AnySchema);
  const managedValidate = managedAjv.compile(await json("adapters/playwright/schemas/web-workflow-report-0.2.schema.json") as AnySchema);
  assert.equal(typeof managedValidate, "function");
});

test("previous strict schemas remain available and reject current documents", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const previousAcquisition = ajv.compile(await json("evaluation/web/browser-acquisition-0.1.schema.json") as AnySchema);
  assert.equal(previousAcquisition(await json("evaluation/web/annotations/browser-acquisition.json")), false);
  assert.match(ajv.errorsText(previousAcquisition.errors), /schemaVersion|must be equal to constant/u);

  const previousExtension = await json("adapters/playwright/schemas/web-extension-0.1.schema.json") as Record<string, unknown>;
  assert.equal(previousExtension["$id"], "urn:sightlint:schema:web-extension:0.1.0");
  assert.deepEqual((previousExtension["properties"] as Record<string, unknown>)["extensionVersion"], { const: "0.1.0" });

  const retainedCurrentExtension = await json("adapters/playwright/schemas/web-extension-0.3.schema.json") as Record<string, unknown>;
  assert.equal(retainedCurrentExtension["$id"], "urn:sightlint:schema:web-extension:0.3.0");
});

test("protocol schemas reject extension fields", async () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const validate = ajv.compile(await json("adapters/playwright/schemas/capture-request.schema.json") as AnySchema);
  const request = await json("evaluation/web/requests/dashboard-browser-clean.json") as Record<string, unknown>;
  request["futureField"] = true;
  assert.equal(validate(request), false);
  assert.match(ajv.errorsText(validate.errors), /additional properties/u);

  const managedValidate = ajv.compile(await json("adapters/playwright/schemas/capture-request-0.2.schema.json") as AnySchema);
  const managed = await managedRequest() as { server: { argv: string[] } };
  managed.server.argv = ["node", "server.mjs", "--port={port}{port}"];
  assert.equal(managedValidate(managed), false);
  assert.match(ajv.errorsText(managedValidate.errors), /must match pattern/u);
});
