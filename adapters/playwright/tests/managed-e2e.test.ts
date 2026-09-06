import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createConnection, createServer, type Server } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type AnySchema } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const adapterCli = resolve(repositoryRoot, "adapters/playwright/dist/src/cli.js");
const workflowCli = resolve(repositoryRoot, "adapters/playwright/dist/src/check-cli.js");
const sightlintBinary = process.env["SIGHTLINT_BINARY"] ?? resolve(repositoryRoot, "target/debug/sightlint");
const cleanRequest = "evaluation/web/managed-requests/dashboard-managed-clean.json";

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface ExpectedResult {
  ruleId: string;
  ruleVersion: string;
  outcome: string;
  maturity: string;
  enforcement: string;
  targetKind: string;
  targetId: string;
  targetAspect: string | null;
}

interface AcquisitionExpectation {
  aspect: string;
  operator: "equals" | "atLeast" | "includesAll" | "excludesAll";
  value: unknown;
}

interface AcquisitionAbstention {
  aspect: string;
  outcome: "cantTell" | "untested";
  rationale: string;
}

interface AcquisitionCase {
  caseId: string;
  request: string;
  split: string;
  classification: string;
  expectations: AcquisitionExpectation[];
  abstentions: AcquisitionAbstention[];
}

interface RuleCase {
  caseId: string;
  request: string;
  split: string;
  classification: string;
  expectedExitCode: number;
  expectedFailureCount: number;
  expectedBlockingFailureCount: number;
  expectedResults: ExpectedResult[];
}

function run(program: string, args: string[], cwd = repositoryRoot): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, { cwd, env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    child.stdout.on("data", (chunk: Buffer) => stdout.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderr.push(chunk));
    child.once("error", reject);
    child.once("close", (code, signal) => {
      if (signal !== null) reject(new Error(`${program} terminated by ${signal}`));
      else resolveRun({ code: code ?? -1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) });
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

async function freePort(): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const server = createServer();
    server.unref();
    server.once("error", reject);
    server.listen({ host: "127.0.0.1", port: 0 }, () => {
      const address = server.address();
      assert.ok(address !== null && typeof address === "object");
      const port = address.port;
      server.close((error) => error === undefined ? resolvePort(port) : reject(error));
    });
  });
}

function connected(port: number): Promise<boolean> {
  return new Promise((resolveConnected) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    const finish = (value: boolean): void => {
      socket.removeAllListeners();
      socket.destroy();
      resolveConnected(value);
    };
    socket.setTimeout(200, () => finish(false));
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}

async function waitForConnection(port: number, expected: boolean, timeoutMs = 10_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  do {
    if (await connected(port) === expected) return;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 25));
  } while (Date.now() < deadline);
  assert.fail(`port ${port} did not reach connected=${String(expected)}`);
}

async function materializeRequest(
  directory: string,
  sourcePath: string,
  port: number,
  mutate?: (request: Record<string, unknown>) => void,
): Promise<string> {
  const request = structuredClone(await json(sourcePath)) as Record<string, unknown>;
  object(request["server"], "server")["port"] = port;
  mutate?.(request);
  const requestPath = join(directory, `${String(object(request["artifact"], "artifact")["id"])}-${port}.json`);
  await writeFile(requestPath, `${JSON.stringify(request)}\n`, { flag: "wx" });
  return requestPath;
}

function workflowArguments(requestPath: string, binary = sightlintBinary): string[] {
  return [
    workflowCli,
    "--request", requestPath,
    "--repository-root", repositoryRoot,
    "--sightlint-binary", binary,
    "--format", "json",
    "--allow-server-command",
  ];
}

function adapterArguments(requestPath: string, outputRoot: string): string[] {
  return [
    adapterCli,
    "--request", requestPath,
    "--repository-root", repositoryRoot,
    "--artifact-ir-out", join(outputRoot, "artifact-ir.json"),
    "--screenshot-out", join(outputRoot, "screenshot.png"),
    "--allow-server-command",
  ];
}

