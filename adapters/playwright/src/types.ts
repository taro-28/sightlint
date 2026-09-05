export const PROTOCOL_VERSION = "0.1.0";
export const ADAPTER_NAME = "sightlint-playwright";
export const ADAPTER_VERSION = "0.2.0";
export const PLAYWRIGHT_VERSION = "1.63.0";
export const WEB_EXTENSION_KEY = "org.sightlint.web";
export const WEB_EXTENSION_VERSION = "0.2.0";

export const LIMITS = Object.freeze({
  requestBytes: 1024 * 1024,
  maxFrames: 1,
  maxNodes: 200,
  maxViewportAxis: 4096,
  maxDeviceScaleFactor: 2,
  maxScreenshotPixels: 16_777_216,
  maxScreenshotBytes: 16 * 1024 * 1024,
  maxOutputBytes: 16 * 1024 * 1024,
  timeoutMs: 20_000,
});

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };
export type JsonObject = { [key: string]: JsonValue };

export interface CaptureRequest {
  $schema: string;
  protocolVersion: string;
  artifact: {
    id: string;
    title: string;
  };
  fixture: {
    entrypoint: string;
    state: string;
    readinessSelector: string;
  };
  environment: {
    viewport: {
      width: number;
      height: number;
      unit: "cssPixel";
    };
    deviceScaleFactor: number;
    textScale: 1 | 1.25;
    locale: "en-US";
    timezoneId: "UTC";
    colorScheme: "light";
    reducedMotion: "reduce";
  };
  privacy: {
    accessibleNameMode: "selectedNodes";
    externalProcessing: false;
  };
  network: {
    mode: "deny";
  };
  screenshot: {
    reference: string;
  };
}

export interface RectValue {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface BrowserNode {
  locator: {
    type: "testId" | "id" | "css";
    value: string;
    selector: string;
  };
  parentLocatorValue: string | null;
  tagName: string;
  explicitRole: string | null;
  ariaLabel: string | null;
  disabled: boolean;
  interactive: boolean;
  layoutBox: RectValue | null;
  layoutUnavailableReason: string | null;
  renderBox: RectValue;
  clientSize: { width: number; height: number };
  scrollSize: { width: number; height: number };
  centerHitSample: {
    point: { x: number; y: number };
    outcome: "hit" | "notInteractive" | "zeroArea" | "offViewport" | "occluded";
    hitLocator: string | null;
    method: "elementFromPointAtRenderBoxCenter" | "notSampled";
  };
  computedStyle: {
    display: string;
    visibility: string;
    opacity: number;
    overflowX: string;
    overflowY: string;
    whiteSpace: string;
    textOverflow: string;
    fontSize: string;
    lineHeight: string;
    fontWeight: string;
    direction: string;
    writingMode: string;
    transform: string;
    pointerEvents: string;
  };
  clippingAncestors: Array<{
    locator: string;
    overflowX: string;
    overflowY: string;
    rect: RectValue;
  }>;
}

export interface AccessibilitySummary {
  status: "observed" | "cantTell";
  role: string | null;
  name: string | null;
  states: string[];
  rootLine: string | null;
  snapshotDigest: string;
  descendantsRedacted: true;
}

export interface CaptureOutputs {
  artifactIrPath: string;
  screenshotPath: string;
}

export interface FileRecord {
  sha256: string;
  byteLength: number;
}

export interface ScreenshotRecord extends FileRecord {
  reference: string;
  format: "png";
  pixelSize: { width: number; height: number };
  colorAssumptions: string;
}

export interface CaptureResponse {
  protocolVersion: string;
  status: "captured";
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
  artifactIr: FileRecord;
  screenshot: ScreenshotRecord;
  capture: {
    pageCount: 1;
    frameCount: number;
    nodeCount: number;
    externalRequests: string[];
    deterministicOptions: JsonObject;
  };
  limitations: string[];
}

export class AdapterError extends Error {
  public constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "AdapterError";
  }
}
