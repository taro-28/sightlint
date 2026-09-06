import { createHash } from "node:crypto";
import { mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { chromium, type BrowserContext, type Page, type Route } from "playwright";

import { canonicalJson, sha256 } from "./canonical.js";
import {
  ADAPTER_NAME,
  ADAPTER_VERSION,
  AdapterError,
  LEGACY_ADAPTER_VERSION,
  LEGACY_PROTOCOL_VERSION,
  LEGACY_WEB_EXTENSION_VERSION,
  LIMITS,
  MANAGED_PROTOCOL_VERSION,
  PLAYWRIGHT_VERSION,
  WEB_EXTENSION_KEY,
  WEB_EXTENSION_VERSION,
  type AccessibilitySummary,
  type BrowserNode,
  type CaptureOutputs,
  type CaptureOptions,
  type CaptureRequest,
  type CaptureResponse,
  type JsonObject,
  type JsonValue,
  type RectValue,
} from "./types.js";
import { ManagedServer } from "./server.js";
import { resolveFixture, resolveRepositoryRoot } from "./validate.js";

interface BrowserSnapshot {
  nodes: BrowserNode[];
  duplicateLocators: string[];
  scroll: { x: number; y: number };
  documentSize: { width: number; height: number };
  documentDirection: string;
}

interface CapturedNode {
  nodeId: string;
  browser: BrowserNode;
  accessibility: AccessibilitySummary;
}

interface SourceBundle {
  digest: string;
  files: string[];
  kind: "repositoryFiles" | "loopbackResponses";
  routePath?: string;
  targetDigest?: string;
  requestCount?: number;
  responseBytes?: number;
  blockedWebSocketCount?: number;
  blockedServiceWorkerCount?: number;
}

function roundCss(value: number): number {
  const rounded = Math.round(value * 10_000) / 10_000;
  return Object.is(rounded, -0) ? 0 : rounded;
}

function nodeIdentifier(locatorValue: string): string {
  if (locatorValue.startsWith("testid:")) {
    const testId = locatorValue.slice("testid:".length);
    if (/^[A-Za-z0-9._-]+$/u.test(testId)) {
      return `web-${testId}`;
    }
  }
  return `web-${sha256(locatorValue).slice("sha256:".length, "sha256:".length + 20)}`;
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
    .sort((left, right) => (left < right ? -1 : left > right ? 1 : 0));
  const digest = createHash("sha256");
  for (const file of files) {
    digest.update(file, "utf8");
    digest.update(Buffer.from([0]));
    digest.update(await readFile(join(root, file)));
  }
  return { digest: `sha256:${digest.digest("hex")}`, files, kind: "repositoryFiles" };
}

function externalRequestLabel(url: URL): string {
  return `${url.protocol}//${url.host || "local"}`;
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
      throw new AdapterError("invalid-resource", "fixture requested a missing local resource");
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
  externalRequests.add(externalRequestLabel(requestUrl));
  await route.abort("blockedbyclient");
}

interface LoopbackResponse {
  method: string;
  targetDigest: string;
  requestBodyDigest: string;
  status: number;
  body: Buffer;
  responseDigest: string;
}

class ManagedNetworkRecorder {
  readonly #allowedOrigin: string;
  readonly #target: URL;
  readonly #responses: LoopbackResponse[] = [];
  readonly #externalRequests = new Set<string>();
  #requestCount = 0;
  #responseBytes = 0;
  #blockedWebSocketCount = 0;
  #blockedServiceWorkerCount = 0;
  #failure: AdapterError | null = null;

  public constructor(port: number, pathAndQuery: string) {
    this.#allowedOrigin = `http://127.0.0.1:${port}`;
    this.#target = new URL(pathAndQuery, this.#allowedOrigin);
  }

  public get destination(): string {
    return this.#target.href;
  }

  public get routePath(): string {
    return this.#target.pathname;
  }

  public get targetDigest(): string {
    return sha256(this.#target.href);
  }

  public get externalRequests(): string[] {
    return [...this.#externalRequests].sort();
  }

  public blockWebSocket(url: string): void {
    const target = new URL(url);
    if (target.protocol === "ws:" || target.protocol === "wss:") {
      this.#blockedWebSocketCount += 1;
    }
  }

  public blockServiceWorker(): void {
    this.#blockedServiceWorkerCount += 1;
  }

  public async route(route: Route): Promise<void> {
    const request = route.request();
    const target = new URL(request.url());
    if (["about:", "data:", "blob:"].includes(target.protocol)) {
      await route.continue();
      return;
    }
    if (target.protocol !== "http:" || target.origin !== this.#allowedOrigin) {
      this.#externalRequests.add(externalRequestLabel(target));
      await route.abort("blockedbyclient");
      return;
    }
    if (this.#requestCount >= LIMITS.maxLoopbackResponses) {
      this.#failure = new AdapterError("response-count-limit", "capture exceeded 512 loopback responses");
      await route.abort("blockedbyclient");
      return;
    }
    this.#requestCount += 1;
    const requestBody = request.postDataBuffer() ?? Buffer.alloc(0);
    if (requestBody.byteLength > LIMITS.maxRequestBodyBytes) {
      this.#failure = new AdapterError("request-body-too-large", "loopback request body exceeded 1048576 bytes");
      await route.abort("blockedbyclient");
      return;
    }
    try {
      const response = await route.fetch({ maxRedirects: 0, timeout: LIMITS.timeoutMs });
      const body = await response.body();
      if (body.byteLength > LIMITS.maxResponseBytes) {
        this.#failure = new AdapterError("response-too-large", "one loopback response exceeded 16777216 bytes");
        await route.abort("blockedbyclient");
        return;
      }
      if (this.#responseBytes + body.byteLength > LIMITS.maxAggregateResponseBytes) {
        this.#failure = new AdapterError("response-bytes-limit", "loopback responses exceeded 67108864 bytes");
        await route.abort("blockedbyclient");
        return;
      }
      this.#responseBytes += body.byteLength;
      this.#responses.push({
        method: request.method(),
        targetDigest: sha256(target.href),
        requestBodyDigest: sha256(requestBody),
        status: response.status(),
        body,
        responseDigest: sha256(body),
      });
      await route.fulfill({ response, body });
    } catch (error) {
      if (this.#failure !== null) throw this.#failure;
      if (error instanceof AdapterError) throw error;
      this.#failure = new AdapterError("loopback-request", "failed to capture a same-origin loopback response");
      throw this.#failure;
    }
  }

  public throwIfFailed(): void {
    if (this.#failure !== null) throw this.#failure;
    if (this.#externalRequests.size > 0) {
      throw new AdapterError("external-request", "page attempted a request outside the same-origin loopback allowlist");
    }
  }

  public source(): SourceBundle {
    this.throwIfFailed();
    if (this.#responses.length === 0) {
      throw new AdapterError("missing-response", "capture observed no loopback HTTP response");
    }
    const records = [...this.#responses].sort((left, right) => {
      const leftKey = `${left.method}\0${left.targetDigest}\0${left.requestBodyDigest}\0${left.status}\0${left.responseDigest}`;
      const rightKey = `${right.method}\0${right.targetDigest}\0${right.requestBodyDigest}\0${right.status}\0${right.responseDigest}`;
      return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
    });
    const digest = createHash("sha256");
    for (const response of records) {
      digest.update(canonicalJson({
        method: response.method,
        targetDigest: response.targetDigest,
        requestBodyDigest: response.requestBodyDigest,
        status: response.status,
        responseBytes: response.body.byteLength,
      }));
      digest.update(response.body);
    }
    return {
      digest: `sha256:${digest.digest("hex")}`,
      files: [],
      kind: "loopbackResponses",
      routePath: this.routePath,
      targetDigest: this.targetDigest,
      requestCount: records.length,
      responseBytes: this.#responseBytes,
      blockedWebSocketCount: this.#blockedWebSocketCount,
      blockedServiceWorkerCount: this.#blockedServiceWorkerCount,
    };
  }
}

function accessibilityRootEligible(node: BrowserNode): boolean {
  return node.explicitRole !== null || [
    "a",
    "aside",
    "button",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "img",
    "input",
    "main",
    "nav",
    "select",
    "table",
    "textarea",
  ].includes(node.tagName);
}

export function parseAccessibilitySnapshot(snapshot: string, rootEligible: boolean): AccessibilitySummary {
  const digest = sha256(snapshot);
  const firstLine = snapshot
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .find((line) => line.length > 0);
  if (firstLine === undefined || !rootEligible) {
    return {
      status: "cantTell",
      role: null,
      name: null,
      states: [],
      rootLine: null,
      snapshotDigest: digest,
      descendantsRedacted: true,
    };
  }
  const match = /^-\s+([A-Za-z][A-Za-z0-9_-]*)(?:\s+"((?:\\.|[^"\\])*)")?(?:\s+\[([^\]]+)\])?:?$/u.exec(firstLine);
  if (match === null || match[1] === undefined) {
    return {
      status: "cantTell",
      role: null,
      name: null,
      states: [],
      rootLine: firstLine,
      snapshotDigest: digest,
      descendantsRedacted: true,
    };
  }
  let name: string | null = null;
  if (match[2] !== undefined) {
    try {
      name = JSON.parse(`"${match[2]}"`) as string;
    } catch {
      name = null;
    }
  }
  return {
    status: "observed",
    role: match[1],
    name,
    states: match[3]?.split(/\s+/u).filter(Boolean) ?? [],
    rootLine: firstLine,
    snapshotDigest: digest,
    descendantsRedacted: true,
  };
}

