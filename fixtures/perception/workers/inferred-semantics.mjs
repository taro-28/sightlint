import { canonicalJson } from "../../../adapters/perception/src/canonical.mjs";

const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const request = JSON.parse(Buffer.concat(chunks).toString("utf8"));
const source = (selector, sourceObservationIds) => ({
  class: "visionInferred",
  selector,
  sourceObservationIds,
  hypothesisId: `fixture-model/${selector}`,
  hypothesisConfirmed: false,
  semanticApplicability: "cantTell",
});
const notProvided = { status: "notProvided", reason: "The fixture model has no calibrated probability for this family." };
const bounds = (x) => ({ x, y: 8, width: 20, height: 16, coordinateSpaceId: request.input.canvas.id, unit: "devicePixel", format: "xywh-half-open" });

process.stdout.write(canonicalJson({
  $schema: "../schemas/response.schema.json",
  protocolVersion: "0.1.0",
  requestId: request.requestId,
  status: "partial",
  worker: {
    name: "fixture-inferred-worker",
    version: "0.1.0",
    runtime: { name: "node", version: process.versions.node },
    backend: request.worker.backend,
    model: request.worker.model,
  },
  inputSha256: request.input.sha256,
  familyStatus: [
    { family: "hierarchy", status: "observed", reason: "One synthetic relationship exercises protocol validation only." },
    { family: "peerGroup", status: "observed", reason: "One synthetic peer candidate exercises protocol validation only." },
    { family: "region", status: "observed", reason: "Two synthetic inferred regions exercise protocol validation only." },
    { family: "role", status: "observed", reason: "One synthetic role candidate exercises protocol validation only." },
    { family: "text", status: "observed", reason: "One synthetic OCR candidate exercises protocol validation only." },
  ],
  observations: [
    {
      id: "hierarchy:a", family: "hierarchy", status: "observed",
      value: { kind: "hierarchy", parentObservationId: "region:a", childObservationId: "text:a" },
      confidence: notProvided, alternatives: [], uncertaintyReasons: ["Synthetic conformance data has no semantic authority."],
      sourceEvidence: source("hierarchy/a", ["region:a", "text:a"]),
    },
    {
      id: "peer:a", family: "peerGroup", status: "observed",
      value: { kind: "peerGroup", memberObservationIds: ["region:a", "region:b"], axis: "horizontal" },
      confidence: notProvided, alternatives: [], uncertaintyReasons: ["Visual repetition alone does not establish semantic peers."],
      sourceEvidence: source("peer/a", ["region:a", "region:b"]),
    },
    {
      id: "region:a", family: "region", status: "observed",
      value: { kind: "pixelComponent", bounds: bounds(4), pixelCount: 280 },
      confidence: notProvided, alternatives: [], uncertaintyReasons: ["The region was proposed by a synthetic model fixture."],
      sourceEvidence: source("region/a", []),
    },
    {
      id: "region:b", family: "region", status: "observed",
      value: { kind: "pixelComponent", bounds: bounds(30), pixelCount: 280 },
      confidence: notProvided, alternatives: [], uncertaintyReasons: ["The region was proposed by a synthetic model fixture."],
      sourceEvidence: source("region/b", []),
    },
    {
      id: "role:a", family: "role", status: "observed",
      value: { kind: "role", targetObservationId: "region:a", role: "button" },
      confidence: { status: "calibratedProbability", value: 0.6, calibrationId: "fixture-calibration-v1" },
      alternatives: [{ value: "link", probability: 0.3 }], uncertaintyReasons: ["The top role candidates are materially ambiguous."],
      sourceEvidence: source("role/a", ["region:a"]),
    },
    {
      id: "text:a", family: "text", status: "observed",
      value: { kind: "text", text: "Synthetic label", bounds: bounds(4) },
      confidence: notProvided, alternatives: [{ value: "Synthetic labe1", probability: null }], uncertaintyReasons: ["No calibrated OCR probability is available."],
      sourceEvidence: source("text/a", ["region:a"]),
    },
  ],
  repeatedRunAgreement: { status: "notMeasured", reason: "One worker invocation cannot establish repeated-run agreement." },
  limitations: ["This fixture tests the protocol shape; its outputs are not acquisition or rule truth."],
}));
