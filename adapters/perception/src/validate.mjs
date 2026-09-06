import { canonicalJson, sha256 } from "./canonical.mjs";
import { PerceptionError } from "./errors.mjs";

export const PROTOCOL_VERSION = "0.1.0";
export const MAX_REQUEST_BYTES = 20 * 1024 * 1024;
const FAMILIES = ["hierarchy", "peerGroup", "region", "role", "text"];
const COVERAGE_STATUSES = new Set(["observed", "partial", "unsupported", "ambiguous", "untested"]);
const RESPONSE_STATUSES = new Set(["complete", "partial", "unsupported", "ambiguous"]);
const TOKEN = /^[a-z0-9][a-z0-9._:-]{0,127}$/;
const VERSION = /^[0-9]+\.[0-9]+\.[0-9]+$/;
const SHA256 = /^sha256:[0-9a-f]{64}$/;

function fail(path, message) {
  throw new PerceptionError("protocol-invalid", `${path} ${message}`);
}

function record(value, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(path, "must be an object");
  }
  return value;
}

function exactKeys(value, keys, path) {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    fail(path, `must contain exactly ${expected.join(", ")}`);
  }
}

function string(value, path, maximum = 4096) {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum) {
    fail(path, `must be a non-empty string of at most ${maximum} characters`);
  }
  return value;
}

function token(value, path) {
  const parsed = string(value, path, 128);
  if (!TOKEN.test(parsed)) fail(path, "must be a stable token");
  return parsed;
}

function version(value, path) {
  const parsed = string(value, path, 64);
  if (!VERSION.test(parsed)) fail(path, "must be a semantic version");
  return parsed;
}

function digest(value, path) {
  const parsed = string(value, path, 71);
  if (!SHA256.test(parsed)) fail(path, "must be a lowercase sha256 digest");
  return parsed;
}

function integer(value, path, minimum, maximum) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    fail(path, `must be an integer from ${minimum} through ${maximum}`);
  }
  return value;
}

function array(value, path, minimum, maximum) {
  if (!Array.isArray(value) || value.length < minimum || value.length > maximum) {
    fail(path, `must contain ${minimum} through ${maximum} items`);
  }
  return value;
}

function requireSorted(values, path, key = (value) => value) {
  if (values.some((value, index) => index > 0 && key(value) < key(values[index - 1]))) {
    fail(path, "must use canonical ascending order");
  }
}

function model(value, path) {
  const parsed = record(value, path);
  if (parsed.status === "notApplicable") {
    exactKeys(parsed, ["status"], path);
    return parsed;
  }
  exactKeys(parsed, ["status", "name", "version", "sha256"], path);
  if (parsed.status !== "selected") fail(`${path}.status`, "must be selected or notApplicable");
  token(parsed.name, `${path}.name`);
  string(parsed.version, `${path}.version`);
  digest(parsed.sha256, `${path}.sha256`);
  return parsed;
}

function notStatus(value, path, expected) {
  const parsed = record(value, path);
  exactKeys(parsed, ["status"], path);
  if (parsed.status !== expected) fail(`${path}.status`, `must be ${expected}`);
}

function validateCanvas(value, path) {
  const parsed = record(value, path);
  exactKeys(parsed, ["id", "width", "height", "unit", "origin"], path);
  token(parsed.id, `${path}.id`);
  integer(parsed.width, `${path}.width`, 1, 100000);
  integer(parsed.height, `${path}.height`, 1, 100000);
  if (parsed.unit !== "devicePixel") fail(`${path}.unit`, "must be devicePixel");
  if (parsed.origin !== "topLeft") fail(`${path}.origin`, "must be topLeft");
  return parsed;
}