async function collectAccessibility(page: Page, nodes: BrowserNode[]): Promise<CapturedNode[]> {
  const captured: CapturedNode[] = [];
  for (const node of nodes) {
    const snapshot = await page.locator(node.locator.selector).ariaSnapshot({ timeout: LIMITS.timeoutMs });
    captured.push({
      nodeId: nodeIdentifier(node.locator.value),
      browser: node,
      accessibility: parseAccessibilitySnapshot(snapshot, accessibilityRootEligible(node)),
    });
  }
  return captured.sort((left, right) =>
    left.nodeId < right.nodeId ? -1 : left.nodeId > right.nodeId ? 1 : 0,
  );
}

async function collectBrowserSnapshot(page: Page): Promise<BrowserSnapshot> {
  return page.evaluate(() => {
    type Locator = BrowserNode["locator"];
    const semanticTags = new Set([
      "a",
      "article",
      "aside",
      "button",
      "canvas",
      "footer",
      "form",
      "h1",
      "h2",
      "h3",
      "h4",
      "h5",
      "h6",
      "header",
      "img",
      "input",
      "main",
      "nav",
      "section",
      "select",
      "table",
      "textarea",
    ]);
    const allElements = [...document.querySelectorAll<HTMLElement>("*")];
    const candidates = allElements.filter((element) =>
      element.hasAttribute("data-testid") ||
      element.id.length > 0 ||
      element.hasAttribute("role") ||
      semanticTags.has(element.tagName.toLowerCase()),
    );
    const candidateSet = new Set(candidates);
    const duplicateLocators = new Set<string>();

    function structuralPath(element: Element): string {
      const parts: string[] = [];
      let current: Element | null = element;
      while (current !== null && current !== document.documentElement) {
        const tag = current.tagName.toLowerCase();
        const parent: Element | null = current.parentElement;
        if (parent === null) {
          parts.unshift(tag);
          break;
        }
        const sameTag = [...parent.children].filter((child) => child.tagName === current?.tagName);
        const suffix = sameTag.length > 1 ? `:nth-of-type(${sameTag.indexOf(current) + 1})` : "";
        parts.unshift(`${tag}${suffix}`);
        current = parent;
      }
      return `html > ${parts.join(" > ")}`;
    }

    function locatorFor(element: HTMLElement): Locator {
      const testId = element.getAttribute("data-testid");
      if (testId !== null && testId.length > 0) {
        const matches = allElements.filter((candidate) => candidate.getAttribute("data-testid") === testId);
        if (matches.length > 1) duplicateLocators.add(`testid:${testId}`);
        return {
          type: "testId",
          value: `testid:${testId}`,
          selector: `[data-testid="${CSS.escape(testId)}"]`,
        };
      }
      if (element.id.length > 0) {
        if (document.querySelectorAll(`#${CSS.escape(element.id)}`).length > 1) {
          duplicateLocators.add(`id:${element.id}`);
        }
        return { type: "id", value: `id:${element.id}`, selector: `#${CSS.escape(element.id)}` };
      }
      const selector = structuralPath(element);
      return { type: "css", value: `css:${selector}`, selector };
    }

    const locators = new Map(candidates.map((element) => [element, locatorFor(element)]));
    function rect(rectangle: DOMRect): RectValue {
      return {
        x: rectangle.x,
        y: rectangle.y,
        width: rectangle.width,
        height: rectangle.height,
      };
    }
    function layoutBox(element: HTMLElement): { rect: RectValue | null; reason: string | null } {
      let x = 0;
      let y = 0;
      let current: HTMLElement | null = element;
      let first = true;
      while (current !== null) {
        const style = getComputedStyle(current);
        if (!first && style.transform !== "none") {
          return { rect: null, reason: "ancestorTransform" };
        }
        if (style.zoom !== "1") {
          return { rect: null, reason: "cssZoom" };
        }
        x += current.offsetLeft;
        y += current.offsetTop;
        const offsetParent = current.offsetParent as HTMLElement | null;
        if (offsetParent !== null) {
          x += offsetParent.clientLeft;
          y += offsetParent.clientTop;
        }
        first = false;
        current = offsetParent;
      }
      return {
        rect: {
          x,
          y,
          width: element.offsetWidth,
          height: element.offsetHeight,
        },
        reason: null,
      };
    }
    function nearestCapturedParent(element: HTMLElement): string | null {
      let parent = element.parentElement;
      while (parent !== null) {
        if (candidateSet.has(parent)) return locators.get(parent)?.value ?? null;
        parent = parent.parentElement;
      }
      return null;
    }
    function nearestCapturedLocator(element: Element | null): string | null {
      let current = element;
      while (current instanceof HTMLElement) {
        const locator = locators.get(current);
        if (locator !== undefined) return locator.value;
        current = current.parentElement;
      }
      return null;
    }
    function isInteractive(element: HTMLElement, role: string | null): boolean {
      const tag = element.tagName.toLowerCase();
      return ["a", "button", "input", "select", "textarea"].includes(tag) ||
        element.tabIndex >= 0 ||
        (role !== null && ["button", "checkbox", "combobox", "link", "menuitem", "option", "radio", "slider", "spinbutton", "switch", "tab", "textbox"].includes(role));
    }
    function clippingAncestors(element: HTMLElement): BrowserNode["clippingAncestors"] {
      const ancestors: BrowserNode["clippingAncestors"] = [];
      let parent = element.parentElement;
      while (parent !== null) {
        const style = getComputedStyle(parent);
        if (["auto", "clip", "hidden", "scroll"].includes(style.overflowX) ||
            ["auto", "clip", "hidden", "scroll"].includes(style.overflowY)) {
          const parentRect = parent.getBoundingClientRect();
          ancestors.push({
            locator: locators.get(parent)?.value ?? `css:${structuralPath(parent)}`,
            overflowX: style.overflowX,
            overflowY: style.overflowY,
            rect: {
              x: parentRect.x + window.scrollX + parent.clientLeft,
              y: parentRect.y + window.scrollY + parent.clientTop,
              width: parent.clientWidth,
              height: parent.clientHeight,
            },
          });
        }
        parent = parent.parentElement;
      }
      return ancestors;
    }

    const nodes = candidates.map((element): BrowserNode => {
      const locator = locators.get(element);
      if (locator === undefined) throw new Error("missing locator");
      const style = getComputedStyle(element);
      const rendered = element.getBoundingClientRect();
      const role = element.getAttribute("role");
      const interactive = isInteractive(element, role);
      const centerX = rendered.left + rendered.width / 2;
      const centerY = rendered.top + rendered.height / 2;
      let hitOutcome: BrowserNode["centerHitSample"]["outcome"] = "notInteractive";
      let hitLocator: string | null = null;
      let hitMethod: BrowserNode["centerHitSample"]["method"] = "notSampled";
      if (interactive) {
        if (rendered.width <= 0 || rendered.height <= 0) {
          hitOutcome = "zeroArea";
        } else if (centerX < 0 || centerY < 0 || centerX >= window.innerWidth || centerY >= window.innerHeight) {
          hitOutcome = "offViewport";
        } else {
          const hit = document.elementFromPoint(centerX, centerY);
          hitMethod = "elementFromPointAtRenderBoxCenter";
          hitLocator = nearestCapturedLocator(hit);
          hitOutcome = hit !== null && element.contains(hit) ? "hit" : "occluded";
        }
      }
      const layout = layoutBox(element);
      return {
        locator,
        parentLocatorValue: nearestCapturedParent(element),
        tagName: element.tagName.toLowerCase(),
        explicitRole: role,
        ariaLabel: element.getAttribute("aria-label"),
        disabled: element.matches(":disabled") || element.getAttribute("aria-disabled") === "true",
        interactive,
        layoutBox: layout.rect,
        layoutUnavailableReason: layout.reason,
        renderBox: {
          ...rect(rendered),
          x: rendered.x + window.scrollX,
          y: rendered.y + window.scrollY,
        },
        clientSize: { width: element.clientWidth, height: element.clientHeight },
        scrollSize: { width: element.scrollWidth, height: element.scrollHeight },
        centerHitSample: {
          point: { x: centerX + window.scrollX, y: centerY + window.scrollY },
          outcome: hitOutcome,
          hitLocator,
          method: hitMethod,
        },
        computedStyle: {
          display: style.display,
          visibility: style.visibility,
          opacity: Number.parseFloat(style.opacity),
          overflowX: style.overflowX,
          overflowY: style.overflowY,
          whiteSpace: style.whiteSpace,
          textOverflow: style.textOverflow,
          fontSize: style.fontSize,
          lineHeight: style.lineHeight,
          fontWeight: style.fontWeight,
          direction: style.direction,
          writingMode: style.writingMode,
          transform: style.transform,
          pointerEvents: style.pointerEvents,
        },
        clippingAncestors: clippingAncestors(element),
      };
    });
    return {
      nodes,
      duplicateLocators: [...duplicateLocators].sort(),
      scroll: { x: window.scrollX, y: window.scrollY },
      documentSize: {
        width: Math.max(
          document.documentElement.clientWidth,
          document.documentElement.scrollWidth,
          document.body?.clientWidth ?? 0,
          document.body?.scrollWidth ?? 0,
        ),
        height: Math.max(
          document.documentElement.clientHeight,
          document.documentElement.scrollHeight,
          document.body?.clientHeight ?? 0,
          document.body?.scrollHeight ?? 0,
        ),
      },
      documentDirection: getComputedStyle(document.documentElement).direction,
    };
  });
}

