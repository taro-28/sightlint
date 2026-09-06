import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");
const webExtensionKey = "org.sightlint.web";

type JsonObject = Record<string, unknown>;

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
  layoutSize?: { width: number; height: number; tolerance: number; unit: string };
  renderSize?: { width: number; height: number; tolerance: number; unit: string };
  interactive?: boolean;
  disabled?: boolean;
  centerHitSample?: { outcome: string; hitLocator: string | null };
  ancestorClipStatus?: string;
  layoutRenderStatus: string;
  screenshotGeometryCoverage: string;
}

interface AcquisitionCase {
  caseId: string;
  request: string;
  split: "smoke" | "development" | "challenge";
  classification: "clean" | "targetedMutation" | "hardNegative" | "ambiguous";
  expectations: {
    sourceFiles: string[];
    viewport: { width: number; height: number; unit: string };
    minimumDocumentHeight: number;
    documentDirection: string;
    frameCount: number;
    minimumNodeCount: number;
    coreRelationCount: number;
    screenshotViewport: string;
    pixelContentComparison: string;
    nodes: ExpectedNode[];
    gaps: unknown[];
  };
  abstentions: Array<{ aspect: string; outcome: "cantTell" | "untested"; rationale: string }>;
  mutation?: { baselineRequest: string; target: string; evidenceExpectations: string[] };
  hardNegative?: { category: string; rationale: string };
}

interface ExpectedResult {
  ruleId: string;
  ruleVersion: string;
  maturity: string;
  enforcement: string;
  policy: JsonObject;
  outcome: string;
  targetKind: string;
  targetId: string;
  targetAspect: string | null;
}

interface RuleCase {
  caseId: string;
  request: string;
  classification: string;
  expectedExitCode: number;
  expectedFailureCount: number;
  expectedBlockingFailureCount: number;
  expectedResults: ExpectedResult[];
}

interface CaptureRun {
  directory: string;
  process: ProcessResult;
  responseBytes: Buffer;
  artifactIrBytes: Buffer;
  screenshotBytes: Buffer;
  response: JsonObject;
  artifactIr: JsonObject;
}

interface SplitMetrics {
  cases: number;
  acquisitionExpectations: number;
  reviewedRuleResults: number;
  matchedRuleResults: number;
  reviewedAbstentions: number;
  matchedAbstentions: number;
  expectedFailures: number;
  emittedFailures: number;
  matchedFailures: number;
  falsePositiveFailures: number;
  mutations: number;
  killedMutations: number;
  hardNegativeFailures: number;
}

function run(program: string, args: string[]): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, { cwd: repositoryRoot, env: process.env, stdio: ["ignore", "pipe", "pipe"] });
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

async function loadJson(path: string): Promise<JsonObject> {
  return JSON.parse(await readFile(resolve(repositoryRoot, path), "utf8")) as JsonObject;
}

function array(value: unknown, context: string): JsonObject[] {
  assert.ok(Array.isArray(value), `${context} must be an array`);
  return value as JsonObject[];
}

