import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");
const webExtensionKey = "org.sightlint.web";

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface ExpectedNode {
  id: string;
  parentId: string | null;
  accessibilityStatus: string;
  role: string | null;
  name: string | null;
  display: string;
  fontSize?: string;
  layoutRenderStatus: string;
  screenshotGeometryCoverage: string;
  renderOffset?: { x: number; y: number; tolerance: number; unit: string };
}

interface AcquisitionCase {
  caseId: string;
  request: string;
  split: string;
  classification: string;
  expectations: {
    sourceFiles: string[];
    viewport: { width: number; height: number; unit: string };
    minimumDocumentHeight: number;
    frameCount: number;
    minimumNodeCount: number;
    coreRelationCount: number;
    screenshotViewport: string;
    pixelContentComparison: string;
    nodes: ExpectedNode[];
    gaps: Array<{ from: string; to: string; axis: "horizontal" | "vertical"; value: number; tolerance: number; unit: string }>;
  };
  abstentions: Array<{ aspect: string; outcome: string; rationale: string }>;
}

interface RuleCase {
  caseId: string;
  request: string;
  classification: string;
  expectedExitCode: number;
  expectedFailureCount: number;
  expectedResults: Array<{
    ruleId: string;
    ruleVersion: string;
    outcome: string;
    targetKind: string;
    targetId: string;
    targetAspect: string | null;
  }>;
  falsePositiveRisk: string;
  nonClaim: string;
}

interface CaptureRun {
  directory: string;
  process: ProcessResult;
  responseBytes: Buffer;
  artifactIrBytes: Buffer;
  screenshotBytes: Buffer;
  response: Record<string, unknown>;
  artifactIr: Record<string, unknown>;
}

interface RuleMetrics {
  emittedFailures: number;
  matchedFailures: number;
  unexpectedFailures: number;
  abstentions: number;
}

function run(program: string, args: string[], cwd = repositoryRoot): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, { cwd, env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    child.stdout.on("data", (chunk: Buffer) => stdout.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderr.push(chunk));
    child.on("error", reject);
    child.on("close", (code, signal) => {
      if (signal !== null) {
        reject(new Error(`${program} terminated by ${signal}`));
        return;
      }
      resolveRun({ code: code ?? -1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) });
    });
  });
}

async function loadJson(path: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(resolve(repositoryRoot, path), "utf8")) as Record<string, unknown>;
}

function array(value: unknown, context: string): Array<Record<string, unknown>> {
  assert.ok(Array.isArray(value), `${context} must be an array`);
  return value as Array<Record<string, unknown>>;
}

