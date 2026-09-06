import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

import { canonicalJson, sha256 } from "../src/canonical.js";
import type { JsonValue } from "../src/types.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/cli.js");
const perceptionCli = resolve(repositoryRoot, "adapters/perception/src/cli.mjs");
const referenceWorker = resolve(repositoryRoot, "adapters/perception/src/reference-worker.mjs");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface CorpusCase {
  id: string;
  split: "smoke" | "development" | "challenge";
  classification: "clean" | "targetedMutation" | "hardNegative";
  request: string;
  baselineCaseId?: string;
}

interface FamilyExpectation {
  family: "hierarchy" | "peerGroup" | "region" | "role" | "text";
  expectedStatus: "observed" | "partial" | "unsupported" | "ambiguous" | "untested";
}

interface AcquisitionCase {
  caseId: string;
  familyExpectations: FamilyExpectation[];
  nativePixelComparison: "recordSeparately" | "conflictRequired";
  requiredPreservedFacts: string[];
  abstentions: string[];
}

interface RuleCase {
  caseId: string;
  executableRule: null;
  applicabilityGroundTruth: "cantTell" | "inapplicable";
  expectedOutcome: "untested";
  blockingAllowed: false;
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
      if (signal !== null) return reject(new Error(`${program} terminated by ${signal}`));
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

function objects(value: unknown, context: string): Array<Record<string, unknown>> {
  assert.ok(Array.isArray(value), `${context} must be an array`);
  return value as Array<Record<string, unknown>>;
}

async function validator(path: string): Promise<ValidateFunction> {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  return ajv.compile(await loadJson(path));
}

function assertValid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), true, `${context}: ${JSON.stringify(validate.errors)}`);
}

function indexByCase<T extends { caseId: string }>(values: T[]): Map<string, T> {
  const ids = values.map((value) => value.caseId);
  assert.deepEqual(ids, [...ids].sort());
  return new Map(values.map((value) => [value.caseId, value]));
}

function perceptionRequest(
  caseRecord: CorpusCase,
  captureRequest: Record<string, unknown>,
  benchmark: Record<string, unknown>,
): Record<string, unknown> {
  const contentBytes = canonicalJson(benchmark as JsonValue);
  const artifact = object(captureRequest["artifact"], "capture request artifact");
  const canvas = object(benchmark["canvas"], "benchmark canvas");
  return {
    $schema: "../schemas/request.schema.json",
    protocolVersion: "0.1.0",
    requestId: `perception-${caseRecord.id}`,
    artifact: { id: artifact["id"], kind: "web", title: artifact["title"] },
    input: {
      reference: `benchmark/${caseRecord.id}.json`,
      mediaType: "application/vnd.sightlint.image-segmentation-benchmark+json",
      sha256: sha256(contentBytes),
      byteLength: Buffer.byteLength(contentBytes),
      content: benchmark,
      canvas,
    },
    preprocessing: {
      pipeline: "sightlint-image-segmentation-report",
      version: "0.1.0",
      policyId: "ranked-exact-border-flood-v1",
      crop: null,
      scale: { x: 1, y: 1 },
      tile: { status: "notApplied" },
      randomSeed: { status: "notApplicable" },
    },
    worker: {
      expectedName: "sightlint-reference-region-worker",
      expectedVersion: "0.1.0",
      backend: "cpu",
      model: { status: "notApplicable" },
    },
    execution: {
      mode: "local", timeoutMs: 2000, maxOutputBytes: 4_194_304, maxStderrBytes: 4096,
      maxObservations: 1024, maxTextLength: 4096, maxHierarchyDepth: 32,
    },
    privacy: { externalProcessing: false, remoteTransmittedFields: [], retention: "none", redaction: { status: "notApplied" } },
    output: { artifactIrReference: `perception/${caseRecord.id}-artifact-ir.json`, responseReference: `perception/${caseRecord.id}-response.json` },
  };
}

function nodeById(artifact: Record<string, unknown>, id: string): Record<string, unknown> {
  const node = objects(artifact["nodes"], "Artifact IR nodes").find((item) => item["id"] === id);
  assert.ok(node, `missing native node ${id}`);
  return node;
}

function regionSignatures(response: Record<string, unknown>): string[] {
  return objects(response["observations"], "perception observations")
    .map((observation) => canonicalJson(object(object(observation["value"], "observation value")["bounds"], "bounds") as JsonValue))
    .sort();
}

