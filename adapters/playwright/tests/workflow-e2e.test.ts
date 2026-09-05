import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type AnySchema } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const workflowCli = resolve(repositoryRoot, "adapters/playwright/dist/src/check-cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface ExpectedResult {
  ruleId: string;
  ruleVersion: string;
  outcome: string;
  enforcement: string;
  targetKind: string;
  targetId: string;
}

interface WorkflowCase {
  request: string;
  initialResult: ExpectedResult;
  sourceTarget: {
    nodeId: string;
    locatorType: string;
    locatorValue: string;
    selector: string;
    sourceFiles: string[];
    attribution: string;
  };
  reviewedEdit: {
    file: string;
    before: string;
    after: string;
    occurrences: number;
    scope: string;
  };
  postconditions: {
    removedFinding: ExpectedResult;
    maximumNewFailures: number;
    doNotInferUnrelatedSuccess: boolean;
  };
  abstentionControl: { request: string; expectedResult: ExpectedResult };
  hardNegativeControl: { request: string; expectedResult: ExpectedResult };
}

interface WorkflowOracle {
  provenance: {
    implementationOutputUsedAsOracle: boolean;
    externalProcessing: boolean;
  };
  split: { holdoutStatus: string };
  cases: WorkflowCase[];
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

async function json(path: string): Promise<unknown> {
  return JSON.parse(await readFile(resolve(repositoryRoot, path), "utf8")) as unknown;
}

function object(value: unknown, context: string): Record<string, unknown> {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${context} must be an object`);
  return value as Record<string, unknown>;
}

function array(value: unknown, context: string): Array<Record<string, unknown>> {
  assert.ok(Array.isArray(value), `${context} must be an array`);
  return value as Array<Record<string, unknown>>;
}

function resultKey(result: Record<string, unknown>): string {
  const target = object(result["target"], "result target");
  return [result["ruleId"], result["ruleVersion"], result["outcome"], result["enforcement"], target["kind"], target["id"]].join("\0");
}

function expectedKey(expected: ExpectedResult): string {
  return [expected.ruleId, expected.ruleVersion, expected.outcome, expected.enforcement, expected.targetKind, expected.targetId].join("\0");
}

function results(report: Record<string, unknown>): Array<Record<string, unknown>> {
  return array(object(report["checkReport"], "workflow checkReport")["results"], "check results");
}

function assertExpectedResult(report: Record<string, unknown>, expected: ExpectedResult): void {
  assert.ok(
    results(report).some((result) => resultKey(result) === expectedKey(expected)),
    `missing reviewed result ${expected.ruleId}/${expected.targetId}/${expected.outcome}`,
  );
}

async function runWorkflow(
  temporaryRepository: string,
  request: string,
  format: "json" | "human",
  binary = sightlintBinary,
): Promise<ProcessResult> {
  return run(process.execPath, [
    workflowCli,
    "--request", join(temporaryRepository, request),
    "--repository-root", temporaryRepository,
    "--sightlint-binary", binary,
    "--format", format,
  ]);
}

async function copyWebInputs(): Promise<string> {
  const temporaryRepository = await mkdtemp(join(tmpdir(), "sightlint-agent-workflow-e2e-"));
  const evaluationRoot = join(temporaryRepository, "evaluation/web");
  await mkdir(evaluationRoot, { recursive: true });
  await cp(
    resolve(repositoryRoot, "evaluation/web/fixture-app"),
    join(evaluationRoot, "fixture-app"),
    { recursive: true },
  );
  await cp(
    resolve(repositoryRoot, "evaluation/web/requests"),
    join(evaluationRoot, "requests"),
    { recursive: true },
  );
  return temporaryRepository;
}

async function workflowValidator(): Promise<(value: unknown) => boolean> {
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  ajv.addSchema(await json("adapters/playwright/schemas/capture-response.schema.json") as AnySchema);
  return ajv.compile(await json("adapters/playwright/schemas/web-workflow-report.schema.json") as AnySchema);
}

test("one-command workflow exposes a reviewed source target and verifies the focused fix", async () => {
  const oracle = await json("evaluation/web/annotations/agent-workflow.json") as WorkflowOracle;
  assert.equal(oracle.provenance.implementationOutputUsedAsOracle, false);
  assert.equal(oracle.provenance.externalProcessing, false);
  assert.equal(oracle.split.holdoutStatus, "notHoldout");
  assert.equal(oracle.cases.length, 1);
  const workflowCase = oracle.cases[0];
  assert.ok(workflowCase);
  const validate = await workflowValidator();
  const temporaryRepository = await copyWebInputs();
  try {
    const initial = await runWorkflow(temporaryRepository, workflowCase.request, "json");
    const repeatedInitial = await runWorkflow(temporaryRepository, workflowCase.request, "json");
    assert.equal(initial.code, 0, initial.stderr.toString("utf8"));
    assert.equal(initial.stderr.byteLength, 0);
    assert.deepEqual(repeatedInitial, initial);
    const initialReport = JSON.parse(initial.stdout.toString("utf8")) as Record<string, unknown>;
    assert.equal(validate(initialReport), true);
    assert.equal(validate({ ...initialReport, futureField: true }), false, "workflow schema must reject unknown fields");
    assertExpectedResult(initialReport, workflowCase.initialResult);

    const sourceTarget = array(initialReport["sourceTargets"], "source targets")
      .find((candidate) => candidate["nodeId"] === workflowCase.sourceTarget.nodeId);
    assert.ok(sourceTarget, "reviewed node must have a source target");
    const locator = object(sourceTarget["locator"], "source target locator");
    assert.equal(locator["type"], workflowCase.sourceTarget.locatorType);
    assert.equal(locator["value"], workflowCase.sourceTarget.locatorValue);
    assert.equal(locator["selector"], workflowCase.sourceTarget.selector);
    assert.deepEqual(sourceTarget["sourceFiles"], workflowCase.sourceTarget.sourceFiles);
    assert.equal(workflowCase.sourceTarget.attribution, "navigationHintNotExactSourceLine");

    const editPath = join(temporaryRepository, workflowCase.reviewedEdit.file);
    const beforeEdit = await readFile(editPath, "utf8");
    assert.equal(beforeEdit.split(workflowCase.reviewedEdit.before).length - 1, workflowCase.reviewedEdit.occurrences);
    assert.equal(workflowCase.reviewedEdit.scope, "temporaryFixtureCopyOnly");
    await writeFile(
      editPath,
      beforeEdit.replace(workflowCase.reviewedEdit.before, workflowCase.reviewedEdit.after),
      "utf8",
    );

    const fixed = await runWorkflow(temporaryRepository, workflowCase.request, "json");
    const repeatedFixed = await runWorkflow(temporaryRepository, workflowCase.request, "json");
    assert.equal(fixed.code, 0, fixed.stderr.toString("utf8"));
    assert.equal(fixed.stderr.byteLength, 0);
    assert.deepEqual(repeatedFixed, fixed);
    assert.notDeepEqual(fixed.stdout, initial.stdout, "the source digest and named finding must change after the edit");
    const fixedReport = JSON.parse(fixed.stdout.toString("utf8")) as Record<string, unknown>;
    assert.equal(validate(fixedReport), true);
    assert.equal(
      results(fixedReport).some((result) => resultKey(result) === expectedKey(workflowCase.postconditions.removedFinding)),
      false,
      "the reviewed finding must be absent after the fix",
    );
    const initialFailureKeys = new Set(results(initialReport).filter((result) => result["outcome"] === "failed").map(resultKey));
    const fixedFailureKeys = results(fixedReport).filter((result) => result["outcome"] === "failed").map(resultKey);
    const newFailures = fixedFailureKeys.filter((key) => !initialFailureKeys.has(key));
    assert.equal(newFailures.length, workflowCase.postconditions.maximumNewFailures);
    assert.equal(workflowCase.postconditions.doNotInferUnrelatedSuccess, true);

    const human = await runWorkflow(temporaryRepository, workflowCase.request, "human");
    const repeatedHuman = await runWorkflow(temporaryRepository, workflowCase.request, "human");
    assert.equal(human.code, 0, human.stderr.toString("utf8"));
    assert.deepEqual(repeatedHuman, human);
    assert.match(human.stdout.toString("utf8"), /SightLint Web check 0\.1\.0/u);
    assert.match(human.stdout.toString("utf8"), /source selector: \[data-testid="help-action"\]/u);
    assert.match(human.stdout.toString("utf8"), /PASS web\.accessibility\.interactive-name \(advisory\)/u);
    process.stdout.write(
      "agent workflow v0.1: cases=1/1, initial_findings=1/1, source_targets=1/1, " +
      "fixes_verified=1/1, new_failures=0, json_determinism=2/2, human_determinism=2/2\n",
    );
  } finally {
    await rm(temporaryRepository, { recursive: true, force: true });
  }
});

test("one-command workflow preserves reviewed abstention and hard-negative controls", async () => {
  const oracle = await json("evaluation/web/annotations/agent-workflow.json") as WorkflowOracle;
  const workflowCase = oracle.cases[0];
  assert.ok(workflowCase);
  const validate = await workflowValidator();
  const temporaryRepository = await copyWebInputs();
  try {
    for (const control of [workflowCase.abstentionControl, workflowCase.hardNegativeControl]) {
      const execution = await runWorkflow(temporaryRepository, control.request, "json");
      assert.equal(execution.code, 0, execution.stderr.toString("utf8"));
      assert.equal(execution.stderr.byteLength, 0);
      const report = JSON.parse(execution.stdout.toString("utf8")) as Record<string, unknown>;
      assert.equal(validate(report), true);
      assertExpectedResult(report, control.expectedResult);
      assert.equal(results(report).filter((result) => result["outcome"] === "failed").length, 0);
    }
    process.stdout.write("agent workflow controls: reviewed_cantTell=2/2, false_positive_failures=0\n");
  } finally {
    await rm(temporaryRepository, { recursive: true, force: true });
  }
});

test("one-command workflow returns stable operational diagnostics", async () => {
  const temporaryRepository = await copyWebInputs();
  try {
    const request = "evaluation/web/requests/dashboard-browser-clean.json";
    const invalidFormat = await run(process.execPath, [
      workflowCli,
      "--request", join(temporaryRepository, request),
      "--repository-root", temporaryRepository,
      "--sightlint-binary", sightlintBinary,
      "--format", "yaml",
    ]);
    assert.equal(invalidFormat.code, 2);
    assert.equal(invalidFormat.stdout.byteLength, 0);
    assert.equal(invalidFormat.stderr.toString("utf8"), "sightlint-web-check: usage: format must be human or json\n");

    const missingBinary = await runWorkflow(temporaryRepository, request, "json", join(temporaryRepository, "missing-sightlint"));
    assert.equal(missingBinary.code, 2);
    assert.equal(missingBinary.stdout.byteLength, 0);
    assert.equal(missingBinary.stderr.toString("utf8"), "sightlint-web-check: kernel-spawn: failed to start the sightlint binary\n");

    const blocking = await runWorkflow(
      temporaryRepository,
      "evaluation/web/requests/dashboard-browser-out-of-viewport.json",
      "json",
    );
    assert.equal(blocking.code, 1, blocking.stderr.toString("utf8"));
    assert.equal(blocking.stderr.byteLength, 0);
    const blockingReport = JSON.parse(blocking.stdout.toString("utf8")) as Record<string, unknown>;
    assert.equal(
      results(blockingReport).some((result) => result["outcome"] === "failed" && result["enforcement"] === "blocking"),
      true,
    );
  } finally {
    await rm(temporaryRepository, { recursive: true, force: true });
  }
});
