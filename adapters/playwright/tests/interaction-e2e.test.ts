import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

import { canonicalJson, sha256 } from "../src/canonical.js";
import type { JsonValue } from "../src/types.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/interaction-cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface CorpusCase {
  id: string;
  split: "smoke" | "development" | "challenge";
  request: { path: string; sha256: string };
  acquisitionAnnotationId: string;
  ruleAnnotationId: string;
  relation: { kind: "baseline" | "targetedMutation" | "hardNegative" | "abstentionCase" };
}

interface AcquisitionFact {
  kind: string;
  attemptId: string;
  state?: string;
  resolution?: string;
  recovery?: string;
  evidenceSources: string[];
}

interface AcquisitionAnnotation {
  id: string;
  caseId: string;
  traceExecution: string;
  orderedFacts: AcquisitionFact[];
  conflicts: string[];
  abstentions: string[];
}

interface RuleExpectation {
  ruleId: string;
  ruleVersion: string;
  outcome: string;
}

interface RuleAnnotation {
  id: string;
  caseId: string;
  expectedExit: number;
  expectations: RuleExpectation[];
  forbidUnexpectedFailures: boolean;
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

function strings(value: unknown, context: string): string[] {
  assert.ok(Array.isArray(value) && value.every((item) => typeof item === "string"), `${context} must be strings`);
  return value as string[];
}

async function validator(path: string): Promise<ValidateFunction> {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  return ajv.compile(await loadJson(path));
}

function assertValid(validate: ValidateFunction, value: unknown, context: string): void {
  assert.equal(validate(value), true, `${context}: ${JSON.stringify(validate.errors)}`);
}

function sourceFamily(evidenceId: string): string {
  if (evidenceId.startsWith("e-action-")) return "browserAction";
  if (evidenceId.startsWith("e-dom-")) return "domState";
  if (evidenceId.startsWith("e-ax-")) return "accessibilityState";
  if (evidenceId.startsWith("e-screenshot-")) return "screenshot";
  if (evidenceId.startsWith("e-app-")) return "appDeclaredInstrumentation";
  throw new Error(`unexpected interaction evidence ${evidenceId}`);
}

function observedFacts(ir: Record<string, unknown>): AcquisitionFact[] {
  const interaction = object(object(ir["extensions"], "extensions")["org.sightlint.interaction"], "interaction extension");
  const trace = objects(interaction["traces"], "traces")[0];
  assert.ok(trace !== undefined);
  return objects(trace["events"], "trace events").map((event) => {
    const detail = object(event["detail"], "event detail");
    const fact: AcquisitionFact = {
      kind: String(detail["kind"]),
      attemptId: String(event["attemptId"]),
      evidenceSources: [...new Set(strings(event["evidenceIds"], "event evidence").map(sourceFamily))].sort(),
    };
    for (const field of ["state", "resolution", "recovery"] as const) {
      if (typeof detail[field] === "string") fact[field] = detail[field];
    }
    return fact;
  });
}

function normalizedFacts(facts: AcquisitionFact[]): AcquisitionFact[] {
  return facts.map((fact) => ({ ...fact, evidenceSources: [...fact.evidenceSources].sort() }));
}

async function fixtureDigest(): Promise<string> {
  const directory = resolve(repositoryRoot, "evaluation/interaction/fixture-app");
  const files = (await readdir(directory)).sort();
  const digest = createHash("sha256");
  for (const name of files) {
    const path = `evaluation/interaction/fixture-app/${name}`;
    digest.update(path, "utf8");
    digest.update(Buffer.from([0]));
    digest.update(await readFile(resolve(repositoryRoot, path)));
  }
  return `sha256:${digest.digest("hex")}`;
}

test("interaction schemas, provenance, annotations, and request digests are strict", async () => {
  const corpus = await loadJson("evaluation/interaction/corpus.json");
  const acquisition = await loadJson("evaluation/interaction/annotations/acquisition.json");
  const rules = await loadJson("evaluation/interaction/annotations/rules.json");
  const metrics = await loadJson("evaluation/interaction/metric-contract.json");
  for (const [schemaPath, value] of [
    ["evaluation/interaction/corpus.schema.json", corpus],
    ["evaluation/interaction/acquisition-annotation.schema.json", acquisition],
    ["evaluation/interaction/rule-annotation.schema.json", rules],
    ["evaluation/interaction/metric-contract.schema.json", metrics],
  ] as const) {
    assertValid(await validator(schemaPath), value, schemaPath);
  }
  assert.equal(corpus["fixtureSourceSha256"], await fixtureDigest());
  const cases = objects(corpus["cases"], "cases") as unknown as CorpusCase[];
  assert.deepEqual(cases.map((item) => item.id), [...cases.map((item) => item.id)].sort());
  const acquisitionIds = new Set(objects(acquisition["annotations"], "acquisition annotations").map((item) => item["id"]));
  const ruleIds = new Set(objects(rules["annotations"], "rule annotations").map((item) => item["id"]));
  assert.equal(acquisitionIds.size, cases.length);
  assert.equal(ruleIds.size, cases.length);
  assert.deepEqual(
    objects(metrics["metrics"], "metrics").map((item) => item["id"]).sort(),
    [
      "abstentionRetention",
      "acquisitionFactCoverage",
      "evaluatedCaseCoverage",
      "failurePrecision",
      "falsePositiveRate",
      "mutationKillRate",
    ],
  );
  const requestValidator = await validator("adapters/playwright/schemas/interaction-request.schema.json");
  for (const item of cases) {
    assert.ok(acquisitionIds.has(item.acquisitionAnnotationId));
    assert.ok(ruleIds.has(item.ruleAnnotationId));
    const requestBytes = await readFile(resolve(repositoryRoot, item.request.path));
    assert.equal(sha256(requestBytes), item.request.sha256);
    assertValid(requestValidator, JSON.parse(requestBytes.toString("utf8")), item.id);
  }
  assert.equal(metrics["implementationOutputsStoredAsOracle"], false);
  assert.equal(object(corpus["holdout"], "holdout")["status"], "notEstablished");
});

test("controlled adapter and public binary match separate acquisition and rule truth", async () => {
  const corpus = await loadJson("evaluation/interaction/corpus.json");
  const acquisitionDocument = await loadJson("evaluation/interaction/annotations/acquisition.json");
  const ruleDocument = await loadJson("evaluation/interaction/annotations/rules.json");
  const cases = objects(corpus["cases"], "cases") as unknown as CorpusCase[];
  const acquisition = new Map(
    (objects(acquisitionDocument["annotations"], "acquisition annotations") as unknown as AcquisitionAnnotation[])
      .map((item) => [item.id, item]),
  );
  const rules = new Map(
    (objects(ruleDocument["annotations"], "rule annotations") as unknown as RuleAnnotation[])
      .map((item) => [item.id, item]),
  );
  const responseValidator = await validator("adapters/playwright/schemas/interaction-response.schema.json");
  const schemaRun = await run(sightlintBinary, ["schema", "--kind", "interaction"]);
  assert.equal(schemaRun.code, 0, schemaRun.stderr.toString("utf8"));
  const extensionValidator = new Ajv2020({ allErrors: true, strict: true, validateFormats: false })
    .compile(JSON.parse(schemaRun.stdout.toString("utf8")));
  let reviewedFacts = 0;
  let matchedFacts = 0;
  let expectedFailures = 0;
  let matchedFailures = 0;
  let targetedMutations = 0;
  let killedMutations = 0;
  let expectedAbstentions = 0;
  let matchedAbstentions = 0;
  let evaluatedCases = 0;
  let reviewedCleanCases = 0;
  let falsePositiveCases = 0;

  for (const item of cases) {
    const expectedAcquisition = acquisition.get(item.acquisitionAnnotationId);
    const expectedRules = rules.get(item.ruleAnnotationId);
    assert.ok(expectedAcquisition !== undefined && expectedRules !== undefined);
    const temporary = await mkdtemp(join(tmpdir(), `sightlint-interaction-${item.id}-`));
    try {
      const irPath = join(temporary, "artifact.json");
      const adapter = await run(process.execPath, [
        adapterCli,
        "--request", resolve(repositoryRoot, item.request.path),
        "--repository-root", repositoryRoot,
        "--artifact-ir-out", irPath,
      ]);
      assert.equal(adapter.code, 0, adapter.stderr.toString("utf8"));
      assert.equal(adapter.stderr.byteLength, 0);
      const response = JSON.parse(adapter.stdout.toString("utf8")) as Record<string, unknown>;
      assertValid(responseValidator, response, `${item.id} response`);
      const irBytes = await readFile(irPath);
      const ir = JSON.parse(irBytes.toString("utf8")) as Record<string, unknown>;
      const interaction = object(object(ir["extensions"], "extensions")["org.sightlint.interaction"], "interaction extension");
      assertValid(extensionValidator, interaction, `${item.id} interaction extension`);
      assert.equal(response["status"], expectedAcquisition.traceExecution);
      const actualFacts = normalizedFacts(observedFacts(ir));
      const expectedFacts = normalizedFacts(expectedAcquisition.orderedFacts);
      reviewedFacts += expectedFacts.length;
      assert.deepEqual(actualFacts, expectedFacts, item.id);
      matchedFacts += expectedFacts.length;
      const trace = objects(interaction["traces"], "traces")[0];
      assert.ok(trace !== undefined);
      const request = await loadJson(item.request.path);
      const requestEnvironment = object(request["environment"], "request environment");
      const requestViewport = object(requestEnvironment["viewport"], "request viewport");
      assert.deepEqual(object(trace["environment"], "trace environment"), {
        clock: "controlledSteps",
        network: "denyExternal",
        viewportSize: { width: requestViewport["width"], height: requestViewport["height"] },
        viewportUnit: requestViewport["unit"],
        locale: requestEnvironment["locale"],
        timezoneId: requestEnvironment["timezoneId"],
        colorScheme: requestEnvironment["colorScheme"],
        reducedMotion: requestEnvironment["reducedMotion"],
        externalProcessing: false,
      });
      const consistency = object(trace["consistency"], "consistency");
      const conflictReasons = consistency["status"] === "conflict" ? String(consistency["reason"]).split("; ") : [];
      assert.deepEqual(conflictReasons, expectedAcquisition.conflicts);

      const normalize = await run(sightlintBinary, ["normalize", irPath]);
      assert.equal(normalize.code, 0, normalize.stderr.toString("utf8"));
      assert.deepEqual(JSON.parse(normalize.stdout.toString("utf8")), ir);
      const normalizedPath = join(temporary, "normalized.json");
      await writeFile(normalizedPath, normalize.stdout);
      const normalizedAgain = await run(sightlintBinary, ["normalize", normalizedPath]);
      assert.equal(normalizedAgain.code, 0, normalizedAgain.stderr.toString("utf8"));
      assert.deepEqual(normalizedAgain.stdout, normalize.stdout);
      const check = await run(sightlintBinary, ["check", irPath, "--profile", "base", "--format", "json"]);
      assert.equal(check.code, expectedRules.expectedExit, check.stderr.toString("utf8"));
      const report = JSON.parse(check.stdout.toString("utf8")) as Record<string, unknown>;
      const interactionResults = objects(report["results"], "results")
        .filter((result) => String(result["ruleId"]).startsWith("interaction."));
      assert.equal(interactionResults.length, expectedRules.expectations.length);
      for (const expectation of expectedRules.expectations) {
        const actual = interactionResults.find((result) => result["ruleId"] === expectation.ruleId);
        assert.ok(actual !== undefined, `${item.id} missing ${expectation.ruleId}`);
        assert.equal(actual["ruleVersion"], expectation.ruleVersion);
        assert.equal(actual["outcome"], expectation.outcome);
        assert.equal(actual["enforcement"], "advisory");
        if (expectation.outcome === "failed") expectedFailures += 1;
        if (actual["outcome"] === "failed" && expectation.outcome === "failed") matchedFailures += 1;
        if (["cantTell", "inapplicable", "untested"].includes(expectation.outcome)) {
          expectedAbstentions += 1;
          if (actual["outcome"] === expectation.outcome) matchedAbstentions += 1;
        }
      }
      const actualFailed = interactionResults.filter((result) => result["outcome"] === "failed");
      const expectedFailed = expectedRules.expectations.filter((result) => result.outcome === "failed");
      if (expectedRules.forbidUnexpectedFailures) assert.equal(actualFailed.length, expectedFailed.length);
      if (item.relation.kind === "targetedMutation") {
        targetedMutations += 1;
        if (actualFailed.length > 0) killedMutations += 1;
      }
      if (item.relation.kind === "baseline" || item.relation.kind === "hardNegative") {
        reviewedCleanCases += 1;
        if (actualFailed.length > 0) falsePositiveCases += 1;
      }
      const serialized = irBytes.toString("utf8");
      for (const privateText of ["Northstar editor", "Settings saved", "could not be saved"]) {
        assert.equal(serialized.includes(privateText), false, `${item.id} leaked fixture text`);
      }
      assert.deepEqual((await readdir(temporary)).sort(), ["artifact.json", "normalized.json"]);
      evaluatedCases += 1;
    } finally {
      await rm(temporary, { recursive: true, force: true });
    }
  }
  assert.equal(matchedFacts, reviewedFacts);
  assert.equal(evaluatedCases, cases.length);
  assert.equal(matchedFailures, expectedFailures);
  assert.equal(reviewedCleanCases, 3);
  assert.equal(falsePositiveCases, 0);
  assert.equal(killedMutations, targetedMutations);
  assert.equal(matchedAbstentions, expectedAbstentions);
});

test("interaction outputs are byte-stable and malformed or colliding requests fail closed", async () => {
  const requestPath = resolve(repositoryRoot, "evaluation/interaction/requests/slow-success-clean.json");
  const temporary = await mkdtemp(join(tmpdir(), "sightlint-interaction-determinism-"));
  try {
    const runs: Array<{ response: Buffer; ir: Buffer; report: Buffer }> = [];
    for (let index = 0; index < 2; index += 1) {
      const irPath = join(temporary, `artifact-${index}.json`);
      const response = await run(process.execPath, [adapterCli, "--request", requestPath, "--repository-root", repositoryRoot, "--artifact-ir-out", irPath]);
      assert.equal(response.code, 0, response.stderr.toString("utf8"));
      const report = await run(sightlintBinary, ["check", irPath, "--profile", "base", "--format", "json"]);
      assert.equal(report.code, 0, report.stderr.toString("utf8"));
      runs.push({ response: response.stdout, ir: await readFile(irPath), report: report.stdout });
    }
    assert.deepEqual(runs[0], runs[1]);

    const valid = await loadJson("evaluation/interaction/requests/slow-success-clean.json");
    valid["unexpected"] = true;
    const malformedPath = join(temporary, "malformed.json");
    await writeFile(malformedPath, canonicalJson(valid as JsonValue));
    const malformedOutput = join(temporary, "malformed-output.json");
    const malformed = await run(process.execPath, [adapterCli, "--request", malformedPath, "--repository-root", repositoryRoot, "--artifact-ir-out", malformedOutput]);
    assert.equal(malformed.code, 2);
    assert.equal(malformed.stdout.byteLength, 0);
    assert.match(malformed.stderr.toString("utf8"), /invalid-interaction-request/u);

    const collisionPath = join(temporary, "collision.json");
    await writeFile(collisionPath, "owned by caller\n");
    const collision = await run(process.execPath, [adapterCli, "--request", requestPath, "--repository-root", repositoryRoot, "--artifact-ir-out", collisionPath]);
    assert.equal(collision.code, 2);
    assert.match(collision.stderr.toString("utf8"), /output-exists/u);
    assert.equal(await readFile(collisionPath, "utf8"), "owned by caller\n");
    assert.equal(basename(malformedOutput), "malformed-output.json");
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
});