function resultMatches(result: Record<string, unknown>, expected: ExpectedResult): boolean {
  const target = object(result["target"], "result target");
  return result["ruleId"] === expected.ruleId &&
    result["ruleVersion"] === expected.ruleVersion &&
    result["outcome"] === expected.outcome &&
    result["maturity"] === expected.maturity &&
    result["enforcement"] === expected.enforcement &&
    target["kind"] === expected.targetKind &&
    target["id"] === expected.targetId &&
    (target["aspect"] ?? null) === expected.targetAspect;
}

function assertAcquisitionExpectation(
  actual: Map<string, unknown>,
  expectation: AcquisitionExpectation,
  caseId: string,
): void {
  assert.ok(actual.has(expectation.aspect), `${caseId}: unsupported acquisition aspect ${expectation.aspect}`);
  const observed = actual.get(expectation.aspect);
  switch (expectation.operator) {
    case "equals":
      assert.deepEqual(observed, expectation.value, `${caseId}: ${expectation.aspect}`);
      return;
    case "atLeast":
      if (typeof observed !== "number") assert.fail(`${caseId}: ${expectation.aspect} must be numeric`);
      if (typeof expectation.value !== "number") assert.fail(`${caseId}: ${expectation.aspect} minimum must be numeric`);
      assert.ok(observed >= expectation.value, `${caseId}: ${expectation.aspect} must be at least ${String(expectation.value)}`);
      return;
    case "includesAll": {
      assert.ok(Array.isArray(observed), `${caseId}: ${expectation.aspect} must be an array`);
      assert.ok(Array.isArray(expectation.value), `${caseId}: ${expectation.aspect} expected value must be an array`);
      const observedValues = new Set(observed as unknown[]);
      for (const value of expectation.value) {
        assert.ok(observedValues.has(value), `${caseId}: ${expectation.aspect} is missing ${String(value)}`);
      }
      return;
    }
    case "excludesAll":
      if (typeof observed !== "string") assert.fail(`${caseId}: ${expectation.aspect} must be UTF-8 text`);
      assert.ok(Array.isArray(expectation.value), `${caseId}: ${expectation.aspect} exclusions must be an array`);
      for (const value of expectation.value) {
        assert.equal(typeof value, "string");
        assert.equal(observed.includes(value), false, `${caseId}: ${expectation.aspect} disclosed ${value}`);
      }
  }
}

function assertAcquisitionAbstention(
  abstention: AcquisitionAbstention,
  captureLimitations: string[],
  sourceAttributionUnavailable: boolean,
  caseId: string,
): void {
  assert.ok(abstention.rationale.length > 0);
  assert.ok(abstention.outcome === "cantTell" || abstention.outcome === "untested");
  if (abstention.aspect === "screenshot.pixelContentIdentity") {
    assert.ok(captureLimitations.some((value) => value.includes("pixel-content matching is cantTell")), `${caseId}: pixel-content abstention missing`);
  } else if (abstention.aspect === "web.completeHitRegions") {
    assert.ok(captureLimitations.some((value) => value.includes("complete hit regions are cantTell")), `${caseId}: hit-region abstention missing`);
  } else if (abstention.aspect === "source.fileCausality") {
    assert.equal(sourceAttributionUnavailable, true, `${caseId}: source causality must remain unavailable`);
  } else {
    assert.fail(`${caseId}: unsupported acquisition abstention ${abstention.aspect}`);
  }
}

async function listen(port: number): Promise<Server> {
  return new Promise((resolveServer, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen({ host: "127.0.0.1", port }, () => resolveServer(server));
  });
}

async function close(server: Server): Promise<void> {
  await new Promise<void>((resolveClose, reject) => server.close((error) => error === undefined ? resolveClose() : reject(error)));
}

