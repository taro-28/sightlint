import { isAbsolute } from "node:path";

import { AdapterError, type JsonObject, type JsonValue } from "./types.js";
import {
  INTERACTION_LIMITS,
  INTERACTION_PROTOCOL_VERSION,
  type InteractionRequest,
  type InteractionStep,
  type RecoveryKind,
} from "./interaction-types.js";

const REQUEST_SCHEMA = "../../../adapters/playwright/schemas/interaction-request.schema.json";
const TOKEN = /^[A-Za-z0-9._-]+$/u;
const REPOSITORY_PATH = /^[A-Za-z0-9._/-]+$/u;

function object(value: JsonValue | undefined, context: string): JsonObject {
  if (value === null || Array.isArray(value) || typeof value !== "object") {
    throw new AdapterError("invalid-interaction-request", `${context} must be an object`);
  }
  return value;
}

function exactFields(value: JsonObject, fields: readonly string[], context: string): void {
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  if (actual.join("\0") !== expected.join("\0")) {
    throw new AdapterError(
      "invalid-interaction-request",
      `${context} fields must be exactly: ${expected.join(", ")}`,
    );
  }
}

function string(value: JsonValue | undefined, context: string, maximum = 256): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum) {
    throw new AdapterError("invalid-interaction-request", `${context} must be a non-empty string`);
  }
  return value;
}

function constant(value: JsonValue | undefined, expected: JsonValue, context: string): void {
  if (value !== expected) {
    throw new AdapterError(
      "invalid-interaction-request",
      `${context} must be ${JSON.stringify(expected)}`,
    );
  }
}

function token(value: JsonValue | undefined, context: string): string {
  const parsed = string(value, context);
  if (!TOKEN.test(parsed)) {
    throw new AdapterError("invalid-interaction-request", `${context} must be a stable token`);
  }
  return parsed;
}

function recovery(value: JsonValue | undefined, context: string): RecoveryKind {
  const parsed = string(value, context);
  if (parsed !== "retry" && parsed !== "saveDraft") {
    throw new AdapterError("invalid-interaction-request", `${context} is unsupported`);
  }
  return parsed;
}

function parseStep(value: JsonValue, index: number): InteractionStep {
  const context = `request.trace.steps[${index}]`;
  const step = object(value, context);
  const kind = string(step.kind, `${context}.kind`);
  if (kind === "activate" || kind === "resolveSuccess" || kind === "reject") {
    exactFields(step, ["kind"], context);
    return { kind };
  }
  if (kind === "activateRecovery") {
    exactFields(step, ["kind", "recovery"], context);
    return { kind, recovery: recovery(step.recovery, `${context}.recovery`) };
  }
  throw new AdapterError("invalid-interaction-request", `${context}.kind is unsupported`);
}

