import assert from "node:assert/strict";
import { type ChildProcess, spawn } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, stat, symlink, writeFile } from "node:fs/promises";
import { request } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";
import { chromium, type Browser, type Page } from "playwright";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const runWorkbench = resolve(repositoryRoot, "tools/run_web_review.py");
const prepare = resolve(repositoryRoot, "tools/prepare_web_review.py");
const compare = resolve(repositoryRoot, "tools/compare_web_review.py");
const packetPath = resolve(repositoryRoot, "evaluation/web/review-packet.json");
const questionnairePath = resolve(repositoryRoot, "evaluation/web/harbor-review-questionnaire.json");
const stateSchemaPath = resolve(repositoryRoot, "evaluation/web/harbor-review-workbench-state.schema.json");
const submissionSchemaPath = resolve(repositoryRoot, "evaluation/web/reviewer-submission.schema.json");
const questionnaireSchemaPath = resolve(repositoryRoot, "evaluation/web/harbor-review-questionnaire.schema.json");
const python = process.env["PYTHON"] ?? (process.platform === "win32" ? "python" : "python3");
const platformNewline = process.platform === "win32" ? "\r\n" : "\n";

type JsonObject = Record<string, unknown>;

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface RunningWorkbench {
  child: ChildProcess;
  exit: Promise<ProcessResult>;
  origin: string;
  token: string;
  url: string;
}

interface HttpResult {
  body: Buffer;
  headers: Record<string, string | string[] | undefined>;
  status: number;
}