test("managed evaluation schemas keep acquisition and rule truth separate", async () => {
  const acquisitionSchema = await json("evaluation/web/managed-loopback-acquisition.schema.json") as AnySchema;
  const ruleSchema = await json("evaluation/web/managed-loopback-rule.schema.json") as AnySchema;
  const acquisitionOracle = await json("evaluation/web/annotations/managed-loopback-acquisition.json") as Record<string, unknown>;
  const ruleOracle = await json("evaluation/web/annotations/managed-loopback-rules.json") as Record<string, unknown>;
  const acquisitionAjv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const ruleAjv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  const validateAcquisition = acquisitionAjv.compile(acquisitionSchema);
  const validateRules = ruleAjv.compile(ruleSchema);
  assert.equal(validateAcquisition(acquisitionOracle), true, acquisitionAjv.errorsText(validateAcquisition.errors));
  assert.equal(validateRules(ruleOracle), true, ruleAjv.errorsText(validateRules.errors));
  assert.equal(validateAcquisition(ruleOracle), false, "rule truth must not validate as acquisition truth");
  assert.equal(validateRules(acquisitionOracle), false, "acquisition truth must not validate as rule truth");

  const mixedAcquisition = structuredClone(acquisitionOracle);
  array(mixedAcquisition["cases"], "acquisition cases")[0]!["expectedFailureCount"] = 0;
  assert.equal(validateAcquisition(mixedAcquisition), false, "rule fields must be rejected from acquisition cases");
  const mixedRules = structuredClone(ruleOracle);
  array(mixedRules["cases"], "rule cases")[0]!["expectations"] = [];
  assert.equal(validateRules(mixedRules), false, "acquisition fields must be rejected from rule cases");

  for (const oracle of [acquisitionOracle, ruleOracle]) {
    const sourceFiles = object(oracle["provenance"], "oracle provenance")["sourceFiles"];
    assert.ok(Array.isArray(sourceFiles));
    for (const sourceFile of sourceFiles) {
      assert.equal(typeof sourceFile, "string");
      assert.ok((await readFile(resolve(repositoryRoot, sourceFile))).byteLength > 0, `${sourceFile} must exist and be nonempty`);
    }
  }
});