function roundedRect(rect: RectValue): RectValue {
  return {
    x: roundCss(rect.x),
    y: roundCss(rect.y),
    width: roundCss(rect.width),
    height: roundCss(rect.height),
  };
}

function observedRect(rect: RectValue, evidenceId: string, coordinateSpaceId = "document"): JsonObject {
  return {
    rect: roundedRect(rect),
    coordinateSpaceId,
    evidenceId,
  } as unknown as JsonObject;
}

function nodeKind(node: BrowserNode): string {
  if (node.interactive) return "control";
  if (node.tagName === "img") return "image";
  if (node.tagName === "table") return "table";
  if (/^h[1-6]$/u.test(node.tagName)) return "text";
  if (node.tagName === "canvas") return "chart";
  return "container";
}

function geometryCoverage(rect: RectValue, viewport: { width: number; height: number }): string {
  if (rect.width === 0 || rect.height === 0) return "zeroArea";
  const left = Math.max(0, rect.x);
  const top = Math.max(0, rect.y);
  const right = Math.min(viewport.width, rect.x + rect.width);
  const bottom = Math.min(viewport.height, rect.y + rect.height);
  if (right <= left || bottom <= top) return "outsideScreenshot";
  if (left === rect.x && top === rect.y && right === rect.x + rect.width && bottom === rect.y + rect.height) {
    return "insideScreenshotExtent";
  }
  return "partiallyOutsideScreenshot";
}