export function parseRequest(bytes) {
  if (!Buffer.isBuffer(bytes) || bytes.byteLength < 2 || bytes.byteLength > MAX_REQUEST_BYTES) {
    throw new PerceptionError("request-budget", `request must contain 2 through ${MAX_REQUEST_BYTES} bytes`);
  }
  let root;
  try {
    root = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new PerceptionError("request-json", "request must be valid UTF-8 JSON");
  }
  const request = record(root, "request");
  exactKeys(request, ["$schema", "protocolVersion", "requestId", "artifact", "input", "preprocessing", "worker", "execution", "privacy", "output"], "request");
  if (request.$schema !== "../schemas/request.schema.json") fail("request.$schema", "must identify request.schema.json");
  if (request.protocolVersion !== PROTOCOL_VERSION) fail("request.protocolVersion", `must be ${PROTOCOL_VERSION}`);
  token(request.requestId, "request.requestId");

  const artifact = record(request.artifact, "request.artifact");
  exactKeys(artifact, ["id", "kind", "title"], "request.artifact");
  token(artifact.id, "request.artifact.id");
  if (!["web", "mobile", "slide", "document", "pdf", "image", "other"].includes(artifact.kind)) fail("request.artifact.kind", "is unsupported");
  string(artifact.title, "request.artifact.title");

  const input = record(request.input, "request.input");
  exactKeys(input, ["reference", "mediaType", "sha256", "byteLength", "content", "canvas"], "request.input");
  string(input.reference, "request.input.reference");
  if (input.mediaType !== "application/vnd.sightlint.image-segmentation-benchmark+json") fail("request.input.mediaType", "is unsupported by protocol v0");
  digest(input.sha256, "request.input.sha256");
  integer(input.byteLength, "request.input.byteLength", 2, 16 * 1024 * 1024);
  record(input.content, "request.input.content");
  const contentBytes = Buffer.from(canonicalJson(input.content));
  if (contentBytes.byteLength !== input.byteLength) fail("request.input.byteLength", "does not match canonical input content");
  if (sha256(contentBytes) !== input.sha256) fail("request.input.sha256", "does not match canonical input content");
  const canvas = validateCanvas(input.canvas, "request.input.canvas");

  const preprocessing = record(request.preprocessing, "request.preprocessing");
  exactKeys(preprocessing, ["pipeline", "version", "policyId", "crop", "scale", "tile", "randomSeed"], "request.preprocessing");
  if (preprocessing.pipeline !== "sightlint-image-segmentation-report" || preprocessing.version !== "0.1.0") fail("request.preprocessing", "uses an unsupported pipeline");
  if (!["qualified-corner-95-row-runs-v1", "ranked-exact-border-flood-v1", "strict-uniform-perimeter-flood-v1"].includes(preprocessing.policyId)) fail("request.preprocessing.policyId", "is unsupported");
  if (preprocessing.crop !== null) fail("request.preprocessing.crop", "must be null");
  const scale = record(preprocessing.scale, "request.preprocessing.scale");
  exactKeys(scale, ["x", "y"], "request.preprocessing.scale");
  if (scale.x !== 1 || scale.y !== 1) fail("request.preprocessing.scale", "must preserve scale in v0");
  notStatus(preprocessing.tile, "request.preprocessing.tile", "notApplied");
  notStatus(preprocessing.randomSeed, "request.preprocessing.randomSeed", "notApplicable");

  const worker = record(request.worker, "request.worker");
  exactKeys(worker, ["expectedName", "expectedVersion", "backend", "model"], "request.worker");
  token(worker.expectedName, "request.worker.expectedName");
  version(worker.expectedVersion, "request.worker.expectedVersion");
  token(worker.backend, "request.worker.backend");
  model(worker.model, "request.worker.model");

  const execution = record(request.execution, "request.execution");
  exactKeys(execution, ["mode", "timeoutMs", "maxOutputBytes", "maxStderrBytes", "maxObservations", "maxTextLength", "maxHierarchyDepth"], "request.execution");
  if (execution.mode !== "local") fail("request.execution.mode", "must be local in protocol v0");
  integer(execution.timeoutMs, "request.execution.timeoutMs", 50, 30000);
  integer(execution.maxOutputBytes, "request.execution.maxOutputBytes", 1024, 4 * 1024 * 1024);
  integer(execution.maxStderrBytes, "request.execution.maxStderrBytes", 0, 65536);
  integer(execution.maxObservations, "request.execution.maxObservations", 1, 1024);
  integer(execution.maxTextLength, "request.execution.maxTextLength", 1, 4096);
  integer(execution.maxHierarchyDepth, "request.execution.maxHierarchyDepth", 1, 128);

  const privacy = record(request.privacy, "request.privacy");
  exactKeys(privacy, ["externalProcessing", "remoteTransmittedFields", "retention", "redaction"], "request.privacy");
  if (privacy.externalProcessing !== false) fail("request.privacy.externalProcessing", "must be false in protocol v0");
  array(privacy.remoteTransmittedFields, "request.privacy.remoteTransmittedFields", 0, 0);
  if (privacy.retention !== "none") fail("request.privacy.retention", "must be none");
  notStatus(privacy.redaction, "request.privacy.redaction", "notApplied");

  const output = record(request.output, "request.output");
  exactKeys(output, ["artifactIrReference", "responseReference"], "request.output");
  string(output.artifactIrReference, "request.output.artifactIrReference");
  string(output.responseReference, "request.output.responseReference");

  const reportCanvas = record(input.content.canvas, "request.input.content.canvas");
  if (reportCanvas.id !== canvas.id || reportCanvas.width !== canvas.width || reportCanvas.height !== canvas.height || reportCanvas.unit !== canvas.unit) {
    fail("request.input.canvas", "does not match the embedded report canvas");
  }
  return request;
}

