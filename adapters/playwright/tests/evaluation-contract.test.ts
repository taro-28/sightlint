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
  const ajv = new Ajv2020({ allErrors: true, strict: true, strictRequired: false, validateFormats: false });
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

function assertMaximum(
  validate: ValidateFunction,
  base: JsonObject,
  maximum: number,
  mutate: (value: JsonObject, boundary: number) => void,
  context: string,
): void {
  const exactBoundary = clone(base);
  mutate(exactBoundary, maximum);
  assertValid(validate, exactBoundary, `${context} exact boundary`);
  const oneOver = clone(base);
  mutate(oneOver, maximum + 1);
  assertInvalid(validate, oneOver, `${context} one over`);
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

test("holdout foundation schemas separate private evidence from sanitized public status", async () => {
  const bundle = await loadJson("evaluation/web/conformance/holdout/bundle-manifest.json");
  const oracle = await loadJson("evaluation/web/conformance/holdout/oracle-manifest.json");
  const invocation = await loadJson("evaluation/web/conformance/holdout/invocation-manifest.json");
  const privateResult = await loadJson("evaluation/web/conformance/holdout/private-result-manifest.json");
  const publicConformance = await loadJson("evaluation/web/conformance/holdout/public-attestation.json");
  const currentStatus = await loadJson("evaluation/web/holdout-run.json");

  const validateBundle = validator(await loadJson("evaluation/web/protected-holdout-bundle.schema.json"));
  const validateOracle = validator(await loadJson("evaluation/web/protected-holdout-oracle.schema.json"));
  const validateInvocation = validator(await loadJson("evaluation/web/holdout-invocation.schema.json"));
  const validatePrivateResult = validator(await loadJson("evaluation/web/private-holdout-result.schema.json"));
  const validateAttestation = validator(await loadJson("evaluation/web/holdout-run.schema.json"));

  assertValid(validateBundle, bundle, "conformance bundle manifest");
  assertValid(validateOracle, oracle, "conformance oracle manifest");
  assertValid(validateInvocation, invocation, "conformance invocation manifest");
  assertValid(validatePrivateResult, privateResult, "conformance private result manifest");
  assertValid(validateAttestation, publicConformance, "conformance public attestation");
  assertValid(validateAttestation, currentStatus, "current not-run status");

  const exposedProtectedBundle = clone(bundle);
  exposedProtectedBundle["dataClassification"] = "protectedHoldout";
  assertInvalid(validateBundle, exposedProtectedBundle, "protected bundle visible to tuning");

  const privatePublicFixture = clone(bundle);
  object(privatePublicFixture["privacy"], "bundle privacy")["containsPersonalData"] = true;
  assertInvalid(validateBundle, privatePublicFixture, "public conformance bundle containing personal data");

  const mixedOracle = clone(oracle);
  mixedOracle["implementationOutput"] = { acceptedAsTruth: true };
  assertInvalid(validateOracle, mixedOracle, "oracle containing implementation output");

  const unreviewedProtectedOracle = clone(oracle);
  unreviewedProtectedOracle["dataClassification"] = "protectedHoldout";
  assertInvalid(validateOracle, unreviewedProtectedOracle, "protected oracle without independent review");

  const shellInvocation = clone(invocation);
  object(shellInvocation["commands"], "invocation commands")["shellInterpolation"] = true;
  assertInvalid(validateInvocation, shellInvocation, "holdout invocation using a shell");

  const privateResultWithPublicClaim = clone(privateResult);
  privateResultWithPublicClaim["evidenceEligible"] = true;
  assertInvalid(validatePrivateResult, privateResultWithPublicClaim, "private result containing a public evidence claim");

  const disclosedSmallCell = clone(publicConformance);
  const suppressed = array(disclosedSmallCell["metrics"], "public metrics")
    .find((metric) => metric["publication"] === "suppressed");
  assert.ok(suppressed);
  suppressed["numerator"] = 2;
  assertInvalid(validateAttestation, disclosedSmallCell, "suppressed metric disclosing a numerator");

  const notRunWithResultBinding = clone(currentStatus);
  notRunWithResultBinding["bindings"] = object(publicConformance["bindings"], "conformance bindings");
  assertInvalid(validateAttestation, notRunWithResultBinding, "not-run status containing result bindings");

  const futureEvidenceShape = clone(publicConformance);
  futureEvidenceShape["recordPurpose"] = "holdoutEvidence";
  futureEvidenceShape["dataClassification"] = "sanitizedProtectedResult";
  futureEvidenceShape["evidenceEligible"] = true;
  object(futureEvidenceShape["admission"], "future evidence admission")["status"] = "operational";
  assertValid(validateAttestation, futureEvidenceShape, "future operational evidence shape");
  object(futureEvidenceShape["admission"], "future evidence admission")["status"] = "notOperational";
  assertInvalid(validateAttestation, futureEvidenceShape, "evidence claim without operational admission");

  const invalidatedEvidenceShape = clone(futureEvidenceShape);
  object(invalidatedEvidenceShape["admission"], "invalidated evidence admission")["status"] = "operational";
  invalidatedEvidenceShape["lifecycle"] = "invalidated";
  invalidatedEvidenceShape["evidenceEligible"] = false;
  invalidatedEvidenceShape["invalidation"] = {
    invalidatedAt: "2026-09-06T00:20:00Z",
    reason: "Conformance-only lifecycle shape test.",
    authorityId: "conformance-authority",
  };
  assertValid(validateAttestation, invalidatedEvidenceShape, "future invalidated evidence shape");

  const incompleteInvalidation = clone(publicConformance);
  incompleteInvalidation["lifecycle"] = "invalidated";
  assertInvalid(validateAttestation, incompleteInvalidation, "invalidated evidence without an invalidation record");
});

test("holdout foundation schemas enforce every declared collection and byte boundary", async () => {
  const bundle = await loadJson("evaluation/web/conformance/holdout/bundle-manifest.json");
  const oracle = await loadJson("evaluation/web/conformance/holdout/oracle-manifest.json");
  const invocation = await loadJson("evaluation/web/conformance/holdout/invocation-manifest.json");
  const privateResult = await loadJson("evaluation/web/conformance/holdout/private-result-manifest.json");
  const publicAttestation = await loadJson("evaluation/web/conformance/holdout/public-attestation.json");
  const currentStatus = await loadJson("evaluation/web/holdout-run.json");
  const validateBundle = validator(await loadJson("evaluation/web/protected-holdout-bundle.schema.json"));
  const validateOracle = validator(await loadJson("evaluation/web/protected-holdout-oracle.schema.json"));
  const validateInvocation = validator(await loadJson("evaluation/web/holdout-invocation.schema.json"));
  const validatePrivateResult = validator(await loadJson("evaluation/web/private-holdout-result.schema.json"));
  const validateAttestation = validator(await loadJson("evaluation/web/holdout-run.schema.json"));

  assertMaximum(validateBundle, bundle, 128, (value, boundary) => {
    object(value["limits"], "bundle limits")["maximumFamilies"] = boundary;
  }, "declared family limit");
  assertMaximum(validateBundle, bundle, 4096, (value, boundary) => {
    object(value["limits"], "bundle limits")["maximumCases"] = boundary;
  }, "declared case limit");
  assertMaximum(validateBundle, bundle, 64, (value, boundary) => {
    object(value["limits"], "bundle limits")["maximumFilesPerCase"] = boundary;
  }, "declared files-per-case limit");
  assertMaximum(validateBundle, bundle, 1_048_576, (value, boundary) => {
    object(value["limits"], "bundle limits")["maximumManifestBytes"] = boundary;
  }, "declared manifest-byte limit");
  assertMaximum(validateBundle, bundle, 128, (value, boundary) => {
    const family = array(value["families"], "families")[0]!;
    value["families"] = Array.from({ length: boundary }, () => clone(family));
  }, "family collection");
  assertMaximum(validateBundle, bundle, 4096, (value, boundary) => {
    const sample = clone(array(value["cases"], "cases")[0]!);
    sample["files"] = [clone(array(sample["files"], "sample files")[0]!)];
    value["cases"] = Array.from({ length: boundary }, () => clone(sample));
  }, "case collection");
  assertMaximum(validateBundle, bundle, 64, (value, boundary) => {
    const firstCase = array(value["cases"], "cases")[0]!;
    const sample = array(firstCase["files"], "case files")[0]!;
    firstCase["files"] = Array.from({ length: boundary }, () => clone(sample));
  }, "files-per-case collection");
  assertMaximum(validateBundle, bundle, 1_048_576, (value, boundary) => {
    array(array(value["cases"], "cases")[0]!["files"], "case files")[0]!["byteLength"] = boundary;
  }, "raw file byte length");
  assertMaximum(validateBundle, bundle, 512, (value, boundary) => {
    object(value["provenance"], "provenance")["ownershipBasis"] = "a".repeat(boundary);
  }, "printable string length");
  assertMaximum(validateBundle, bundle, 128, (value, boundary) => {
    object(value["bundle"], "bundle")["id"] = `a${"b".repeat(boundary - 1)}`;
  }, "stable identifier length");
  assertMaximum(validateBundle, bundle, 512, (value, boundary) => {
    array(array(value["cases"], "cases")[0]!["files"], "case files")[0]!["path"] = "a".repeat(boundary);
  }, "relative path length");

  assertMaximum(validateOracle, oracle, 4096, (value, boundary) => {
    value["caseIds"] = Array.from({ length: boundary }, (_, index) => `case-${index.toString().padStart(4, "0")}`);
  }, "oracle case ID collection");
  for (const field of ["acquisitionDocuments", "ruleDocuments"] as const) {
    assertMaximum(validateOracle, oracle, 128, (value, boundary) => {
      const sample = array(value[field], field)[0]!;
      value[field] = Array.from({ length: boundary }, () => clone(sample));
    }, `${field} collection`);
  }
  assertMaximum(validateOracle, oracle, 64, (value, boundary) => {
    const oracleMetadata = object(value["oracle"], "oracle");
    const sample = array(oracleMetadata["reviewers"], "reviewers")[0]!;
    oracleMetadata["reviewers"] = Array.from({ length: boundary }, () => clone(sample));
  }, "reviewer collection");
  for (const [field, maximum] of [["classificationCounts", 6], ["acquisitionExpectationCounts", 4], ["ruleOutcomeCounts", 5]] as const) {
    assertMaximum(validateOracle, oracle, maximum, (value, boundary) => {
      const sample = array(value[field], field)[0]!;
      value[field] = Array.from({ length: boundary }, () => clone(sample));
    }, `${field} collection`);
  }
  assertMaximum(validateOracle, oracle, 1_000_000, (value, boundary) => {
    object(value["oracle"], "oracle")["unresolvedDisagreements"] = boundary;
  }, "oracle disagreement count");
  assertMaximum(validateOracle, oracle, 1_048_576, (value, boundary) => {
    array(value["acquisitionDocuments"], "acquisition documents")[0]!["byteLength"] = boundary;
  }, "oracle document byte length");

  assertMaximum(validateInvocation, invocation, 128, (value, boundary) => {
    object(value["commands"], "commands")["captureArgv"] = Array.from({ length: boundary }, () => "argument");
  }, "command argument collection");
  assertMaximum(validateInvocation, invocation, 128, (value, boundary) => {
    const evaluation = object(value["evaluationContract"], "evaluation contract");
    const sample = array(evaluation["rules"], "rules")[0]!;
    evaluation["rules"] = Array.from({ length: boundary }, () => clone(sample));
  }, "rule binding collection");
  const invocationLimits: Array<[string, number]> = [
    ["maximumCases", 4096],
    ["maximumCaseSeconds", 3600],
    ["maximumOutputBytes", 1_073_741_824],
    ["maximumManifestBytes", 1_048_576],
  ];
  for (const [field, maximum] of invocationLimits) {
    assertMaximum(validateInvocation, invocation, maximum, (value, boundary) => {
      object(value["resourceLimits"], "resource limits")[field] = boundary;
    }, `invocation ${field}`);
  }
  for (const [field, maximum] of [["viewportWidthCssPixels", 32768], ["viewportHeightCssPixels", 32768], ["deviceScaleFactor", 8], ["textScale", 4]] as const) {
    assertMaximum(validateInvocation, invocation, maximum, (value, boundary) => {
      object(value["environment"], "environment")[field] = boundary;
    }, `environment ${field}`);
  }

  assertMaximum(validatePrivateResult, privateResult, 4096, (value, boundary) => {
    const sample = array(value["caseResults"], "case results")[0]!;
    value["caseResults"] = Array.from({ length: boundary }, () => clone(sample));
  }, "private case result collection");
  assertMaximum(validatePrivateResult, privateResult, 512, (value, boundary) => {
    const sample = array(value["metricCells"], "metric cells")[0]!;
    value["metricCells"] = Array.from({ length: boundary }, () => clone(sample));
  }, "private metric collection");
  for (const field of ["attemptedCases", "completedCases", "executionErrors"] as const) {
    assertMaximum(validatePrivateResult, privateResult, 4096, (value, boundary) => {
      object(value["execution"], "execution")[field] = boundary;
    }, `private execution ${field}`);
  }
  for (const field of ["numerator", "denominator"] as const) {
    assertMaximum(validatePrivateResult, privateResult, 1_000_000, (value, boundary) => {
      array(value["metricCells"], "metric cells")[0]![field] = boundary;
    }, `private metric ${field}`);
  }

  assertMaximum(validateAttestation, publicAttestation, 512, (value, boundary) => {
    const sample = array(value["metrics"], "public metrics")[0]!;
    value["metrics"] = Array.from({ length: boundary }, () => clone(sample));
  }, "public metric collection");
  assertMaximum(validateAttestation, publicAttestation, 32, (value, boundary) => {
    value["nonClaims"] = Array.from({ length: boundary }, (_, index) => `claim-${index}`);
  }, "public non-claim collection");
  assertMaximum(validateAttestation, currentStatus, 32, (value, boundary) => {
    value["blockers"] = Array.from({ length: boundary }, (_, index) => `blocker-${index}`);
  }, "current blocker collection");
  assertMaximum(validateAttestation, publicAttestation, 1_000_000, (value, boundary) => {
    object(value["disclosure"], "disclosure")["minimumPublishedDenominator"] = boundary;
  }, "publication denominator threshold");
});