function translateToViewport(rect: RectValue, scroll: { x: number; y: number }): RectValue {
  return { ...rect, x: rect.x - scroll.x, y: rect.y - scroll.y };
}

function rectanglesAgree(left: RectValue, right: RectValue, tolerance: number): boolean {
  return (["x", "y", "width", "height"] as const).every(
    (field) => Math.abs(left[field] - right[field]) <= tolerance,
  );
}

function ancestorClip(node: BrowserNode): JsonObject {
  const render = node.renderBox;
  if (render.width <= 0 || render.height <= 0) {
    return {
      status: "cantTell",
      reason: "a zero-area render box cannot establish ancestor clipping",
    };
  }
  let left = render.x;
  let top = render.y;
  let right = render.x + render.width;
  let bottom = render.y + render.height;
  const clippingAncestorLocators: string[] = [];
  for (const ancestor of node.clippingAncestors) {
    clippingAncestorLocators.push(ancestor.locator);
    if (["auto", "clip", "hidden", "scroll"].includes(ancestor.overflowX)) {
      left = Math.max(left, ancestor.rect.x);
      right = Math.min(right, ancestor.rect.x + ancestor.rect.width);
    }
    if (["auto", "clip", "hidden", "scroll"].includes(ancestor.overflowY)) {
      top = Math.max(top, ancestor.rect.y);
      bottom = Math.min(bottom, ancestor.rect.y + ancestor.rect.height);
    }
  }
  const visibleWidth = Math.max(0, right - left);
  const visibleHeight = Math.max(0, bottom - top);
  const status = visibleWidth === 0 || visibleHeight === 0
    ? "fullyClipped"
    : visibleWidth < render.width || visibleHeight < render.height
      ? "partiallyClipped"
      : "notClipped";
  return {
    status,
    clippingAncestorLocators,
    method: "rectangularOverflowAncestorIntersection",
  };
}

function evidenceRecord(
  id: string,
  evidenceClass: string,
  sourceDigest: string,
  adapterVersion: string,
  nativeId?: string,
): JsonObject {
  const value: JsonObject = {
    id,
    class: evidenceClass,
    source: {
      adapter: ADAPTER_NAME,
      adapterVersion,
      inputDigest: sourceDigest,
      externalProcessing: false,
    },
  };
  if (nativeId !== undefined) {
    value.selector = { type: "nativeId", nativeId };
  }
  return value;
}

