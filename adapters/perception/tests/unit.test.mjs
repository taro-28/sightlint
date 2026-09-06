import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { canonicalJson, compareUtf16, sha256 } from "../src/canonical.mjs";
import { mapResponseToArtifactIr } from "../src/map.mjs";
import { parseRequest, parseWorkerResponse } from "../src/validate.mjs";
import { requestFor } from "./helpers.mjs";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const sourceDigest = `sha256:${"a".repeat(64)}`;

async function report(name = "observed-segmentation-report.json") {
  return JSON.parse(await readFile(resolve(repositoryRoot, `fixtures/perception/${name}`), "utf8"));
}

function rawResponse(request) {
  return {
    $schema: "../schemas/response.schema.json",
    protocolVersion: "0.1.0",
    requestId: request.requestId,
    status: "partial",
    worker: {
      name: "sightlint-reference-region-worker",
      version: "0.1.0",
      runtime: { name: "node", version: process.versions.node },
      backend: "cpu",
      model: { status: "notApplicable" },
    },
    inputSha256: request.input.sha256,
    familyStatus: [
      { family: "hierarchy", status: "untested", reason: "no hierarchy" },
      { family: "peerGroup", status: "untested", reason: "no peers" },
      { family: "region", status: "observed", reason: "measured" },
      { family: "role", status: "untested", reason: "no roles" },
      { family: "text", status: "unsupported", reason: "no OCR" },
    ],
    observations: [{
      id: "region:4:8:20:16:280",
      family: "region",
      status: "observed",
      value: { kind: "pixelComponent", bounds: { x: 4, y: 8, width: 20, height: 16, coordinateSpaceId: "canvas", unit: "devicePixel", format: "xywh-half-open" }, pixelCount: 280 },
      confidence: { status: "notApplicable", reason: "deterministic measurement" },
      alternatives: [],
      uncertaintyReasons: ["background is unconfirmed"],
      sourceEvidence: { class: "visionMeasured", selector: "/policies/0/regions/1", sourceObservationIds: [], hypothesisId: "qualified/candidate", hypothesisConfirmed: false, semanticApplicability: "cantTell" },
    }],
    repeatedRunAgreement: { status: "notMeasured", reason: "one invocation" },
    limitations: ["no semantic inference"],
  };
}

test("canonical JSON is stable and SHA-256 covers exact bytes", () => {
  const first = canonicalJson({ z: -0, a: [{ y: 2, x: 1 }] });
  const second = canonicalJson({ a: [{ x: 1, y: 2 }], z: 0 });
  assert.equal(first, second);
  assert.match(sha256(Buffer.from(first)), /^sha256:[0-9a-f]{64}$/);
  assert.deepEqual(["region:2", "region:10", "region:_"].sort(compareUtf16), ["region:10", "region:2", "region:_"]);
});

test("request validation binds canonical content, local privacy, preprocessing, and budgets", async () => {
  const request = requestFor(await report());
  assert.deepEqual(parseRequest(Buffer.from(canonicalJson(request))), request);
  const changed = structuredClone(request);
  changed.input.sha256 = `sha256:${"0".repeat(64)}`;
  assert.throws(() => parseRequest(Buffer.from(canonicalJson(changed))), /does not match canonical input content/);
  const remote = structuredClone(request);
  remote.privacy.externalProcessing = true;
  assert.throws(() => parseRequest(Buffer.from(canonicalJson(remote))), /must be false/);
});

test("response validation adds source provenance and rejects identity or geometry drift", async () => {
  const request = requestFor(await report());
  const response = parseWorkerResponse(Buffer.from(canonicalJson(rawResponse(request))), request, sourceDigest);
  assert.equal(response.worker.sourceSha256, sourceDigest);
  const wrong = rawResponse(request);
  wrong.worker.name = "wrong-worker";
  assert.throws(() => parseWorkerResponse(Buffer.from(canonicalJson(wrong)), request, sourceDigest), /identity does not match/);
  const outside = rawResponse(request);
  outside.observations[0].value.bounds.x = 60;
  assert.throws(() => parseWorkerResponse(Buffer.from(canonicalJson(outside)), request, sourceDigest), /must stay within/);
});

test("mapping promotes only measured regions and no semantic roles or relations", async () => {
  const request = requestFor(await report());
  const response = parseWorkerResponse(Buffer.from(canonicalJson(rawResponse(request))), request, sourceDigest);
  const artifact = mapResponseToArtifactIr(request, response);
  assert.equal(artifact.nodes.length, 1);
  assert.equal(artifact.nodes[0].kind.value, "other");
  assert.equal(artifact.nodes[0].role, undefined);
  assert.equal(artifact.nodes[0].parentId, undefined);
  assert.equal(artifact.relations, undefined);
  assert.equal(artifact.evidence[1].class, "visionMeasured");
  assert.equal(artifact.extensions["org.sightlint.perception"].mapping.coreSemanticPromotionCount, 0);
  assert.equal(artifact.extensions["org.sightlint.perception"].mapping.unmappedObservationCount, 0);
  assert.deepEqual(artifact.extensions["org.sightlint.perception"].mapping.unmappedFamilies, ["hierarchy", "peerGroup", "role", "text"]);
});

test("inferred semantic families remain outside core IR", async () => {
  const request = requestFor(await report(), {
    expectedName: "fixture-inferred-worker",
    model: { status: "selected", name: "fixture-model", version: "1", sha256: `sha256:${"b".repeat(64)}` },
  });
  const response = rawResponse(request);
  response.worker.name = "fixture-inferred-worker";
  response.worker.model = request.worker.model;
  response.observations[0].confidence = { status: "notProvided", reason: "fixture model has no calibrated probability" };
  response.observations[0].sourceEvidence.class = "visionInferred";
  const parsed = parseWorkerResponse(Buffer.from(canonicalJson(response)), request, sourceDigest);
  const artifact = mapResponseToArtifactIr(request, parsed);
  assert.deepEqual(artifact.nodes, []);
  assert.equal(artifact.extensions["org.sightlint.perception"].mapping.unmappedObservationCount, 1);
  assert.deepEqual(artifact.extensions["org.sightlint.perception"].observationIds, ["region:4:8:20:16:280"]);
});
