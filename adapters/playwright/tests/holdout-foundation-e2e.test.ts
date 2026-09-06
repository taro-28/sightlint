import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const checker = resolve(repositoryRoot, "tools/check_web_holdout_foundation.py");
const admission = resolve(repositoryRoot, "evaluation/web/holdout-admission.json");
const currentRecord = resolve(repositoryRoot, "evaluation/web/holdout-run.json");
const conformanceSource = resolve(repositoryRoot, "evaluation/web/conformance/holdout");
const python = process.env["PYTHON"] ?? (process.platform === "win32" ? "python" : "python3");

type JsonObject = Record<string, unknown>;

interface ProcessResult {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

function run(args: string[]): Promise<ProcessResult> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(python, [checker, ...args], {
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
        reject(new Error(`${python} terminated by ${signal}`));
        return;
      }
      resolveRun({ code: code ?? -1, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) });
    });
  });
}

async function temporaryConformance(): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-holdout-foundation-"));
  await cp(conformanceSource, directory, { recursive: true });
  return directory;
}

async function json(path: string): Promise<JsonObject> {
  return JSON.parse(await readFile(path, "utf8")) as JsonObject;
}

async function writeJson(path: string, value: JsonObject): Promise<void> {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function object(value: unknown, label: string): JsonObject {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${label} must be an object`);
  return value as JsonObject;
}

function array(value: unknown, label: string): JsonObject[] {
  assert.ok(Array.isArray(value), `${label} must be an array`);
  return value as JsonObject[];
}

async function assertFailure(directory: string, category: string, detail: string): Promise<void> {
  const first = await run(["--conformance-dir", directory]);
  const second = await run(["--conformance-dir", directory]);
  assert.deepEqual(second, first, "failure diagnostics must be byte-stable");
  assert.equal(first.code, 2);
  assert.equal(first.stdout.byteLength, 0);
  assert.equal(first.stderr.toString("utf8"), `web-holdout-foundation: ${category}: ${detail}\n`);
}

test("public holdout checker reports stable non-operational and conformance-only states", async () => {
  const current = await run([]);
  const repeatedCurrent = await run([]);
  assert.deepEqual(repeatedCurrent, current);
  assert.equal(current.code, 0);
  assert.equal(current.stderr.byteLength, 0);
  assert.equal(
    current.stdout.toString("utf8"),
    "web holdout foundation: lifecycle=notRun, admission=notOperational, evidence_eligible=false\n",
  );

  const conformance = await run(["--conformance-dir", conformanceSource]);
  const repeatedConformance = await run(["--conformance-dir", conformanceSource]);
  assert.deepEqual(repeatedConformance, conformance);
  assert.equal(conformance.code, 0);
  assert.equal(conformance.stderr.byteLength, 0);
  assert.equal(
    conformance.stdout.toString("utf8"),
    "web holdout foundation: conformance_chain=valid, cases=6, metrics=9, evidence_eligible=false\n",
  );
});

test("public holdout checker rejects digest, path, command, metric, disclosure, and leakage mutations", async () => {
  const mutations: Array<{
    file: string;
    category: string;
    detail: string;
    mutate: (document: JsonObject) => void;
  }> = [
    {
      file: "bundle-manifest.json",
      category: "digest",
      detail: "bundle manifest manifestDigest does not match its canonical projection",
      mutate: (document) => { document["manifestDigest"] = `sha256:${"f".repeat(64)}`; },
    },
    {
      file: "bundle-manifest.json",
      category: "version",
      detail: "bundle manifest uses an unsupported schema or document type",
      mutate: (document) => { document["schemaVersion"] = "2.0.0"; },
    },
    {
      file: "bundle-manifest.json",
      category: "path",
      detail: "bundle file must use a contained relative POSIX path",
      mutate: (document) => {
        const firstCase = array(document["cases"], "cases")[0]!;
        array(firstCase["files"], "files")[0]!["path"] = "../protected/request.json";
      },
    },
    {
      file: "bundle-manifest.json",
      category: "privacy",
      detail: "public conformance data must remain reviewed, fictional, credential-free, and local",
      mutate: (document) => {
        object(document["privacy"], "privacy")["containsCustomerData"] = true;
      },
    },
    {
      file: "bundle-manifest.json",
      category: "oracle",
      detail: "implementation output must not be used as bundle truth",
      mutate: (document) => {
        object(document["bundle"], "bundle")["implementationOutputUsedAsOracle"] = true;
      },
    },
    {
      file: "oracle-manifest.json",
      category: "oracle",
      detail: "acquisition and rule truth must use distinct documents",
      mutate: (document) => {
        const acquisitionDocument = array(document["acquisitionDocuments"], "acquisition documents")[0]!;
        array(document["ruleDocuments"], "rule documents")[0]!["path"] = acquisitionDocument["path"];
      },
    },
    {
      file: "invocation-manifest.json",
      category: "digest",
      detail: "commandDigest does not match its canonical projection",
      mutate: (document) => {
        object(document["commands"], "commands")["commandDigest"] = `sha256:${"f".repeat(64)}`;
      },
    },
    {
      file: "private-result-manifest.json",
      category: "metric",
      detail: "metric numerator must not exceed denominator",
      mutate: (document) => {
        const firstMetric = array(document["metricCells"], "metric cells")[0]!;
        firstMetric["numerator"] = 13;
      },
    },
    {
      file: "public-attestation.json",
      category: "disclosure",
      detail: "reported metrics must meet the threshold and match private counts",
      mutate: (document) => {
        const metric = array(document["metrics"], "public metrics")
          .find((candidate) => candidate["id"] === "failure-precision");
        assert.ok(metric);
        metric["publication"] = "reported";
        metric["numerator"] = 2;
        metric["denominator"] = 2;
        delete metric["suppressionReason"];
        delete metric["privateCellDigest"];
      },
    },
    {
      file: "public-attestation.json",
      category: "leakage",
      detail: "public attestation contains a prohibited fixture identity, path, or URL",
      mutate: (document) => {
        document["nonClaims"] = [
          "Private source: https://private.invalid",
          "Fictional identities do not establish independent human review or evaluation.",
          "No representative accuracy, WCAG conformance, or blocking maturity is claimed.",
        ];
      },
    },
  ];

  for (const mutation of mutations) {
    const directory = await temporaryConformance();
    try {
      const path = join(directory, mutation.file);
      const document = await json(path);
      mutation.mutate(document);
      await writeJson(path, document);
      await assertFailure(directory, mutation.category, mutation.detail);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  }
});

test("public holdout checker rejects admission drift and one-byte-over manifest input", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sightlint-holdout-status-"));
  const conformanceDirectory = await temporaryConformance();
  try {
    const driftedRecord = await json(currentRecord);
    object(driftedRecord["admission"], "record admission")["recordDigest"] = `sha256:${"0".repeat(64)}`;
    const driftedPath = join(directory, "drifted.json");
    await writeJson(driftedPath, driftedRecord);

    const drift = await run(["--record", driftedPath, "--admission", admission]);
    const repeatedDrift = await run(["--record", driftedPath, "--admission", admission]);
    assert.deepEqual(repeatedDrift, drift);
    assert.equal(drift.code, 2);
    assert.equal(drift.stdout.byteLength, 0);
    assert.equal(
      drift.stderr.toString("utf8"),
      "web-holdout-foundation: binding: current holdout record does not match holdout admission\n",
    );

    const oversizedPath = join(directory, "oversized.json");
    await writeFile(oversizedPath, Buffer.alloc(1_048_577, 32));
    const oversized = await run(["--record", oversizedPath, "--admission", admission]);
    assert.equal(oversized.code, 2);
    assert.equal(oversized.stdout.byteLength, 0);
    assert.equal(
      oversized.stderr.toString("utf8"),
      "web-holdout-foundation: input-budget: current holdout record exceeds the 1048576-byte manifest limit\n",
    );

    await writeFile(join(conformanceDirectory, "payloads/request-001.json"), Buffer.alloc(1_048_577, 32));
    const oversizedPayload = await run(["--conformance-dir", conformanceDirectory]);
    assert.equal(oversizedPayload.code, 2);
    assert.equal(oversizedPayload.stdout.byteLength, 0);
    assert.equal(
      oversizedPayload.stderr.toString("utf8"),
      "web-holdout-foundation: input-budget: bundle file exceeds the 1048576-byte file limit\n",
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
    await rm(conformanceDirectory, { recursive: true, force: true });
  }
});
