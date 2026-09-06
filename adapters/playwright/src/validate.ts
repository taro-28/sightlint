import { realpath, stat } from "node:fs/promises";
import { isAbsolute, relative, resolve, sep } from "node:path";

import {
  AdapterError,
  LEGACY_PROTOCOL_VERSION,
  LIMITS,
  MANAGED_PROTOCOL_VERSION,
  type CaptureRequest,
  type JsonObject,
  type JsonValue,
  type LegacyCaptureRequest,
  type ManagedCaptureRequest,
} from "./types.js";

const LEGACY_REQUEST_SCHEMA = "../../../adapters/playwright/schemas/capture-request.schema.json";
const MANAGED_REQUEST_SCHEMA = "../../../adapters/playwright/schemas/capture-request-0.2.schema.json";
const TOKEN = /^[A-Za-z0-9._-]+$/u;
const REPOSITORY_PATH = /^[A-Za-z0-9._/-]+$/u;
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f]/u;
const ENCODED_CONTROL_CHARACTER = /%(?:0[0-9a-f]|1[0-9a-f]|7f)/iu;

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

function parseEnvironment(value: JsonValue | undefined, managed: boolean): CaptureRequest["environment"] {
  const environment = object(value, "request.environment");
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
  const locale = string(environment.locale, "request.environment.locale");
  if (locale !== "en-US" && (!managed || locale !== "ja-JP")) {
    throw new AdapterError(
      "invalid-request",
      managed
        ? "request.environment.locale must be en-US or ja-JP"
        : "request.environment.locale must be \"en-US\"",
    );
  }
  constant(environment.timezoneId, "UTC", "request.environment.timezoneId");
  constant(environment.colorScheme, "light", "request.environment.colorScheme");
  constant(environment.reducedMotion, "reduce", "request.environment.reducedMotion");
  return {
    viewport: { width, height, unit: "cssPixel" },
    deviceScaleFactor,
    textScale: textScale as 1 | 1.25,
    locale,
    timezoneId: "UTC",
    colorScheme: "light",
    reducedMotion: "reduce",
  };
}

function parseCommon(root: JsonObject, managed: boolean): Pick<CaptureRequest, "artifact" | "environment" | "privacy" | "screenshot"> {
  const artifact = object(root.artifact, "request.artifact");
  exactFields(artifact, ["id", "title"], "request.artifact");
  const artifactId = string(artifact.id, "request.artifact.id");
  const artifactTitle = string(artifact.title, "request.artifact.title");
  if (!TOKEN.test(artifactId)) {
    throw new AdapterError("invalid-request", "request.artifact.id must be a stable token");
  }

  const privacy = object(root.privacy, "request.privacy");
  exactFields(privacy, ["accessibleNameMode", "externalProcessing"], "request.privacy");
  constant(privacy.accessibleNameMode, "selectedNodes", "request.privacy.accessibleNameMode");
  constant(privacy.externalProcessing, false, "request.privacy.externalProcessing");

  const screenshot = object(root.screenshot, "request.screenshot");
  exactFields(screenshot, ["reference"], "request.screenshot");
  const screenshotReference = string(screenshot.reference, "request.screenshot.reference", 512);
  if (!REPOSITORY_PATH.test(screenshotReference) || screenshotReference.split("/").includes("..")) {
    throw new AdapterError("invalid-path", "screenshot reference must be a stable relative path");
  }

  return {
    artifact: { id: artifactId, title: artifactTitle },
    environment: parseEnvironment(root.environment, managed),
    privacy: { accessibleNameMode: "selectedNodes", externalProcessing: false },
    screenshot: { reference: screenshotReference },
  };
}

function parseLegacy(root: JsonObject): LegacyCaptureRequest {
  exactFields(
    root,
    ["$schema", "protocolVersion", "artifact", "fixture", "environment", "privacy", "network", "screenshot"],
    "request",
  );
  constant(root.$schema, LEGACY_REQUEST_SCHEMA, "request.$schema");
  const common = parseCommon(root, false);
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
  const network = object(root.network, "request.network");
  exactFields(network, ["mode"], "request.network");
  constant(network.mode, "deny", "request.network.mode");
  return {
    $schema: LEGACY_REQUEST_SCHEMA,
    protocolVersion: LEGACY_PROTOCOL_VERSION,
    ...common,
    fixture: { entrypoint, state, readinessSelector },
    network: { mode: "deny" },
  };
}

