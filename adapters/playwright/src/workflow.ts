import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { capture } from "./capture.js";
import {
  AdapterError,
  WEB_EXTENSION_KEY,
  type CaptureRequest,
  type CaptureResponse,
  type JsonObject,
  type JsonValue,
} from "./types.js";

export const WORKFLOW_REPORT_VERSION = "0.1.0";
export const WORKFLOW_COMMAND = "sightlint-web-check";
const CHECK_REPORT_VERSION = "0.3.0";
const MAX_CHILD_OUTPUT_BYTES = 16 * 1024 * 1024;

const WORKFLOW_LIMITATIONS = [
  "Source targets join exact captured node identifiers to native locators and source-bundle files; they do not prove an exact source-code line or cause.",
  "Pixel-content identity, complete hit regions, and unresolved native/render conflicts keep their existing cantTell or untested status.",
  "Advisory findings do not become blocking, and this public fixture workflow is not representative real-world UI/UX or agent-accuracy evidence.",
] as const;

interface KernelRun {
  code: number;
  stdout: Buffer;
  stderr: Buffer;
}

interface Locator {
  type: "testId" | "id" | "css";
  value: string;
  selector: string;
}

export interface SourceTarget {
  nodeId: string;
  locator: Locator;
  sourceFiles: string[];
  evidenceIds: string[];
}

export interface WorkflowReport {
  schemaVersion: string;
  workflow: {
    command: string;
    version: string;
    profile: "sightlint:recommended";
    verdictOwner: "sightlint-rust-kernel";
    externalProcessing: false;
  };
  capture: CaptureResponse;
  sourceTargets: SourceTarget[];
  checkReport: JsonObject;
  limitations: string[];
}

export interface WorkflowExecution {
  report: WorkflowReport;
  exitCode: 0 | 1;
}

function object(value: JsonValue | undefined, context: string): JsonObject {
  if (value === null || Array.isArray(value) || typeof value !== "object") {
    throw new AdapterError("invalid-kernel-report", `${context} must be an object`);
  }
  return value;
}

function string(value: JsonValue | undefined, context: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new AdapterError("invalid-kernel-report", `${context} must be a non-empty string`);
  }
  return value;
}

function array(value: JsonValue | undefined, context: string): JsonValue[] {
  if (!Array.isArray(value)) {
    throw new AdapterError("invalid-kernel-report", `${context} must be an array`);
  }
  return value;
}

function oneOf(value: JsonValue | undefined, expected: readonly string[], context: string): string {
  const parsed = string(value, context);
  if (!expected.includes(parsed)) {
    throw new AdapterError("invalid-kernel-report", `${context} is unsupported`);
  }
  return parsed;
}

function count(value: JsonValue | undefined, context: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new AdapterError("invalid-kernel-report", `${context} must be a non-negative integer`);
  }
  return value;
}

function exactFields(value: JsonObject, allowed: readonly string[], required: readonly string[], context: string): void {
  const unexpected = Object.keys(value).filter((field) => !allowed.includes(field));
  if (unexpected.length > 0) {
    throw new AdapterError("invalid-kernel-report", `${context} contains unsupported fields`);
  }
  if (required.some((field) => !(field in value))) {
    throw new AdapterError("invalid-kernel-report", `${context} is missing required fields`);
  }
}

function parseJson(bytes: Buffer, code: string, message: string): JsonObject {
  let decoded: JsonValue;
  try {
    decoded = JSON.parse(bytes.toString("utf8")) as JsonValue;
  } catch {
    throw new AdapterError(code, message);
  }
  if (decoded === null || Array.isArray(decoded) || typeof decoded !== "object") {
    throw new AdapterError(code, message);
  }
  return decoded;
}

