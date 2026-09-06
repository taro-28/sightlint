import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");
const policyIds = [
  "qualified-corner-95-row-runs-v1",
  "ranked-exact-border-flood-v1",
  "strict-uniform-perimeter-flood-v1",
] as const;

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface CorpusCase {
  id: string;
  split: "smoke" | "development" | "challenge";
  classification: "baseline" | "targetedMutation" | "metamorphic" | "hardNegative" | "stress";
  sourceId: string;
  request: string;
  features: string[];
  relation?: {
    baselineCaseId: string;
    changedProperty: string;
    preservedProperties: string[];
  };
  hardNegative?: { category: string; rationale: string };
}

interface PolicyExpectation {
  policy: typeof policyIds[number];
  expectedStatus: "observed" | "unavailable";
  expectedReason: string | null;
  backgroundUsability: "usable" | "unsafe" | "notSelected";
  rationale: string;
}

interface RegionTarget {
  id: string;
  sourceSelector: string;
  bounds: [number, number, number, number];
  edgeTolerance: number;
  unit: "devicePixel";
  annotationBasis: "sourceAuthoredCssAndHumanVisualReview";
}

interface AcquisitionCase {
  caseId: string;
  policyExpectations: PolicyExpectation[];
  regionTargets: RegionTarget[];
  abstentions: string[];
}

interface RuleCase {
  caseId: string;
  executableRule: null;
  semanticQuestion: string;
  applicabilityGroundTruth: "cantTell" | "inapplicable";
  expectedOutcome: "untested";
  blockingAllowed: false;
  rationale: string;
}

interface PolicyMetrics {
  usableCases: number;
  observedUsableCases: number;
  targetRegions: number;
  predictedRegions: number;
  matchedRegions: number;
  unmatchedPredictions: number;
  falseGroups: number;
  maximumEdgeError: number;
  unsafeCases: number;
  unsafeObservations: number;
  requiredAbstentions: number;
  correctAbstentions: number;
  acquisitionMutations: number;
  observedAcquisitionMutations: number;
}

