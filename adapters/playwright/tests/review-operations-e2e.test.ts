import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const prepare = resolve(repositoryRoot, "tools/prepare_web_review.py");
const compare = resolve(repositoryRoot, "tools/compare_web_review.py");
const packetPath = resolve(repositoryRoot, "evaluation/web/review-packet.json");
const blankPath = resolve(repositoryRoot, "evaluation/web/reviewer-submission.blank.json");
const submissionPath = resolve(repositoryRoot, "evaluation/web/conformance/review/fictional-submission.json");
const registryPath = resolve(repositoryRoot, "evaluation/web/evaluation-v1.json");
const python = process.env["PYTHON"] ?? (process.platform === "win32" ? "python" : "python3");

type JsonObject = Record<string, unknown>;

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
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

function pythonRun(program: string, args: string[]): Promise<ProcessResult> {
  return run(python, [program, ...args]);
}

async function loadJson(path: string): Promise<JsonObject> {
  return JSON.parse(await readFile(path, "utf8")) as JsonObject;
}

function object(value: unknown, label: string): JsonObject {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${label} must be an object`);
  return value as JsonObject;
}

function array(value: unknown, label: string): JsonObject[] {
  assert.ok(Array.isArray(value), `${label} must be an array`);
  return value as JsonObject[];
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function canonicalValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalValue);
  if (value !== null && typeof value === "object") {
    const source = value as JsonObject;
    return Object.fromEntries(Object.keys(source).toSorted().map((key) => [key, canonicalValue(source[key])]));
  }
  return value;
}

function canonicalBytes(value: JsonObject, omit?: string): Buffer {
  const projected = Object.fromEntries(Object.entries(value).filter(([key]) => key !== omit));
  const json = JSON.stringify(canonicalValue(projected)).replace(
    /[^\x00-\x7f]/g,
    (character) => `\\u${character.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );
  return Buffer.from(json, "utf8");
}

function digest(value: JsonObject, omit?: string): string {
  return `sha256:${createHash("sha256").update(canonicalBytes(value, omit)).digest("hex")}`;
}

function rawDigest(raw: Buffer): string {
  return createHash("sha256").update(raw).digest("hex");
}