function validateCheckReport(report: JsonObject): void {
  exactFields(
    report,
    ["reportSchemaVersion", "engineVersion", "artifactId", "artifactKind", "extensionVersions", "profiles", "summary", "results"],
    ["reportSchemaVersion", "engineVersion", "artifactId", "artifactKind", "profiles", "summary", "results"],
    "check report",
  );
  if (report.reportSchemaVersion !== CHECK_REPORT_VERSION) {
    throw new AdapterError("unsupported-kernel-report", `check report version must be ${CHECK_REPORT_VERSION}`);
  }
  string(report.engineVersion, "check report engineVersion");
  string(report.artifactId, "check report artifactId");
  string(report.artifactKind, "check report artifactKind");
  const profiles = array(report.profiles, "check report profiles");
  for (const profile of profiles) string(profile, "check report profile");
  if (!profiles.includes("sightlint:recommended")) {
    throw new AdapterError("invalid-kernel-report", "check report must include sightlint:recommended");
  }
  const summary = object(report.summary, "check report summary");
  exactFields(
    summary,
    ["passed", "failed", "inapplicable", "cantTell", "untested"],
    ["passed", "failed", "inapplicable", "cantTell", "untested"],
    "check report summary",
  );
  const observedCounts = new Map([
    ["passed", 0],
    ["failed", 0],
    ["inapplicable", 0],
    ["cantTell", 0],
    ["untested", 0],
  ]);
  for (const outcome of observedCounts.keys()) count(summary[outcome], `check report summary ${outcome}`);
  for (const [index, value] of array(report.results, "check report results").entries()) {
    const result = object(value, `check report result ${index}`);
    exactFields(
      result,
      ["ruleId", "ruleVersion", "title", "kind", "maturity", "policy", "enforcement", "target", "outcome", "message", "evidenceIds", "evidenceClasses", "relatedNodeIds", "measurements"],
      ["ruleId", "ruleVersion", "title", "kind", "maturity", "policy", "enforcement", "target", "outcome", "message"],
      `check report result ${index}`,
    );
    string(result.ruleId, `check report result ${index} ruleId`);
    string(result.ruleVersion, `check report result ${index} ruleVersion`);
    string(result.title, `check report result ${index} title`);
    oneOf(result.kind, ["atomic", "composite"], `check report result ${index} kind`);
    oneOf(result.maturity, ["experimental", "advisory", "blockingEligible"], `check report result ${index} maturity`);
    string(result.message, `check report result ${index} message`);
    const outcome = oneOf(
      result.outcome,
      ["passed", "failed", "inapplicable", "cantTell", "untested"],
      `check report result ${index} outcome`,
    );
    observedCounts.set(outcome, (observedCounts.get(outcome) ?? 0) + 1);
    oneOf(result.enforcement, ["advisory", "blocking"], `check report result ${index} enforcement`);
    const policy = object(result.policy, `check report result ${index} policy`);
    exactFields(
      policy,
      ["profile", "sourceKind", "sourceId", "sourceVersion", "reference"],
      ["profile", "sourceKind", "sourceId", "sourceVersion", "reference"],
      `check report result ${index} policy`,
    );
    string(policy.profile, `check report result ${index} policy profile`);
    oneOf(
      policy.sourceKind,
      ["declaredContract", "platformStandard", "conservativeBuiltIn"],
      `check report result ${index} policy sourceKind`,
    );
    string(policy.sourceId, `check report result ${index} policy sourceId`);
    string(policy.sourceVersion, `check report result ${index} policy sourceVersion`);
    string(policy.reference, `check report result ${index} policy reference`);
    const target = object(result.target, `check report result ${index} target`);
    exactFields(target, ["kind", "id", "aspect"], ["kind", "id"], `check report result ${index} target`);
    oneOf(target.kind, ["artifact", "canvas", "node", "relation"], `check report result ${index} target kind`);
    string(target.id, `check report result ${index} target id`);
    if (target.aspect !== undefined) string(target.aspect, `check report result ${index} target aspect`);
    if (result.evidenceIds !== undefined) {
      for (const evidenceId of array(result.evidenceIds, `check report result ${index} evidenceIds`)) {
        string(evidenceId, `check report result ${index} evidenceId`);
      }
    }
  }
  for (const [outcome, observed] of observedCounts) {
    if (summary[outcome] !== observed) {
      throw new AdapterError("invalid-kernel-report", "check report summary does not match its results");
    }
  }
}

function parseArtifactIr(bytes: Buffer): JsonObject {
  return parseJson(bytes, "invalid-capture-output", "captured Artifact IR is not valid JSON");
}