function validateConfidence(value, path) {
  const confidence = record(value, path);
  if (confidence.status === "calibratedProbability") {
    exactKeys(confidence, ["status", "value", "calibrationId"], path);
    if (typeof confidence.value !== "number" || !Number.isFinite(confidence.value) || confidence.value < 0 || confidence.value > 1) fail(`${path}.value`, "must be a finite probability");
    token(confidence.calibrationId, `${path}.calibrationId`);
  } else {
    exactKeys(confidence, ["status", "reason"], path);
    if (!["notProvided", "notApplicable"].includes(confidence.status)) fail(`${path}.status`, "is invalid");
    string(confidence.reason, `${path}.reason`);
  }
  return confidence;
}

function validateFamilyStatuses(value) {
  const statuses = array(value, "response.familyStatus", 5, 5);
  const names = statuses.map((item, index) => {
    const status = record(item, `response.familyStatus[${index}]`);
    exactKeys(status, ["family", "status", "reason"], `response.familyStatus[${index}]`);
    if (!FAMILIES.includes(status.family)) fail(`response.familyStatus[${index}].family`, "is invalid");
    if (!COVERAGE_STATUSES.has(status.status)) fail(`response.familyStatus[${index}].status`, "is invalid");
    string(status.reason, `response.familyStatus[${index}].reason`);
    return status.family;
  });
  if (names.some((name, index) => name !== FAMILIES[index])) fail("response.familyStatus", "must contain each family in canonical order");
  return statuses;
}

function validateBounds(value, path, request) {
  const bounds = record(value, path);
  exactKeys(bounds, ["x", "y", "width", "height", "coordinateSpaceId", "unit", "format"], path);
  integer(bounds.x, `${path}.x`, 0, request.input.canvas.width - 1);
  integer(bounds.y, `${path}.y`, 0, request.input.canvas.height - 1);
  integer(bounds.width, `${path}.width`, 1, request.input.canvas.width);
  integer(bounds.height, `${path}.height`, 1, request.input.canvas.height);
  if (bounds.x + bounds.width > request.input.canvas.width || bounds.y + bounds.height > request.input.canvas.height) fail(path, "must stay within the declared canvas");
  if (bounds.coordinateSpaceId !== request.input.canvas.id || bounds.unit !== "devicePixel" || bounds.format !== "xywh-half-open") fail(path, "uses a mismatched coordinate contract");
  return bounds;
}

