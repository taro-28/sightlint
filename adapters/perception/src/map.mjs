import { canonicalJson, sha256 } from "./canonical.mjs";

const WRAPPER_NAME = "sightlint-perception-wrapper";
const WRAPPER_VERSION = "0.1.0";
const EXTENSION_KEY = "org.sightlint.perception";

function modelName(model) {
  return model.status === "selected"
    ? `${model.name}@${model.version}#${model.sha256}`
    : undefined;
}

function evidenceSource(response, request) {
  const source = {
    adapter: WRAPPER_NAME,
    adapterVersion: WRAPPER_VERSION,
    inputDigest: request.input.sha256,
    externalProcessing: false,
  };
  const selectedModel = modelName(response.worker.model);
  if (selectedModel !== undefined) source.model = selectedModel;
  return source;
}

export function mapResponseToArtifactIr(request, response) {
  const responseBytes = Buffer.from(canonicalJson(response));
  const source = evidenceSource(response, request);
  const canvasEvidenceId = "e-perception:canvas";
  const evidence = [{
    id: canvasEvidenceId,
    class: "visionMeasured",
    source,
    selector: { type: "jsonPointer", pointer: "/canvas" },
  }];
  const mappedObservations = response.observations.filter((observation) =>
    observation.family === "region" && observation.sourceEvidence.class === "visionMeasured"
  );
  const nodes = mappedObservations.map((observation) => {
    const evidenceId = `e-perception:${observation.id}`;
    const bounds = observation.value.bounds;
    evidence.push({
      id: evidenceId,
      class: "visionMeasured",
      source,
      selector: {
        type: "region",
        coordinateSpaceId: bounds.coordinateSpaceId,
        rect: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
      },
    });
    return {
      id: `perception:${observation.id}`,
      kind: { value: "other", evidenceId },
      coordinateSpaceId: bounds.coordinateSpaceId,
      geometry: {
        renderBox: {
          rect: { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height },
          coordinateSpaceId: bounds.coordinateSpaceId,
          evidenceId,
        },
      },
    };
  });
  evidence.sort((left, right) => left.id.localeCompare(right.id));
  nodes.sort((left, right) => left.id.localeCompare(right.id));
  const extension = {
    version: "0.1.0",
    protocolVersion: response.protocolVersion,
    status: response.status,
    responseSha256: sha256(responseBytes),
    worker: response.worker,
    familyStatus: response.familyStatus,
    observationIds: response.observations.map((observation) => observation.id).sort(),
    mapping: {
      mappedRegionCount: nodes.length,
      unmappedObservationCount: response.observations.length - nodes.length,
      coreSemanticPromotionCount: 0,
      unmappedFamilies: ["hierarchy", "peerGroup", "role", "text"],
      reconciliationStatus: "sourcesRetainedSeparately",
    },
    limitations: [
      "Mapped nodes are unconfirmed pixel components, not semantic UI objects.",
      "Native and pixel evidence remain separate; protocol v0 performs no automatic reconciliation.",
      "No perception observation creates a rule result or blocking authority.",
    ],
  };
  return {
    schemaVersion: "0.1.0",
    artifact: {
      id: request.artifact.id,
      kind: "image",
      title: request.artifact.title,
      sourceName: request.input.reference,
    },
    canvases: [{
      id: request.input.canvas.id,
      size: { width: request.input.canvas.width, height: request.input.canvas.height },
      unit: "devicePixel",
      horizontalDirection: "right",
      verticalDirection: "down",
      evidenceId: canvasEvidenceId,
    }],
    nodes,
    evidence,
    extensions: { [EXTENSION_KEY]: extension },
  };
}