function sourceTargets(ir: JsonObject, report: JsonObject): SourceTarget[] {
  const extensions = object(ir.extensions, "Artifact IR extensions");
  const web = object(extensions[WEB_EXTENSION_KEY], "Artifact IR Web extension");
  const captureRecord = object(web.capture, "Web extension capture");
  const sourceFiles = array(captureRecord.sourceFiles, "Web extension sourceFiles")
    .map((value) => string(value, "Web extension source file"))
    .sort();
  const nodes = new Map<string, JsonObject>();
  for (const value of array(web.nodes, "Web extension nodes")) {
    const node = object(value, "Web extension node");
    const nodeId = string(node.nodeId, "Web extension nodeId");
    if (nodes.has(nodeId)) {
      throw new AdapterError("invalid-capture-output", "Web extension contains duplicate node identifiers");
    }
    nodes.set(nodeId, node);
  }

  const evidenceByTarget = new Map<string, Set<string>>();
  for (const value of array(report.results, "check report results")) {
    const result = object(value, "check report result");
    const target = object(result.target, "check report target");
    if (target.kind !== "node") continue;
    const targetId = string(target.id, "check report target id");
    const evidence = evidenceByTarget.get(targetId) ?? new Set<string>();
    if (result.evidenceIds !== undefined) {
      for (const evidenceId of array(result.evidenceIds, "node result evidenceIds")) {
        evidence.add(string(evidenceId, "node result evidenceId"));
      }
    }
    evidenceByTarget.set(targetId, evidence);
  }

  return [...evidenceByTarget.entries()]
    .map(([nodeId, evidenceIds]) => {
      const node = nodes.get(nodeId);
      if (node === undefined) {
        throw new AdapterError("source-target-missing", "a node result has no captured native locator");
      }
      const locatorValue = object(node.locator, "Web extension node locator");
      const type = string(locatorValue.type, "Web extension locator type");
      if (type !== "testId" && type !== "id" && type !== "css") {
        throw new AdapterError("invalid-capture-output", "Web extension locator type is unsupported");
      }
      const locatorType: Locator["type"] = type;
      evidenceIds.add(string(node.domEvidenceId, "Web extension DOM evidenceId"));
      evidenceIds.add(string(node.renderEvidenceId, "Web extension render evidenceId"));
      if (node.accessibilityEvidenceId !== null && node.accessibilityEvidenceId !== undefined) {
        evidenceIds.add(string(node.accessibilityEvidenceId, "Web extension accessibility evidenceId"));
      }
      if (evidenceIds.size === 0) {
        throw new AdapterError("source-target-missing", "a node result has no evidence identifiers");
      }
      return {
        nodeId,
        locator: {
          type: locatorType,
          value: string(locatorValue.value, "Web extension locator value"),
          selector: string(locatorValue.selector, "Web extension locator selector"),
        },
        sourceFiles: [...sourceFiles],
        evidenceIds: [...evidenceIds].sort(),
      };
    })
    .sort((left, right) => left.nodeId < right.nodeId ? -1 : left.nodeId > right.nodeId ? 1 : 0);
}

function runKernel(program: string, repositoryRoot: string, artifactIrPath: string): Promise<KernelRun> {
  return new Promise((resolveRun, reject) => {
    const child = spawn(
      program,
      ["check", artifactIrPath, "--profile", "recommended", "--format", "json"],
      { cwd: repositoryRoot, env: process.env, stdio: ["ignore", "pipe", "pipe"] },
    );
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let byteLength = 0;
    let exceeded = false;
    const collect = (destination: Buffer[], chunk: Buffer): void => {
      byteLength += chunk.byteLength;
      if (byteLength > MAX_CHILD_OUTPUT_BYTES) {
        exceeded = true;
        child.kill();
        return;
      }
      destination.push(chunk);
    };
    child.stdout.on("data", (chunk: Buffer) => collect(stdout, chunk));
    child.stderr.on("data", (chunk: Buffer) => collect(stderr, chunk));
    child.on("error", () => reject(new AdapterError("kernel-spawn", "failed to start the sightlint binary")));
    child.on("close", (code, signal) => {
      if (exceeded) {
        reject(new AdapterError("kernel-output-too-large", "sightlint output exceeded 16777216 bytes"));
        return;
      }
      if (signal !== null || code === null) {
        reject(new AdapterError("kernel-execution", "sightlint did not exit normally"));
        return;
      }
      resolveRun({ code, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr) });
    });
  });
}