function validateObservationValue(observation, path, request) {
  const value = record(observation.value, `${path}.value`);
  if (observation.family === "region") {
    exactKeys(value, ["kind", "bounds", "pixelCount"], `${path}.value`);
    if (value.kind !== "pixelComponent") fail(`${path}.value.kind`, "must be pixelComponent");
    validateBounds(value.bounds, `${path}.value.bounds`, request);
    integer(value.pixelCount, `${path}.value.pixelCount`, 1, request.input.canvas.width * request.input.canvas.height);
  } else if (observation.family === "text") {
    exactKeys(value, ["kind", "text", "bounds"], `${path}.value`);
    if (value.kind !== "text") fail(`${path}.value.kind`, "must be text");
    string(value.text, `${path}.value.text`, request.execution.maxTextLength);
    validateBounds(value.bounds, `${path}.value.bounds`, request);
  } else if (observation.family === "role") {
    exactKeys(value, ["kind", "targetObservationId", "role"], `${path}.value`);
    if (value.kind !== "role") fail(`${path}.value.kind`, "must be role");
    token(value.targetObservationId, `${path}.value.targetObservationId`);
    token(value.role, `${path}.value.role`);
  } else if (observation.family === "hierarchy") {
    exactKeys(value, ["kind", "parentObservationId", "childObservationId"], `${path}.value`);
    if (value.kind !== "hierarchy") fail(`${path}.value.kind`, "must be hierarchy");
    token(value.parentObservationId, `${path}.value.parentObservationId`);
    token(value.childObservationId, `${path}.value.childObservationId`);
    if (value.parentObservationId === value.childObservationId) fail(`${path}.value`, "must link distinct observations");
  } else if (observation.family === "peerGroup") {
    exactKeys(value, ["kind", "memberObservationIds", "axis"], `${path}.value`);
    if (value.kind !== "peerGroup") fail(`${path}.value.kind`, "must be peerGroup");
    const members = array(value.memberObservationIds, `${path}.value.memberObservationIds`, 2, 1024);
    members.forEach((member, memberIndex) => token(member, `${path}.value.memberObservationIds[${memberIndex}]`));
    if (new Set(members).size !== members.length) fail(`${path}.value.memberObservationIds`, "must contain unique IDs");
    requireSorted(members, `${path}.value.memberObservationIds`);
    if (!["horizontal", "vertical", "unordered", "unknown"].includes(value.axis)) fail(`${path}.value.axis`, "is invalid");
  }
}

function validateObservation(value, index, request) {
  const path = `response.observations[${index}]`;
  const observation = record(value, path);
  exactKeys(observation, ["id", "family", "status", "value", "confidence", "alternatives", "uncertaintyReasons", "sourceEvidence"], path);
  token(observation.id, `${path}.id`);
  if (!FAMILIES.includes(observation.family) || observation.status !== "observed") fail(path, "uses an invalid family or status");
  validateObservationValue(observation, path, request);
  const confidence = validateConfidence(observation.confidence, `${path}.confidence`);
  const alternatives = array(observation.alternatives, `${path}.alternatives`, 0, 16);
  for (const [alternativeIndex, item] of alternatives.entries()) {
    const alternative = record(item, `${path}.alternatives[${alternativeIndex}]`);
    exactKeys(alternative, ["value", "probability"], `${path}.alternatives[${alternativeIndex}]`);
    string(alternative.value, `${path}.alternatives[${alternativeIndex}].value`, request.execution.maxTextLength);
    if (alternative.probability !== null && (typeof alternative.probability !== "number" || !Number.isFinite(alternative.probability) || alternative.probability < 0 || alternative.probability > 1)) fail(`${path}.alternatives[${alternativeIndex}].probability`, "must be null or a finite probability");
  }
  requireSorted(alternatives, `${path}.alternatives`, (alternative) => canonicalJson(alternative));
  array(observation.uncertaintyReasons, `${path}.uncertaintyReasons`, 1, 16).forEach((reason, reasonIndex) => string(reason, `${path}.uncertaintyReasons[${reasonIndex}]`));
  const source = record(observation.sourceEvidence, `${path}.sourceEvidence`);
  exactKeys(source, ["class", "selector", "sourceObservationIds", "hypothesisId", "hypothesisConfirmed", "semanticApplicability"], `${path}.sourceEvidence`);
  if (!["visionMeasured", "visionInferred"].includes(source.class) || source.hypothesisConfirmed !== false || source.semanticApplicability !== "cantTell") fail(`${path}.sourceEvidence`, "must retain an unconfirmed, cantTell perception class");
  string(source.selector, `${path}.sourceEvidence.selector`);
  const sourceIds = array(source.sourceObservationIds, `${path}.sourceEvidence.sourceObservationIds`, 0, 1024);
  sourceIds.forEach((sourceId, sourceIndex) => token(sourceId, `${path}.sourceEvidence.sourceObservationIds[${sourceIndex}]`));
  if (new Set(sourceIds).size !== sourceIds.length) fail(`${path}.sourceEvidence.sourceObservationIds`, "must contain unique IDs");
  requireSorted(sourceIds, `${path}.sourceEvidence.sourceObservationIds`);
  string(source.hypothesisId, `${path}.sourceEvidence.hypothesisId`);
  if (observation.family !== "region" && source.class !== "visionInferred") fail(`${path}.sourceEvidence.class`, "semantic families must remain visionInferred");
  if (source.class === "visionMeasured") {
    if (observation.family !== "region" || confidence.status !== "notApplicable" || request.worker.model.status !== "notApplicable") {
      fail(path, "visionMeasured is limited to deterministic model-free region measurements");
    }
  } else if (confidence.status === "notApplicable") {
    fail(`${path}.confidence.status`, "must be calibratedProbability or notProvided for inferred observations");
  }
  return observation;
}