test("managed loopback workflow exercises redirect, same-origin API, rules, attribution, and cleanup", { timeout: 300_000 }, async () => {
  const acquisitionOracle = await json("evaluation/web/annotations/managed-loopback-acquisition.json") as { cases: AcquisitionCase[] };
  const ruleOracle = await json("evaluation/web/annotations/managed-loopback-rules.json") as { cases: RuleCase[] };
  const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
  ajv.addSchema(await json("adapters/playwright/schemas/capture-response-0.2.schema.json") as AnySchema);
  const validateWorkflow = ajv.compile(await json("adapters/playwright/schemas/web-workflow-report-0.2.schema.json") as AnySchema);
  const validateExtension = new Ajv2020({ allErrors: true, strict: true, validateFormats: false })
    .compile(await json("adapters/playwright/schemas/web-extension.schema.json") as AnySchema);
  const acquisitionByCase = new Map(acquisitionOracle.cases.map((oracleCase) => [oracleCase.caseId, oracleCase]));
  const ruleCaseIds = new Set(ruleOracle.cases.map((oracleCase) => oracleCase.caseId));
  assert.equal(acquisitionByCase.size, acquisitionOracle.cases.length, "acquisition case identifiers must be unique");
  assert.equal(ruleCaseIds.size, ruleOracle.cases.length, "rule case identifiers must be unique");
  assert.equal(ruleOracle.cases.length, acquisitionOracle.cases.length, "acquisition and rule case counts must match");
  assert.deepEqual([...ruleCaseIds].sort(), [...acquisitionByCase.keys()].sort(), "acquisition and rule case identifiers must match");
  let executedCases = 0;
  let matchedAcquisitionExpectations = 0;
  const acquisitionExpectationCount = acquisitionOracle.cases.reduce((total, oracleCase) => total + oracleCase.expectations.length, 0);
  let matchedAcquisitionAbstentions = 0;
  const acquisitionAbstentionCount = acquisitionOracle.cases.reduce((total, oracleCase) => total + oracleCase.abstentions.length, 0);
  let emittedFailures = 0;
  let matchedReviewedFailures = 0;
  let unexpectedFailures = 0;
  let matchedRuleAbstentions = 0;
  const reviewedRuleAbstentions = ruleOracle.cases.reduce(
    (total, oracleCase) => total + oracleCase.expectedResults.filter((result) => ["cantTell", "inapplicable", "untested"].includes(result.outcome)).length,
    0,
  );
  let reviewedMutations = 0;
  let killedMutations = 0;
  let reviewedHardNegatives = 0;
  let hardNegativeFailures = 0;
  const directory = await mkdtemp(join(tmpdir(), "sightlint-managed-product-"));
  try {
    for (const ruleCase of ruleOracle.cases) {
      const acquisitionCase = acquisitionByCase.get(ruleCase.caseId);
      assert.ok(acquisitionCase !== undefined, `${ruleCase.caseId}: missing acquisition oracle`);
      assert.equal(acquisitionCase.request, ruleCase.request, `${ruleCase.caseId}: request authorities disagree`);
      assert.equal(acquisitionCase.split, ruleCase.split, `${ruleCase.caseId}: split authorities disagree`);
      assert.equal(acquisitionCase.classification, ruleCase.classification, `${ruleCase.caseId}: classification authorities disagree`);
      assert.equal(new Set(acquisitionCase.expectations.map((expectation) => expectation.aspect)).size, acquisitionCase.expectations.length, `${ruleCase.caseId}: acquisition aspects must be unique`);
      assert.equal(new Set(acquisitionCase.abstentions.map((abstention) => abstention.aspect)).size, acquisitionCase.abstentions.length, `${ruleCase.caseId}: acquisition abstention aspects must be unique`);
      const port = await freePort();
      const requestPath = await materializeRequest(directory, ruleCase.request, port);
      const first = await run(process.execPath, workflowArguments(requestPath));
      const second = await run(process.execPath, workflowArguments(requestPath));
      assert.equal(first.code, ruleCase.expectedExitCode, first.stderr.toString("utf8"));
      assert.equal(first.stderr.byteLength, 0);
      assert.deepEqual(second, first, `${ruleCase.caseId} deterministic workflow bytes`);
      const report = JSON.parse(first.stdout.toString("utf8")) as Record<string, unknown>;
      assert.equal(validateWorkflow(report), true, ajv.errorsText(validateWorkflow.errors));
      const capture = object(report["capture"], "capture");
      const captureDetails = object(capture["capture"], "capture details");
      const loopback = object(captureDetails["loopbackResponses"], "loopback responses");
      const sourceTargets = array(report["sourceTargets"], "source targets");
      const sourceAttributionUnavailable = sourceTargets.length > 0 && sourceTargets.every(
        (sourceTarget) => sourceTarget["sourceAttribution"] === "unavailable" &&
          Array.isArray(sourceTarget["sourceFiles"]) && sourceTarget["sourceFiles"].length === 0,
      );
      const checkReport = object(report["checkReport"], "check report");
      const results = array(checkReport["results"], "check results");
      const failures = results.filter((result) => result["outcome"] === "failed");
      const blockingFailures = failures.filter((result) => result["enforcement"] === "blocking");
      assert.equal(failures.length, ruleCase.expectedFailureCount, `${ruleCase.caseId}: failure count`);
      assert.equal(blockingFailures.length, ruleCase.expectedBlockingFailureCount, `${ruleCase.caseId}: blocking failure count`);
      for (const expectedResult of ruleCase.expectedResults) {
        const matched = results.some((result) => resultMatches(result, expectedResult));
        assert.ok(matched, `${ruleCase.caseId}: missing reviewed ${expectedResult.ruleId} ${expectedResult.outcome}`);
        if (["cantTell", "inapplicable", "untested"].includes(expectedResult.outcome)) {
          matchedRuleAbstentions += 1;
        }
      }
      const reviewedFailedResults = ruleCase.expectedResults.filter((result) => result.outcome === "failed");
      for (const failure of failures) {
        if (reviewedFailedResults.some((expected) => resultMatches(failure, expected))) matchedReviewedFailures += 1;
        else unexpectedFailures += 1;
      }
      emittedFailures += failures.length;
      if (ruleCase.classification === "targetedMutation") {
        reviewedMutations += 1;
        if (reviewedFailedResults.length > 0 && reviewedFailedResults.every((expected) => results.some((result) => resultMatches(result, expected)))) {
          killedMutations += 1;
        }
      }
      if (ruleCase.classification === "hardNegative") {
        reviewedHardNegatives += 1;
        hardNegativeFailures += failures.length;
      }
      await waitForConnection(port, false);

      const artifactOutput = join(directory, `direct-${ruleCase.caseId}`);
      const direct = await run(process.execPath, adapterArguments(requestPath, artifactOutput));
      assert.equal(direct.code, 0, direct.stderr.toString("utf8"));
      const directResponse = JSON.parse(direct.stdout.toString("utf8")) as Record<string, unknown>;
      const artifactIr = JSON.parse(await readFile(join(artifactOutput, "artifact-ir.json"), "utf8")) as Record<string, unknown>;
      const webExtension = object(object(artifactIr["extensions"], "extensions")["org.sightlint.web"], "Web extension");
      assert.equal(validateExtension(webExtension), true, JSON.stringify(validateExtension.errors));
      const extensionCapture = object(webExtension["capture"], "Web capture");
      assert.equal(extensionCapture["sourceDigest"], directResponse["sourceDigest"]);
      assert.equal(
        object(extensionCapture["loopbackResponses"], "Web loopback responses")["digest"],
        directResponse["sourceDigest"],
      );
      await waitForConnection(port, false);
      const targetDigest = loopback["targetDigest"];
      const screenshot = object(capture["screenshot"], "capture screenshot");
      const nodeIds = array(artifactIr["nodes"], "Artifact IR nodes").map((node) => node["id"]);
      const actualAcquisition = new Map<string, unknown>([
        ["capture.protocolVersion", capture["protocolVersion"]],
        ["capture.adapterVersion", object(capture["adapter"], "adapter")["version"]],
        ["workflow.schemaVersion", report["schemaVersion"]],
        ["kernel.webExtensionVersion", object(checkReport["extensionVersions"], "extension versions")["org.sightlint.web"]],
        ["capture.loopback.routePath", loopback["routePath"]],
        ["capture.loopback.requestCount", loopback["requestCount"]],
        ["capture.loopback.targetDigestFormat", typeof targetDigest === "string" && /^sha256:[0-9a-f]{64}$/u.test(targetDigest) ? "sha256" : "invalid"],
        ["capture.sourceDigestMatchesLoopback", capture["sourceDigest"] === loopback["digest"]],
        ["screenshot.pixelSize", screenshot["pixelSize"]],
        ["workflow.sourceAttribution", sourceAttributionUnavailable ? "unavailable" : "mixedOrMissing"],
        ["workflow.sourceFiles", sourceAttributionUnavailable ? [] : sourceTargets.flatMap((target) => target["sourceFiles"] as unknown[])],
        ["workflow.outputUtf8", first.stdout.toString("utf8")],
        ["lifecycle.workflowPortReleased", true],
        ["artifact.id", object(artifactIr["artifact"], "Artifact IR artifact")["id"]],
        ["web.document.state", object(webExtension["document"], "Web document")["state"]],
        ["web.nodeIds", nodeIds],
        ["web.capture.sourceDigestMatchesResponse", extensionCapture["sourceDigest"] === directResponse["sourceDigest"]],
        ["lifecycle.directPortReleased", true],
      ]);
      for (const expectation of acquisitionCase.expectations) {
        assertAcquisitionExpectation(actualAcquisition, expectation, acquisitionCase.caseId);
        matchedAcquisitionExpectations += 1;
      }
      const captureLimitations = (capture["limitations"] as unknown[]).map(String);
      for (const abstention of acquisitionCase.abstentions) {
        assertAcquisitionAbstention(abstention, captureLimitations, sourceAttributionUnavailable, acquisitionCase.caseId);
        matchedAcquisitionAbstentions += 1;
      }
      const firstKernel = await run(sightlintBinary, ["check", join(artifactOutput, "artifact-ir.json"), "--format", "json"]);
      const secondKernel = await run(sightlintBinary, ["check", join(artifactOutput, "artifact-ir.json"), "--format", "json"]);
      assert.deepEqual(secondKernel, firstKernel, "one fixed Artifact IR must produce deterministic kernel bytes");
      if (ruleCase.caseId === "managed-clean") {
        const malformed = structuredClone(artifactIr);
        const malformedLoopback = object(
          object(object(malformed["extensions"], "extensions")["org.sightlint.web"], "Web extension")["capture"],
          "Web capture",
        )["loopbackResponses"];
        object(malformedLoopback, "loopback responses")["targetDigest"] = "not-a-digest";
        const malformedPath = join(directory, "malformed-web-extension-0.4.json");
        await writeFile(malformedPath, `${JSON.stringify(malformed)}\n`, { flag: "wx" });
        const rejected = await run(sightlintBinary, ["check", malformedPath, "--format", "json"]);
        assert.equal(rejected.code, 2);
        assert.equal(rejected.stdout.byteLength, 0);
        assert.match(rejected.stderr.toString("utf8"), /Web extension 0\.4 requires bounded same-origin loopback response evidence/u);
      }
      executedCases += 1;
    }
    assert.equal(executedCases, ruleOracle.cases.length);
    assert.equal(matchedAcquisitionExpectations, acquisitionExpectationCount);
    assert.equal(matchedAcquisitionAbstentions, acquisitionAbstentionCount);
    assert.equal(matchedRuleAbstentions, reviewedRuleAbstentions);
    assert.equal(unexpectedFailures, 0);
    assert.equal(killedMutations, reviewedMutations);
    assert.equal(hardNegativeFailures, 0);
    process.stdout.write(
      `managed loopback acquisition: cases=${executedCases}/${acquisitionOracle.cases.length}, exact=${matchedAcquisitionExpectations}/${acquisitionExpectationCount}, abstention=${matchedAcquisitionAbstentions}/${acquisitionAbstentionCount}\n`,
    );
    process.stdout.write(
      `managed loopback rules: cases=${executedCases}/${ruleOracle.cases.length}, precision=${matchedReviewedFailures}/${emittedFailures}, abstention=${matchedRuleAbstentions}/${reviewedRuleAbstentions}, false_positives=${unexpectedFailures}/${emittedFailures}, mutation_kills=${killedMutations}/${reviewedMutations}, hard_negative_failures=${hardNegativeFailures}/${reviewedHardNegatives}, cleanup=6/6\n`,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("managed transport attempts are blocked, counted, and redacted", { timeout: 120_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-managed-transport-"));
  const port = await freePort();
  try {
    const requestPath = await materializeRequest(directory, cleanRequest, port, (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=blocked-transports&secret=not-serialized";
    });
    const execution = await run(process.execPath, adapterArguments(requestPath, join(directory, "output")));
    assert.equal(execution.code, 0, execution.stderr.toString("utf8"));
    const response = JSON.parse(execution.stdout.toString("utf8")) as Record<string, unknown>;
    const loopback = object(object(response["capture"], "capture")["loopbackResponses"], "loopback responses");
    assert.equal(loopback["blockedWebSocketCount"], 1);
    assert.equal(loopback["blockedServiceWorkerCount"], 1);
    assert.equal(execution.stdout.includes(Buffer.from("secret=not-serialized")), false);
    await waitForConnection(port, false);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

async function expectManagedFailure(
  directory: string,
  code: string,
  mutate: (request: Record<string, unknown>) => void,
  occupyPort = false,
): Promise<void> {
  const port = await freePort();
  const requestPath = await materializeRequest(directory, cleanRequest, port, mutate);
  const blocker = occupyPort ? await listen(port) : null;
  try {
    const execution = await run(process.execPath, adapterArguments(requestPath, join(directory, `${code}-output`)));
    assert.equal(execution.code, 2);
    assert.equal(execution.stdout.byteLength, 0);
    assert.match(execution.stderr.toString("utf8"), new RegExp(`^sightlint-web: ${code}:`, "u"));
  } finally {
    if (blocker !== null) await close(blocker);
  }
  await waitForConnection(port, false);
}

test("managed lifecycle reports authorization, startup, network, and resource failures without occupied ports", { timeout: 240_000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-managed-failures-"));
  try {
    const unauthorizedPort = await freePort();
    const unauthorizedRequest = await materializeRequest(directory, cleanRequest, unauthorizedPort);
    const unauthorizedArgs = adapterArguments(unauthorizedRequest, join(directory, "unauthorized-output"))
      .filter((value) => value !== "--allow-server-command");
    const unauthorized = await run(process.execPath, unauthorizedArgs);
    assert.equal(unauthorized.code, 2);
    assert.equal(unauthorized.stdout.byteLength, 0);
    assert.equal(unauthorized.stderr.toString("utf8"), "sightlint-web: server-command-not-allowed: managed loopback capture requires --allow-server-command\n");
    const unauthorizedWorkflow = await run(process.execPath, [
      workflowCli,
      "--request", unauthorizedRequest,
      "--repository-root", repositoryRoot,
      "--sightlint-binary", sightlintBinary,
      "--format", "json",
    ]);
    assert.equal(unauthorizedWorkflow.code, 2);
    assert.equal(unauthorizedWorkflow.stdout.byteLength, 0);
    assert.equal(unauthorizedWorkflow.stderr.toString("utf8"), "sightlint-web-check: server-command-not-allowed: managed loopback capture requires --allow-server-command\n");
    await waitForConnection(unauthorizedPort, false);

    await expectManagedFailure(directory, "server-port-conflict", () => {}, true);
    await expectManagedFailure(directory, "server-spawn", (request) => {
      object(request["server"], "server")["argv"] = ["./definitely-missing-sightlint-managed-server", "{port}"];
    });
    await expectManagedFailure(directory, "server-early-exit", (request) => {
      object(request["server"], "server")["argv"] = ["node", "adapters/playwright/tests/fixtures/managed-server.mjs", "--mode", "early-exit", "--port", "{port}"];
    });
    await expectManagedFailure(directory, "server-startup-timeout", (request) => {
      const server = object(request["server"], "server");
      server["argv"] = ["node", "adapters/playwright/tests/fixtures/managed-server.mjs", "--mode", "timeout", "--port", "{port}"];
      server["startupTimeoutMs"] = 100;
    });
    await expectManagedFailure(directory, "server-output-too-large", (request) => {
      object(request["server"], "server")["argv"] = ["node", "adapters/playwright/tests/fixtures/managed-server.mjs", "--mode", "log-overflow", "--port", "{port}"];
    });
    await expectManagedFailure(directory, "external-request", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=external";
    });
    await expectManagedFailure(directory, "http-status", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/missing";
    });
    await expectManagedFailure(directory, "request-body-too-large", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=request-large";
    });
    await expectManagedFailure(directory, "response-too-large", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=response-large";
    });
    await expectManagedFailure(directory, "response-bytes-limit", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=response-aggregate";
    });
    await expectManagedFailure(directory, "response-count-limit", (request) => {
      object(request["target"], "target")["pathAndQuery"] = "/redirect?case=peer-spacing-clean&networkCase=response-count";
    });

    const kernelPort = await freePort();
    const kernelRequest = await materializeRequest(directory, cleanRequest, kernelPort);
    const missingKernel = await run(process.execPath, workflowArguments(kernelRequest, join(directory, "missing-sightlint")));
    assert.equal(missingKernel.code, 2);
    assert.match(missingKernel.stderr.toString("utf8"), /^sightlint-web-check: kernel-spawn:/u);
    await waitForConnection(kernelPort, false);
    process.stdout.write("managed loopback failures: authorization=1, lifecycle=5, navigation=1, network=1, resource=4, kernel_cleanup=1\n");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("managed process tree is removed after SIGINT and SIGTERM", { timeout: 120_000 }, async (context) => {
  if (process.platform === "win32") {
    context.skip("Windows process-tree cleanup is exercised by every successful taskkill-based capture");
    return;
  }
  const directory = await mkdtemp(join(tmpdir(), "sightlint-managed-signals-"));
  try {
    for (const signal of ["SIGINT", "SIGTERM"] as const) {
      const port = await freePort();
      const childPort = await freePort();
      const requestPath = await materializeRequest(directory, cleanRequest, port, (request) => {
        const argv = object(request["server"], "server")["argv"] as unknown[];
        argv.push("--child-port", String(childPort));
      });
      const child = spawn(process.execPath, adapterArguments(requestPath, join(directory, `signal-${signal}`)), {
        cwd: repositoryRoot,
        env: process.env,
        stdio: ["ignore", "pipe", "pipe"],
      });
      await waitForConnection(port, true);
      await waitForConnection(childPort, true);
      child.kill(signal);
      const code = await new Promise<number | null>((resolveExit, reject) => {
        child.once("error", reject);
        child.once("close", resolveExit);
      });
      assert.equal(code, signal === "SIGINT" ? 130 : 143);
      await waitForConnection(port, false);
      await waitForConnection(childPort, false);
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
