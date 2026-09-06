import { createHash } from "node:crypto";
import { mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { chromium, type BrowserContext, type Page, type Route } from "playwright";

import { canonicalJson, sha256 } from "./canonical.js";
import {
  INTERACTION_ADAPTER_NAME,
  INTERACTION_ADAPTER_VERSION,
  INTERACTION_EXTENSION_KEY,
  INTERACTION_EXTENSION_VERSION,
  INTERACTION_LIMITS,
  INTERACTION_PROTOCOL_VERSION,
  type InteractionCapture,
  type InteractionRequest,
  type InteractionSnapshotRecord,
  type InteractionState,
  type InteractionStep,
  type RecoveryKind,
  type VisibleInteractionState,
} from "./interaction-types.js";
import { AdapterError, PLAYWRIGHT_VERSION, type JsonObject, type JsonValue } from "./types.js";
import { resolveFixture } from "./validate.js";

interface SourceBundle {
  digest: string;
  files: string[];
}

interface EvidenceSpec {
  id: string;
  evidenceClass: "exactSource" | "exactRender" | "platformSemantics" | "interactionTrace" | "declaredContract";
  nativeId?: string;
  inputDigest?: string;
}

interface DeclaredEvent {
  kind: "stateChanged" | "effectResolved";
  state?: InteractionState;
  resolution?: "success" | "failure";
}

interface BrowserObservation {
  state: InteractionState;
  recoveries: RecoveryKind[];
  declaredEvents: DeclaredEvent[];
  accessibilitySnapshot: string;
  screenshot: Buffer;
}

interface TraceBuild {
  evidence: EvidenceSpec[];
  events: JsonObject[];
  conflictEvidenceIds: Set<string>;
  conflictReasons: Set<string>;
  snapshots: InteractionSnapshotRecord[];
  currentAttempt: string;
  lastEventId: string | null;
}

function repositoryRelative(root: string, path: string): string {
  const value = relative(root, path);
  if (value === "" || value === ".." || value.startsWith(`..${sep}`)) {
    throw new AdapterError("path-escape", "browser resource escaped the repository root");
  }
  return value.split(sep).join("/");
}

async function sourceBundle(root: string, resourcePaths: Set<string>): Promise<SourceBundle> {
  const files = [...resourcePaths]
    .map((path) => repositoryRelative(root, path))
    .sort((left, right) => left.localeCompare(right, "en"));
  const digest = createHash("sha256");
  for (const file of files) {
    digest.update(file, "utf8");
    digest.update(Buffer.from([0]));
    digest.update(await readFile(join(root, file)));
  }
  return { digest: `sha256:${digest.digest("hex")}`, files };
}

async function routeRequest(
  route: Route,
  root: string,
  resourcePaths: Set<string>,
  externalRequests: Set<string>,
): Promise<void> {
  const requestUrl = new URL(route.request().url());
  if (requestUrl.protocol === "file:") {
    const resolved = await realpath(fileURLToPath(requestUrl)).catch(() => {
      throw new AdapterError("invalid-resource", "interaction fixture requested a missing local resource");
    });
    repositoryRelative(root, resolved);
    resourcePaths.add(resolved);
    await route.continue();
    return;
  }
  if (["about:", "data:", "blob:"].includes(requestUrl.protocol)) {
    await route.continue();
    return;
  }
  externalRequests.add(`${requestUrl.protocol}//${requestUrl.host || "local"}`);
  await route.abort("blockedbyclient");
}

async function configureContext(
  context: BrowserContext,
  root: string,
  resourcePaths: Set<string>,
  externalRequests: Set<string>,
): Promise<void> {
  context.setDefaultTimeout(INTERACTION_LIMITS.timeoutMs);
  context.setDefaultNavigationTimeout(INTERACTION_LIMITS.timeoutMs);
  await context.route("**/*", async (route) => routeRequest(route, root, resourcePaths, externalRequests));
}

function pngDimensions(bytes: Buffer): { width: number; height: number } {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (
    bytes.byteLength < 24 ||
    !bytes.subarray(0, 8).equals(signature) ||
    bytes.toString("ascii", 12, 16) !== "IHDR"
  ) {
    throw new AdapterError("invalid-screenshot", "Playwright did not return a valid PNG header");
  }
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

function visibleState(state: InteractionState): state is VisibleInteractionState {
  return state !== "idle";
}

function addEvent(
  build: TraceBuild,
  attemptId: string,
  detail: JsonObject,
  evidenceIds: string[],
): string {
  if (build.events.length >= INTERACTION_LIMITS.maxEvents) {
    throw new AdapterError("interaction-event-budget", "captured trace exceeds its event budget");
  }
  const sequence = build.events.length + 1;
  const id = `event-${String(sequence).padStart(2, "0")}`;
  const event: JsonObject = {
    id,
    sequence,
    attemptId,
    evidenceIds: [...evidenceIds].sort(),
    detail,
  };
  if (build.lastEventId !== null) event.causeEventId = build.lastEventId;
  build.events.push(event);
  build.lastEventId = id;
  return id;
}

function assertDeclaredEvents(value: JsonValue): DeclaredEvent[] {
  if (!Array.isArray(value)) {
    throw new AdapterError("invalid-interaction-harness", "fixture harness events must be an array");
  }
  return value.map((entry, index) => {
    if (entry === null || Array.isArray(entry) || typeof entry !== "object") {
      throw new AdapterError("invalid-interaction-harness", `fixture event ${index} must be an object`);
    }
    const keys = Object.keys(entry).sort().join(",");
    if (entry.kind === "stateChanged" && keys === "kind,state") {
      if (!["idle", "pending", "optimistic", "success", "failure"].includes(String(entry.state))) {
        throw new AdapterError("invalid-interaction-harness", `fixture event ${index} has an invalid state`);
      }
      return { kind: "stateChanged", state: entry.state as InteractionState };
    }
    if (entry.kind === "effectResolved" && keys === "kind,resolution") {
      if (entry.resolution !== "success" && entry.resolution !== "failure") {
        throw new AdapterError("invalid-interaction-harness", `fixture event ${index} has an invalid resolution`);
      }
      return { kind: "effectResolved", resolution: entry.resolution };
    }
    throw new AdapterError("invalid-interaction-harness", `fixture event ${index} has unsupported fields`);
  });
}

async function verifyHarness(page: Page, request: InteractionRequest): Promise<void> {
  const metadata = await page.evaluate(() => {
    const value = (globalThis as typeof globalThis & {
      __sightlintInteraction?: { metadata?: unknown; drainEvents?: unknown; control?: unknown };
    }).__sightlintInteraction;
    return {
      metadata: value?.metadata,
      drainEvents: typeof value?.drainEvents,
      control: typeof value?.control,
    };
  });
  if (
    metadata.drainEvents !== "function" ||
    metadata.control !== "function" ||
    metadata.metadata === null ||
    Array.isArray(metadata.metadata) ||
    typeof metadata.metadata !== "object"
  ) {
    throw new AdapterError("invalid-interaction-harness", "fixture does not expose the bounded interaction harness");
  }
  const fixtureMetadata = metadata.metadata as Record<string, unknown>;
  if (
    fixtureMetadata.actionId !== request.action.id ||
    fixtureMetadata.targetTestId !== request.action.targetTestId
  ) {
    throw new AdapterError("invalid-interaction-harness", "fixture harness identity does not match the request");
  }
}

async function observe(page: Page, request: InteractionRequest): Promise<BrowserObservation> {
  const stateSelector = "[data-testid=interaction-state]";
  const native = await page.evaluate((selector) => {
    const element = document.querySelector<HTMLElement>(selector);
    if (element === null) throw new Error("missing interaction state element");
    const state = element.dataset.sightlintState;
    const recoveries = [...document.querySelectorAll<HTMLElement>("[data-sightlint-recovery]")]
      .filter((candidate) => {
        const style = getComputedStyle(candidate);
        const rect = candidate.getBoundingClientRect();
        return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
      })
      .map((candidate) => candidate.dataset.sightlintRecovery);
    const harness = (globalThis as typeof globalThis & {
      __sightlintInteraction?: { drainEvents?: () => unknown };
    }).__sightlintInteraction;
    return { state, recoveries, declaredEvents: harness?.drainEvents?.() };
  }, stateSelector).catch(() => {
    throw new AdapterError("invalid-interaction-harness", "failed to read the fixture state contract");
  });
  if (!["idle", "pending", "optimistic", "success", "failure"].includes(String(native.state))) {
    throw new AdapterError("invalid-interaction-harness", "fixture exposed an unsupported visible state");
  }
  if (
    !Array.isArray(native.recoveries) ||
    native.recoveries.some((value) => value !== "retry" && value !== "saveDraft")
  ) {
    throw new AdapterError("invalid-interaction-harness", "fixture exposed an unsupported recovery control");
  }
  const accessibilitySnapshot = await page.locator(stateSelector).ariaSnapshot();
  const screenshot = await page.screenshot({
    type: "png",
    fullPage: false,
    animations: "disabled",
    caret: "hide",
    scale: "css",
  });
  if (screenshot.byteLength > INTERACTION_LIMITS.maxScreenshotBytes) {
    throw new AdapterError("interaction-screenshot-budget", "state screenshot exceeds its byte budget");
  }
  const dimensions = pngDimensions(screenshot);
  if (
    dimensions.width !== request.environment.viewport.width ||
    dimensions.height !== request.environment.viewport.height
  ) {
    throw new AdapterError("interaction-viewport-conflict", "state screenshot and requested viewport disagree");
  }
  return {
    state: native.state as InteractionState,
    recoveries: [...new Set(native.recoveries as RecoveryKind[])].sort(),
    declaredEvents: assertDeclaredEvents(native.declaredEvents as JsonValue),
    accessibilitySnapshot,
    screenshot,
  };
}

function recordObservation(
  build: TraceBuild,
  observation: BrowserObservation,
  stepNumber: number,
  request: InteractionRequest,
): void {
  const suffix = String(stepNumber).padStart(2, "0");
  const domEvidence = `e-dom-step-${suffix}`;
  const accessibilityEvidence = `e-ax-step-${suffix}`;
  const screenshotEvidence = `e-screenshot-step-${suffix}`;
  const appEvidence = `e-app-step-${suffix}`;
  const screenshotDigest = sha256(observation.screenshot);
  build.evidence.push(
    { id: domEvidence, evidenceClass: "interactionTrace", nativeId: "testid:interaction-state" },
    { id: accessibilityEvidence, evidenceClass: "platformSemantics", nativeId: "testid:interaction-state", inputDigest: sha256(observation.accessibilitySnapshot) },
    { id: screenshotEvidence, evidenceClass: "exactRender", inputDigest: screenshotDigest },
  );
  if (observation.declaredEvents.length > 0) {
    build.evidence.push({
      id: appEvidence,
      evidenceClass: "interactionTrace",
      nativeId: `fixture-harness:step-${suffix}`,
    });
  }

  for (const event of observation.declaredEvents) {
    if (event.kind === "effectResolved") {
      addEvent(
        build,
        build.currentAttempt,
        { kind: "effectResolved", resolution: event.resolution as "success" | "failure" },
        [appEvidence],
      );
    }
  }
  if (visibleState(observation.state)) {
    addEvent(
      build,
      build.currentAttempt,
      { kind: "stateObserved", state: observation.state },
      [domEvidence, accessibilityEvidence, screenshotEvidence],
    );
  }
  for (const recovery of observation.recoveries) {
    addEvent(
      build,
      build.currentAttempt,
      { kind: "recoveryOffered", recovery },
      [domEvidence, accessibilityEvidence, screenshotEvidence],
    );
  }

  const declaredState = [...observation.declaredEvents]
    .reverse()
    .find((event) => event.kind === "stateChanged")?.state;
  if (declaredState !== undefined && declaredState !== observation.state) {
    build.conflictEvidenceIds.add(appEvidence);
    build.conflictEvidenceIds.add(domEvidence);
    build.conflictReasons.add(
      `step ${stepNumber} app-declared state ${declaredState} conflicts with DOM state ${observation.state}`,
    );
  }

  const dimensions = pngDimensions(observation.screenshot);
  build.snapshots.push({
    step: stepNumber,
    state: observation.state,
    recoveries: observation.recoveries,
    viewport: request.environment.viewport,
    accessibilityDigest: sha256(observation.accessibilitySnapshot),
    screenshot: {
      sha256: screenshotDigest,
      byteLength: observation.screenshot.byteLength,
      pixelSize: dimensions,
    },
    acquisitionOrder: ["dom", "accessibility", "screenshot"],
  });
}

async function performStep(
  page: Page,
  request: InteractionRequest,
  step: InteractionStep,
  stepNumber: number,
  build: TraceBuild,
): Promise<void> {
  const actionSelector = `[data-testid="${request.action.targetTestId}"]`;
  if (step.kind === "activate") {
    await page.locator(actionSelector).click();
    const evidenceId = `e-action-step-${String(stepNumber).padStart(2, "0")}`;
    build.evidence.push({ evidenceClass: "interactionTrace", id: evidenceId, nativeId: `testid:${request.action.targetTestId}` });
    addEvent(build, build.currentAttempt, { kind: "actionActivated" }, [evidenceId]);
  } else if (step.kind === "activateRecovery") {
    build.currentAttempt = `attempt-${step.recovery}`;
    const selector = `[data-sightlint-recovery="${step.recovery}"]`;
    await page.locator(selector).click();
    const evidenceId = `e-action-step-${String(stepNumber).padStart(2, "0")}`;
    build.evidence.push({ evidenceClass: "interactionTrace", id: evidenceId, nativeId: `recovery:${step.recovery}` });
    addEvent(
      build,
      build.currentAttempt,
      { kind: "recoveryActivated", recovery: step.recovery },
      [evidenceId],
    );
  } else {
    await page.evaluate((control) => {
      const harness = (globalThis as typeof globalThis & {
        __sightlintInteraction?: { control?: (value: string) => void };
      }).__sightlintInteraction;
      harness?.control?.(control);
    }, step.kind);
  }
  recordObservation(build, await observe(page, request), stepNumber, request);
}

function evidenceRecord(spec: EvidenceSpec, sourceDigest: string): JsonObject {
  const record: JsonObject = {
    id: spec.id,
    class: spec.evidenceClass,
    source: {
      adapter: INTERACTION_ADAPTER_NAME,
      adapterVersion: INTERACTION_ADAPTER_VERSION,
      inputDigest: spec.inputDigest ?? sourceDigest,
      externalProcessing: false,
    },
  };
  if (spec.nativeId !== undefined) {
    record.selector = { type: "nativeId", nativeId: spec.nativeId };
  }
  return record;
}

async function targetRect(page: Page, testId: string): Promise<{ x: number; y: number; width: number; height: number }> {
  const selector = `[data-testid="${testId}"]`;
  const count = await page.locator(selector).count();
  if (count !== 1) {
    throw new AdapterError("invalid-interaction-target", "action targetTestId must resolve exactly once");
  }
  const rect = await page.locator(selector).boundingBox();
  if (rect === null) {
    throw new AdapterError("invalid-interaction-target", "action target must have a visible render box");
  }
  return rect;
}

function buildArtifactIr(
  request: InteractionRequest,
  source: SourceBundle,
  requestDigest: string,
  rect: { x: number; y: number; width: number; height: number },
  build: TraceBuild,
): JsonObject {
  const evidence = [
    evidenceRecord({ id: "e-contract", evidenceClass: "declaredContract", nativeId: `action:${request.action.id}`, inputDigest: requestDigest }, source.digest),
    evidenceRecord({ id: "e-target-source", evidenceClass: "exactSource", nativeId: `testid:${request.action.targetTestId}` }, source.digest),
    evidenceRecord({ id: "e-target-render", evidenceClass: "exactRender", nativeId: `testid:${request.action.targetTestId}` }, source.digest),
    evidenceRecord({ id: "e-viewport", evidenceClass: "exactRender" }, source.digest),
    ...build.evidence.map((spec) => evidenceRecord(spec, source.digest)),
  ].sort((left, right) => String(left.id).localeCompare(String(right.id), "en"));
  const consistency = build.conflictReasons.size === 0
    ? { status: "agreement" }
    : {
        status: "conflict",
        evidenceIds: [...build.conflictEvidenceIds].sort(),
        reason: [...build.conflictReasons].sort().join("; "),
      };
  const execution = request.trace.execution === "captured"
    ? { status: "captured" }
    : { status: "untested", reason: request.trace.reason };
  return {
    schemaVersion: "0.1.0",
    artifact: {
      id: request.artifact.id,
      kind: "web",
      title: request.artifact.title,
      sourceName: `${request.fixture.entrypoint}?case=${request.fixture.state}`,
    },
    canvases: [{
      id: "viewport",
      size: { width: request.environment.viewport.width, height: request.environment.viewport.height },
      unit: "cssPixel",
      horizontalDirection: "right",
      verticalDirection: "down",
      evidenceId: "e-viewport",
    }],
    nodes: [{
      id: `interaction-target-${request.action.id}`,
      kind: { value: "control", evidenceId: "e-target-source" },
      coordinateSpaceId: "viewport",
      geometry: {
        renderBox: {
          rect,
          coordinateSpaceId: "viewport",
          evidenceId: "e-target-render",
        },
      },
    }],
    evidence,
    extensions: {
      [INTERACTION_EXTENSION_KEY]: {
        extensionVersion: INTERACTION_EXTENSION_VERSION,
        actions: [{
          id: request.action.id,
          targetNodeId: `interaction-target-${request.action.id}`,
          contractEvidenceId: "e-contract",
          effectLatency: request.action.effectLatency,
          recovery: request.action.recovery,
        }],
        traces: [{
          id: request.trace.id,
          actionId: request.action.id,
          execution,
          environment: {
            clock: "controlledSteps",
            network: "denyExternal",
            viewportSize: {
              width: request.environment.viewport.width,
              height: request.environment.viewport.height,
            },
            viewportUnit: "cssPixel",
            locale: request.environment.locale,
            timezoneId: request.environment.timezoneId,
            colorScheme: request.environment.colorScheme,
            reducedMotion: request.environment.reducedMotion,
            externalProcessing: false,
          },
          consistency,
          events: build.events,
        }],
      },
    },
  } as JsonObject;
}

async function writeExclusive(path: string, bytes: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes, { flag: "wx" }).catch((error: NodeJS.ErrnoException) => {
    if (error.code === "EEXIST") {
      throw new AdapterError("output-exists", "refusing to overwrite an existing interaction output");
    }
    throw new AdapterError("output-write", "failed to write interaction output");
  });
}

export async function captureInteraction(
  request: InteractionRequest,
  repositoryRoot: string,
  artifactIrPath: string,
): Promise<InteractionCapture> {
  const resolved = await resolveFixture(repositoryRoot, request.fixture.entrypoint);
  const requestBytes = canonicalJson(request as unknown as JsonValue);
  const requestDigest = sha256(requestBytes);
  const resourcePaths = new Set<string>([resolved.entrypoint]);
  const externalRequests = new Set<string>();
  const browser = await chromium.launch({
    headless: true,
    args: [
      "--disable-background-networking",
      "--disable-component-update",
      "--disable-default-apps",
      "--disable-features=Translate,BackForwardCache",
      "--disable-sync",
      "--metrics-recording-only",
      "--no-first-run",
    ],
  }).catch(() => {
    throw new AdapterError("browser-launch", "failed to launch the pinned Chromium build");
  });
  let artifactIrBytes: string | undefined;
  let capture: InteractionCapture | undefined;
  try {
    const context = await browser.newContext({
      viewport: request.environment.viewport,
      deviceScaleFactor: 1,
      locale: request.environment.locale,
      timezoneId: request.environment.timezoneId,
      colorScheme: request.environment.colorScheme,
      reducedMotion: request.environment.reducedMotion,
      serviceWorkers: "block",
      offline: true,
      permissions: [],
    });
    await configureContext(context, resolved.root, resourcePaths, externalRequests);
    const page = await context.newPage();
    const destination = pathToFileURL(resolved.entrypoint);
    destination.searchParams.set("case", request.fixture.state);
    await page.goto(destination.href, { waitUntil: "load", timeout: INTERACTION_LIMITS.timeoutMs });
    await page.waitForSelector(request.fixture.readinessSelector, { state: "attached" });
    await page.evaluate(async () => document.fonts.ready);
    if (page.frames().length !== 1 || context.pages().length !== 1) {
      throw new AdapterError("unsupported-frame", "interaction protocol 0.1 supports one page and main frame");
    }
    await verifyHarness(page, request);
    const rect = await targetRect(page, request.action.targetTestId);
    const build: TraceBuild = {
      evidence: [],
      events: [],
      conflictEvidenceIds: new Set(),
      conflictReasons: new Set(),
      snapshots: [],
      currentAttempt: "attempt-primary",
      lastEventId: null,
    };
    if (request.trace.execution === "captured") {
      for (const [index, step] of request.trace.steps.entries()) {
        await performStep(page, request, step, index + 1, build);
      }
    }
    if (externalRequests.size > 0) {
      throw new AdapterError("external-request", "interaction fixture attempted an external request");
    }
    const source = await sourceBundle(resolved.root, resourcePaths);
    const artifactIr = buildArtifactIr(request, source, requestDigest, rect, build);
    artifactIrBytes = canonicalJson(artifactIr as JsonValue);
    if (Buffer.byteLength(artifactIrBytes) > INTERACTION_LIMITS.maxOutputBytes) {
      throw new AdapterError("interaction-output-budget", "canonical interaction Artifact IR exceeds its output budget");
    }
    const response = {
      protocolVersion: INTERACTION_PROTOCOL_VERSION,
      status: request.trace.execution,
      adapter: {
        name: INTERACTION_ADAPTER_NAME,
        version: INTERACTION_ADAPTER_VERSION,
        nodeVersion: process.versions.node,
        platform: process.platform,
        architecture: process.arch,
        playwrightVersion: PLAYWRIGHT_VERSION,
        browserName: "chromium" as const,
        browserVersion: browser.version(),
      },
      requestDigest,
      sourceDigest: source.digest,
      artifactIr: {
        sha256: sha256(artifactIrBytes),
        byteLength: Buffer.byteLength(artifactIrBytes),
      },
      trace: {
        id: request.trace.id,
        eventCount: build.events.length,
        conflictCount: build.conflictReasons.size,
        stateCaptureCount: build.snapshots.length,
        clock: "controlledSteps" as const,
        snapshots: build.snapshots,
      },
      controls: {
        externalRequests: [] as string[],
        externalProcessing: false as const,
        screenshotPersistence: "digestOnly" as const,
      },
      limitations: [
        "controlled fixture instrumentation reports effect resolution; pixels never establish invisible effects",
        "DOM, accessibility, and screenshot acquisition within one step is sequential and conflicts are retained",
        "public maintainer-authored fixtures are regression evidence, not representative interaction or UI/UX accuracy",
        "focus, duplicate activation, destructive safeguards, offline, permission, partial, undo, and mobile traces are untested",
      ],
    };
    const responseBytes = canonicalJson(response as unknown as JsonValue);
    if (Buffer.byteLength(responseBytes) > INTERACTION_LIMITS.maxOutputBytes) {
      throw new AdapterError("interaction-output-budget", "canonical interaction response exceeds its output budget");
    }
    await context.close();
    capture = { artifactIr, response };
  } finally {
    await browser.close();
  }
  if (artifactIrBytes === undefined || capture === undefined) {
    throw new AdapterError("execution-error", "interaction capture did not produce a complete output");
  }
  await writeExclusive(artifactIrPath, artifactIrBytes);
  return capture;
}
