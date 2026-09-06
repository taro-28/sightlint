#!/usr/bin/env node

import { canonicalJson } from "./canonical.mjs";
import { PerceptionError } from "./errors.mjs";
import { MAX_REQUEST_BYTES, parseRequest } from "./validate.mjs";

const WORKER_NAME = "sightlint-reference-region-worker";
const WORKER_VERSION = "0.1.0";

async function readStdin() {
  const chunks = [];
  let length = 0;
  for await (const chunk of process.stdin) {
    length += chunk.byteLength;
    if (length > MAX_REQUEST_BYTES) throw new PerceptionError("request-budget", "worker request exceeds the byte budget");
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

function stableRegionId(region) {
  const [x, y, width, height] = region.bounds;
  return `region:${x}:${y}:${width}:${height}:${region.pixelCount}`;
}

function familyStatus(regionStatus, regionReason) {
  return [
    { family: "hierarchy", status: "untested", reason: "Exact-color components do not establish parent-child semantics." },
    { family: "peerGroup", status: "untested", reason: "Visual similarity and alignment do not establish semantic peers." },
    { family: "region", status: regionStatus, reason: regionReason },
    { family: "role", status: "untested", reason: "The reference worker has no semantic role classifier." },
    { family: "text", status: "unsupported", reason: "The reference worker performs no OCR." },
  ];
}

function responseFor(request) {
  const report = request.input.content;
  if (report.benchmarkSchemaVersion !== "0.1.0" || report.blocking !== false || report.ruleOutcome !== "untested") {
    throw new PerceptionError("input-contract", "input must be a nonblocking segmentation benchmark report 0.1.0");
  }
  if (!Array.isArray(report.policies)) throw new PerceptionError("input-contract", "input policies must be an array");
  const policyId = request.preprocessing.policyId;
  const policyIndex = report.policies.findIndex((item) => item !== null && typeof item === "object" && item.policyId === policyId);
  if (policyIndex < 0) throw new PerceptionError("input-contract", `input must include ${policyId}`);
  const policy = report.policies[policyIndex];
  if (!Array.isArray(policy.regions)) throw new PerceptionError("input-contract", "selected policy regions must be an array");

  const observed = policy.status === "observed";
  if (!observed && policy.status !== "unavailable") throw new PerceptionError("input-contract", "selected policy status is invalid");
  const observations = observed
    ? policy.regions.map((region, index) => {
      if (!Array.isArray(region.bounds) || region.bounds.length !== 4 || !region.bounds.every(Number.isSafeInteger) || !Number.isSafeInteger(region.pixelCount)) {
        throw new PerceptionError("input-contract", `selected policy region ${index} is malformed`);
      }
      const [x, y, width, height] = region.bounds;
      return {
        id: stableRegionId(region),
        family: "region",
        status: "observed",
        value: {
          kind: "pixelComponent",
          bounds: { x, y, width, height, coordinateSpaceId: request.input.canvas.id, unit: "devicePixel", format: "xywh-half-open" },
          pixelCount: region.pixelCount,
        },
        confidence: { status: "notApplicable", reason: "Deterministic pixel measurement has no semantic probability." },
        alternatives: [],
        uncertaintyReasons: [
          "The selected exact-color background is an unconfirmed acquisition hypothesis.",
          "A connected pixel component is not necessarily one semantic object.",
        ],
        sourceEvidence: {
          class: "visionMeasured",
          selector: `/policies/${policyIndex}/regions/${index}`,
          sourceObservationIds: [],
          hypothesisId: `${policyId}/${region.hypothesisId}`,
          hypothesisConfirmed: false,
          semanticApplicability: "cantTell",
        },
      };
    }).sort((left, right) => left.id.localeCompare(right.id))
    : [];
  const regionStatus = observed ? "observed" : "unsupported";
  const reason = observed
    ? `The selected ${policyId} exact-color pixel components were measured under an unconfirmed background hypothesis.`
    : `Selected pixel acquisition is unavailable: ${String(policy.reason)}`;
  return {
    $schema: "../schemas/response.schema.json",
    protocolVersion: "0.1.0",
    requestId: request.requestId,
    status: observed ? "partial" : "unsupported",
    worker: {
      name: WORKER_NAME,
      version: WORKER_VERSION,
      runtime: { name: "node", version: process.versions.node },
      backend: request.worker.backend,
      model: { status: "notApplicable" },
    },
    inputSha256: request.input.sha256,
    familyStatus: familyStatus(regionStatus, reason),
    observations,
    repeatedRunAgreement: { status: "notMeasured", reason: "One worker invocation cannot establish repeated-run agreement." },
    limitations: [
      "The worker exposes an existing deterministic CV baseline and performs no OCR or learned inference.",
      "Background candidates, regions, and object boundaries remain unconfirmed hypotheses.",
      "No observation establishes semantic role, hierarchy, peer membership, applicability, or a rule verdict.",
    ],
  };
}

try {
  const request = parseRequest(await readStdin());
  process.stdout.write(canonicalJson(responseFor(request)));
} catch (error) {
  const known = error instanceof PerceptionError ? error : new PerceptionError("execution-error", "reference worker failed");
  process.stderr.write(`sightlint-reference-region-worker: ${known.code}: ${known.message}\n`);
  process.exitCode = 2;
}