export async function runWebCheck(
  request: CaptureRequest,
  repositoryRoot: string,
  sightlintBinary: string,
): Promise<WorkflowExecution> {
  const temporaryDirectory = await mkdtemp(join(tmpdir(), "sightlint-web-check-")).catch(() => {
    throw new AdapterError("temporary-directory", "failed to create private capture storage");
  });
  const artifactIrPath = join(temporaryDirectory, "artifact-ir.json");
  const screenshotPath = join(temporaryDirectory, "screenshot.png");
  try {
    const captureResponse = await capture(request, repositoryRoot, { artifactIrPath, screenshotPath });
    const artifactIrBytes = await readFile(artifactIrPath).catch(() => {
      throw new AdapterError("invalid-capture-output", "failed to read captured Artifact IR");
    });
    const artifactIr = parseArtifactIr(artifactIrBytes);
    const kernel = await runKernel(sightlintBinary, repositoryRoot, artifactIrPath);
    if (kernel.code !== 0 && kernel.code !== 1) {
      throw new AdapterError("kernel-execution", "sightlint check failed before producing a report");
    }
    if (kernel.stderr.byteLength !== 0) {
      throw new AdapterError("kernel-execution", "sightlint emitted an unexpected diagnostic with a report");
    }
    const checkReport = parseJson(
      kernel.stdout,
      "invalid-kernel-report",
      "sightlint did not emit a valid JSON report",
    );
    validateCheckReport(checkReport);
    if (checkReport.artifactId !== request.artifact.id || checkReport.artifactKind !== "web") {
      throw new AdapterError("invalid-kernel-report", "check report does not identify the captured Web artifact");
    }
    const report: WorkflowReport = {
      schemaVersion: WORKFLOW_REPORT_VERSION,
      workflow: {
        command: WORKFLOW_COMMAND,
        version: WORKFLOW_REPORT_VERSION,
        profile: "sightlint:recommended",
        verdictOwner: "sightlint-rust-kernel",
        externalProcessing: false,
      },
      capture: captureResponse,
      sourceTargets: sourceTargets(artifactIr, checkReport),
      checkReport,
      limitations: [...WORKFLOW_LIMITATIONS],
    };
    return { report, exitCode: kernel.code };
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

function outcomeLabel(value: JsonValue | undefined): string {
  switch (value) {
    case "passed": return "PASS";
    case "failed": return "FAIL";
    case "inapplicable": return "INAPPLICABLE";
    case "cantTell": return "CANT_TELL";
    case "untested": return "UNTESTED";
    default: throw new AdapterError("invalid-kernel-report", "check report contains an unsupported outcome");
  }
}

export function humanWorkflowReport(report: WorkflowReport): string {
  const check = report.checkReport;
  const summary = object(check.summary, "check report summary");
  const targets = new Map(report.sourceTargets.map((target) => [target.nodeId, target]));
  const total = ["passed", "failed", "cantTell", "inapplicable", "untested"]
    .map((key) => summary[key])
    .reduce<number>((sum, value) => sum + (typeof value === "number" ? value : 0), 0);
  const lines = [
    `SightLint Web check ${report.workflow.version} — artifact ${String(check.artifactId)}`,
    `${total} result(s): ${String(summary.passed)} passed, ${String(summary.failed)} failed, ${String(summary.cantTell)} cantTell, ${String(summary.inapplicable)} inapplicable, ${String(summary.untested)} untested`,
    `profiles: ${array(check.profiles, "check report profiles").join(", ")}`,
    `source digest: ${report.capture.sourceDigest}`,
  ];
  for (const value of array(check.results, "check report results")) {
    const result = object(value, "check report result");
    const target = object(result.target, "check report target");
    const targetKind = string(target.kind, "check report target kind");
    const targetId = string(target.id, "check report target id");
    lines.push(
      "",
      `${outcomeLabel(result.outcome)} ${string(result.ruleId, "check report ruleId")} (${string(result.enforcement, "check report enforcement")}) [${targetKind}:${targetId}]`,
      `  ${string(result.message, "check report message")}`,
    );
    const policy = object(result.policy, "check report policy");
    lines.push(`  policy: ${String(policy.sourceId)}@${String(policy.sourceVersion)} (${String(policy.sourceKind)})`);
    const sourceTarget = targets.get(targetId);
    if (sourceTarget !== undefined) {
      lines.push(`  source selector: ${sourceTarget.locator.selector}`);
      lines.push(`  source bundle: ${sourceTarget.sourceFiles.join(", ")}`);
    }
    if (result.evidenceIds !== undefined) {
      lines.push(`  evidence: ${array(result.evidenceIds, "check report evidenceIds").join(", ")}`);
    }
  }
  lines.push("", "limitations:", ...report.limitations.map((limitation) => `  - ${limitation}`));
  return `${lines.join("\n")}\n`;
}
