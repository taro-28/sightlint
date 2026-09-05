import { realpath, stat } from "node:fs/promises";
import { isAbsolute, relative, resolve, sep } from "node:path";

import {
  AdapterError,
  LIMITS,
  PROTOCOL_VERSION,
  type CaptureRequest,
  type JsonObject,
  type JsonValue,
} from "./types.js";

const REQUEST_SCHEMA = "../../../adapters/playwright/schemas/capture-request.schema.json";
const TOKEN = /^[A-Za-z0-9._-]+$/u;
const REPOSITORY_PATH = /^[A-Za-z0-9._/-]+$/u;

function object(value: JsonValue | undefined, context: string): JsonObject {
  if (value === null || Array.isArray(value) || typeof value !== "object") {
    throw new AdapterError("invalid-request", `${context} must be an object`);
  }
  return value;
}

function exactFields(value: JsonObject, fields: readonly string[], context: string): void {
  const actual = Object.keys(value).sort();
  const expected = [...fields].sort();
  if (actual.join("\0") !== expected.join("\0")) {
    throw new AdapterError(
      "invalid-request",
      `${context} fields must be exactly: ${expected.join(", ")}`,
    );
  }
}

function string(value: JsonValue | undefined, context: string, maximum = 256): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum) {
    throw new AdapterError("invalid-request", `${context} must be a non-empty string`);
  }
  return value;
}

function finiteNumber(value: JsonValue | undefined, context: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new AdapterError("invalid-request", `${context} must be a finite number`);
  }
  return value;
}

function constant(value: JsonValue | undefined, expected: JsonValue, context: string): void {
  if (value !== expected) {
    throw new AdapterError("invalid-request", `${context} must be ${JSON.stringify(expected)}`);
  }
}