function observationReferences(observation) {
  const references = [...observation.sourceEvidence.sourceObservationIds];
  if (observation.family === "role") references.push(observation.value.targetObservationId);
  if (observation.family === "hierarchy") references.push(observation.value.parentObservationId, observation.value.childObservationId);
  if (observation.family === "peerGroup") references.push(...observation.value.memberObservationIds);
  return references;
}

function validateObservationLinks(observations, request) {
  const byId = new Map(observations.map((observation) => [observation.id, observation]));
  for (const [index, observation] of observations.entries()) {
    const references = observationReferences(observation);
    const missing = references.filter((identifier) => !byId.has(identifier));
    if (missing.length > 0) fail(`response.observations[${index}]`, `references missing observations ${missing.join(", ")}`);
    if (observation.sourceEvidence.sourceObservationIds.includes(observation.id)) fail(`response.observations[${index}].sourceEvidence`, "must not reference itself");
    if (observation.family === "role" && byId.get(observation.value.targetObservationId).family !== "region") {
      fail(`response.observations[${index}].value.targetObservationId`, "must reference a region observation");
    }
    if (observation.family === "hierarchy") {
      for (const identifier of [observation.value.parentObservationId, observation.value.childObservationId]) {
        if (!["region", "text"].includes(byId.get(identifier).family)) fail(`response.observations[${index}].value`, "hierarchy endpoints must reference region or text observations");
      }
    }
    if (observation.family === "peerGroup") {
      for (const identifier of observation.value.memberObservationIds) {
        if (byId.get(identifier).family !== "region") fail(`response.observations[${index}].value.memberObservationIds`, "must reference region observations");
      }
    }
  }

  const children = new Map();
  for (const observation of observations.filter((item) => item.family === "hierarchy")) {
    const parent = observation.value.parentObservationId;
    const descendants = children.get(parent) ?? [];
    descendants.push(observation.value.childObservationId);
    children.set(parent, descendants);
  }
  const visiting = new Set();
  const depths = new Map();
  const depth = (identifier) => {
    if (depths.has(identifier)) return depths.get(identifier);
    if (visiting.has(identifier)) fail("response.observations", "hierarchy must be acyclic");
    visiting.add(identifier);
    const value = 1 + Math.max(0, ...(children.get(identifier) ?? []).map(depth));
    visiting.delete(identifier);
    depths.set(identifier, value);
    return value;
  };
  for (const identifier of byId.keys()) {
    if (depth(identifier) > request.execution.maxHierarchyDepth) fail("response.observations", "hierarchy exceeds the requested depth budget");
  }
}