function object(value: unknown, context: string): Record<string, unknown> {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${context} must be an object`);
  return value as Record<string, unknown>;
}

function string(value: unknown, context: string): string {
  assert.equal(typeof value, "string", `${context} must be a string`);
  return value as string;
}

function number(value: unknown, context: string): number {
  assert.equal(typeof value, "number", `${context} must be a number`);
  return value as number;
}

function indexBy(items: Array<Record<string, unknown>>, field: string): Map<string, Record<string, unknown>> {
  return new Map(items.map((item) => [string(item[field], field), item]));
}

function digest(bytes: Buffer): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

async function capture(requestPath: string): Promise<CaptureRun> {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-playwright-e2e-"));
  const artifactIrPath = join(directory, "artifact-ir.json");
  const screenshotPath = join(directory, "screenshot.png");
  const processResult = await run(process.execPath, [
    adapterCli,
    "--request", resolve(repositoryRoot, requestPath),
    "--repository-root", repositoryRoot,
    "--artifact-ir-out", artifactIrPath,
    "--screenshot-out", screenshotPath,
  ]);
  assert.equal(processResult.code, 0, processResult.stderr.toString("utf8"));
  assert.equal(processResult.stderr.byteLength, 0);
  const artifactIrBytes = await readFile(artifactIrPath);
  const screenshotBytes = await readFile(screenshotPath);
  return {
    directory,
    process: processResult,
    responseBytes: processResult.stdout,
    artifactIrBytes,
    screenshotBytes,
    response: JSON.parse(processResult.stdout.toString("utf8")) as Record<string, unknown>,
    artifactIr: JSON.parse(artifactIrBytes.toString("utf8")) as Record<string, unknown>,
  };
}

async function schemaValidator(path: string): Promise<ValidateFunction> {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  return ajv.compile(await loadJson(path));
}

function assertValid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), true, `${context}: ${JSON.stringify(validate.errors)}`);
}

function assertAcquisition(runResult: CaptureRun, oracle: AcquisitionCase): void {
  const response = runResult.response;
  const ir = runResult.artifactIr;
  const extension = object(object(ir["extensions"], "IR extensions")[webExtensionKey], "Web extension");
  const document = object(extension["document"], "Web document");
  const captureRecord = object(extension["capture"], "Web capture");
  const reconciliation = object(extension["reconciliation"], "Web reconciliation");
  const responseCapture = object(response["capture"], "capture response");

  assert.deepEqual(captureRecord["sourceFiles"], oracle.expectations.sourceFiles);
  assert.deepEqual(document["viewportSize"], oracle.expectations.viewport);
  assert.ok(number(object(document["documentSize"], "document size")["height"], "document height") >= oracle.expectations.minimumDocumentHeight);
  assert.equal(document["frameCount"], oracle.expectations.frameCount);
  assert.equal(responseCapture["frameCount"], oracle.expectations.frameCount);
  assert.ok(array(ir["nodes"], "IR nodes").length >= oracle.expectations.minimumNodeCount);
  assert.ok(number(responseCapture["nodeCount"], "response node count") >= oracle.expectations.minimumNodeCount);
  assert.equal(array(ir["relations"] ?? [], "IR relations").length, oracle.expectations.coreRelationCount);
  assert.equal(object(reconciliation["screenshotViewport"], "screenshot viewport")["status"], oracle.expectations.screenshotViewport);
  assert.equal(object(reconciliation["pixelContentComparison"], "pixel comparison")["status"], oracle.expectations.pixelContentComparison);
  assert.ok(oracle.abstentions.some((item) => item.outcome === "cantTell" || item.outcome === "untested"));

  const coreNodes = indexBy(array(ir["nodes"], "IR nodes"), "id");
  const extensionNodes = indexBy(array(extension["nodes"], "extension nodes"), "nodeId");
  const reconciliationNodes = indexBy(array(reconciliation["nodes"], "reconciliation nodes"), "nodeId");
  for (const expected of oracle.expectations.nodes) {
    const core = coreNodes.get(expected.id);
    const acquired = extensionNodes.get(expected.id);
    const reconciled = reconciliationNodes.get(expected.id);
    assert.ok(core, `missing core node ${expected.id}`);
    assert.ok(acquired, `missing extension node ${expected.id}`);
    assert.ok(reconciled, `missing reconciliation node ${expected.id}`);
    assert.equal(core["parentId"] ?? null, expected.parentId, `${expected.id} parent`);
    assert.equal(object(acquired["accessibility"], `${expected.id} accessibility`)["status"], expected.accessibilityStatus);
    assert.equal(object(acquired["accessibility"], `${expected.id} accessibility`)["role"], expected.role);
    assert.equal(object(acquired["accessibility"], `${expected.id} accessibility`)["name"], expected.name);
    assert.equal(object(acquired["computedStyle"], `${expected.id} style`)["display"], expected.display);
    if (expected.fontSize !== undefined) {
      assert.equal(object(acquired["computedStyle"], `${expected.id} style`)["fontSize"], expected.fontSize);
    }
    const layoutRender = object(reconciled["layoutRender"], `${expected.id} layout/render`);
    assert.equal(layoutRender["status"], expected.layoutRenderStatus);
    assert.equal(reconciled["screenshotGeometryCoverage"], expected.screenshotGeometryCoverage);
    assert.equal(object(reconciled["pixelContentMatch"], `${expected.id} pixel match`)["status"], "cantTell");
    if (expected.renderOffset !== undefined) {
      const layout = object(layoutRender["layoutBox"], `${expected.id} layout box`);
      const render = object(layoutRender["renderBox"], `${expected.id} render box`);
      const actualX = number(render["x"], "render x") - number(layout["x"], "layout x");
      const actualY = number(render["y"], "render y") - number(layout["y"], "layout y");
      assert.ok(Math.abs(actualX - expected.renderOffset.x) <= expected.renderOffset.tolerance, `${expected.id} x offset ${actualX}`);
      assert.ok(Math.abs(actualY - expected.renderOffset.y) <= expected.renderOffset.tolerance, `${expected.id} y offset ${actualY}`);
    }
  }

  for (const expected of oracle.expectations.gaps) {
    const from = object(object(coreNodes.get(expected.from), expected.from)["geometry"], `${expected.from} geometry`);
    const to = object(object(coreNodes.get(expected.to), expected.to)["geometry"], `${expected.to} geometry`);
    const fromRect = object(object(from["renderBox"], `${expected.from} renderBox`)["rect"], `${expected.from} rect`);
    const toRect = object(object(to["renderBox"], `${expected.to} renderBox`)["rect"], `${expected.to} rect`);
    const gap = expected.axis === "horizontal"
      ? number(toRect["x"], "to x") - number(fromRect["x"], "from x") - number(fromRect["width"], "from width")
      : number(toRect["y"], "to y") - number(fromRect["y"], "from y") - number(fromRect["height"], "from height");
    assert.ok(Math.abs(gap - expected.value) <= expected.tolerance, `${expected.from}->${expected.to} gap ${gap}`);
  }

  assert.equal(object(response["artifactIr"], "Artifact IR response")["sha256"], digest(runResult.artifactIrBytes));
  assert.equal(object(response["screenshot"], "screenshot response")["sha256"], digest(runResult.screenshotBytes));
}

function resultMatches(candidate: Record<string, unknown>, expected: RuleCase["expectedResults"][number]): boolean {
  const target = object(candidate["target"], "rule target");
  return candidate["ruleId"] === expected.ruleId &&
    candidate["ruleVersion"] === expected.ruleVersion &&
    candidate["outcome"] === expected.outcome &&
    target["kind"] === expected.targetKind &&
    target["id"] === expected.targetId &&
    (target["aspect"] ?? null) === expected.targetAspect;
}

function assertRuleReport(result: ProcessResult, oracle: RuleCase): RuleMetrics {
  assert.equal(result.code, oracle.expectedExitCode, result.stderr.toString("utf8"));
  assert.equal(result.stderr.byteLength, 0);
  const report = JSON.parse(result.stdout.toString("utf8")) as Record<string, unknown>;
  assert.equal(object(report["summary"], "report summary")["failed"], oracle.expectedFailureCount);
  const results = array(report["results"], "rule results");
  for (const expected of oracle.expectedResults) {
    const matches = results.filter((candidate) => resultMatches(candidate, expected));
    assert.equal(matches.length, 1, `missing or duplicate reviewed result in ${oracle.caseId}: ${JSON.stringify(expected)}`);
  }
  assert.ok(oracle.falsePositiveRisk.length > 0);
  assert.ok(oracle.nonClaim.length > 0);
  const failures = results.filter((candidate) => candidate["outcome"] === "failed");
  const reviewedFailures = oracle.expectedResults.filter((expected) => expected.outcome === "failed");
  const matchedFailures = failures.filter((candidate) => reviewedFailures.some((expected) => resultMatches(candidate, expected))).length;
  return {
    emittedFailures: failures.length,
    matchedFailures,
    unexpectedFailures: failures.length - matchedFailures,
    abstentions: results.filter((candidate) => ["cantTell", "inapplicable", "untested"].includes(String(candidate["outcome"]))).length,
  };
}

test("reviewed browser acquisition and rule oracles pass through the public processes", { timeout: 120_000 }, async () => {
  const acquisitionDocument = await loadJson("evaluation/web/annotations/browser-acquisition.json");
  const ruleDocument = await loadJson("evaluation/web/annotations/browser-rules.json");
  const acquisitionCases = array(acquisitionDocument["cases"], "acquisition cases") as unknown as AcquisitionCase[];
  const ruleCases = new Map((array(ruleDocument["cases"], "rule cases") as unknown as RuleCase[]).map((item) => [item.caseId, item]));
  const responseValidator = await schemaValidator("adapters/playwright/schemas/capture-response.schema.json");
  const extensionValidator = await schemaValidator("adapters/playwright/schemas/web-extension.schema.json");
  const completed: CaptureRun[] = [];
  let completedCases = 0;
  let acquisitionExpectations = 0;
  let acquisitionAbstentions = 0;
  let acquisitionMutations = 0;
  let detectedAcquisitionMutations = 0;
  let eligibleRuleMutations = 0;
  let killedRuleMutations = 0;
  let emittedFailures = 0;
  let matchedFailures = 0;
  let falsePositiveFailures = 0;
  let ruleAbstentions = 0;
  let hardNegativeFailures = 0;

  try {
    for (const oracle of acquisitionCases) {
      const ruleOracle = ruleCases.get(oracle.caseId);
      assert.ok(ruleOracle, `missing rule oracle ${oracle.caseId}`);
      assert.equal(ruleOracle.request, oracle.request);
      assert.equal(ruleOracle.classification, oracle.classification);
      const captured = await capture(oracle.request);
      completed.push(captured);
      assertValid(responseValidator, captured.response, `${oracle.caseId} response`);
      assertValid(extensionValidator, object(object(captured.artifactIr["extensions"], "extensions")[webExtensionKey], "Web extension"), `${oracle.caseId} extension`);
      assertAcquisition(captured, oracle);
      const report = await run(sightlintBinary, ["check", join(captured.directory, "artifact-ir.json"), "--format", "json"]);
      const metrics = assertRuleReport(report, ruleOracle);
      completedCases += 1;
      acquisitionExpectations += oracle.expectations.nodes.length + oracle.expectations.gaps.length;
      acquisitionAbstentions += oracle.abstentions.length;
      emittedFailures += metrics.emittedFailures;
      matchedFailures += metrics.matchedFailures;
      falsePositiveFailures += metrics.unexpectedFailures;
      ruleAbstentions += metrics.abstentions;
      if (oracle.classification === "targetedMutation") {
        acquisitionMutations += 1;
        if (oracle.expectations.nodes.some((node) => node.layoutRenderStatus === "conflict")) {
          detectedAcquisitionMutations += 1;
        }
        if (ruleOracle.expectedFailureCount > 0) {
          eligibleRuleMutations += 1;
          if (metrics.matchedFailures > 0) killedRuleMutations += 1;
        }
      }
      if (oracle.classification === "hardNegative") hardNegativeFailures += metrics.emittedFailures;
    }

    const firstClean = completed.find((item) => object(item.artifactIr["artifact"], "artifact")["id"] === "web-dashboard-browser-clean");
    assert.ok(firstClean);
    const repeatedClean = await capture("evaluation/web/requests/dashboard-browser-clean.json");
    completed.push(repeatedClean);
    assert.deepEqual(repeatedClean.responseBytes, firstClean.responseBytes, "capture response must be byte-stable");
    assert.deepEqual(repeatedClean.artifactIrBytes, firstClean.artifactIrBytes, "Artifact IR must be byte-stable");
    assert.deepEqual(repeatedClean.screenshotBytes, firstClean.screenshotBytes, "screenshot must be byte-stable in one compatibility environment");
    const firstReport = await run(sightlintBinary, ["check", join(firstClean.directory, "artifact-ir.json"), "--format", "json"]);
    const repeatedReport = await run(sightlintBinary, ["check", join(repeatedClean.directory, "artifact-ir.json"), "--format", "json"]);
    assert.deepEqual(repeatedReport, firstReport, "rule report, stderr, and exit code must be stable");

    assert.equal(completedCases, 7);
    assert.equal(acquisitionAbstentions, 13);
    assert.equal(detectedAcquisitionMutations, acquisitionMutations);
    assert.equal(eligibleRuleMutations, 1);
    assert.equal(killedRuleMutations, eligibleRuleMutations);
    assert.equal(matchedFailures, 1);
    assert.equal(emittedFailures, 1);
    assert.equal(falsePositiveFailures, 0);
    assert.equal(hardNegativeFailures, 0);
    process.stdout.write(
      `browser evaluation v0: cases=${completedCases}/7, acquisition_expectations=${acquisitionExpectations}, ` +
      `failure_precision=${matchedFailures}/${emittedFailures}, false_positive_failures=${falsePositiveFailures}, ` +
      `acquisition_abstentions=${acquisitionAbstentions}, rule_abstentions=${ruleAbstentions}, ` +
      `acquisition_mutations_detected=${detectedAcquisitionMutations}/${acquisitionMutations}, ` +
      `eligible_rule_mutations_killed=${killedRuleMutations}/${eligibleRuleMutations}, ` +
      `hard_negative_failures=${hardNegativeFailures}\n`,
    );
  } finally {
    await Promise.all(completed.map(async (item) => rm(item.directory, { recursive: true, force: true })));
  }
});

function localRequest(entrypoint: string): Record<string, unknown> {
  return {
    $schema: "../../../adapters/playwright/schemas/capture-request.schema.json",
    protocolVersion: "0.1.0",
    artifact: { id: "malformed-e2e", title: "Malformed E2E" },
    fixture: { entrypoint, state: "clean", readinessSelector: "html[data-fixture-ready=\"true\"]" },
    environment: {
      viewport: { width: 320, height: 240, unit: "cssPixel" }, deviceScaleFactor: 1, textScale: 1,
      locale: "en-US", timezoneId: "UTC", colorScheme: "light", reducedMotion: "reduce",
    },
    privacy: { accessibleNameMode: "selectedNodes", externalProcessing: false },
    network: { mode: "deny" },
    screenshot: { reference: "evaluation/web/generated/malformed.png" },
  };
}

async function expectAdapterFailure(root: string, request: Record<string, unknown>, expected: string): Promise<void> {
  const requestPath = join(root, `${expected}.json`);
  const output = join(root, `${expected}-out`);
  await writeFile(requestPath, `${JSON.stringify(request)}\n`);
  const first = await run(process.execPath, [adapterCli, "--request", requestPath, "--repository-root", root, "--artifact-ir-out", join(output, "ir.json"), "--screenshot-out", join(output, "shot.png")]);
  const second = await run(process.execPath, [adapterCli, "--request", requestPath, "--repository-root", root, "--artifact-ir-out", join(output, "ir.json"), "--screenshot-out", join(output, "shot.png")]);
  assert.equal(first.code, 2);
  assert.equal(first.stdout.byteLength, 0);
  assert.match(first.stderr.toString("utf8"), new RegExp(`^sightlint-web: ${expected}:`, "u"));
  assert.deepEqual(second, first, `${expected} diagnostics must be stable`);
}

test("public adapter rejects unsafe/resource states without partial outputs", { timeout: 60_000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), "sightlint-playwright-invalid-"));
  try {
    await mkdir(join(root, "fixtures"));
    await writeFile(join(root, "fixtures/external.html"), '<html data-fixture-ready="true"><main data-testid="main"><img src="https://example.invalid/private.png"></main></html>\n');
    await writeFile(join(root, "fixtures/duplicate.html"), '<html data-fixture-ready="true"><main data-testid="duplicate"><div data-testid="duplicate"></div></main></html>\n');
    await writeFile(join(root, "fixtures/frame.html"), '<html data-fixture-ready="true"><main data-testid="main"></main><iframe srcdoc="<p>child</p>"></iframe></html>\n');
    await writeFile(join(root, "fixtures/valid.html"), '<html data-fixture-ready="true"><main data-testid="main"></main></html>\n');
    const boundaryNodes = Array.from({ length: 199 }, (_, index) => `<div data-testid="node-${index}"></div>`).join("");
    const excessNodes = `${boundaryNodes}<div data-testid="node-199"></div>`;
    await writeFile(join(root, "fixtures/nodes-boundary.html"), `<html data-fixture-ready="true"><main>${boundaryNodes}</main></html>\n`);
    await writeFile(join(root, "fixtures/nodes.html"), `<html data-fixture-ready="true"><main>${excessNodes}</main></html>\n`);

    await expectAdapterFailure(root, localRequest("fixtures/external.html"), "external-request");
    await expectAdapterFailure(root, localRequest("fixtures/duplicate.html"), "duplicate-locator");
    await expectAdapterFailure(root, localRequest("fixtures/frame.html"), "frame-budget");
    await expectAdapterFailure(root, localRequest("fixtures/nodes.html"), "node-budget");

    const boundaryRequestPath = join(root, "node-boundary.json");
    const boundaryOutput = join(root, "node-boundary-output");
    await writeFile(boundaryRequestPath, `${JSON.stringify(localRequest("fixtures/nodes-boundary.html"))}\n`);
    const boundary = await run(process.execPath, [adapterCli, "--request", boundaryRequestPath, "--repository-root", root, "--artifact-ir-out", join(boundaryOutput, "ir.json"), "--screenshot-out", join(boundaryOutput, "shot.png")]);
    assert.equal(boundary.code, 0, boundary.stderr.toString("utf8"));
    const boundaryResponse = JSON.parse(boundary.stdout.toString("utf8")) as Record<string, unknown>;
    assert.equal(boundaryResponse["status"], "captured");
    assert.equal(object(boundaryResponse["capture"], "boundary capture")["nodeCount"], 200);

    const requestPath = join(root, "output-exists.json");
    const outputDirectory = join(root, "existing-output");
    const artifactIrPath = join(outputDirectory, "ir.json");
    const screenshotPath = join(outputDirectory, "shot.png");
    await mkdir(outputDirectory);
    await writeFile(requestPath, `${JSON.stringify(localRequest("fixtures/valid.html"))}\n`);
    await writeFile(screenshotPath, "owned-by-caller\n");
    const outputExists = await run(process.execPath, [adapterCli, "--request", requestPath, "--repository-root", root, "--artifact-ir-out", artifactIrPath, "--screenshot-out", screenshotPath]);
    assert.equal(outputExists.code, 2);
    assert.equal(outputExists.stdout.byteLength, 0);
    assert.match(outputExists.stderr.toString("utf8"), /^sightlint-web: output-exists:/u);
    await assert.rejects(readFile(artifactIrPath), /ENOENT/u, "failed pair write must remove its partial Artifact IR");
    assert.equal(await readFile(screenshotPath, "utf8"), "owned-by-caller\n");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
