import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

type JsonObject = Record<string, unknown>;

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

function string(value: unknown, context: string): string {
  assert.equal(typeof value, "string", `${context} must be a string`);
  return value as string;
}

function indexBy(items: JsonObject[], field: string, context: string): Map<string, JsonObject> {
  const result = new Map<string, JsonObject>();
  for (const item of items) {
    const identifier = string(item[field], `${context}.${field}`);
    assert.equal(result.has(identifier), false, `${context} repeats ${identifier}`);
    result.set(identifier, item);
  }
  return result;
}

function validator(schema: JsonObject): ValidateFunction {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  return ajv.compile(schema);
}

function assertValid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), true, `${context}: ${JSON.stringify(validate.errors)}`);
}

function assertInvalid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), false, `${context} unexpectedly passed schema validation`);
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

test("multi-family registry preserves oracle, review, exposure, and holdout boundaries", async () => {
  const registry = await loadJson("evaluation/web/evaluation-v1.json");
  const registrySchema = await loadJson("evaluation/web/evaluation-v1.schema.json");
  const holdout = await loadJson("evaluation/web/holdout-admission.json");
  const holdoutSchema = await loadJson("evaluation/web/holdout-admission.schema.json");
  const validateRegistry = validator(registrySchema);
  const validateHoldout = validator(holdoutSchema);

  assertValid(validateRegistry, registry, "Web evaluation registry");
  assertValid(validateHoldout, holdout, "holdout admission");

  const families = array(registry["families"], "families");
  const familyIndex = indexBy(families, "id", "families");
  assert.deepEqual([...familyIndex.keys()], [...familyIndex.keys()].toSorted(), "families must be sorted by ID");
  for (const family of families) {
    const sourceRoot = resolve(repositoryRoot, string(family["sourceRoot"], "family.sourceRoot"));
    assert.ok(sourceRoot.startsWith(`${repositoryRoot}/`), "family source root must remain in the repository");
    assert.equal((await stat(sourceRoot)).isDirectory(), true, "family source root must be a directory");

    const review = object(family["review"], "family.review");
    const reviewers = array(review["reviewers"], "family.review.reviewers");
    const reviewerIds = reviewers.map((item) => string(item["id"], "reviewer.id"));
    assert.deepEqual(reviewerIds, [...new Set(reviewerIds)], "reviewer IDs must be unique within a family");
  }

  const datasets = array(registry["datasets"], "datasets");
  const datasetIndex = indexBy(datasets, "id", "datasets");
  assert.deepEqual([...datasetIndex.keys()], [...datasetIndex.keys()].toSorted(), "datasets must be sorted by ID");
  for (const dataset of datasets) {
    const familyId = string(dataset["familyId"], "dataset.familyId");
    assert.ok(familyIndex.has(familyId), `dataset references missing family ${familyId}`);

    const acquisitionReference = object(dataset["acquisitionOracle"], "dataset.acquisitionOracle");
    const ruleReference = object(dataset["ruleOracle"], "dataset.ruleOracle");
    const acquisition = await loadJson(string(acquisitionReference["document"], "acquisition document"));
    const rules = await loadJson(string(ruleReference["document"], "rule document"));
    const acquisitionSchema = await loadJson(string(acquisitionReference["schema"], "acquisition schema"));
    const ruleSchema = await loadJson(string(ruleReference["schema"], "rule schema"));
    const validateAcquisition = validator(acquisitionSchema);
    const validateRules = validator(ruleSchema);

    assertValid(validateAcquisition, acquisition, `${familyId} acquisition oracle`);
    assertValid(validateRules, rules, `${familyId} rule oracle`);
    assert.equal(acquisition["documentType"], acquisitionReference["documentType"]);
    assert.equal(rules["documentType"], ruleReference["documentType"]);

    const acquisitionCases = indexBy(array(acquisition["cases"], "acquisition cases"), "caseId", "acquisition cases");
    const ruleCases = indexBy(array(rules["cases"], "rule cases"), "caseId", "rule cases");
    const inventory = indexBy(array(dataset["cases"], "dataset cases"), "caseId", "dataset cases");
    const expectedIds = [...inventory.keys()].toSorted();
    assert.deepEqual([...acquisitionCases.keys()].toSorted(), expectedIds, `${familyId} acquisition inventory`);
    assert.deepEqual([...ruleCases.keys()].toSorted(), expectedIds, `${familyId} rule inventory`);
    assert.deepEqual([...inventory.keys()], expectedIds, `${familyId} registry inventory must be sorted`);

    for (const [caseId, registered] of inventory) {
      const acquisitionCase = object(acquisitionCases.get(caseId), `${caseId} acquisition case`);
      const ruleCase = object(ruleCases.get(caseId), `${caseId} rule case`);
      for (const field of ["request", "classification"] as const) {
        assert.equal(acquisitionCase[field], registered[field], `${caseId} acquisition ${field}`);
        assert.equal(ruleCase[field], registered[field], `${caseId} rule ${field}`);
      }
      assert.equal(acquisitionCase["split"], registered["split"], `${caseId} acquisition split`);
    }

    const mixedAcquisition = clone(acquisition);
    array(mixedAcquisition["cases"], "mixed acquisition cases")[0]!["expectedOutcome"] = "passed";
    assertInvalid(validateAcquisition, mixedAcquisition, `${familyId} acquisition oracle containing a rule verdict`);
    const mixedRule = clone(rules);
    array(mixedRule["cases"], "mixed rule cases")[0]!["expectations"] = {};
    assertInvalid(validateRules, mixedRule, `${familyId} rule oracle containing acquisition expectations`);
  }

  assert.equal(object(registry["holdoutAdmission"], "holdout reference")["status"], holdout["status"]);
  assert.equal(holdout["status"], "notOperational");
  assert.equal(holdout["publicCasesEligible"], false);

  const invalidIndependentReview = clone(registry);
  object(array(invalidIndependentReview["families"], "families")[0]!["review"], "review")["status"] = "independentlyReviewed";
  assertInvalid(validateRegistry, invalidIndependentReview, "independent review without an independent reviewer");

  const invalidExposure = clone(registry);
  object(array(invalidExposure["families"], "families")[0]!["exposure"], "exposure")["classification"] = "controlledHoldout";
  assertInvalid(validateRegistry, invalidExposure, "controlled holdout exposed to tuning");

  const incompleteOperational = clone(holdout);
  incompleteOperational["status"] = "operational";
  delete incompleteOperational["blockers"];
  assertInvalid(validateHoldout, incompleteOperational, "operational holdout without admission metadata");

  const operational = clone(incompleteOperational);
  operational["operationalRecord"] = {
    bundleId: "opaque-web-holdout",
    bundleVersion: "1.0.0",
    bundleDigest: `sha256:${"0".repeat(64)}`,
    freezeCommit: "1".repeat(40),
    storageAuthority: "Separately administered evaluation storage",
    accessPolicyVersion: "1.0.0",
    accessRoles: [{ role: "independentEvaluator", artifactAccess: true, labelAccess: true, tuningAccess: false }],
    evaluator: {
      id: "independent-evaluator",
      qualification: "Qualified UI evaluation reviewer",
      independentFromTuning: true,
      conflictOfInterestReviewed: true,
    },
    exposureLog: [],
    tuningExclusion: "Bundle artifacts, labels, membership, and case-level results are excluded from implementation tuning.",
    evaluationCommand: "pinned evaluator command",
    environmentManifestDigest: `sha256:${"2".repeat(64)}`,
    oracleCorrectionProcedure: "evaluation/web/annotation-guide-v1.md",
    reportingPlan: "Release split- and family-specific integer metrics with exclusions and non-claims.",
    admittedAt: "2026-09-06",
    admittedBy: "independent-evaluation-authority",
  };
  assertValid(validateHoldout, operational, "complete future operational holdout record");
  delete object(operational["operationalRecord"], "operational record")["evaluator"];
  assertInvalid(validateHoldout, operational, "operational holdout without evaluator");
});