function buildArtifactIr(
  request: CaptureRequest,
  nodes: CapturedNode[],
  snapshot: BrowserSnapshot,
  source: SourceBundle,
  screenshot: { reference: string; digest: string; byteLength: number; width: number; height: number },
  browserVersion: string,
  externalRequests: string[],
): JsonObject {
  const managed = request.protocolVersion === MANAGED_PROTOCOL_VERSION;
  const adapterVersion = managed ? ADAPTER_VERSION : LEGACY_ADAPTER_VERSION;
  const extensionVersion = managed ? WEB_EXTENSION_VERSION : LEGACY_WEB_EXTENSION_VERSION;
  const sourcePath = managed ? source.routePath : request.fixture.entrypoint;
  const state = managed ? request.target.state : request.fixture.state;
  const readinessSelector = managed
    ? request.target.readinessSelector
    : request.fixture.readinessSelector;
  if (sourcePath === undefined) {
    throw new AdapterError("invalid-capture-output", "managed capture is missing its route path");
  }
  if (managed && (source.blockedWebSocketCount === undefined || source.blockedServiceWorkerCount === undefined)) {
    throw new AdapterError("invalid-capture-output", "managed capture is missing blocked transport counts");
  }
  const locatorToId = new Map(nodes.map((node) => [node.browser.locator.value, node.nodeId]));
  const evidence: JsonObject[] = [
    evidenceRecord("e-web-viewport", "exactRender", source.digest, adapterVersion),
    evidenceRecord("e-web-screenshot", "exactRender", source.digest, adapterVersion),
  ];
  const coreNodes: JsonObject[] = [];
  const extensionNodes: JsonObject[] = [];
  const reconciliationNodes: JsonObject[] = [];
  for (const node of nodes) {
    const domEvidence = `e-dom-${node.nodeId}`;
    const renderEvidence = `e-render-${node.nodeId}`;
    const axEvidence = `e-ax-${node.nodeId}`;
    evidence.push(evidenceRecord(domEvidence, "exactSource", source.digest, adapterVersion, node.browser.locator.value));
    evidence.push(evidenceRecord(renderEvidence, "exactRender", source.digest, adapterVersion, node.browser.locator.value));
    if (node.accessibility.status === "observed") {
      evidence.push(evidenceRecord(axEvidence, "platformSemantics", source.digest, adapterVersion, node.browser.locator.value));
    }
    const geometry: JsonObject = {
      renderBox: observedRect(node.browser.renderBox, renderEvidence),
    };
    if (node.browser.layoutBox !== null) {
      geometry.layoutBox = observedRect(node.browser.layoutBox, renderEvidence);
    }
    const core: JsonObject = {
      id: node.nodeId,
      kind: { value: nodeKind(node.browser), evidenceId: domEvidence },
      coordinateSpaceId: "document",
      geometry,
    };
    const parentId = node.browser.parentLocatorValue === null
      ? undefined
      : locatorToId.get(node.browser.parentLocatorValue);
    if (parentId !== undefined) core.parentId = parentId;
    if (node.accessibility.status === "observed" && node.accessibility.role !== null) {
      core.role = { value: node.accessibility.role, evidenceId: axEvidence };
      if (node.accessibility.name !== null && node.accessibility.name.length > 0) {
        core.name = { value: node.accessibility.name, evidenceId: axEvidence };
      }
    } else if (node.browser.explicitRole !== null) {
      core.role = { value: node.browser.explicitRole, evidenceId: domEvidence };
      if (node.browser.ariaLabel !== null) {
        core.name = { value: node.browser.ariaLabel, evidenceId: domEvidence };
      }
    }
    coreNodes.push(core);
    extensionNodes.push({
      nodeId: node.nodeId,
      locator: node.browser.locator as unknown as JsonObject,
      tagName: node.browser.tagName,
      domEvidenceId: domEvidence,
      renderEvidenceId: renderEvidence,
      accessibilityEvidenceId: node.accessibility.status === "observed" ? axEvidence : null,
      disabled: node.browser.disabled,
      interactive: node.browser.interactive,
      layoutMethod: node.browser.layoutBox === null ? "unavailable" : "offsetParentBorderBoxToDocument",
      layoutUnavailableReason: node.browser.layoutUnavailableReason,
      renderMethod: "getBoundingClientRect",
      clientSize: { ...node.browser.clientSize, unit: "cssPixel" },
      scrollSize: { ...node.browser.scrollSize, unit: "cssPixel" },
      overflowMeasurement: {
        horizontal: node.browser.scrollSize.width > node.browser.clientSize.width ? "present" : "absent",
        vertical: node.browser.scrollSize.height > node.browser.clientSize.height ? "present" : "absent",
        method: "scrollSizeComparedWithClientSize",
      },
      centerHitSample: {
        point: {
          x: roundCss(node.browser.centerHitSample.point.x),
          y: roundCss(node.browser.centerHitSample.point.y),
          unit: "cssPixel",
        },
        outcome: node.browser.centerHitSample.outcome,
        hitLocator: node.browser.centerHitSample.hitLocator,
        method: node.browser.centerHitSample.method,
      },
      hitRegion: {
        status: "cantTell",
        reason: "a center-point sample does not measure the complete activation region",
      },
      computedStyle: node.browser.computedStyle as unknown as JsonObject,
      clippingAncestors: node.browser.clippingAncestors.map((ancestor) => ({
        locator: ancestor.locator,
        overflowX: ancestor.overflowX,
        overflowY: ancestor.overflowY,
        rect: roundedRect(ancestor.rect),
      })) as unknown as JsonValue,
      accessibility: node.accessibility as unknown as JsonObject,
    });
    reconciliationNodes.push({
      nodeId: node.nodeId,
      screenshotGeometryCoverage: geometryCoverage(
        translateToViewport(node.browser.renderBox, snapshot.scroll),
        request.environment.viewport,
      ),
      nativeVisibility: {
        display: node.browser.computedStyle.display,
        visibility: node.browser.computedStyle.visibility,
        opacity: node.browser.computedStyle.opacity,
        centerHitTest: node.browser.centerHitSample.outcome,
      },
      layoutRender: (node.browser.layoutBox === null
        ? { status: "cantTell", reason: node.browser.layoutUnavailableReason }
        : {
            status: rectanglesAgree(node.browser.layoutBox, node.browser.renderBox, 1)
              ? "agreement"
              : "conflict",
            tolerance: { value: 1, unit: "cssPixel" },
            layoutBox: roundedRect(node.browser.layoutBox),
            renderBox: roundedRect(node.browser.renderBox),
          }) as unknown as JsonValue,
      ancestorClip: ancestorClip(node.browser),
      pixelContentMatch: {
        status: "cantTell",
        reason: `web extension ${extensionVersion.slice(0, 3)} does not perform pixel-content segmentation or identity matching`,
      },
    });
  }
  evidence.sort((left, right) => String(left.id) < String(right.id) ? -1 : String(left.id) > String(right.id) ? 1 : 0);
  const captureRecord: JsonObject = {
    sourceFiles: source.files,
    sourceDigest: source.digest,
    screenshot: {
      reference: screenshot.reference,
      sha256: screenshot.digest,
      byteLength: screenshot.byteLength,
      pixelSize: { width: screenshot.width, height: screenshot.height },
      format: "png",
      scale: "css",
      animations: "disabled",
      caret: "hide",
      colorAssumptions: "PNG encoded channels; no colorimetric or compositing claim",
    },
    network: managed
      ? {
          mode: "sameOriginLoopback",
          externalRequests,
          blockedWebSocketCount: source.blockedWebSocketCount ?? 0,
          blockedServiceWorkerCount: source.blockedServiceWorkerCount ?? 0,
        }
      : { mode: "deny", externalRequests },
    privacy: { accessibleNameMode: "selectedNodes", descendantsRedacted: true, externalProcessing: false },
  };
  if (managed) {
    captureRecord.sourceKind = "loopbackResponses";
    captureRecord.loopbackResponses = {
      digest: source.digest,
      requestCount: source.requestCount,
      responseBytes: source.responseBytes,
      routePath: sourcePath,
      targetDigest: source.targetDigest,
    } as unknown as JsonObject;
  }
  const extension: JsonObject = {
    extensionVersion,
    document: {
      id: "document-main",
      frameId: "frame-main",
      frames: [{ id: "frame-main", parentId: null, kind: "main", sourcePath }],
      frameCount: 1,
      sourcePath,
      state,
      readinessSelector,
      scroll: { x: roundCss(snapshot.scroll.x), y: roundCss(snapshot.scroll.y), unit: "cssPixel" },
      documentSize: { ...snapshot.documentSize, unit: "cssPixel" },
      viewportSize: request.environment.viewport,
      direction: snapshot.documentDirection,
    },
    environment: {
      ...request.environment,
      browserName: "chromium",
      browserVersion,
      playwrightVersion: PLAYWRIGHT_VERSION,
      platform: process.platform,
      architecture: process.arch,
    } as unknown as JsonObject,
    capture: captureRecord,
    nodes: extensionNodes,
    reconciliation: {
      screenshotViewport: {
        status: screenshot.width === request.environment.viewport.width && screenshot.height === request.environment.viewport.height
          ? "agreement"
          : "conflict",
        viewportCssPixels: request.environment.viewport,
        screenshotPixels: { width: screenshot.width, height: screenshot.height },
        screenshotScale: "css",
      },
      nodes: reconciliationNodes,
      pixelContentComparison: {
        status: "cantTell",
        reason: `pixel-content identity is outside web extension ${extensionVersion.slice(0, 3)}`,
      },
    },
  };
  return {
    schemaVersion: "0.1.0",
    artifact: {
      id: request.artifact.id,
      kind: "web",
      title: request.artifact.title,
      sourceName: managed
        ? `${sourcePath}#state=${state}&textScale=${request.environment.textScale}`
        : `${request.fixture.entrypoint}?case=${request.fixture.state}&textScale=${request.environment.textScale}`,
    },
    canvases: [
      {
        id: "document",
        size: { width: snapshot.documentSize.width, height: snapshot.documentSize.height },
        unit: "cssPixel",
        horizontalDirection: "right",
        verticalDirection: "down",
        evidenceId: "e-web-viewport",
      },
      {
        id: "viewport",
        size: { width: request.environment.viewport.width, height: request.environment.viewport.height },
        unit: "cssPixel",
        horizontalDirection: "right",
        verticalDirection: "down",
        evidenceId: "e-web-viewport",
      },
    ],
    nodes: coreNodes,
    evidence,
    extensions: { [WEB_EXTENSION_KEY]: extension },
  };
}