function object(value: unknown, context: string): JsonObject {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${context} must be an object`);
  return value as JsonObject;
}

function number(value: unknown, context: string): number {
  assert.equal(typeof value, "number", `${context} must be a number`);
  return value as number;
}

function string(value: unknown, context: string): string {
  assert.equal(typeof value, "string", `${context} must be a string`);
  return value as string;
}

function indexBy(items: JsonObject[], field: string, context: string): Map<string, JsonObject> {
  const indexed = new Map<string, JsonObject>();
  for (const item of items) {
    const identifier = string(item[field], `${context}.${field}`);
    assert.equal(indexed.has(identifier), false, `${context} repeats ${identifier}`);
    indexed.set(identifier, item);
  }
  return indexed;
}

async function capture(requestPath: string): Promise<CaptureRun> {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-multi-family-e2e-"));
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
    response: JSON.parse(processResult.stdout.toString("utf8")) as JsonObject,
    artifactIr: JSON.parse(artifactIrBytes.toString("utf8")) as JsonObject,
  };
}

function assertSize(
  value: unknown,
  expected: { width: number; height: number; tolerance: number; unit: string },
  context: string,
): void {
  const geometry = object(value, context);
  assert.equal(geometry["unit"] ?? "cssPixel", expected.unit, `${context} unit`);
  assert.ok(Math.abs(number(geometry["width"], `${context} width`) - expected.width) <= expected.tolerance);
  assert.ok(Math.abs(number(geometry["height"], `${context} height`) - expected.height) <= expected.tolerance);
}

function assertAcquisition(captured: CaptureRun, oracle: AcquisitionCase, sourceDigest: string): number {
  const ir = captured.artifactIr;
  const responseCapture = object(captured.response["capture"], "capture response");
  const extension = object(object(ir["extensions"], "IR extensions")[webExtensionKey], "Web extension");
  const document = object(extension["document"], "Web document");
  const captureRecord = object(extension["capture"], "Web capture");
  const reconciliation = object(extension["reconciliation"], "Web reconciliation");

  assert.deepEqual(captureRecord["sourceFiles"], oracle.expectations.sourceFiles);
  assert.equal(captureRecord["sourceDigest"], sourceDigest);
  assert.deepEqual(document["viewportSize"], oracle.expectations.viewport);
  assert.ok(number(object(document["documentSize"], "document size")["height"], "document height") >= oracle.expectations.minimumDocumentHeight);
  assert.equal(document["direction"], oracle.expectations.documentDirection);
  assert.equal(document["frameCount"], oracle.expectations.frameCount);
  assert.equal(responseCapture["frameCount"], oracle.expectations.frameCount);
  assert.ok(array(ir["nodes"], "core nodes").length >= oracle.expectations.minimumNodeCount);
  assert.equal(array(ir["relations"] ?? [], "core relations").length, oracle.expectations.coreRelationCount);
  assert.equal(object(reconciliation["screenshotViewport"], "screenshot viewport")["status"], oracle.expectations.screenshotViewport);
  assert.equal(object(reconciliation["pixelContentComparison"], "pixel comparison")["status"], oracle.expectations.pixelContentComparison);
  assert.ok(oracle.abstentions.length > 0);
  assert.ok(oracle.abstentions.every((item) => item.rationale.length > 0));

  const coreNodes = indexBy(array(ir["nodes"], "core nodes"), "id", "core nodes");
  const extensionNodes = indexBy(array(extension["nodes"], "extension nodes"), "nodeId", "extension nodes");
  const reconciliationNodes = indexBy(array(reconciliation["nodes"], "reconciliation nodes"), "nodeId", "reconciliation nodes");
  for (const expected of oracle.expectations.nodes) {
    const core = object(coreNodes.get(expected.id), `${expected.id} core node`);
    const acquired = object(extensionNodes.get(expected.id), `${expected.id} extension node`);
    const reconciled = object(reconciliationNodes.get(expected.id), `${expected.id} reconciliation`);
    const accessibility = object(acquired["accessibility"], `${expected.id} accessibility`);
    const style = object(acquired["computedStyle"], `${expected.id} style`);
    assert.equal(core["parentId"] ?? null, expected.parentId, `${expected.id} parent`);
    assert.equal(accessibility["status"], expected.accessibilityStatus);
    assert.equal(accessibility["role"], expected.role);
    assert.equal(accessibility["name"], expected.name);
    assert.equal(style["display"], expected.display);
    if (expected.interactive !== undefined) assert.equal(acquired["interactive"], expected.interactive);
    if (expected.disabled !== undefined) assert.equal(acquired["disabled"], expected.disabled);
    if (expected.centerHitSample !== undefined) {
      const sample = object(acquired["centerHitSample"], `${expected.id} center hit sample`);
      assert.equal(sample["outcome"], expected.centerHitSample.outcome);
      assert.equal(sample["hitLocator"], expected.centerHitSample.hitLocator);
    }
    assert.equal(object(acquired["hitRegion"], `${expected.id} hit region`)["status"], "cantTell");
    assert.equal(object(reconciled["layoutRender"], `${expected.id} layout/render`)["status"], expected.layoutRenderStatus);
    assert.equal(reconciled["screenshotGeometryCoverage"], expected.screenshotGeometryCoverage);
    assert.equal(object(reconciled["pixelContentMatch"], `${expected.id} pixel match`)["status"], "cantTell");
    if (expected.ancestorClipStatus !== undefined) {
      assert.equal(object(reconciled["ancestorClip"], `${expected.id} ancestor clip`)["status"], expected.ancestorClipStatus);
    }
    const geometry = object(core["geometry"], `${expected.id} geometry`);
    if (expected.layoutSize !== undefined) {
      assertSize(object(geometry["layoutBox"], `${expected.id} layout box`)["rect"], expected.layoutSize, `${expected.id} layout size`);
    }
    if (expected.renderSize !== undefined) {
      assertSize(object(geometry["renderBox"], `${expected.id} render box`)["rect"], expected.renderSize, `${expected.id} render size`);
    }
  }
  return oracle.expectations.nodes.length + oracle.expectations.gaps.length;
}

function resultMatches(candidate: JsonObject, expected: ExpectedResult): boolean {
  const target = object(candidate["target"], "result target");
  return candidate["ruleId"] === expected.ruleId &&
    candidate["ruleVersion"] === expected.ruleVersion &&
    candidate["maturity"] === expected.maturity &&
    candidate["enforcement"] === expected.enforcement &&
    candidate["outcome"] === expected.outcome &&
    target["kind"] === expected.targetKind &&
    target["id"] === expected.targetId &&
    (target["aspect"] ?? null) === expected.targetAspect;
}

function emptyMetrics(): SplitMetrics {
  return {
    cases: 0,
    acquisitionExpectations: 0,
    reviewedRuleResults: 0,
    matchedRuleResults: 0,
    reviewedAbstentions: 0,
    matchedAbstentions: 0,
    expectedFailures: 0,
    emittedFailures: 0,
    matchedFailures: 0,
    falsePositiveFailures: 0,
    mutations: 0,
    killedMutations: 0,
    hardNegativeFailures: 0,
  };
}

test("second Web fixture family preserves synchronized evidence, abstention, hard negatives, and rule truth", { timeout: 300_000 }, async () => {
  const registry = await loadJson("evaluation/web/evaluation-v1.json");
  const dataset = array(registry["datasets"], "datasets").find((item) => item["id"] === "harbor-support-browser-v1");
  assert.ok(dataset, "support-inbox dataset must be registered");
  assert.equal(dataset["familyId"], "harbor-support-inbox-v1");
  const family = array(registry["families"], "families").find((item) => item["id"] === "harbor-support-inbox-v1");
  assert.ok(family, "support-inbox family must be registered");
  const sourceDigest = string(family["sourceDigest"], "support-inbox source digest");
  const acquisitionReference = object(dataset["acquisitionOracle"], "acquisition reference");
  const ruleReference = object(dataset["ruleOracle"], "rule reference");
  const acquisitionDocument = await loadJson(string(acquisitionReference["document"], "acquisition document"));
  const ruleDocument = await loadJson(string(ruleReference["document"], "rule document"));
  const acquisitionCases = array(acquisitionDocument["cases"], "acquisition cases") as unknown as AcquisitionCase[];
  const ruleCases = new Map(
    (array(ruleDocument["cases"], "rule cases") as unknown as RuleCase[]).map((item) => [item.caseId, item]),
  );
  assert.equal(object(acquisitionDocument["provenance"], "acquisition provenance")["implementationOutputUsedAsOracle"], false);
  assert.equal(object(acquisitionDocument["provenance"], "acquisition provenance")["holdoutStatus"], "publicDevelopmentData");

  const runs = new Map<string, CaptureRun>();
  const temporaryRuns: CaptureRun[] = [];
  const metricsBySplit = new Map<string, SplitMetrics>();
  let totalEmittedFailures = 0;
  let totalMatchedFailures = 0;
  let totalFalsePositiveFailures = 0;
  let totalMutations = 0;
  let totalKilledMutations = 0;
  let totalHardNegativeFailures = 0;

  try {
    for (const oracle of acquisitionCases) {
      const ruleOracle = ruleCases.get(oracle.caseId);
      assert.ok(ruleOracle, `missing rule oracle ${oracle.caseId}`);
      assert.equal(ruleOracle.request, oracle.request);
      assert.equal(ruleOracle.classification, oracle.classification);
      const first = await capture(oracle.request);
      const repeated = await capture(oracle.request);
      temporaryRuns.push(first, repeated);
      runs.set(oracle.caseId, first);
      assert.deepEqual(repeated.process, first.process, `${oracle.caseId} adapter process bytes`);
      assert.deepEqual(repeated.responseBytes, first.responseBytes, `${oracle.caseId} response bytes`);
      assert.deepEqual(repeated.artifactIrBytes, first.artifactIrBytes, `${oracle.caseId} Artifact IR bytes`);
      assert.deepEqual(repeated.screenshotBytes, first.screenshotBytes, `${oracle.caseId} screenshot bytes`);

      const splitMetrics = metricsBySplit.get(oracle.split) ?? emptyMetrics();
      splitMetrics.cases += 1;
      splitMetrics.acquisitionExpectations += assertAcquisition(first, oracle, sourceDigest);

      const firstReport = await run(sightlintBinary, ["check", join(first.directory, "artifact-ir.json"), "--profile", "recommended", "--format", "json"]);
      const repeatedReport = await run(sightlintBinary, ["check", join(repeated.directory, "artifact-ir.json"), "--profile", "recommended", "--format", "json"]);
      assert.deepEqual(repeatedReport, firstReport, `${oracle.caseId} report, diagnostics, and exit code bytes`);
      assert.equal(firstReport.code, ruleOracle.expectedExitCode, firstReport.stderr.toString("utf8"));
      assert.equal(firstReport.stderr.byteLength, 0);
      const report = JSON.parse(firstReport.stdout.toString("utf8")) as JsonObject;
      const reportResults = array(report["results"], "rule results");
      const failures = reportResults.filter((item) => item["outcome"] === "failed");
      assert.equal(object(report["summary"], "report summary")["failed"], ruleOracle.expectedFailureCount);
      assert.equal(failures.filter((item) => item["enforcement"] === "blocking").length, ruleOracle.expectedBlockingFailureCount);

      const reviewedFailures = ruleOracle.expectedResults.filter((item) => item.outcome === "failed");
      for (const expected of ruleOracle.expectedResults) {
        const matched = reportResults.filter((item) => resultMatches(item, expected));
        assert.equal(matched.length, 1, `${oracle.caseId} missing or duplicate reviewed result ${expected.ruleId}/${expected.targetId}/${expected.outcome}`);
        assert.deepEqual(matched[0]?.["policy"], expected.policy, `${oracle.caseId} policy provenance`);
        splitMetrics.reviewedRuleResults += 1;
        splitMetrics.matchedRuleResults += 1;
        if (["cantTell", "inapplicable", "untested"].includes(expected.outcome)) {
          splitMetrics.reviewedAbstentions += 1;
          splitMetrics.matchedAbstentions += 1;
        }
      }
      const matchedFailures = failures.filter((item) => reviewedFailures.some((expected) => resultMatches(item, expected))).length;
      const falsePositiveFailures = failures.length - matchedFailures;
      splitMetrics.expectedFailures += reviewedFailures.length;
      splitMetrics.emittedFailures += failures.length;
      splitMetrics.matchedFailures += matchedFailures;
      splitMetrics.falsePositiveFailures += falsePositiveFailures;
      totalEmittedFailures += failures.length;
      totalMatchedFailures += matchedFailures;
      totalFalsePositiveFailures += falsePositiveFailures;

      if (oracle.classification === "targetedMutation") {
        assert.ok(oracle.mutation, `${oracle.caseId} mutation contract`);
        assert.deepEqual(oracle.mutation.evidenceExpectations, ["accessibilityName"]);
        assert.equal(reviewedFailures.length, 1);
        splitMetrics.mutations += 1;
        totalMutations += 1;
        if (matchedFailures === 1) {
          splitMetrics.killedMutations += 1;
          totalKilledMutations += 1;
        }
      }
      if (oracle.classification === "hardNegative") {
        assert.ok(oracle.hardNegative, `${oracle.caseId} hard-negative contract`);
        splitMetrics.hardNegativeFailures += failures.length;
        totalHardNegativeFailures += failures.length;
      }
      metricsBySplit.set(oracle.split, splitMetrics);
    }

    const clean = runs.get("support-inbox-clean");
    const mutation = runs.get("support-inbox-unnamed-control");
    const hardNegative = runs.get("support-inbox-labelledby-hard-negative");
    assert.ok(clean && mutation && hardNegative);
    assert.deepEqual(mutation.screenshotBytes, clean.screenshotBytes, "name-only mutation must preserve rendered pixels");
    assert.deepEqual(hardNegative.screenshotBytes, clean.screenshotBytes, "alternative name source must preserve rendered pixels");

    assert.equal(acquisitionCases.length, 4);
    assert.equal(totalMatchedFailures, 1);
    assert.equal(totalEmittedFailures, 1);
    assert.equal(totalFalsePositiveFailures, 0);
    assert.equal(totalKilledMutations, 1);
    assert.equal(totalMutations, 1);
    assert.equal(totalHardNegativeFailures, 0);
    for (const [split, metrics] of [...metricsBySplit.entries()].toSorted(([left], [right]) => left.localeCompare(right))) {
      assert.equal(metrics.matchedRuleResults, metrics.reviewedRuleResults, `${split} reviewed rule coverage`);
      assert.equal(metrics.matchedAbstentions, metrics.reviewedAbstentions, `${split} reviewed abstention agreement`);
      assert.equal(metrics.falsePositiveFailures, 0, `${split} false-positive failures`);
      process.stdout.write(
        `web evaluation v1 family=harbor-support-inbox-v1 split=${split}: ` +
        `case_coverage=${metrics.cases}/${metrics.cases}, ` +
        `acquisition_expectations=${metrics.acquisitionExpectations}/${metrics.acquisitionExpectations}, ` +
        `rule_results=${metrics.matchedRuleResults}/${metrics.reviewedRuleResults}, ` +
        `failure_precision=${metrics.matchedFailures}/${metrics.emittedFailures}, ` +
        `reviewed_abstentions=${metrics.matchedAbstentions}/${metrics.reviewedAbstentions}, ` +
        `false_positive_failures=${metrics.falsePositiveFailures}, ` +
        `mutation_kill_rate=${metrics.killedMutations}/${metrics.mutations}, ` +
        `hard_negative_failures=${metrics.hardNegativeFailures}\n`,
      );
    }
  } finally {
    await Promise.all(temporaryRuns.map(async (item) => rm(item.directory, { recursive: true, force: true })));
  }
});