export function parseInteractionRequest(bytes: Buffer): InteractionRequest {
  if (bytes.byteLength > INTERACTION_LIMITS.requestBytes) {
    throw new AdapterError(
      "interaction-request-too-large",
      `request exceeds ${INTERACTION_LIMITS.requestBytes} bytes`,
    );
  }
  let decoded: JsonValue;
  try {
    decoded = JSON.parse(bytes.toString("utf8")) as JsonValue;
  } catch {
    throw new AdapterError("invalid-json", "interaction request is not valid JSON");
  }
  const root = object(decoded, "request");
  exactFields(
    root,
    ["$schema", "protocolVersion", "artifact", "fixture", "action", "trace", "environment", "privacy", "network"],
    "request",
  );
  constant(root.$schema, REQUEST_SCHEMA, "request.$schema");
  constant(root.protocolVersion, INTERACTION_PROTOCOL_VERSION, "request.protocolVersion");

  const artifact = object(root.artifact, "request.artifact");
  exactFields(artifact, ["id", "title"], "request.artifact");
  const artifactValue = {
    id: token(artifact.id, "request.artifact.id"),
    title: string(artifact.title, "request.artifact.title"),
  };

  const fixture = object(root.fixture, "request.fixture");
  exactFields(fixture, ["entrypoint", "state", "readinessSelector"], "request.fixture");
  const entrypoint = string(fixture.entrypoint, "request.fixture.entrypoint", 512);
  if (!REPOSITORY_PATH.test(entrypoint) || isAbsolute(entrypoint) || entrypoint.split("/").includes("..")) {
    throw new AdapterError("invalid-path", "fixture entrypoint must be a repository-relative path");
  }
  const fixtureValue = {
    entrypoint,
    state: token(fixture.state, "request.fixture.state"),
    readinessSelector: string(fixture.readinessSelector, "request.fixture.readinessSelector"),
  };

  const action = object(root.action, "request.action");
  exactFields(action, ["id", "targetTestId", "effectLatency", "recovery"], "request.action");
  const effectLatency = string(action.effectLatency, "request.action.effectLatency");
  if (effectLatency !== "immediate" && effectLatency !== "observable") {
    throw new AdapterError("invalid-interaction-request", "request.action.effectLatency is unsupported");
  }
  const recoveryValue = object(action.recovery, "request.action.recovery");
  const applicability = string(recoveryValue.applicability, "request.action.recovery.applicability");
  let recoveryContract: InteractionRequest["action"]["recovery"];
  if (applicability === "inapplicable") {
    exactFields(recoveryValue, ["applicability"], "request.action.recovery");
    recoveryContract = { applicability };
  } else if (applicability === "required") {
    exactFields(
      recoveryValue,
      ["applicability", "acceptedAlternatives"],
      "request.action.recovery",
    );
    if (!Array.isArray(recoveryValue.acceptedAlternatives) || recoveryValue.acceptedAlternatives.length === 0) {
      throw new AdapterError(
        "invalid-interaction-request",
        "required recovery must contain accepted alternatives",
      );
    }
    const acceptedAlternatives = recoveryValue.acceptedAlternatives.map((value, index) =>
      recovery(value, `request.action.recovery.acceptedAlternatives[${index}]`)
    );
    if (new Set(acceptedAlternatives).size !== acceptedAlternatives.length) {
      throw new AdapterError("invalid-interaction-request", "accepted recovery alternatives must be unique");
    }
    recoveryContract = { applicability, acceptedAlternatives: acceptedAlternatives.sort() };
  } else {
    throw new AdapterError("invalid-interaction-request", "recovery applicability is unsupported");
  }
  const actionValue: InteractionRequest["action"] = {
    id: token(action.id, "request.action.id"),
    targetTestId: token(action.targetTestId, "request.action.targetTestId"),
    effectLatency,
    recovery: recoveryContract,
  };

  const trace = object(root.trace, "request.trace");
  const execution = string(trace.execution, "request.trace.execution");
  let traceValue: InteractionRequest["trace"];
  if (execution === "captured") {
    exactFields(trace, ["id", "execution", "steps"], "request.trace");
    if (!Array.isArray(trace.steps) || trace.steps.length === 0 || trace.steps.length > INTERACTION_LIMITS.maxSteps) {
      throw new AdapterError("invalid-interaction-request", "captured trace steps exceed their bounds");
    }
    const steps = trace.steps.map(parseStep);
    if (steps[0]?.kind !== "activate") {
      throw new AdapterError("invalid-interaction-request", "a captured trace must start with activate");
    }
    for (const step of steps) {
      if (step.kind === "activateRecovery" && (
        actionValue.recovery.applicability !== "required" ||
        !actionValue.recovery.acceptedAlternatives.includes(step.recovery)
      )) {
        throw new AdapterError("invalid-interaction-request", "trace activates an unaccepted recovery");
      }
    }
    traceValue = { id: token(trace.id, "request.trace.id"), execution, steps };
  } else if (execution === "untested") {
    exactFields(trace, ["id", "execution", "reason", "steps"], "request.trace");
    if (!Array.isArray(trace.steps) || trace.steps.length !== 0) {
      throw new AdapterError("invalid-interaction-request", "an untested trace must have no steps");
    }
    traceValue = {
      id: token(trace.id, "request.trace.id"),
      execution,
      reason: string(trace.reason, "request.trace.reason"),
      steps: [],
    };
  } else {
    throw new AdapterError("invalid-interaction-request", "trace execution is unsupported");
  }

  const environment = object(root.environment, "request.environment");
  exactFields(environment, ["viewport", "locale", "timezoneId", "colorScheme", "reducedMotion"], "request.environment");
  const viewport = object(environment.viewport, "request.environment.viewport");
  exactFields(viewport, ["width", "height", "unit"], "request.environment.viewport");
  const width = viewport.width;
  const height = viewport.height;
  if (
    typeof width !== "number" || !Number.isInteger(width) || width < 1 || width > INTERACTION_LIMITS.maxViewportAxis ||
    typeof height !== "number" || !Number.isInteger(height) || height < 1 || height > INTERACTION_LIMITS.maxViewportAxis
  ) {
    throw new AdapterError("invalid-interaction-request", "viewport axes are outside supported bounds");
  }
  constant(viewport.unit, "cssPixel", "request.environment.viewport.unit");
  constant(environment.locale, "en-US", "request.environment.locale");
  constant(environment.timezoneId, "UTC", "request.environment.timezoneId");
  constant(environment.colorScheme, "light", "request.environment.colorScheme");
  constant(environment.reducedMotion, "reduce", "request.environment.reducedMotion");

  const privacy = object(root.privacy, "request.privacy");
  exactFields(privacy, ["textMode", "externalProcessing"], "request.privacy");
  constant(privacy.textMode, "digestOnly", "request.privacy.textMode");
  constant(privacy.externalProcessing, false, "request.privacy.externalProcessing");
  const network = object(root.network, "request.network");
  exactFields(network, ["mode"], "request.network");
  constant(network.mode, "deny", "request.network.mode");

  return {
    $schema: REQUEST_SCHEMA,
    protocolVersion: INTERACTION_PROTOCOL_VERSION,
    artifact: artifactValue,
    fixture: fixtureValue,
    action: actionValue,
    trace: traceValue,
    environment: {
      viewport: { width, height, unit: "cssPixel" },
      locale: "en-US",
      timezoneId: "UTC",
      colorScheme: "light",
      reducedMotion: "reduce",
    },
    privacy: { textMode: "digestOnly", externalProcessing: false },
    network: { mode: "deny" },
  };
}