function pngDimensions(bytes: Buffer): { width: number; height: number } {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (bytes.byteLength < 24 || !bytes.subarray(0, 8).equals(signature) || bytes.toString("ascii", 12, 16) !== "IHDR") {
    throw new AdapterError("invalid-screenshot", "Playwright did not return a valid PNG header");
  }
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

async function writeExclusive(path: string, bytes: string | Uint8Array): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes, { flag: "wx" }).catch((error: NodeJS.ErrnoException) => {
    if (error.code === "EEXIST") {
      throw new AdapterError("output-exists", "refusing to overwrite an existing output file");
    }
    throw new AdapterError("output-write", "failed to write capture output");
  });
}

async function writeOutputPair(outputs: CaptureOutputs, artifactIr: string, screenshot: Uint8Array): Promise<void> {
  const created: string[] = [];
  try {
    await writeExclusive(outputs.artifactIrPath, artifactIr);
    created.push(outputs.artifactIrPath);
    await writeExclusive(outputs.screenshotPath, screenshot);
    created.push(outputs.screenshotPath);
  } catch (error) {
    await Promise.all(created.map(async (path) => rm(path, { force: true })));
    throw error;
  }
}

async function configureLegacyContext(
  context: BrowserContext,
  root: string,
  resourcePaths: Set<string>,
  externalRequests: Set<string>,
): Promise<void> {
  context.setDefaultTimeout(LIMITS.timeoutMs);
  context.setDefaultNavigationTimeout(LIMITS.timeoutMs);
  await context.route("**/*", async (route) => routeRequest(route, root, resourcePaths, externalRequests));
}

