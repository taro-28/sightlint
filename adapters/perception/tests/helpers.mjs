import { canonicalJson, sha256 } from "../src/canonical.mjs";

export function requestFor(report, options = {}) {
  const contentBytes = Buffer.from(canonicalJson(report));
  return {
    $schema: "../schemas/request.schema.json",
    protocolVersion: "0.1.0",
    requestId: options.requestId ?? "reference-observed",
    artifact: { id: options.artifactId ?? "perception-reference", kind: "web", title: "Perception reference input" },
    input: {
      reference: options.inputReference ?? "fixtures/perception/observed-segmentation-report.json",
      mediaType: "application/vnd.sightlint.image-segmentation-benchmark+json",
      sha256: sha256(contentBytes),
      byteLength: contentBytes.byteLength,
      content: report,
      canvas: { ...report.canvas },
    },
    preprocessing: {
      pipeline: "sightlint-image-segmentation-report",
      version: "0.1.0",
      policyId: options.policyId ?? "qualified-corner-95-row-runs-v1",
      crop: null,
      scale: { x: 1, y: 1 },
      tile: { status: "notApplied" },
      randomSeed: { status: "notApplicable" },
    },
    worker: {
      expectedName: options.expectedName ?? "sightlint-reference-region-worker",
      expectedVersion: options.expectedVersion ?? "0.1.0",
      backend: options.backend ?? "cpu",
      model: options.model ?? { status: "notApplicable" },
    },
    execution: {
      mode: "local",
      timeoutMs: options.timeoutMs ?? 2000,
      maxOutputBytes: options.maxOutputBytes ?? 1024 * 1024,
      maxStderrBytes: options.maxStderrBytes ?? 4096,
      maxObservations: 1024,
      maxTextLength: 4096,
      maxHierarchyDepth: 32,
    },
    privacy: {
      externalProcessing: false,
      remoteTransmittedFields: [],
      retention: "none",
      redaction: { status: "notApplied" },
    },
    output: {
      artifactIrReference: "perception/reference-artifact-ir.json",
      responseReference: "perception/reference-response.json",
    },
  };
}