async function writeJson(path: string, value: JsonObject): Promise<void> {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function validator(schema: JsonObject): ValidateFunction {
  return new Ajv2020({ allErrors: true, strict: true, strictRequired: false, validateFormats: false }).compile(schema);
}

function assertValid(validate: ValidateFunction, value: unknown, label: string): void {
  assert.equal(validate(value), true, `${label}: ${JSON.stringify(validate.errors)}`);
}

function assertInvalid(validate: ValidateFunction, value: unknown, label: string): void {
  assert.equal(validate(value), false, `${label} unexpectedly passed schema validation`);
}

async function assertFailure(
  program: string,
  args: string[],
  prefix: string,
  category: string,
  detail: string,
): Promise<void> {
  const first = await pythonRun(program, args);
  const repeated = await pythonRun(program, args);
  assert.deepEqual(repeated, first, "failure process behavior must be byte-stable");
  assert.equal(first.code, 2);
  assert.equal(first.stdout.byteLength, 0);
  assert.equal(first.stderr.toString("utf8"), `${prefix}: ${category}: ${detail}\n`);
}

function finalizedDigest(document: JsonObject): void {
  document["submissionDigest"] = digest(document, "submissionDigest");
}

function packetDigest(document: JsonObject): void {
  document["packetDigest"] = digest(document, "packetDigest");
}

function draft(document: JsonObject): JsonObject {
  const value = clone(document);
  value["lifecycle"] = "draft";
  value["submissionDigest"] = null;
  return value;
}

test("review schemas keep packet, acquisition, rule, outcome, and evidence authorities strict", async () => {
  const packet = await loadJson(packetPath);
  const blank = await loadJson(blankPath);
  const submission = await loadJson(submissionPath);
  const packetSchema = await loadJson(resolve(repositoryRoot, "evaluation/web/review-packet.schema.json"));
  const submissionSchema = await loadJson(resolve(repositoryRoot, "evaluation/web/reviewer-submission.schema.json"));
  const comparisonSchema = await loadJson(resolve(repositoryRoot, "evaluation/web/review-comparison.schema.json"));
  const validatePacket = validator(packetSchema);
  const validateSubmission = validator(submissionSchema);
  const validateComparison = validator(comparisonSchema);

  assertValid(validatePacket, packet, "source-only review packet");
  assertValid(validateSubmission, blank, "blank submission template");
  assertValid(validateSubmission, submission, "fictional finalized submission");
  assert.deepEqual(packet["governance"], {
    ownership: "sightlintRepository",
    license: "MIT OR Apache-2.0",
    redistribution: "permittedUnderRepositoryLicense",
    privacyReview: "syntheticNoPersonalData",
    containsPersonalOrCustomerData: false,
    containsCredentials: false,
    externalAssets: false,
    externalNetwork: false,
    externalProcessing: false,
    exposure: "publicTuningVisible",
  });
  assert.ok(array(packet["files"], "packet files").every((file) =>
    file["kind"] === "fixtureSource" || file["kind"] === "captureRequest"));
  assert.ok(array(packet["files"], "packet files").every((file) =>
    typeof file["path"] === "string" && !file["path"].includes("/annotations/") && !file["path"].endsWith(".png")));
  assert.ok(array(blank["cases"], "blank cases").every((caseRecord) =>
    array(caseRecord["acquisitionJudgments"], "blank acquisition judgments").length === 0
      && array(caseRecord["ruleJudgments"], "blank rule judgments").length === 0));

  const processResult = await pythonRun(compare, ["--submission", submissionPath]);
  assert.equal(processResult.code, 0, processResult.stderr.toString("utf8"));
  const comparisonOutput = JSON.parse(processResult.stdout.toString("utf8")) as JsonObject;
  assertValid(validateComparison, comparisonOutput, "comparison output");
  const flattenedCount = clone(comparisonOutput);
  object(flattenedCount["counts"], "comparison counts")["acquisitionAgreement"] = 3;
  assertInvalid(validateComparison, flattenedCount, "comparison with an implicit denominator");

  const unknownPacket = clone(packet);
  unknownPacket["expectedVerdict"] = "passed";
  assertInvalid(validatePacket, unknownPacket, "packet containing an expected verdict");

  const mixedSubmission = clone(submission);
  const firstAcquisition = array(array(mixedSubmission["cases"], "cases")[0]!["acquisitionJudgments"], "acquisition")[0]!;
  firstAcquisition["outcome"] = "failed";
  assertInvalid(validateSubmission, mixedSubmission, "acquisition judgment containing a rule verdict");

  const guessed = clone(submission);
  const cantTellCase = array(guessed["cases"], "cases")[1]!;
  array(cantTellCase["acquisitionJudgments"], "acquisition")[0]!["value"] = "button";
  assertInvalid(validateSubmission, guessed, "cantTell judgment containing a guessed value");

  const ruleOutcomes = new Set(
    array(submission["cases"], "cases").flatMap((item) =>
      array(item["ruleJudgments"], "ruleJudgments").map((judgment) => judgment["outcome"]),
    ),
  );
  assert.deepEqual([...ruleOutcomes].toSorted(), ["cantTell", "failed", "inapplicable", "passed", "untested"]);
  assert.ok(array(submission["cases"], "cases").some((item) => object(item["caseContext"], "context")["reviewedAs"] === "hardNegative"));
});

test("generation, finalization, and comparison are byte-stable and comparison is read-only", async () => {
  const firstCheck = await pythonRun(prepare, ["--check"]);
  const secondCheck = await pythonRun(prepare, ["--check"]);
  assert.deepEqual(secondCheck, firstCheck);
  assert.equal(firstCheck.code, 0, firstCheck.stderr.toString("utf8"));
  assert.equal(firstCheck.stdout.toString("utf8"), "web review prepare: packet=valid, blank_submission=valid, drift=false\n");

  const directory = await mkdtemp(join(tmpdir(), "sightlint-web-review-"));
  try {
    const finalized = await loadJson(submissionPath);
    const draftPath = join(directory, "draft.json");
    await writeJson(draftPath, draft(finalized));
    const firstFinalization = await pythonRun(prepare, ["--finalize-submission", draftPath]);
    const secondFinalization = await pythonRun(prepare, ["--finalize-submission", draftPath]);
    assert.deepEqual(secondFinalization, firstFinalization);
    assert.equal(firstFinalization.code, 0, firstFinalization.stderr.toString("utf8"));
    assert.equal(firstFinalization.stdout.at(-1), 0x7d, "canonical finalized stdout has no trailing newline");
    assert.deepEqual(JSON.parse(firstFinalization.stdout.toString("utf8")), finalized);

    const watched = [
      packetPath,
      submissionPath,
      registryPath,
      resolve(repositoryRoot, "evaluation/web/annotations/browser-acquisition.json"),
      resolve(repositoryRoot, "evaluation/web/annotations/browser-rules.json"),
      resolve(repositoryRoot, "evaluation/web/annotations/support-inbox-acquisition.json"),
      resolve(repositoryRoot, "evaluation/web/annotations/support-inbox-rules.json"),
    ];
    const before = await Promise.all(watched.map(async (path) => rawDigest(await readFile(path))));
    const firstComparison = await pythonRun(compare, ["--submission", submissionPath]);
    const secondComparison = await pythonRun(compare, ["--submission", submissionPath]);
    const after = await Promise.all(watched.map(async (path) => rawDigest(await readFile(path))));
    assert.deepEqual(secondComparison, firstComparison);
    assert.deepEqual(after, before, "comparison must not mutate packet, submission, registry, or oracles");
    assert.equal(firstComparison.code, 0, firstComparison.stderr.toString("utf8"));
    assert.equal(firstComparison.stdout.at(-1), 0x7d, "canonical comparison stdout has no trailing newline");
    const report = JSON.parse(firstComparison.stdout.toString("utf8")) as JsonObject;
    assert.deepEqual(report["counts"], {
      abstentionAgreement: { numerator: 4, denominator: 4 },
      acquisitionAgreement: { numerator: 3, denominator: 5 },
      adjudicated: { numerator: 0, denominator: 3 },
      disagreement: { numerator: 1, denominator: 9 },
      ruleAgreement: { numerator: 5, denominator: 6 },
      unresolved: { numerator: 3, denominator: 11 },
    });
    assert.equal(report["evidenceStatus"], "ineligibleConformance");
    const rows = array(report["comparisons"], "comparisons");
    assert.equal(rows.length, 11);
    assert.ok(rows.some((row) => row["status"] === "disagreement" && row["unresolved"] === true));
    assert.ok(rows.some((row) => row["status"] === "unresolved"));
    assert.ok(rows.every((row) => object(row["adjudication"], "adjudication")["status"] === "notPerformed"));

    const singleAuthority = draft(finalized);
    object(singleAuthority["reviewScope"], "single-authority scope")["familyIds"] = ["harbor-support-inbox-v1"];
    object(singleAuthority["reviewScope"], "single-authority scope")["caseIds"] = ["support-inbox-clean"];
    const cleanCase = clone(array(singleAuthority["cases"], "single-authority cases")
      .find((item) => item["caseId"] === "support-inbox-clean")!);
    cleanCase["ruleJudgments"] = [];
    singleAuthority["cases"] = [cleanCase];
    const singleDraftPath = join(directory, "single-authority-draft.json");
    const singleFinalPath = join(directory, "single-authority-final.json");
    await writeJson(singleDraftPath, singleAuthority);
    const singleFinalization = await pythonRun(prepare, ["--finalize-submission", singleDraftPath]);
    assert.equal(singleFinalization.code, 0, singleFinalization.stderr.toString("utf8"));
    await writeFile(singleFinalPath, singleFinalization.stdout);
    const singleComparison = await pythonRun(compare, ["--submission", singleFinalPath]);
    assert.equal(singleComparison.code, 0, singleComparison.stderr.toString("utf8"));
    const singleReport = JSON.parse(singleComparison.stdout.toString("utf8")) as JsonObject;
    assert.deepEqual(singleReport["counts"], {
      abstentionAgreement: { numerator: 0, denominator: 0 },
      acquisitionAgreement: { numerator: 0, denominator: 1 },
      adjudicated: { numerator: 0, denominator: 1 },
      disagreement: { numerator: 1, denominator: 1 },
      ruleAgreement: { numerator: 0, denominator: 0 },
      unresolved: { numerator: 1, denominator: 1 },
    });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("review processes fail closed on leakage, ambiguity, duplicate IDs, fields, and digests", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-web-review-negative-"));
  try {
    const packet = await loadJson(packetPath);
    const submission = await loadJson(submissionPath);

    const leakingPacket = clone(packet);
    array(leakingPacket["files"], "files")[0]!["kind"] = "generatedScreenshot";
    packetDigest(leakingPacket);
    const leakingPacketPath = join(directory, "leaking-packet.json");
    await writeJson(leakingPacketPath, leakingPacket);
    await assertFailure(
      prepare,
      ["--validate-packet", leakingPacketPath],
      "web-review-prepare",
      "leakage",
      "packet file evaluation/web/fixture-app/app.js has an unsupported input kind",
    );

    const unknown = clone(submission);
    unknown["expectedOutput"] = "passed";
    const unknownPath = join(directory, "unknown.json");
    await writeJson(unknownPath, unknown);
    await assertFailure(
      prepare,
      ["--validate-submission", unknownPath],
      "web-review-prepare",
      "shape",
      "reviewer submission contains unsupported fields: expectedOutput",
    );

    const duplicate = clone(submission);
    const duplicateCases = array(duplicate["cases"], "cases");
    duplicateCases[1]!["caseId"] = duplicateCases[0]!["caseId"];
    finalizedDigest(duplicate);
    const duplicatePath = join(directory, "duplicate.json");
    await writeJson(duplicatePath, duplicate);
    await assertFailure(
      prepare,
      ["--validate-submission", duplicatePath],
      "web-review-prepare",
      "ordering",
      "reviewer submission cases must be unique and sorted",
    );

    const duplicateKeyPath = join(directory, "duplicate-key.json");
    const rawSubmission = await readFile(submissionPath, "utf8");
    await writeFile(duplicateKeyPath, `{"$schema":"duplicate",${rawSubmission.slice(1)}`, "utf8");
    await assertFailure(
      prepare,
      ["--validate-submission", duplicateKeyPath],
      "web-review-prepare",
      "json",
      "a document contains a duplicate object key",
    );

    const staleDigest = clone(submission);
    staleDigest["submissionDigest"] = `sha256:${"0".repeat(64)}`;
    const staleDigestPath = join(directory, "stale-digest.json");
    await writeJson(staleDigestPath, staleDigest);
    await assertFailure(
      compare,
      ["--submission", staleDigestPath],
      "web-review-compare",
      "digest",
      "reviewer submission digest does not match its canonical projection",
    );

    const exposed = clone(submission);
    object(exposed["declarations"], "declarations")["existingOracleViewedBeforeFinalization"] = true;
    finalizedDigest(exposed);
    const exposedPath = join(directory, "exposed.json");
    await writeJson(exposedPath, exposed);
    await assertFailure(
      prepare,
      ["--validate-submission", exposedPath],
      "web-review-prepare",
      "declaration",
      "reviewer submission violates the source-only declaration boundary",
    );

    const guessed = clone(submission);
    const ambiguous = array(array(guessed["cases"], "cases")[1]!["acquisitionJudgments"], "acquisition")[0]!;
    ambiguous["value"] = "button";
    finalizedDigest(guessed);
    const guessedPath = join(directory, "guessed.json");
    await writeJson(guessedPath, guessed);
    await assertFailure(
      prepare,
      ["--validate-submission", guessedPath],
      "web-review-prepare",
      "authority",
      "case support-inbox-ambiguous-control acquisition judgment unavailable observation must have null value",
    );

    const privateUrl = clone(submission);
    object(privateUrl["reviewer"], "reviewer")["independenceRationale"] = "Private record at https://private.invalid/review";
    finalizedDigest(privateUrl);
    const privateUrlPath = join(directory, "private-url.json");
    await writeJson(privateUrlPath, privateUrl);
    await assertFailure(
      prepare,
      ["--validate-submission", privateUrlPath],
      "web-review-prepare",
      "privacy",
      "reviewer submission must not contain URLs",
    );

    const draftPath = join(directory, "draft.json");
    await writeJson(draftPath, draft(submission));
    await assertFailure(
      compare,
      ["--submission", draftPath, "--registry", join(directory, "missing-registry.json")],
      "web-review-compare",
      "lifecycle",
      "reviewer submission must be finalized before comparison",
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("review input, string, packet, and judgment exact bounds accept max and reject one over", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-web-review-limits-"));
  try {
    const packetRaw = await readFile(packetPath);
    const packet = await loadJson(packetPath);
    assert.equal(array(packet["files"], "packet files").length, 33, "fixed packet file maximum is exercised");
    assert.equal(array(packet["cases"], "packet cases").length, 27, "fixed packet case maximum is exercised");
    const fileOneOver = clone(packet);
    array(fileOneOver["files"], "packet files").push(clone(array(fileOneOver["files"], "packet files")[0]!));
    packetDigest(fileOneOver);
    const fileOneOverPath = join(directory, "packet-file-count-oversized.json");
    await writeJson(fileOneOverPath, fileOneOver);
    await assertFailure(
      prepare,
      ["--validate-packet", fileOneOverPath],
      "web-review-prepare",
      "limit",
      "review packet files has an invalid item count",
    );
    const caseOneOver = clone(packet);
    array(caseOneOver["cases"], "packet cases").push(clone(array(caseOneOver["cases"], "packet cases")[0]!));
    packetDigest(caseOneOver);
    const caseOneOverPath = join(directory, "packet-case-count-oversized.json");
    await writeJson(caseOneOverPath, caseOneOver);
    await assertFailure(
      prepare,
      ["--validate-packet", caseOneOverPath],
      "web-review-prepare",
      "limit",
      "review packet cases has an invalid item count",
    );

    const exactPacketPath = join(directory, "packet-exact.json");
    await writeFile(exactPacketPath, Buffer.concat([packetRaw, Buffer.alloc(8_388_608 - packetRaw.byteLength, 0x20)]));
    const exactPacket = await pythonRun(prepare, ["--validate-packet", exactPacketPath]);
    assert.equal(exactPacket.code, 0, exactPacket.stderr.toString("utf8"));
    const oversizedPacketPath = join(directory, "packet-oversized.json");
    await writeFile(oversizedPacketPath, Buffer.alloc(8_388_609, 0x20));
    await assertFailure(
      prepare,
      ["--validate-packet", oversizedPacketPath],
      "web-review-prepare",
      "input-budget",
      "review packet exceeds the 8388608-byte limit",
    );

    const submissionRaw = await readFile(submissionPath);
    const blank = await loadJson(blankPath);
    assert.equal(array(blank["cases"], "blank cases").length, 27, "submission case maximum is exercised");
    const submissionCaseOneOver = clone(blank);
    array(submissionCaseOneOver["cases"], "blank cases").push(clone(array(submissionCaseOneOver["cases"], "blank cases")[0]!));
    const submissionCaseOneOverPath = join(directory, "submission-case-count-oversized.json");
    await writeJson(submissionCaseOneOverPath, submissionCaseOneOver);
    await assertFailure(
      prepare,
      ["--validate-submission", submissionCaseOneOverPath],
      "web-review-prepare",
      "limit",
      "reviewer submission cases has an invalid item count",
    );
    const exactInputPath = join(directory, "submission-exact.json");
    await writeFile(exactInputPath, Buffer.concat([submissionRaw, Buffer.alloc(1_048_576 - submissionRaw.byteLength, 0x20)]));
    const exactInput = await pythonRun(prepare, ["--validate-submission", exactInputPath]);
    assert.equal(exactInput.code, 0, exactInput.stderr.toString("utf8"));
    const oversizedInputPath = join(directory, "submission-oversized.json");
    await writeFile(oversizedInputPath, Buffer.alloc(1_048_577, 0x20));
    await assertFailure(
      prepare,
      ["--validate-submission", oversizedInputPath],
      "web-review-prepare",
      "input-budget",
      "reviewer submission exceeds the 1048576-byte limit",
    );

    const submission = await loadJson(submissionPath);
    const exactString = draft(submission);
    const exactLimitations = exactString["submissionLimitations"];
    assert.ok(Array.isArray(exactLimitations));
    exactLimitations[0] = "x".repeat(4096);
    const exactStringPath = join(directory, "string-exact.json");
    await writeJson(exactStringPath, exactString);
    const exactStringRun = await pythonRun(prepare, ["--finalize-submission", exactStringPath]);
    assert.equal(exactStringRun.code, 0, exactStringRun.stderr.toString("utf8"));
    const oversizedString = clone(exactString);
    const oversizedLimitations = oversizedString["submissionLimitations"];
    assert.ok(Array.isArray(oversizedLimitations));
    oversizedLimitations[0] = "x".repeat(4097);
    const oversizedStringPath = join(directory, "string-oversized.json");
    await writeJson(oversizedStringPath, oversizedString);
    await assertFailure(
      prepare,
      ["--finalize-submission", oversizedStringPath],
      "web-review-prepare",
      "string-budget",
      "reviewer submission exceeds the 4096-byte string limit",
    );

    const exactCount = draft(submission);
    object(exactCount["reviewScope"], "scope")["familyIds"] = ["harbor-support-inbox-v1"];
    object(exactCount["reviewScope"], "scope")["caseIds"] = ["support-inbox-clean"];
    object(exactCount["reviewer"], "reviewer")["priorExpectedLabelExposure"] = {
      status: "full",
      caseIds: ["support-inbox-clean"],
      rationale: "Fictional boundary data is fully visible.",
    };
    const caseRecord = clone(array(exactCount["cases"], "cases")[2]!);
    caseRecord["acquisitionJudgments"] = [];
    const baseRule = clone(array(caseRecord["ruleJudgments"], "rules")[0]!);
    caseRecord["ruleJudgments"] = Array.from({ length: 512 }, (_, index) => ({
      ...baseRule,
      judgmentId: `r-boundary-${index.toString().padStart(4, "0")}`,
      targetId: `web-boundary-${index.toString().padStart(4, "0")}`,
    }));
    exactCount["cases"] = [caseRecord];
    const exactCountPath = join(directory, "count-exact.json");
    await writeJson(exactCountPath, exactCount);
    assert.ok((await stat(exactCountPath)).size < 1_048_576, "count boundary must be reachable within file limit");
    const exactCountRun = await pythonRun(prepare, ["--finalize-submission", exactCountPath]);
    assert.equal(exactCountRun.code, 0, exactCountRun.stderr.toString("utf8"));

    const oversizedCount = clone(exactCount);
    const rules = array(array(oversizedCount["cases"], "cases")[0]!["ruleJudgments"], "rules");
    rules.push({ ...rules.at(-1)!, judgmentId: "r-boundary-0512", targetId: "web-boundary-0512" });
    const oversizedCountPath = join(directory, "count-oversized.json");
    await writeJson(oversizedCountPath, oversizedCount);
    await assertFailure(
      prepare,
      ["--finalize-submission", oversizedCountPath],
      "web-review-prepare",
      "limit",
      "case support-inbox-clean ruleJudgments has an invalid item count",
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