async function configureManagedContext(
  context: BrowserContext,
  recorder: ManagedNetworkRecorder,
): Promise<void> {
  context.setDefaultTimeout(LIMITS.timeoutMs);
  context.setDefaultNavigationTimeout(LIMITS.timeoutMs);
  await context.exposeBinding("__sightlintRecordBlockedServiceWorker", async () => {
    recorder.blockServiceWorker();
  });
  await context.addInitScript(() => {
    const scope = globalThis as typeof globalThis & {
      __sightlintRecordBlockedServiceWorker?: () => Promise<void>;
    };
    if ("serviceWorker" in navigator) {
      const serviceWorker = navigator.serviceWorker;
      const replacement = async (): Promise<never> => {
        await scope.__sightlintRecordBlockedServiceWorker?.();
        throw new DOMException("Service workers are blocked by SightLint", "SecurityError");
      };
      for (const target of [serviceWorker, Object.getPrototypeOf(serviceWorker) as object]) {
        try {
          Object.defineProperty(target, "register", {
            configurable: true,
            value: replacement,
          });
          break;
        } catch {
          // Try the prototype when Chromium rejects an instance property.
        }
      }
    }
  });
  await context.routeWebSocket("**/*", (webSocket) => {
    recorder.blockWebSocket(webSocket.url());
    webSocket.close({ code: 1008, reason: "blocked by SightLint" });
  });
  await context.route("**/*", async (route) => recorder.route(route));
}