function object(value: unknown, label: string): JsonObject {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${label} must be an object`);
  return value as JsonObject;
}

function array(value: unknown, label: string): JsonObject[] {
  assert.ok(Array.isArray(value), `${label} must be an array`);
  return value as JsonObject[];
}

async function loadJson(path: string): Promise<JsonObject> {
  return JSON.parse(await readFile(path, "utf8")) as JsonObject;
}

function validator(schema: JsonObject): ValidateFunction {
  return new Ajv2020({ allErrors: true, strict: true, strictRequired: false, validateFormats: false }).compile(schema);
}

function assertValid(validate: ValidateFunction, value: unknown, label: string): void {
  assert.equal(validate(value), true, `${label}: ${JSON.stringify(validate.errors)}`);
}

function run(program: string, args: string[], cwd = repositoryRoot): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(program, args, { cwd, env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    child.stdout?.on("data", (chunk: Buffer) => stdout.push(chunk));
    child.stderr?.on("data", (chunk: Buffer) => stderr.push(chunk));
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

function start(
  root: string,
  draftPath: string,
  finalPath: string,
  options: { fictional?: boolean; resume?: boolean } = {},
): Promise<RunningWorkbench> {
  return new Promise((resolveStart, reject) => {
    const args = [
      resolve(root, "tools/run_web_review.py"),
      "--scope",
      "harbor",
      "--draft",
      draftPath,
      "--final",
      finalPath,
      "--port",
      "0",
      "--no-open",
    ];
    if (options.resume === true) args.push("--resume");
    if (options.fictional === true) args.push("--fictional-conformance");
    const child = spawn(python, args, { cwd: root, env: process.env, stdio: ["ignore", "pipe", "pipe"] });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let settled = false;
    let pending = "";
    const exit = new Promise<ProcessResult>((resolveExit, rejectExit) => {
      child.on("error", (error) => {
        rejectExit(error);
        if (!settled) reject(error);
      });
      child.on("close", (code, signal) => {
        if (signal !== null) {
          const error = new Error(`${python} terminated by ${signal}`);
          rejectExit(error);
          if (!settled) reject(error);
          return;
        }
        const result = { code: code ?? -1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) };
        resolveExit(result);
        if (!settled) reject(new Error(`workbench exited before startup: ${result.stderr.toString("utf8")}`));
      });
    });
    child.stderr?.on("data", (chunk: Buffer) => stderr.push(chunk));
    child.stdout?.on("data", (chunk: Buffer) => {
      stdout.push(chunk);
      pending += chunk.toString("utf8");
      const match = pending.match(/web review workbench: url=(\S+)/);
      if (match?.[1] === undefined || settled) return;
      const url = new URL(match[1]);
      const token = new URLSearchParams(url.hash.slice(1)).get("token");
      assert.ok(token !== null && token.length >= 32, "startup URL must carry an in-memory capability token");
      settled = true;
      resolveStart({ child, exit, origin: url.origin, token, url: url.href });
    });
  });
}

async function stop(server: RunningWorkbench): Promise<void> {
  if (server.child.exitCode === null && server.child.signalCode === null) server.child.kill();
  try {
    await server.exit;
  } catch (error) {
    if (server.child.signalCode === null) throw error;
  }
}

async function api(
  server: RunningWorkbench,
  path: string,
  method = "GET",
  body?: JsonObject,
): Promise<{ response: Response; value: JsonObject }> {
  const headers: Record<string, string> = { "X-SightLint-Review-Token": server.token };
  const init: RequestInit = { headers, method };
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
    headers["Origin"] = server.origin;
    init.body = JSON.stringify(body);
  }
  const response = await fetch(new URL(path, server.origin), init);
  const value = object(await response.json(), "workbench API response");
  return { response, value };
}

function http(
  server: RunningWorkbench,
  path: string,
  method: string,
  headers: Record<string, string>,
  body = Buffer.alloc(0),
): Promise<HttpResult> {
  const url = new URL(server.origin);
  return new Promise((resolveRequest, reject) => {
    const call = request(
      {
        host: "127.0.0.1",
        port: Number(url.port),
        path,
        method,
        headers,
      },
      (response) => {
        const chunks: Buffer[] = [];
        response.on("data", (chunk: Buffer) => chunks.push(chunk));
        response.on("end", () => resolveRequest({
          body: Buffer.concat(chunks),
          headers: response.headers,
          status: response.statusCode ?? -1,
        }));
      },
    );
    call.on("error", reject);
    call.end(body);
  });
}

function editablePayload(state: JsonObject): JsonObject {
  return structuredClone({
    expectedStateDigest: state["stateDigest"],
    submissionId: state["submissionId"],
    reviewer: state["reviewer"],
    confirmations: state["confirmations"],
    answers: state["answers"],
  });
}

function completeFictionalPayload(state: JsonObject): JsonObject {
  const payload = editablePayload(state);
  payload["submissionId"] = "fictional-harbor-workbench-e2e";
  payload["reviewer"] = {
    stableProjectId: "fictional-reviewer-e2e",
    qualificationCategory: "webUi",
    qualificationRationale: "Fictional browser-process conformance record only.",
    independence: "declaredFalse",
    independenceRationale: "The fictional test is maintained with the implementation.",
    priorExposureStatus: "full",
    priorExposureCaseIds: [
      "support-inbox-ambiguous-control",
      "support-inbox-clean",
      "support-inbox-labelledby-hard-negative",
      "support-inbox-unnamed-control",
    ],
    priorExposureRationale: "The fictional test deliberately uses tuning-visible expectations.",
    conflictStatus: "declared",
    conflictRationale: "This is automated conformance and not independent review.",
    reviewedOn: "2026-09-08",
  };
  payload["confirmations"] = Object.fromEntries(
    Object.keys(object(payload["confirmations"], "fictional confirmations")).map((key) => [key, true]),
  );
  const answers = array(payload["answers"], "fictional answers");
  const byCase = new Map(answers.map((answer) => [String(answer["caseId"]), answer]));
  Object.assign(object(byCase.get("support-inbox-ambiguous-control")?.["acquisition"], "ambiguous acquisition"), {
    status: "cantTell",
    observedValueKind: null,
    value: null,
    confidence: "low",
    rationale: "Fictional conformance abstains because native semantics are insufficient.",
  });
  Object.assign(object(byCase.get("support-inbox-ambiguous-control")?.["rule"], "ambiguous rule"), {
    outcome: "cantTell",
    requiredEvidence: "insufficient",
    confidence: "low",
    rationale: "Fictional conformance preserves insufficient evidence as cantTell.",
  });
  for (const caseId of ["support-inbox-clean", "support-inbox-labelledby-hard-negative"]) {
    Object.assign(object(byCase.get(caseId)?.["acquisition"], `${caseId} acquisition`), {
      status: "observed",
      observedValueKind: "text",
      value: "Send reply",
      confidence: "high",
      rationale: caseId === "support-inbox-clean"
        ? "Fictional conformance observes a browser-native text name."
        : "Fictional conformance observes text supplied by a native label reference.",
    });
    Object.assign(object(byCase.get(caseId)?.["rule"], `${caseId} rule`), {
      outcome: "passed",
      requiredEvidence: "sufficient",
      confidence: "high",
      rationale: caseId === "support-inbox-clean"
        ? "Fictional conformance records a named interactive control."
        : "Fictional conformance preserves the valid labelled-by alternative.",
    });
  }
  Object.assign(object(byCase.get("support-inbox-unnamed-control")?.["acquisition"], "unnamed acquisition"), {
    status: "observed",
    observedValueKind: "absent",
    value: null,
    confidence: "high",
    rationale: "Fictional conformance observes the native name as absent.",
  });
  Object.assign(object(byCase.get("support-inbox-unnamed-control")?.["rule"], "unnamed rule"), {
    outcome: "failed",
    requiredEvidence: "sufficient",
    confidence: "high",
    rationale: "Fictional conformance records the missing programmatic name.",
  });
  return payload;
}

async function choose(page: Page, selector: string, value: string): Promise<void> {
  await page.locator(selector).selectOption(value);
}

async function fillCase(
  page: Page,
  caseId: string,
  answer: {
    acquisition: "absent" | "cantTell" | "text" | "untested";
    acquisitionConfidence: "high" | "low" | "medium";
    acquisitionRationale: string;
    acquisitionValue?: string;
    evidence: "conflicting" | "insufficient" | "sufficient" | "untested";
    outcome: "cantTell" | "failed" | "inapplicable" | "passed" | "untested";
    ruleConfidence: "high" | "low" | "medium";
    ruleRationale: string;
  },
): Promise<void> {
  await page.locator(`[data-case-id="${caseId}"]`).click();
  const frame = page.frameLocator("#fixture-frame");
  await frame.locator("html[data-fixture-ready='true']").waitFor();
  const expectedState = object(
    array((await loadJson(questionnairePath))["questions"], "questions")
      .find((question) => question["caseId"] === caseId && question["authority"] === "acquisition"),
    "acquisition question",
  )["fixtureState"];
  assert.equal(await frame.locator("body").getAttribute("data-case"), expectedState);

  if (answer.acquisition === "text" || answer.acquisition === "absent") {
    await choose(page, "#acquisition-status", "observed");
    await choose(page, "#acquisition-value-kind", answer.acquisition);
    if (answer.acquisition === "text") await page.locator("#acquisition-value").fill(answer.acquisitionValue ?? "");
  } else {
    await choose(page, "#acquisition-status", answer.acquisition);
  }
  await choose(page, "#acquisition-confidence", answer.acquisitionConfidence);
  await page.locator("#acquisition-rationale").fill(answer.acquisitionRationale);
  await choose(page, "#rule-outcome", answer.outcome);
  await choose(page, "#rule-evidence", answer.evidence);
  await choose(page, "#rule-confidence", answer.ruleConfidence);
  await page.locator("#rule-rationale").fill(answer.ruleRationale);
}

async function copyRepositoryFile(sourceRoot: string, targetRoot: string, relative: string): Promise<void> {
  const target = resolve(targetRoot, relative);
  await mkdir(dirname(target), { recursive: true });
  await copyFile(resolve(sourceRoot, relative), target);
}

test("the Harbor workbench captures exactly eight fictional judgments and only compares after lock", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-harbor-workbench-"));
  const draftPath = join(directory, "draft.json");
  const finalPath = join(directory, "final.json");
  const repeatedDraftPath = join(directory, "repeated-draft.json");
  const repeatedFinalPath = join(directory, "repeated-final.json");
  let browser: Browser | undefined;
  let server: RunningWorkbench | undefined;
  try {
    const questionnaire = await loadJson(questionnairePath);
    const questionnaireSchema = validator(await loadJson(questionnaireSchemaPath));
    const stateSchema = validator(await loadJson(stateSchemaPath));
    const submissionSchema = validator(await loadJson(submissionSchemaPath));
    assertValid(questionnaireSchema, questionnaire, "Harbor questionnaire");
    assert.equal(array(questionnaire["questions"], "questions").length, 8);
    const serializedQuestionnaire = JSON.stringify(questionnaire);
    for (const prohibited of ["expectedObservation", "expectedVerdict", "oracleValue", "sightlintReport"]) {
      assert.equal(serializedQuestionnaire.includes(prohibited), false, `questionnaire leaked ${prohibited}`);
    }

    server = await start(repositoryRoot, draftPath, finalPath, { fictional: true });
    const initial = await api(server, "/api/state");
    assert.equal(initial.response.status, 200);
    const initialState = object(initial.value["state"], "initial state");
    assertValid(stateSchema, initialState, "initial incomplete state");
    assert.equal(initial.value["locked"], false);
    assert.equal(initial.value["finalization"], null);
    assert.equal(array(initial.value["sources"], "sources").length, 3);
    assert.equal(array(initial.value["sources"], "sources").some((entry) => String(entry["path"]).includes("annotations")), false);

    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await page.goto(server.url);
    await page.locator("#status").filter({ hasText: "Ready. Answers have not been compared." }).waitFor();
    assert.equal(await page.locator("#progress-count").textContent(), "0 / 8");
    assert.equal(await page.locator("[data-case-id]").count(), 4);
    assert.match(await page.locator("body").innerText(), /No oracle or SightLint output is available here/);
    assert.equal(await page.locator("body").innerText().then((text) => text.includes("expected verdict")), false);

    await page.locator("#submission-id").fill("fictional-harbor-workbench-e2e");
    await page.locator("#reviewer-id").fill("fictional-reviewer-e2e");
    await choose(page, "#qualification-category", "webUi");
    await page.locator("#qualification-rationale").fill("Fictional browser-process conformance record only.");
    await choose(page, "#independence", "declaredFalse");
    await page.locator("#independence-rationale").fill("The fictional test is maintained with the implementation.");
    await choose(page, "#exposure-status", "full");
    for (const checkbox of await page.locator("[data-exposure-case]").all()) await checkbox.check();
    await page.locator("#exposure-rationale").fill("The fictional test deliberately uses tuning-visible expectations.");
    await choose(page, "#conflict-status", "declared");
    await page.locator("#conflict-rationale").fill("This is automated conformance and not independent review.");
    await page.locator("#reviewed-on").fill("2026-09-08");
    for (const checkbox of await page.locator("#confirmations input[type='checkbox']").all()) await checkbox.check();

    await fillCase(page, "support-inbox-ambiguous-control", {
      acquisition: "cantTell",
      acquisitionConfidence: "low",
      acquisitionRationale: "Fictional conformance abstains because native semantics are insufficient.",
      evidence: "insufficient",
      outcome: "cantTell",
      ruleConfidence: "low",
      ruleRationale: "Fictional conformance preserves insufficient evidence as cantTell.",
    });
    await fillCase(page, "support-inbox-clean", {
      acquisition: "text",
      acquisitionConfidence: "high",
      acquisitionRationale: "Fictional conformance observes a browser-native text name.",
      acquisitionValue: "Send reply",
      evidence: "sufficient",
      outcome: "passed",
      ruleConfidence: "high",
      ruleRationale: "Fictional conformance records a named interactive control.",
    });
    await fillCase(page, "support-inbox-labelledby-hard-negative", {
      acquisition: "text",
      acquisitionConfidence: "high",
      acquisitionRationale: "Fictional conformance observes text supplied by a native label reference.",
      acquisitionValue: "Send reply",
      evidence: "sufficient",
      outcome: "passed",
      ruleConfidence: "high",
      ruleRationale: "Fictional conformance preserves the valid labelled-by alternative.",
    });
    await fillCase(page, "support-inbox-unnamed-control", {
      acquisition: "absent",
      acquisitionConfidence: "high",
      acquisitionRationale: "Fictional conformance observes the native name as absent.",
      evidence: "sufficient",
      outcome: "failed",
      ruleConfidence: "high",
      ruleRationale: "Fictional conformance records the missing programmatic name.",
    });
    assert.equal(await page.locator("#progress-count").textContent(), "8 / 8");
    await page.locator("#save").click();
    await page.locator("#status").filter({ hasText: "Draft saved at state digest" }).waitFor();
    const savedState = await loadJson(draftPath);
    assertValid(stateSchema, savedState, "saved workbench state");
    await stop(server);
    server = undefined;

    server = await start(repositoryRoot, draftPath, finalPath, { fictional: true, resume: true });
    await page.goto(server.url);
    await page.locator("#status").filter({ hasText: "Ready. Answers have not been compared." }).waitFor();
    assert.equal(await page.locator("#progress-count").textContent(), "8 / 8");
    await page.locator("#finalize").click();
    await page.locator("#completion").waitFor({ state: "visible" });
    assert.match(await page.locator("#completion-summary").innerText(), /evidence status ineligibleConformance/);
    const completed = await server.exit;
    server = undefined;
    assert.equal(completed.code, 0, completed.stderr.toString("utf8"));

    const finalized = await loadJson(finalPath);
    assertValid(submissionSchema, finalized, "finalized fictional submission");
    assert.equal(finalized["lifecycle"], "finalized");
    assert.equal(finalized["recordPurpose"], "fictionalConformance");
    assert.equal(finalized["evidenceStatus"], "ineligibleConformance");
    const cases = array(finalized["cases"], "finalized cases");
    assert.equal(cases.length, 4);
    assert.equal(cases.reduce((count, item) => count
      + array(item["acquisitionJudgments"], "acquisition judgments").length
      + array(item["ruleJudgments"], "rule judgments").length, 0), 8);
    const validation = await run(python, [prepare, "--validate-submission", finalPath]);
    assert.equal(validation.code, 0, validation.stderr.toString("utf8"));

    const comparison = await run(python, [compare, "--submission", finalPath]);
    assert.equal(comparison.code, 0, comparison.stderr.toString("utf8"));
    const report = object(JSON.parse(comparison.stdout.toString("utf8")), "post-lock comparison");
    assert.equal(report["evidenceStatus"], "ineligibleConformance");
    assert.equal(array(report["comparisons"], "comparison rows").length, 8);

    server = await start(repositoryRoot, repeatedDraftPath, repeatedFinalPath, { fictional: true });
    const repeatedInitial = await api(server, "/api/state");
    const repeatedState = object(repeatedInitial.value["state"], "repeated initial state");
    const repeatedPayload = editablePayload(savedState);
    repeatedPayload["expectedStateDigest"] = repeatedState["stateDigest"];
    const repeatedFinalization = await api(server, "/api/finalize", "POST", repeatedPayload);
    assert.equal(repeatedFinalization.response.status, 200);
    const repeatedExit = await server.exit;
    server = undefined;
    assert.equal(repeatedExit.code, 0, repeatedExit.stderr.toString("utf8"));
    assert.deepEqual(await readFile(repeatedFinalPath), await readFile(finalPath), "identical human input must produce byte-stable final bytes");
  } finally {
    if (server !== undefined) await stop(server);
    if (browser !== undefined) await browser.close();
    await rm(directory, { recursive: true, force: true });
  }
});

test("the workbench process fails closed at HTTP, state, privacy, and output boundaries", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-harbor-workbench-negative-"));
  const draftPath = join(directory, "draft.json");
  const finalPath = join(directory, "final.json");
  let server: RunningWorkbench | undefined;
  try {
    server = await start(repositoryRoot, draftPath, finalPath, { fictional: true });
    const initial = await api(server, "/api/state");
    const state = object(initial.value["state"], "initial state");
    const port = new URL(server.origin).port;
    const baseHeaders = { Host: `127.0.0.1:${port}` };
    const apiHeaders = { ...baseHeaders, "X-SightLint-Review-Token": server.token };
    const mutationHeaders = { ...apiHeaders, Origin: server.origin, "Content-Type": "application/json" };

    const root = await http(server, "/", "GET", baseHeaders);
    assert.equal(root.status, 200);
    assert.match(String(root.headers["content-security-policy"]), /default-src 'none'/);
    assert.equal(root.headers["access-control-allow-origin"], undefined);
    assert.equal(root.headers["cache-control"], "no-store");
    assert.equal((await http(server, "/api/state", "GET", baseHeaders)).status, 403);
    assert.equal((await http(server, "/api/state", "GET", { Host: "localhost", "X-SightLint-Review-Token": server.token })).status, 400);
    assert.equal((await http(server, "/missing", "GET", baseHeaders)).status, 404);
    assert.equal((await http(server, "/../annotations/rules.json", "GET", baseHeaders)).status, 404);
    assert.equal((await http(server, "/", "TRACE", baseHeaders)).status, 405);
    assert.equal((await http(server, "/", "TRACE", { Host: "localhost" })).status, 400);
    assert.equal((await http(server, "/api/save", "POST", { ...apiHeaders, Origin: "http://example.invalid", "Content-Type": "application/json" }, Buffer.from("{}"))).status, 403);
    assert.equal((await http(server, "/api/save", "POST", { ...apiHeaders, Origin: server.origin, "Content-Type": "text/plain" }, Buffer.from("{}"))).status, 400);

    const exactPrefix = '{"padding":"';
    const exactSuffix = '"}';
    const exactBody = Buffer.from(`${exactPrefix}${"a".repeat(262_144 - exactPrefix.length - exactSuffix.length)}${exactSuffix}`);
    assert.equal(exactBody.byteLength, 262_144);
    const exactResult = await http(server, "/api/save", "POST", { ...mutationHeaders, "Content-Length": String(exactBody.byteLength) }, exactBody);
    assert.equal(exactResult.status, 400);
    assert.equal(object(object(JSON.parse(exactResult.body.toString("utf8")), "exact-limit response")["error"], "error")["category"], "shape");
    const overResult = await http(server, "/api/save", "POST", { ...mutationHeaders, "Content-Length": "262145" }, Buffer.from("{}"));
    assert.equal(overResult.status, 400);
    assert.equal(object(object(JSON.parse(overResult.body.toString("utf8")), "over-limit response")["error"], "error")["category"], "request-budget");

    const duplicateBody = Buffer.from(`{"expectedStateDigest":"${String(state["stateDigest"])}","expectedStateDigest":"duplicate"}`);
    const duplicate = await http(server, "/api/save", "POST", { ...mutationHeaders, "Content-Length": String(duplicateBody.byteLength) }, duplicateBody);
    assert.equal(object(object(JSON.parse(duplicate.body.toString("utf8")), "duplicate response")["error"], "error")["category"], "json");

    const incomplete = await api(server, "/api/finalize", "POST", editablePayload(state));
    assert.equal(incomplete.response.status, 400);
    assert.equal(object(incomplete.value["error"], "incomplete error")["category"], "finalization");

    const stale = editablePayload(state);
    stale["expectedStateDigest"] = `sha256:${"0".repeat(64)}`;
    assert.equal((await api(server, "/api/save", "POST", stale)).response.status, 400);

    const unavailableValue = editablePayload(state);
    const unavailableAnswers = array(unavailableValue["answers"], "unavailable answers");
    object(unavailableAnswers[0]!["acquisition"], "unavailable acquisition")["status"] = "cantTell";
    object(unavailableAnswers[0]!["acquisition"], "unavailable acquisition")["value"] = "guess";
    assert.equal((await api(server, "/api/save", "POST", unavailableValue)).response.status, 400);

    const invalidRule = editablePayload(state);
    const invalidRuleAnswer = object(array(invalidRule["answers"], "invalid rule answers")[0]!["rule"], "invalid rule");
    invalidRuleAnswer["outcome"] = "passed";
    invalidRuleAnswer["requiredEvidence"] = "insufficient";
    assert.equal((await api(server, "/api/save", "POST", invalidRule)).response.status, 400);

    const leaking = editablePayload(state);
    object(leaking["reviewer"], "leaking reviewer")["qualificationRationale"] = "See https://private.invalid/review";
    const leakResponse = await api(server, "/api/save", "POST", leaking);
    assert.equal(object(leakResponse.value["error"], "privacy error")["category"], "privacy");

    const invalidExposure = editablePayload(state);
    object(invalidExposure["reviewer"], "exposure reviewer")["priorExposureStatus"] = "full";
    const exposureResponse = await api(server, "/api/save", "POST", invalidExposure);
    assert.equal(object(exposureResponse.value["error"], "exposure error")["category"], "exposure");

    const invalidDate = editablePayload(state);
    object(invalidDate["reviewer"], "date reviewer")["reviewedOn"] = "2026-02-30";
    const dateResponse = await api(server, "/api/save", "POST", invalidDate);
    assert.equal(object(dateResponse.value["error"], "date error")["category"], "date");
    await stop(server);
    server = undefined;

    const common = ["--scope", "harbor", "--draft", join(directory, "cli-draft.json"), "--final", join(directory, "cli-final.json"), "--no-open"];
    const nonLoopback = await run(python, [runWorkbench, ...common, "--host", "0.0.0.0"]);
    const repeatedNonLoopback = await run(python, [runWorkbench, ...common, "--host", "0.0.0.0"]);
    assert.deepEqual(repeatedNonLoopback, nonLoopback, "failure process behavior must be byte-stable per platform");
    assert.equal(nonLoopback.code, 2);
    assert.equal(
      nonLoopback.stderr.toString("utf8"),
      `web-review-workbench: host: workbench host must be exactly 127.0.0.1${platformNewline}`,
    );

    const inside = await run(python, [runWorkbench, "--scope", "harbor", "--draft", resolve(repositoryRoot, "forbidden-draft.json"), "--final", join(directory, "unused-final.json"), "--no-open"]);
    assert.equal(inside.code, 2);
    assert.match(inside.stderr.toString("utf8"), /output: draft path must be outside the repository/);

    const existingDraft = join(directory, "existing-draft.json");
    await writeFile(existingDraft, "{}", "utf8");
    const noResume = await run(python, [runWorkbench, "--scope", "harbor", "--draft", existingDraft, "--final", join(directory, "no-resume-final.json"), "--no-open"]);
    assert.equal(noResume.code, 2);
    assert.match(noResume.stderr.toString("utf8"), /draft output exists; use --resume/);

    const missingResume = await run(python, [runWorkbench, "--scope", "harbor", "--draft", join(directory, "missing.json"), "--final", join(directory, "missing-final.json"), "--resume", "--no-open"]);
    assert.equal(missingResume.code, 2);
    assert.match(missingResume.stderr.toString("utf8"), /resume requires an existing draft file/);

    const existingFinal = join(directory, "existing-final.json");
    await writeFile(existingFinal, "do not replace", "utf8");
    const overwrite = await run(python, [runWorkbench, "--scope", "harbor", "--draft", join(directory, "overwrite-draft.json"), "--final", existingFinal, "--no-open"]);
    assert.equal(overwrite.code, 2);
    assert.equal(await readFile(existingFinal, "utf8"), "do not replace");

    const samePath = join(directory, "same.json");
    const same = await run(python, [runWorkbench, "--scope", "harbor", "--draft", samePath, "--final", samePath, "--no-open"]);
    assert.equal(same.code, 2);
    assert.match(same.stderr.toString("utf8"), /draft and finalized output paths must differ/);

    if (process.platform !== "win32") {
      const dangling = join(directory, "dangling.json");
      await symlink(join(directory, "missing-target.json"), dangling);
      const symlinkResult = await run(python, [runWorkbench, "--scope", "harbor", "--draft", dangling, "--final", join(directory, "symlink-final.json"), "--no-open"]);
      assert.equal(symlinkResult.code, 2);
      assert.match(symlinkResult.stderr.toString("utf8"), /draft path must not be a symlink/);
    }
  } finally {
    if (server !== undefined) await stop(server);
    await rm(directory, { recursive: true, force: true });
  }
});

test("the workbench finalizes from a source-only tree with no oracle or comparator", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-harbor-source-only-"));
  const isolatedRoot = join(directory, "source");
  const outputRoot = join(directory, "output");
  let server: RunningWorkbench | undefined;
  try {
    await mkdir(outputRoot, { recursive: true });
    const packet = await loadJson(packetPath);
    const relativeFiles = [
      "tools/run_web_review.py",
      "tools/web_review_contract.py",
      "tools/web_review_workbench.py",
      "evaluation/web/review-packet.json",
      "evaluation/web/harbor-review-questionnaire.json",
      "evaluation/web/review-workbench/index.html",
      "evaluation/web/review-workbench/app.js",
      "evaluation/web/review-workbench/styles.css",
      ...array(packet["files"], "packet files").map((entry) => String(entry["path"])),
    ];
    for (const relative of new Set(relativeFiles)) await copyRepositoryFile(repositoryRoot, isolatedRoot, relative);
    await assert.rejects(stat(resolve(isolatedRoot, "evaluation/web/annotations")));
    await assert.rejects(stat(resolve(isolatedRoot, "tools/compare_web_review.py")));

    const draftPath = join(outputRoot, "draft.json");
    const finalPath = join(outputRoot, "final.json");
    server = await start(isolatedRoot, draftPath, finalPath, { fictional: true });
    const initial = await api(server, "/api/state");
    assert.equal(initial.response.status, 200);
    assert.equal(initial.value["finalization"], null);
    assert.equal(array(initial.value["sources"], "isolated sources").length, 3);
    const finalization = await api(
      server,
      "/api/finalize",
      "POST",
      completeFictionalPayload(object(initial.value["state"], "isolated state")),
    );
    assert.equal(finalization.response.status, 200);
    const completed = await server.exit;
    server = undefined;
    assert.equal(completed.code, 0, completed.stderr.toString("utf8"));
    assert.equal((await stat(draftPath)).isFile(), true);
    assert.equal((await stat(finalPath)).isFile(), true);
    const finalized = await loadJson(finalPath);
    assert.equal(finalized["recordPurpose"], "fictionalConformance");
    assert.equal(finalized["evidenceStatus"], "ineligibleConformance");
  } finally {
    if (server !== undefined) await stop(server);
    await rm(directory, { recursive: true, force: true });
  }
});