export function parseCaptureRequest(bytes: Buffer): CaptureRequest {
  if (bytes.byteLength > LIMITS.requestBytes) {
    throw new AdapterError("request-too-large", `request exceeds ${LIMITS.requestBytes} bytes`);
  }
  let decoded: JsonValue;
  try {
    decoded = JSON.parse(bytes.toString("utf8")) as JsonValue;
  } catch {
    throw new AdapterError("invalid-json", "request is not valid JSON");
  }
  const root = object(decoded, "request");
  exactFields(
    root,
    ["$schema", "protocolVersion", "artifact", "fixture", "environment", "privacy", "network", "screenshot"],
    "request",
  );
  constant(root.$schema, REQUEST_SCHEMA, "request.$schema");
  constant(root.protocolVersion, PROTOCOL_VERSION, "request.protocolVersion");

  const artifact = object(root.artifact, "request.artifact");
  exactFields(artifact, ["id", "title"], "request.artifact");
  const artifactId = string(artifact.id, "request.artifact.id");
  const artifactTitle = string(artifact.title, "request.artifact.title");
  if (!TOKEN.test(artifactId)) {
    throw new AdapterError("invalid-request", "request.artifact.id must be a stable token");
  }

  const fixture = object(root.fixture, "request.fixture");
  exactFields(fixture, ["entrypoint", "state", "readinessSelector"], "request.fixture");
  const entrypoint = string(fixture.entrypoint, "request.fixture.entrypoint", 512);
  const state = string(fixture.state, "request.fixture.state", 80);
  const readinessSelector = string(fixture.readinessSelector, "request.fixture.readinessSelector");
  if (!REPOSITORY_PATH.test(entrypoint) || isAbsolute(entrypoint) || entrypoint.split("/").includes("..")) {
    throw new AdapterError("invalid-path", "fixture entrypoint must be a repository-relative path");
  }
  if (!TOKEN.test(state)) {
    throw new AdapterError("invalid-request", "fixture state must be a stable token");
  }

  const environment = object(root.environment, "request.environment");
  exactFields(
    environment,
    ["viewport", "deviceScaleFactor", "textScale", "locale", "timezoneId", "colorScheme", "reducedMotion"],
    "request.environment",
  );
  const viewport = object(environment.viewport, "request.environment.viewport");
  exactFields(viewport, ["width", "height", "unit"], "request.environment.viewport");
  const width = finiteNumber(viewport.width, "request.environment.viewport.width");
  const height = finiteNumber(viewport.height, "request.environment.viewport.height");
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 || width > LIMITS.maxViewportAxis || height > LIMITS.maxViewportAxis) {
    throw new AdapterError("invalid-request", "viewport axes must be integers from 1 through 4096");
  }
  constant(viewport.unit, "cssPixel", "request.environment.viewport.unit");
  const deviceScaleFactor = finiteNumber(environment.deviceScaleFactor, "request.environment.deviceScaleFactor");
  if (deviceScaleFactor < 1 || deviceScaleFactor > LIMITS.maxDeviceScaleFactor) {
    throw new AdapterError("invalid-request", "deviceScaleFactor must be from 1 through 2");
  }
  const textScale = finiteNumber(environment.textScale, "request.environment.textScale");
  if (textScale !== 1 && textScale !== 1.25) {
    throw new AdapterError("invalid-request", "textScale must be 1 or 1.25");
  }
  constant(environment.locale, "en-US", "request.environment.locale");
  constant(environment.timezoneId, "UTC", "request.environment.timezoneId");
  constant(environment.colorScheme, "light", "request.environment.colorScheme");
  constant(environment.reducedMotion, "reduce", "request.environment.reducedMotion");

  const privacy = object(root.privacy, "request.privacy");
  exactFields(privacy, ["accessibleNameMode", "externalProcessing"], "request.privacy");
  constant(privacy.accessibleNameMode, "selectedNodes", "request.privacy.accessibleNameMode");
  constant(privacy.externalProcessing, false, "request.privacy.externalProcessing");

  const network = object(root.network, "request.network");
  exactFields(network, ["mode"], "request.network");
  constant(network.mode, "deny", "request.network.mode");

  const screenshot = object(root.screenshot, "request.screenshot");
  exactFields(screenshot, ["reference"], "request.screenshot");
  const screenshotReference = string(screenshot.reference, "request.screenshot.reference", 512);
  if (!REPOSITORY_PATH.test(screenshotReference) || screenshotReference.split("/").includes("..")) {
    throw new AdapterError("invalid-path", "screenshot reference must be a stable relative path");
  }

  return {
    $schema: REQUEST_SCHEMA,
    protocolVersion: PROTOCOL_VERSION,
    artifact: { id: artifactId, title: artifactTitle },
    fixture: { entrypoint, state, readinessSelector },
    environment: {
      viewport: { width, height, unit: "cssPixel" },
      deviceScaleFactor,
      textScale: textScale as 1 | 1.25,
      locale: "en-US",
      timezoneId: "UTC",
      colorScheme: "light",
      reducedMotion: "reduce",
    },
    privacy: { accessibleNameMode: "selectedNodes", externalProcessing: false },
    network: { mode: "deny" },
    screenshot: { reference: screenshotReference },
  };
}

export async function resolveFixture(repositoryRoot: string, entrypoint: string): Promise<{ root: string; entrypoint: string }> {
  const root = await realpath(repositoryRoot);
  const requested = resolve(root, entrypoint);
  const resolved = await realpath(requested).catch(() => {
    throw new AdapterError("invalid-path", "fixture entrypoint does not exist");
  });
  const inside = relative(root, resolved);
  if (inside === "" || inside === ".." || inside.startsWith(`..${sep}`) || isAbsolute(inside)) {
    throw new AdapterError("path-escape", "fixture entrypoint must remain below the repository root");
  }
  const metadata = await stat(resolved);
  if (!metadata.isFile()) {
    throw new AdapterError("invalid-path", "fixture entrypoint must be a regular file");
  }
  if (!resolved.toLowerCase().endsWith(".html")) {
    throw new AdapterError("unsupported-input", "fixture entrypoint must be an HTML file");
  }
  return { root, entrypoint: resolved };
}
