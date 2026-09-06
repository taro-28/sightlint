import type { JsonObject } from "./types.js";

export const INTERACTION_PROTOCOL_VERSION = "0.1.0";
export const INTERACTION_ADAPTER_NAME = "sightlint-playwright-interaction";
export const INTERACTION_ADAPTER_VERSION = "0.1.0";
export const INTERACTION_EXTENSION_KEY = "org.sightlint.interaction";
export const INTERACTION_EXTENSION_VERSION = "0.1.0";

export const INTERACTION_LIMITS = Object.freeze({
  requestBytes: 1024 * 1024,
  maxSteps: 16,
  maxEvents: 64,
  maxViewportAxis: 2048,
  maxScreenshotBytes: 8 * 1024 * 1024,
  maxOutputBytes: 8 * 1024 * 1024,
  timeoutMs: 20_000,
});

export type InteractionState = "idle" | "pending" | "optimistic" | "success" | "failure";
export type VisibleInteractionState = Exclude<InteractionState, "idle">;
export type RecoveryKind = "retry" | "saveDraft";

export type InteractionStep =
  | { kind: "activate" }
  | { kind: "resolveSuccess" }
  | { kind: "reject" }
  | { kind: "activateRecovery"; recovery: RecoveryKind };

export interface InteractionRequest {
  $schema: string;
  protocolVersion: string;
  artifact: { id: string; title: string };
  fixture: { entrypoint: string; state: string; readinessSelector: string };
  action: {
    id: string;
    targetTestId: string;
    effectLatency: "immediate" | "observable";
    recovery:
      | { applicability: "inapplicable" }
      | { applicability: "required"; acceptedAlternatives: RecoveryKind[] };
  };
  trace:
    | { id: string; execution: "captured"; steps: InteractionStep[] }
    | { id: string; execution: "untested"; reason: string; steps: [] };
  environment: {
    viewport: { width: number; height: number; unit: "cssPixel" };
    locale: "en-US";
    timezoneId: "UTC";
    colorScheme: "light";
    reducedMotion: "reduce";
  };
  privacy: { textMode: "digestOnly"; externalProcessing: false };
  network: { mode: "deny" };
}

export interface InteractionSnapshotRecord {
  step: number;
  state: InteractionState;
  recoveries: RecoveryKind[];
  viewport: { width: number; height: number; unit: "cssPixel" };
  accessibilityDigest: string;
  screenshot: {
    sha256: string;
    byteLength: number;
    pixelSize: { width: number; height: number };
  };
  acquisitionOrder: ["dom", "accessibility", "screenshot"];
}

export interface InteractionResponse {
  protocolVersion: string;
  status: "captured" | "untested";
  adapter: {
    name: string;
    version: string;
    nodeVersion: string;
    platform: string;
    architecture: string;
    playwrightVersion: string;
    browserName: "chromium";
    browserVersion: string;
  };
  requestDigest: string;
  sourceDigest: string;
  artifactIr: { sha256: string; byteLength: number };
  trace: {
    id: string;
    eventCount: number;
    conflictCount: number;
    stateCaptureCount: number;
    clock: "controlledSteps";
    snapshots: InteractionSnapshotRecord[];
  };
  controls: {
    externalRequests: string[];
    externalProcessing: false;
    screenshotPersistence: "digestOnly";
  };
  limitations: string[];
}

export interface InteractionCapture {
  response: InteractionResponse;
  artifactIr: JsonObject;
}