test("local perception worker preserves native/pixel evidence and abstention through public processes", { timeout: 300_000 }, async () => {
  const corpus = await loadJson("evaluation/perception/corpus.json");
  const acquisitionDocument = await loadJson("evaluation/perception/annotations/acquisition.json");
  const ruleDocument = await loadJson("evaluation/perception/annotations/rules.json");
  const captureValidator = await validator("adapters/playwright/schemas/capture-request.schema.json");
  const requestValidator = await validator("adapters/perception/schemas/request.schema.json");
  const responseValidator = await validator("adapters/perception/schemas/response.schema.json");
  const runValidator = await validator("adapters/perception/schemas/run-report.schema.json");
  const extensionValidator = await validator("adapters/perception/schemas/perception-extension.schema.json");
  const cases = objects(corpus["cases"], "perception corpus cases") as unknown as CorpusCase[];
  const ids = cases.map((item) => item.id);
  assert.deepEqual(ids, [...ids].sort());
  const acquisitionCases = indexByCase(objects(acquisitionDocument["cases"], "acquisition cases") as unknown as AcquisitionCase[]);
  const ruleCases = indexByCase(objects(ruleDocument["cases"], "rule cases") as unknown as RuleCase[]);
  assert.deepEqual([...acquisitionCases.keys()], ids);
  assert.deepEqual([...ruleCases.keys()], ids);
  const regions = new Map<string, string[]>();
  let nativeConflicts = 0;
  let hardNegativeFailures = 0;
  let observedRegionFamilies = 0;
  let familyAbstentions = 0;
  const directories: string[] = [];

  try {
    for (const caseRecord of cases) {
      const acquisition = acquisitionCases.get(caseRecord.id)!;
      const rule = ruleCases.get(caseRecord.id)!;
      assert.equal(rule.executableRule, null);
      assert.equal(rule.expectedOutcome, "untested");
      assert.equal(rule.blockingAllowed, false);
      assert.equal(rule.applicabilityGroundTruth === "inapplicable", caseRecord.classification === "hardNegative");
      assert.ok(acquisition.requiredPreservedFacts.length > 0 && acquisition.abstentions.length > 0);

      const captureRequest = await loadJson(caseRecord.request);
      assertValid(captureValidator, captureRequest, `${caseRecord.id} capture request`);
      const directory = await mkdtemp(join(tmpdir(), `sightlint-perception-${caseRecord.id}-`));
      directories.push(directory);
      const nativeArtifactPath = join(directory, "native-artifact-ir.json");
      const screenshotPath = join(directory, "screenshot.png");
      const capture = await run(process.execPath, [
        adapterCli, "--request", resolve(repositoryRoot, caseRecord.request), "--repository-root", repositoryRoot,
        "--artifact-ir-out", nativeArtifactPath, "--screenshot-out", screenshotPath,
      ]);
      assert.equal(capture.code, 0, capture.stderr.toString("utf8"));
      const captureResponse = JSON.parse(capture.stdout.toString("utf8")) as Record<string, unknown>;
      assert.equal(object(captureResponse["capture"], "capture")["externalRequests"] instanceof Array, true);
      assert.deepEqual(object(captureResponse["capture"], "capture")["externalRequests"], []);

      const benchmarkResult = await run(sightlintBinary, ["benchmark-image-segmentation", screenshotPath]);
      assert.equal(benchmarkResult.code, 0, benchmarkResult.stderr.toString("utf8"));
      const benchmark = JSON.parse(benchmarkResult.stdout.toString("utf8")) as Record<string, unknown>;
      const request = perceptionRequest(caseRecord, captureRequest, benchmark);
      assertValid(requestValidator, request, `${caseRecord.id} perception request`);
      const requestPath = join(directory, "perception-request.json");
      await writeFile(requestPath, canonicalJson(request as JsonValue));

      const runOnce = async (suffix: string): Promise<{ result: ProcessResult; response: Buffer; artifact: Buffer }> => {
        const responsePath = join(directory, `perception-response-${suffix}.json`);
        const artifactPath = join(directory, `perception-artifact-${suffix}.json`);
        const result = await run(process.execPath, [
          perceptionCli, "--request", requestPath,
          "--worker-program", process.execPath, "--worker-argument", referenceWorker, "--worker-source", referenceWorker,
          "--sightlint-binary", sightlintBinary, "--response-out", responsePath, "--artifact-ir-out", artifactPath,
        ]);
        return { result, response: await readFile(responsePath), artifact: await readFile(artifactPath) };
      };
      const first = await runOnce("first");
      const second = await runOnce("second");
      assert.equal(first.result.code, 0, first.result.stderr.toString("utf8"));
      assert.equal(first.result.stderr.byteLength, 0);
      assert.deepEqual(second, first, `${caseRecord.id} perception bytes must repeat exactly`);
      const runReport = JSON.parse(first.result.stdout.toString("utf8")) as Record<string, unknown>;
      const response = JSON.parse(first.response.toString("utf8")) as Record<string, unknown>;
      const perceptionArtifact = JSON.parse(first.artifact.toString("utf8")) as Record<string, unknown>;
      assertValid(runValidator, runReport, `${caseRecord.id} run report`);
      assertValid(responseValidator, response, `${caseRecord.id} worker response`);
      const extension = object(object(perceptionArtifact["extensions"], "extensions")["org.sightlint.perception"], "perception extension");
      assertValid(extensionValidator, extension, `${caseRecord.id} perception extension`);
      assert.equal(runReport["blocking"], false);
      assert.equal(runReport["ruleOutcome"], "untested");
      assert.equal(object(extension["mapping"], "mapping")["coreSemanticPromotionCount"], 0);
      assert.equal(perceptionArtifact["relations"], undefined);
      for (const node of objects(perceptionArtifact["nodes"], "perception nodes")) {
        assert.equal(node["role"], undefined);
        assert.equal(node["name"], undefined);
        assert.equal(node["parentId"], undefined);
      }
      const statusByFamily = new Map(objects(response["familyStatus"], "family status").map((item) => [item["family"], item["status"]]));
      for (const expected of acquisition.familyExpectations) {
        assert.equal(statusByFamily.get(expected.family), expected.expectedStatus, `${caseRecord.id} ${expected.family}`);
        if (expected.family === "region" && expected.expectedStatus === "observed") observedRegionFamilies += 1;
        if (["unsupported", "ambiguous", "untested"].includes(expected.expectedStatus)) familyAbstentions += 1;
      }
      regions.set(caseRecord.id, regionSignatures(response));

      const nativeArtifact = JSON.parse(await readFile(nativeArtifactPath, "utf8")) as Record<string, unknown>;
      if (caseRecord.id === "dashboard-browser-spacing-mutant") {
        const node = nodeById(nativeArtifact, "web-metric-retention");
        const geometry = object(node["geometry"], "native geometry");
        const layout = object(object(geometry["layoutBox"], "layoutBox")["rect"], "layout rect");
        const rendered = object(object(geometry["renderBox"], "renderBox")["rect"], "render rect");
        const renderOffset = Number(rendered["x"]) - Number(layout["x"]);
        assert.ok(Math.abs(renderOffset - 16) <= 1, `expected the reviewed 16±1 CSS-pixel offset, got ${renderOffset}`);
        assert.equal(acquisition.nativePixelComparison, "conflictRequired");
        nativeConflicts += 1;
      }
      if (caseRecord.id === "dashboard-browser-intentional-grouping") {
        const promotion = nodeById(nativeArtifact, "web-promotion-card");
        assert.equal(object(promotion["role"], "promotion role")["value"], "complementary");
        const check = await run(sightlintBinary, ["check", join(directory, "perception-artifact-first.json"), "--format", "json"]);
        assert.equal(check.code, 0, check.stderr.toString("utf8"));
        const checked = JSON.parse(check.stdout.toString("utf8")) as Record<string, unknown>;
        const failed = Number(object(checked["summary"], "check summary")["failed"]);
        hardNegativeFailures += failed;
      }
    }

    assert.equal(nativeConflicts, 1);
    assert.equal(hardNegativeFailures, 0);
    assert.equal(observedRegionFamilies, 3);
    assert.equal(familyAbstentions, 12);
    assert.notDeepEqual(regions.get("dashboard-browser-spacing-mutant"), regions.get("dashboard-browser-clean"), "targeted render mutation must change measured region geometry");
    process.stdout.write("perception protocol v0: case_coverage=3/3, region_family_coverage=3/3, family_abstentions=12/15, deterministic=3/3, native_conflicts_preserved=1/1, acquisition_mutations_observed=1/1, semantic_claims=0, semantic_hard_negative_failures=0/1, region_precision=untested, rule_mutation_kill_rate=untested, OCR/role/hierarchy/peer/rule_accuracy=untested\n");
  } finally {
    await Promise.all(directories.map((directory) => rm(directory, { recursive: true, force: true })));
  }
});