function validatePathAndQuery(value: JsonValue | undefined): string {
  const pathAndQuery = string(value, "request.target.pathAndQuery", LIMITS.maxPathAndQueryBytes);
  if (Buffer.byteLength(pathAndQuery, "utf8") > LIMITS.maxPathAndQueryBytes) {
    throw new AdapterError("invalid-target", "request.target.pathAndQuery exceeds 2048 UTF-8 bytes");
  }
  if (!pathAndQuery.startsWith("/") || pathAndQuery.startsWith("//") || pathAndQuery.includes("\\") || pathAndQuery.includes("#") || CONTROL_CHARACTER.test(pathAndQuery) || ENCODED_CONTROL_CHARACTER.test(pathAndQuery)) {
    throw new AdapterError("invalid-target", "request.target.pathAndQuery must be an absolute path/query without authority, fragment, or control characters");
  }
  const parsed = new URL(pathAndQuery, "http://127.0.0.1:1024");
  if (parsed.origin !== "http://127.0.0.1:1024" || `${parsed.pathname}${parsed.search}` !== pathAndQuery) {
    throw new AdapterError("invalid-target", "request.target.pathAndQuery must remain on literal IPv4 loopback");
  }
  return pathAndQuery;
}

function parseManaged(root: JsonObject): ManagedCaptureRequest {
  exactFields(
    root,
    ["$schema", "protocolVersion", "artifact", "target", "server", "environment", "privacy", "network", "screenshot"],
    "request",
  );
  constant(root.$schema, MANAGED_REQUEST_SCHEMA, "request.$schema");
  const common = parseCommon(root, true);

  const target = object(root.target, "request.target");
  exactFields(target, ["kind", "pathAndQuery", "state", "readinessSelector"], "request.target");
  constant(target.kind, "managedLoopbackHttp", "request.target.kind");
  const pathAndQuery = validatePathAndQuery(target.pathAndQuery);
  const state = string(target.state, "request.target.state", 80);
  const readinessSelector = string(target.readinessSelector, "request.target.readinessSelector");
  if (!TOKEN.test(state)) {
    throw new AdapterError("invalid-request", "request.target.state must be a stable token");
  }
  if (CONTROL_CHARACTER.test(readinessSelector)) {
    throw new AdapterError("invalid-request", "request.target.readinessSelector contains a control character");
  }

  const server = object(root.server, "request.server");
  exactFields(server, ["argv", "port", "startupTimeoutMs"], "request.server");
  if (!Array.isArray(server.argv) || server.argv.length === 0 || server.argv.length > LIMITS.maxServerArgv) {
    throw new AdapterError("invalid-server-command", "request.server.argv must contain from 1 through 64 elements");
  }
  const argv = server.argv.map((value, index) => {
    if (typeof value !== "string" || value.length === 0 || CONTROL_CHARACTER.test(value)) {
      throw new AdapterError("invalid-server-command", `request.server.argv[${index}] must be a non-empty control-free string`);
    }
    return value;
  });
  if (Buffer.byteLength(argv.join("\0"), "utf8") > LIMITS.maxServerArgvBytes) {
    throw new AdapterError("invalid-server-command", "request.server.argv exceeds 8192 UTF-8 bytes");
  }
  const placeholderCount = argv.reduce((count, value) => count + value.split("{port}").length - 1, 0);
  if (placeholderCount !== 1) {
    throw new AdapterError("invalid-server-command", "request.server.argv must contain {port} exactly once");
  }
  const port = finiteNumber(server.port, "request.server.port");
  if (!Number.isInteger(port) || port < 1024 || port > 65535) {
    throw new AdapterError("invalid-server", "request.server.port must be an integer from 1024 through 65535");
  }
  const startupTimeoutMs = finiteNumber(server.startupTimeoutMs, "request.server.startupTimeoutMs");
  if (!Number.isInteger(startupTimeoutMs) || startupTimeoutMs < 1 || startupTimeoutMs > LIMITS.maxServerStartupTimeoutMs) {
    throw new AdapterError("invalid-server", "request.server.startupTimeoutMs must be an integer from 1 through 180000");
  }

  const network = object(root.network, "request.network");
  exactFields(network, ["mode"], "request.network");
  constant(network.mode, "sameOriginLoopback", "request.network.mode");

  return {
    $schema: MANAGED_REQUEST_SCHEMA,
    protocolVersion: MANAGED_PROTOCOL_VERSION,
    ...common,
    target: {
      kind: "managedLoopbackHttp",
      pathAndQuery,
      state,
      readinessSelector,
    },
    server: { argv, port, startupTimeoutMs },
    network: { mode: "sameOriginLoopback" },
  };
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
  if (root.protocolVersion === LEGACY_PROTOCOL_VERSION) return parseLegacy(root);
  if (root.protocolVersion === MANAGED_PROTOCOL_VERSION) return parseManaged(root);
  throw new AdapterError(
    "unsupported-protocol",
    `request.protocolVersion must be ${LEGACY_PROTOCOL_VERSION} or ${MANAGED_PROTOCOL_VERSION}`,
  );
}

export async function resolveRepositoryRoot(repositoryRoot: string): Promise<string> {
  return realpath(repositoryRoot).catch(() => {
    throw new AdapterError("invalid-path", "repository root does not exist");
  });
}

export async function resolveFixture(repositoryRoot: string, entrypoint: string): Promise<{ root: string; entrypoint: string }> {
  const root = await resolveRepositoryRoot(repositoryRoot);
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