function run(program: string, args: string[]): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, {
      cwd: repositoryRoot,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
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

function object(value: unknown, context: string): Record<string, unknown> {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${context} must be an object`);
  return value as Record<string, unknown>;
}

function array(value: unknown, context: string): Array<Record<string, unknown>> {
  assert.ok(Array.isArray(value), `${context} must be an array`);
  return value as Array<Record<string, unknown>>;
}

function text(value: unknown, context: string): string {
  assert.equal(typeof value, "string", `${context} must be a string`);
  return value as string;
}

function integer(value: unknown, context: string): number {
  assert.ok(Number.isSafeInteger(value), `${context} must be a safe integer`);
  return value as number;
}

async function validator(path: string): Promise<ValidateFunction> {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  return ajv.compile(await loadJson(path));
}

function assertValid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), true, `${context}: ${JSON.stringify(validate.errors)}`);
}

function indexByCase<T extends { caseId: string }>(cases: T[], context: string): Map<string, T> {
  const ordered = cases.map((item) => item.caseId);
  assert.deepEqual(ordered, [...ordered].sort(), `${context} must be sorted by caseId`);
  const indexed = new Map(cases.map((item) => [item.caseId, item]));
  assert.equal(indexed.size, cases.length, `${context} contains duplicate case IDs`);
  return indexed;
}

function bounds(value: unknown, context: string): [number, number, number, number] {
  assert.ok(Array.isArray(value) && value.length === 4, `${context} must be xywh`);
  return value.map((item, index) => integer(item, `${context}[${index}]`)) as [number, number, number, number];
}

function edges(rectangle: [number, number, number, number]): [number, number, number, number] {
  return [rectangle[0], rectangle[1], rectangle[0] + rectangle[2], rectangle[1] + rectangle[3]];
}

function edgeErrors(
  predicted: [number, number, number, number],
  target: [number, number, number, number],
): [number, number, number, number] {
  const predictedEdges = edges(predicted);
  const targetEdges = edges(target);
  return predictedEdges.map((value, index) => Math.abs(value - targetEdges[index]!)) as [number, number, number, number];
}

function intersects(
  left: [number, number, number, number],
  right: [number, number, number, number],
): boolean {
  return left[0] < right[0] + right[2]
    && right[0] < left[0] + left[2]
    && left[1] < right[1] + right[3]
    && right[1] < left[1] + left[3];
}

function matchRegions(
  predicted: Array<Record<string, unknown>>,
  targets: RegionTarget[],
): { matches: number; unmatched: number; falseGroups: number; maximumEdgeError: number } {
  const remaining = new Set(predicted.map((_, index) => index));
  let matches = 0;
  let maximumEdgeError = 0;
  for (const target of targets) {
    const candidates = [...remaining].flatMap((index) => {
      const region = predicted[index];
      assert.ok(region);
      const errors = edgeErrors(bounds(region["bounds"], `predicted region ${index}`), target.bounds);
      const maximum = Math.max(...errors);
      return maximum <= target.edgeTolerance ? [{ index, maximum, total: errors.reduce((sum, item) => sum + item, 0) }] : [];
    });
    candidates.sort((left, right) => left.total - right.total || left.maximum - right.maximum || left.index - right.index);
    const selected = candidates[0];
    if (selected !== undefined) {
      remaining.delete(selected.index);
      matches += 1;
      maximumEdgeError = Math.max(maximumEdgeError, selected.maximum);
    }
  }
  const falseGroups = predicted.filter((region) => {
    const rectangle = bounds(region["bounds"], "predicted region");
    return targets.filter((target) => intersects(rectangle, target.bounds)).length > 1;
  }).length;
  return { matches, unmatched: remaining.size, falseGroups, maximumEdgeError };
}

function policyMap(report: Record<string, unknown>): Map<string, Record<string, unknown>> {
  const policies = array(report["policies"], "benchmark policies");
  const indexed = new Map(policies.map((policy) => [text(policy["policyId"], "policyId"), policy]));
  assert.deepEqual([...indexed.keys()].sort(), [...policyIds]);
  return indexed;
}

function regionGeometry(value: unknown, context: string): Array<Record<string, unknown>> {
  return array(value, context).map((region) => ({
    id: region["id"],
    bounds: region["bounds"],
    pixelCount: region["pixelCount"],
    singleColorRectangle: region["singleColorRectangle"],
  }));
}

function compareStrictDefault(
  inspection: Record<string, unknown>,
  strict: Record<string, unknown>,
  caseId: string,
): void {
  assert.equal(strict["status"], inspection["status"], `${caseId} strict/default status`);
  assert.equal(strict["reason"], inspection["reason"] ?? null, `${caseId} strict/default reason`);
  assert.deepEqual(
    array(strict["regions"], `${caseId} strict regions`).map((region) => ({
      bounds: region["bounds"],
      pixelCount: region["pixelCount"],
      singleColorRectangle: region["singleColorRectangle"],
    })),
    array(inspection["regions"], `${caseId} inspect-image regions`).map((region) => ({
      bounds: region["bounds"],
      pixelCount: region["pixelCount"],
      singleColorRectangle: region["singleColorRectangle"],
    })),
    `${caseId} benchmark must reproduce the unchanged strict acquisition`,
  );
}

function updateMetrics(
  metrics: PolicyMetrics,
  caseRecord: CorpusCase,
  expectation: PolicyExpectation,
  policy: Record<string, unknown>,
  targets: RegionTarget[],
): void {
  const selection = object(policy["backgroundSelection"], "background selection");
  const selected = selection["selectedCandidateId"];
  const regions = array(policy["regions"], "policy regions");
  if (expectation.backgroundUsability === "usable") {
    metrics.usableCases += 1;
    if (policy["status"] === "observed") {
      metrics.observedUsableCases += 1;
      metrics.targetRegions += targets.length;
      metrics.predictedRegions += regions.length;
      const matched = matchRegions(regions, targets);
      metrics.matchedRegions += matched.matches;
      metrics.unmatchedPredictions += matched.unmatched;
      metrics.falseGroups += matched.falseGroups;
      metrics.maximumEdgeError = Math.max(metrics.maximumEdgeError, matched.maximumEdgeError);
    }
  } else if (expectation.backgroundUsability === "unsafe") {
    metrics.unsafeCases += 1;
    if (selected !== null) metrics.unsafeObservations += 1;
  } else {
    metrics.requiredAbstentions += 1;
    if (selected === null) metrics.correctAbstentions += 1;
  }
  if (caseRecord.classification === "targetedMutation") {
    metrics.acquisitionMutations += 1;
    if (policy["status"] === "observed") metrics.observedAcquisitionMutations += 1;
  }
}

function emptyMetrics(): PolicyMetrics {
  return {
    usableCases: 0,
    observedUsableCases: 0,
    targetRegions: 0,
    predictedRegions: 0,
    matchedRegions: 0,
    unmatchedPredictions: 0,
    falseGroups: 0,
    maximumEdgeError: 0,
    unsafeCases: 0,
    unsafeObservations: 0,
    requiredAbstentions: 0,
    correctAbstentions: 0,
    acquisitionMutations: 0,
    observedAcquisitionMutations: 0,
  };
}

function assertSupportedScreenshotPng(bytes: Buffer, caseId: string): void {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  assert.ok(bytes.byteLength >= 33, `${caseId} screenshot must contain IHDR`);
  assert.deepEqual(bytes.subarray(0, 8), signature, `${caseId} screenshot signature`);
  assert.equal(bytes.readUInt32BE(8), 13, `${caseId} IHDR length`);
  assert.equal(bytes.subarray(12, 16).toString("ascii"), "IHDR", `${caseId} first chunk`);
  assert.equal(bytes[24], 8, `${caseId} browser screenshot bit depth`);
  assert.equal(bytes[25], 2, `${caseId} browser screenshot color type`);
  assert.equal(bytes[28], 0, `${caseId} browser screenshot interlace method`);
}

test("repository-owned screenshots compare segmentation policies without becoming rule truth", { timeout: 300_000 }, async () => {
  const corpus = await loadJson("evaluation/image-segmentation/corpus.json");
  const acquisitionDocument = await loadJson("evaluation/image-segmentation/annotations/acquisition.json");
  const ruleDocument = await loadJson("evaluation/image-segmentation/annotations/rules.json");
  const corpusValidator = await validator("evaluation/image-segmentation/corpus.schema.json");
  const annotationValidator = await validator("evaluation/image-segmentation/annotation.schema.json");
  const requestValidator = await validator("adapters/playwright/schemas/capture-request.schema.json");
  const reportValidator = await validator("evaluation/image-segmentation/benchmark-report.schema.json");
  assertValid(corpusValidator, corpus, "segmentation corpus");
  assertValid(annotationValidator, acquisitionDocument, "segmentation acquisition oracle");
  assertValid(annotationValidator, ruleDocument, "segmentation rule oracle");

  const source = object(corpus["source"], "corpus source");
  assert.equal(source["ownership"], "sightlintRepository");
  assert.equal(source["license"], "MIT OR Apache-2.0");
  assert.equal(source["privacyReview"], "syntheticNoPersonalData");
  assert.equal(source["externalAssets"], false);
  assert.equal(source["fictionalData"], true);
  const governance = object(corpus["dataGovernance"], "data governance");
  assert.equal(governance["implementationOutputIsOracle"], false);
  assert.equal(governance["capturedArtifactsCommitted"], false);
  assert.equal(object(object(corpus["splitPolicy"], "split policy")["holdout"], "holdout")["status"], "notEstablished");
  assert.equal(object(corpus["gates"], "gates")["strictDefaultChanged"], false);
  assert.equal(object(corpus["gates"], "gates")["maximumBlockingFindings"], 0);

  const cases = array(corpus["cases"], "corpus cases") as unknown as CorpusCase[];
  const caseIds = cases.map((item) => item.id);
  assert.deepEqual(caseIds, [...caseIds].sort(), "corpus cases must be sorted");
  assert.equal(new Set(caseIds).size, cases.length);
  const acquisitionCases = indexByCase(
    array(acquisitionDocument["cases"], "acquisition cases") as unknown as AcquisitionCase[],
    "acquisition cases",
  );
  const ruleCases = indexByCase(
    array(ruleDocument["cases"], "rule cases") as unknown as RuleCase[],
    "rule cases",
  );
  assert.deepEqual([...acquisitionCases.keys()], caseIds);
  assert.deepEqual([...ruleCases.keys()], caseIds);

  const metrics = new Map<string, PolicyMetrics>(policyIds.map((id) => [id, emptyMetrics()]));
  const temporaryDirectories: string[] = [];
  const reports = new Map<string, Map<string, Record<string, unknown>>>();
  try {
    for (const caseRecord of cases) {
      const acquisition = acquisitionCases.get(caseRecord.id);
      const rule = ruleCases.get(caseRecord.id);
      assert.ok(acquisition);
      assert.ok(rule);
      assert.equal(rule.executableRule, null);
      assert.equal(rule.expectedOutcome, "untested");
      assert.equal(rule.blockingAllowed, false);
      assert.equal(rule.applicabilityGroundTruth === "inapplicable", caseRecord.classification === "hardNegative");
      const expectationIds = acquisition.policyExpectations.map((item) => item.policy).sort();
      assert.deepEqual(expectationIds, [...policyIds]);
      assert.ok(acquisition.abstentions.length > 0);

      const request = await loadJson(caseRecord.request);
      assertValid(requestValidator, request, `${caseRecord.id} capture request`);
      assert.equal(object(request["network"], "network")["mode"], "deny");
      assert.equal(object(request["privacy"], "privacy")["externalProcessing"], false);
      assert.equal(object(request["fixture"], "fixture")["entrypoint"], source["origin"] + "/index.html");

      const directory = await mkdtemp(join(tmpdir(), `sightlint-segmentation-${caseRecord.id}-`));
      temporaryDirectories.push(directory);
      const artifactIrPath = join(directory, "artifact-ir.json");
      const screenshotPath = join(directory, "screenshot.png");
      const capture = await run(process.execPath, [
        adapterCli,
        "--request", resolve(repositoryRoot, caseRecord.request),
        "--repository-root", repositoryRoot,
        "--artifact-ir-out", artifactIrPath,
        "--screenshot-out", screenshotPath,
      ]);
      assert.equal(capture.code, 0, capture.stderr.toString("utf8"));
      assert.equal(capture.stderr.byteLength, 0);
      const captureResponse = JSON.parse(capture.stdout.toString("utf8")) as Record<string, unknown>;
      assert.equal(captureResponse["status"], "captured");
      const externalRequests = object(captureResponse["capture"], "capture")["externalRequests"];
      assert.ok(Array.isArray(externalRequests) && externalRequests.length === 0);
      assertSupportedScreenshotPng(await readFile(screenshotPath), caseRecord.id);

      const first = await run(sightlintBinary, ["benchmark-image-segmentation", screenshotPath]);
      const second = await run(sightlintBinary, ["benchmark-image-segmentation", screenshotPath]);
      assert.equal(first.code, 0, first.stderr.toString("utf8"));
      assert.equal(first.stderr.byteLength, 0);
      assert.deepEqual(second, first, `${caseRecord.id} benchmark bytes must be stable`);
      const report = JSON.parse(first.stdout.toString("utf8")) as Record<string, unknown>;
      assertValid(reportValidator, report, `${caseRecord.id} benchmark report`);
      assert.equal(report["blocking"], false);
      assert.equal(report["ruleOutcome"], "untested");
      assert.equal(object(report["source"], "report source")["externalProcessing"], false);
      const byPolicy = policyMap(report);
      reports.set(caseRecord.id, byPolicy);

      const inspection = await run(sightlintBinary, ["inspect-image", screenshotPath, "--format", "json"]);
      assert.equal(inspection.code, 0, inspection.stderr.toString("utf8"));
      const inspectionReport = JSON.parse(inspection.stdout.toString("utf8")) as Record<string, unknown>;
      compareStrictDefault(inspectionReport, byPolicy.get("strict-uniform-perimeter-flood-v1")!, caseRecord.id);

      for (const expectation of acquisition.policyExpectations) {
        const policy = byPolicy.get(expectation.policy);
        assert.ok(policy);
        assert.equal(policy["status"], expectation.expectedStatus, `${caseRecord.id} ${expectation.policy} status`);
        assert.equal(policy["reason"], expectation.expectedReason, `${caseRecord.id} ${expectation.policy} reason`);
        assert.equal(policy["semanticApplicability"], "cantTell");
        assert.equal(policy["ruleOutcome"], "untested");
        const selection = object(policy["backgroundSelection"], "background selection");
        assert.equal(selection["confirmed"], false);
        assert.equal(selection["semanticConfidence"], null);
        if (expectation.backgroundUsability === "notSelected") {
          assert.equal(selection["selectedCandidateId"], null, `${caseRecord.id} must abstain before selection`);
        } else {
          assert.equal(typeof selection["selectedCandidateId"], "string", `${caseRecord.id} selected candidate evidence`);
        }
        if (expectation.expectedStatus === "unavailable") {
          assert.deepEqual(policy["regions"], [], `${caseRecord.id} unavailable output must discard partial regions`);
        }
        updateMetrics(metrics.get(expectation.policy)!, caseRecord, expectation, policy, acquisition.regionTargets);
      }
    }

    for (const caseId of ["device-scale-canvas", "modal-surface", "recolored-canvas", "translated-canvas", "uniform-canvas"]) {
      const byPolicy = reports.get(caseId);
      assert.ok(byPolicy);
      assert.deepEqual(
        byPolicy.get("qualified-corner-95-row-runs-v1")!["regions"],
        byPolicy.get("strict-uniform-perimeter-flood-v1")!["regions"],
        `${caseId} row-run and flood-fill four-connectivity must agree`,
      );
    }
    for (const caseId of ["device-scale-canvas", "recolored-canvas", "uniform-canvas"]) {
      const baseline = reports.get("uniform-canvas")!.get("strict-uniform-perimeter-flood-v1")!;
      const candidate = reports.get(caseId)!.get("strict-uniform-perimeter-flood-v1")!;
      assert.deepEqual(
        regionGeometry(candidate["regions"], `${caseId} regions`),
        regionGeometry(baseline["regions"], "uniform-canvas regions"),
        `${caseId} declared invariant topology and bounds`,
      );
    }
    assert.notDeepEqual(
      reports.get("translated-canvas")!.get("strict-uniform-perimeter-flood-v1")!["regions"],
      reports.get("uniform-canvas")!.get("strict-uniform-perimeter-flood-v1")!["regions"],
      "translation mutation must change measured bounds",
    );

    const expected = {
      "strict-uniform-perimeter-flood-v1": {
        usableCases: 6, observedUsableCases: 5, targetRegions: 21, predictedRegions: 5,
        matchedRegions: 1, unmatchedPredictions: 4, falseGroups: 4, maximumEdgeError: 25,
        unsafeCases: 0, unsafeObservations: 0, requiredAbstentions: 3, correctAbstentions: 3,
        acquisitionMutations: 1, observedAcquisitionMutations: 0,
      },
      "ranked-exact-border-flood-v1": {
        usableCases: 7, observedUsableCases: 6, targetRegions: 27, predictedRegions: 7,
        matchedRegions: 2, unmatchedPredictions: 5, falseGroups: 5, maximumEdgeError: 25,
        unsafeCases: 2, unsafeObservations: 2, requiredAbstentions: 0, correctAbstentions: 0,
        acquisitionMutations: 1, observedAcquisitionMutations: 1,
      },
      "qualified-corner-95-row-runs-v1": {
        usableCases: 7, observedUsableCases: 6, targetRegions: 27, predictedRegions: 7,
        matchedRegions: 2, unmatchedPredictions: 5, falseGroups: 5, maximumEdgeError: 25,
        unsafeCases: 0, unsafeObservations: 0, requiredAbstentions: 2, correctAbstentions: 2,
        acquisitionMutations: 1, observedAcquisitionMutations: 1,
      },
    } as const;
    for (const policyId of policyIds) {
      assert.deepEqual(metrics.get(policyId), expected[policyId], `${policyId} reviewed benchmark metrics`);
      const item = metrics.get(policyId)!;
      process.stdout.write(
        `${policyId}: coverage=${item.observedUsableCases}/${item.usableCases}, `
        + `region_precision=${item.matchedRegions}/${item.predictedRegions}, `
        + `region_recall=${item.matchedRegions}/${item.targetRegions}, `
        + `false_groups=${item.falseGroups}, unsafe_hypotheses=${item.unsafeObservations}/${item.unsafeCases}, `
        + `correct_abstentions=${item.correctAbstentions}/${item.requiredAbstentions}, `
        + `acquisition_mutations=${item.observedAcquisitionMutations}/${item.acquisitionMutations}\n`,
      );
    }
    process.stdout.write("rule_mutation_kill_rate=untested (0 executable rules; acquisition and rule ground truth remain separate)\n");
  } finally {
    await Promise.all(temporaryDirectories.map(async (directory) => rm(directory, { recursive: true, force: true })));
  }
});