export function parseWorkerResponse(bytes, request, sourceSha256) {
  let root;
  try {
    root = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new PerceptionError("worker-json", "worker stdout must contain one valid UTF-8 JSON object");
  }
  const response = record(root, "response");
  exactKeys(response, ["$schema", "protocolVersion", "requestId", "status", "worker", "inputSha256", "familyStatus", "observations", "repeatedRunAgreement", "limitations"], "response");
  if (response.$schema !== "../schemas/response.schema.json" || response.protocolVersion !== PROTOCOL_VERSION) fail("response", "uses an unsupported schema or protocol version");
  if (response.requestId !== request.requestId) fail("response.requestId", "does not match the request");
  if (!RESPONSE_STATUSES.has(response.status)) fail("response.status", "is invalid");
  if (response.inputSha256 !== request.input.sha256) fail("response.inputSha256", "does not match the request input");

  const worker = record(response.worker, "response.worker");
  exactKeys(worker, ["name", "version", "runtime", "backend", "model"], "response.worker");
  token(worker.name, "response.worker.name");
  version(worker.version, "response.worker.version");
  if (worker.name !== request.worker.expectedName || worker.version !== request.worker.expectedVersion) fail("response.worker", "identity does not match the request");
  const runtime = record(worker.runtime, "response.worker.runtime");
  exactKeys(runtime, ["name", "version"], "response.worker.runtime");
  token(runtime.name, "response.worker.runtime.name");
  string(runtime.version, "response.worker.runtime.version");
  token(worker.backend, "response.worker.backend");
  if (worker.backend !== request.worker.backend) fail("response.worker.backend", "does not match the request");
  model(worker.model, "response.worker.model");
  if (canonicalJson(worker.model) !== canonicalJson(request.worker.model)) fail("response.worker.model", "does not match the request");
  worker.sourceSha256 = digest(sourceSha256, "response.worker.sourceSha256");

  validateFamilyStatuses(response.familyStatus);
  const observations = array(response.observations, "response.observations", 0, request.execution.maxObservations);
  const ids = observations.map((observation, index) => validateObservation(observation, index, request).id);
  if (new Set(ids).size !== ids.length) fail("response.observations", "must use unique IDs");
  if (ids.some((id, index) => index > 0 && id < ids[index - 1])) fail("response.observations", "must be sorted by stable ID");
  validateObservationLinks(observations, request);
  const statusByFamily = new Map(response.familyStatus.map((item) => [item.family, item.status]));
  for (const family of FAMILIES) {
    const count = observations.filter((observation) => observation.family === family).length;
    if (count > 0 && !["observed", "partial"].includes(statusByFamily.get(family))) fail("response.familyStatus", `${family} cannot contain observations under ${statusByFamily.get(family)}`);
    if (count === 0 && statusByFamily.get(family) === "observed") fail("response.familyStatus", `${family} is observed but contains no observations`);
  }
  if (response.status === "unsupported" && observations.length > 0) fail("response.status", "unsupported responses cannot contain observations");
  const agreement = record(response.repeatedRunAgreement, "response.repeatedRunAgreement");
  if (agreement.status === "notMeasured") {
    exactKeys(agreement, ["status", "reason"], "response.repeatedRunAgreement");
    string(agreement.reason, "response.repeatedRunAgreement.reason");
  } else {
    exactKeys(agreement, ["status", "runs", "identicalRuns"], "response.repeatedRunAgreement");
    if (agreement.status !== "measured") fail("response.repeatedRunAgreement.status", "is invalid");
    integer(agreement.runs, "response.repeatedRunAgreement.runs", 2, 1000);
    integer(agreement.identicalRuns, "response.repeatedRunAgreement.identicalRuns", 0, agreement.runs);
  }
  array(response.limitations, "response.limitations", 1, 32).forEach((limitation, index) => string(limitation, `response.limitations[${index}]`));

  response.worker = {
    name: worker.name,
    version: worker.version,
    sourceSha256: worker.sourceSha256,
    runtime: worker.runtime,
    backend: worker.backend,
    model: worker.model,
  };
  return response;
}