async function captureWithBrowser(
  request: CaptureRequest,
  root: string,
  destination: string,
  outputs: CaptureOutputs,
  resourcePaths: Set<string>,
  recorder: ManagedNetworkRecorder | null,
  managedServer: CaptureResponse["capture"]["managedServer"] | undefined,
): Promise<CaptureResponse> {
  if (request.environment.viewport.width * request.environment.viewport.height > LIMITS.maxScreenshotPixels) {
    throw new AdapterError("screenshot-budget", "requested screenshot exceeds the pixel budget");
  }
  const requestBytes = canonicalJson(request as unknown as JsonValue);
  const externalRequests = new Set<string>();
  const managed = request.protocolVersion === MANAGED_PROTOCOL_VERSION;
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
  try {
    const context = await browser.newContext({
      viewport: request.environment.viewport,
      deviceScaleFactor: request.environment.deviceScaleFactor,
      locale: request.environment.locale,
      timezoneId: request.environment.timezoneId,
      colorScheme: request.environment.colorScheme,
      reducedMotion: request.environment.reducedMotion,
      serviceWorkers: "block",
      offline: !managed,
      permissions: [],
    });
    if (recorder === null) {
      await configureLegacyContext(context, root, resourcePaths, externalRequests);
    } else {
      await configureManagedContext(context, recorder);
    }
    const page = await context.newPage();
    let navigationResponse;
    if (!managed) {
      navigationResponse = await page.goto(destination, { waitUntil: "load", timeout: LIMITS.timeoutMs });
    } else {
      try {
        navigationResponse = await page.goto(destination, { waitUntil: "load", timeout: LIMITS.timeoutMs });
      } catch {
        recorder?.throwIfFailed();
        throw new AdapterError("navigation", "failed to load the requested page");
      }
    }
    if (managed) {
      const finalUrl = new URL(page.url());
      const allowedOrigin = `http://127.0.0.1:${request.server.port}`;
      if (finalUrl.origin !== allowedOrigin) {
        throw new AdapterError("external-navigation", "final navigation left the same-origin loopback target");
      }
      if (navigationResponse === null || navigationResponse.status() < 200 || navigationResponse.status() > 299) {
        throw new AdapterError("http-status", "final loopback navigation did not return HTTP 2xx");
      }
    }
    const readinessSelector = managed
      ? request.target.readinessSelector
      : request.fixture.readinessSelector;
    await page.waitForSelector(readinessSelector, { state: "attached", timeout: LIMITS.timeoutMs });
    if (!managed) {
      await page.evaluate(async () => document.fonts.ready);
    } else {
      let fontTimeout: ReturnType<typeof setTimeout> | undefined;
      try {
        await Promise.race([
          page.evaluate(async () => document.fonts.ready),
          new Promise<never>((_resolve, reject) => {
            fontTimeout = setTimeout(
              () => reject(new AdapterError("fonts-timeout", "document fonts did not become ready within 20000 ms")),
              LIMITS.timeoutMs,
            );
          }),
        ]);
      } finally {
        clearTimeout(fontTimeout);
      }
    }
    recorder?.throwIfFailed();
    const frameCount = page.frames().length;
    if (frameCount > LIMITS.maxFrames) {
      throw new AdapterError("frame-budget", `capture exceeds ${LIMITS.maxFrames} frames`);
    }
    if (frameCount !== 1 || context.pages().length !== 1) {
      throw new AdapterError("unsupported-frame", `protocol ${request.protocolVersion.slice(0, 3)} supports one main frame and one page`);
    }
    const snapshot = await collectBrowserSnapshot(page);
    if (snapshot.duplicateLocators.length > 0) {
      throw new AdapterError("duplicate-locator", "fixture contains a duplicate preferred stable locator");
    }
    if (snapshot.nodes.length === 0 || snapshot.nodes.length > LIMITS.maxNodes) {
      throw new AdapterError("node-budget", `capture node count must be from 1 through ${LIMITS.maxNodes}`);
    }
    const nodes = await collectAccessibility(page, snapshot.nodes);
    const screenshotBytes = await page.screenshot({
      type: "png",
      fullPage: false,
      animations: "disabled",
      caret: "hide",
      scale: "css",
    });
    if (screenshotBytes.byteLength > LIMITS.maxScreenshotBytes) {
      throw new AdapterError("screenshot-budget", "captured screenshot exceeds the byte budget");
    }
    if (recorder === null && externalRequests.size > 0) {
      throw new AdapterError("external-request", "fixture attempted a request outside the local allowlist");
    }
    const dimensions = pngDimensions(screenshotBytes);
    const source = recorder === null ? await sourceBundle(root, resourcePaths) : recorder.source();
    const browserVersion = browser.version();
    const screenshotDigest = sha256(screenshotBytes);
    const artifactIr = buildArtifactIr(
      request,
      nodes,
      snapshot,
      source,
      {
        reference: request.screenshot.reference,
        digest: screenshotDigest,
        byteLength: screenshotBytes.byteLength,
        width: dimensions.width,
        height: dimensions.height,
      },
      browserVersion,
      recorder === null ? [...externalRequests].sort() : recorder.externalRequests,
    );
    const artifactIrBytes = canonicalJson(artifactIr);
    if (Buffer.byteLength(artifactIrBytes) > LIMITS.maxOutputBytes) {
      throw new AdapterError("output-budget", "canonical Artifact IR exceeds the output budget");
    }
    const responseCapture: CaptureResponse["capture"] = {
      pageCount: 1,
      frameCount,
      nodeCount: nodes.length,
      externalRequests: [],
      deterministicOptions: managed
        ? {
            headless: true,
            offline: false,
            serviceWorkers: "blockAndCountRegistration",
            webSockets: "blockAndCount",
            wait: ["tcpListen", "finalHttp2xx", "load", readinessSelector, "document.fonts.ready"],
            screenshot: { animations: "disabled", caret: "hide", scale: "css", format: "png" },
          }
        : {
            headless: true,
            offline: true,
            serviceWorkers: "block",
            wait: ["load", readinessSelector, "document.fonts.ready"],
            screenshot: { animations: "disabled", caret: "hide", scale: "css", format: "png" },
          },
    };
    if (managed) {
      if (source.routePath === undefined || source.targetDigest === undefined || source.requestCount === undefined || source.responseBytes === undefined || source.blockedWebSocketCount === undefined || source.blockedServiceWorkerCount === undefined || managedServer === undefined) {
        throw new AdapterError("invalid-capture-output", "managed capture summary is incomplete");
      }
      responseCapture.loopbackResponses = {
        digest: source.digest,
        requestCount: source.requestCount,
        responseBytes: source.responseBytes,
        routePath: source.routePath,
        targetDigest: source.targetDigest,
        blockedWebSocketCount: source.blockedWebSocketCount,
        blockedServiceWorkerCount: source.blockedServiceWorkerCount,
      };
      responseCapture.managedServer = managedServer;
    }
    const response: CaptureResponse = {
      protocolVersion: request.protocolVersion,
      status: "captured",
      adapter: {
        name: ADAPTER_NAME,
        version: managed ? ADAPTER_VERSION : LEGACY_ADAPTER_VERSION,
        nodeVersion: process.versions.node,
        platform: process.platform,
        architecture: process.arch,
        playwrightVersion: PLAYWRIGHT_VERSION,
        browserName: "chromium",
        browserVersion,
      },
      requestDigest: sha256(requestBytes),
      sourceDigest: source.digest,
      artifactIr: {
        sha256: sha256(artifactIrBytes),
        byteLength: Buffer.byteLength(artifactIrBytes),
      },
      screenshot: {
        reference: request.screenshot.reference,
        sha256: screenshotDigest,
        byteLength: screenshotBytes.byteLength,
        format: "png",
        pixelSize: dimensions,
        colorAssumptions: "PNG encoded channels; no colorimetric or compositing claim",
      },
      capture: responseCapture,
      limitations: managed
        ? [
            "pixel-content matching is cantTell in protocol 0.2.0",
            "complete hit regions are cantTell; center-point samples are not hit rectangles",
            "host font and raster differences are recorded but not normalized across platforms",
            "loopback locators do not establish source file or line attribution",
            "the explicitly authorized server command is not sandboxed and its own network is not controlled",
          ]
        : [
            "pixel-content matching is cantTell in protocol 0.1.0",
            "complete hit regions are cantTell; center-point samples are not hit rectangles",
            "host font and raster differences are recorded but not normalized across platforms",
            "only repository-contained file fixtures are supported",
          ],
    };
    const responseBytes = canonicalJson(response as unknown as JsonValue);
    if (Buffer.byteLength(responseBytes) > LIMITS.maxOutputBytes) {
      throw new AdapterError("output-budget", "canonical response exceeds the output budget");
    }
    await writeOutputPair(outputs, artifactIrBytes, screenshotBytes);
    await context.close();
    return response;
  } finally {
    await browser.close();
  }
}

export async function capture(
  request: CaptureRequest,
  repositoryRoot: string,
  outputs: CaptureOutputs,
  options: CaptureOptions = { allowServerCommand: false },
): Promise<CaptureResponse> {
  if (request.protocolVersion === LEGACY_PROTOCOL_VERSION) {
    const resolved = await resolveFixture(repositoryRoot, request.fixture.entrypoint);
    const destination = pathToFileURL(resolved.entrypoint);
    destination.searchParams.set("case", request.fixture.state);
    destination.searchParams.set("textScale", String(request.environment.textScale));
    return captureWithBrowser(
      request,
      resolved.root,
      destination.href,
      outputs,
      new Set<string>([resolved.entrypoint]),
      null,
      undefined,
    );
  }
  if (!options.allowServerCommand) {
    throw new AdapterError(
      "server-command-not-allowed",
      "managed loopback capture requires --allow-server-command",
    );
  }
  const root = await resolveRepositoryRoot(repositoryRoot);
  const recorder = new ManagedNetworkRecorder(request.server.port, request.target.pathAndQuery);
  const server = new ManagedServer(request, root);
  await server.start();
  try {
    return await server.guard(captureWithBrowser(
      request,
      root,
      recorder.destination,
      outputs,
      new Set<string>(),
      recorder,
      server.record,
    ));
  } finally {
    await server.stop();
  }
}
