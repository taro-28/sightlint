# ADR 0036 — Local Web check orchestration and agent report

- Status: Accepted
- Date: 2026-09-06
- Issue: #42
- Parent: #34
- Owners: @taro-28

## Context

ADRs 0032–0035 establish a reviewed Web evaluation contract, an isolated Playwright acquisition
process, and three advisory rules in the deterministic Rust kernel. A user must still run the
capture process, retain two output paths, and invoke `sightlint check` separately. The resulting
CheckReport identifies a node and its evidence, while the native selector needed to locate that
node remains in the captured Web extension.

Issue #34 requires a coding agent to run one local command, find a reviewed source target, apply a
focused fix, and rerun the same check. This must not move policy into Node, bless implementation
output as an oracle, or imply that an automated replay proves autonomous agent quality.

## Decision

### One-command orchestration

Add a `sightlint-web-check` command to the untrusted Playwright package. It accepts the existing
versioned capture request, a canonical repository root, the built public `sightlint` binary, and a
`json` or `human` output format. The command:

1. creates private temporary capture paths;
2. runs the existing capture implementation without changing its protocol;
3. invokes `sightlint check <captured-ir> --profile recommended --format json` as a child process;
4. verifies that the child emitted a supported CheckReport;
5. joins node result identifiers to already-captured native locators and source-bundle paths;
6. emits either a versioned canonical workflow report or a stable color-free human rendering;
7. removes its temporary artifacts in success and error paths.

The outer process preserves exit code 0 or 1 from the Rust check. Capture, spawn, malformed report,
unsupported report version, and other operational failures exit 2 with empty stdout and one stable
diagnostic. Node never computes, changes, suppresses, or upgrades a rule outcome.

### Agent workflow report

The canonical JSON envelope starts at `0.1.0` and is independently versioned from the capture
protocol, Web extension, Artifact IR, and CheckReport. It contains:

- the workflow implementation identity and the fact that the Rust kernel owns verdicts;
- the complete canonical capture response, including input/source/screenshot digests, runtime
  environment, resource observations, privacy boundary, and capture limitations;
- the complete CheckReport returned by the Rust binary;
- a sorted, deduplicated source-target join for node results, containing the captured native
  locator, repository-relative source-bundle paths, and evidence identifiers;
- explicit limitations and non-claims.

Only exact joins on the stable node identifier are emitted. A source target is a navigation hint:
the selector identifies the captured DOM node, while the source-bundle list identifies files that
participated in the page. It is not an exact source-code line, ownership claim, or proof that one
particular declaration caused the finding. Missing node mapping is an operational contract error,
not a guessed selector.

Temporary absolute paths and random directory names are not serialized. Identical request bytes,
source bytes, compatible environment, engine, and versions must therefore produce byte-identical
canonical output. Cross-platform screenshot or report byte identity remains unclaimed.

### Human output

The human format renders the report summary and every rule result in stable rule/target order. For
node targets it adds the joined native selector and source-bundle files. It labels outcome,
enforcement, policy, evidence, and message separately. Human output is not a second verdict format;
it is a projection of the same validated workflow envelope.

### Evaluation and reviewed edit

Add an independently versioned agent-workflow annotation document. It is authored from fixture and
rule contracts, not generated from the command output. It records:

- source ownership, pending license, privacy, external-asset/processing, split, and holdout status;
- the reviewed initial rule/outcome/target/enforcement and native locator;
- the exact repository-relative source file and a bounded before/after edit approved by review;
- the named postcondition that the original finding is absent;
- the separate postcondition that no new failed result appears;
- an ambiguous control case that must preserve `cantTell` rather than becoming pass or fail;
- false-positive, leakage, and non-claim statements.

The E2E copies the Atlas fixture and requests to a temporary repository, runs the one-command JSON
surface twice, applies only the reviewed edit to that copy, and reruns twice. It compares canonical
bytes within each state, checks the initial and fixed postconditions, and exercises the human
format. Captures and reports stay temporary. The test consumes the human-authored oracle and never
writes implementation results back to it.

This is a deterministic product-path contract. It demonstrates that Codex can be given sufficient
navigational evidence and that the named fix can be verified. It does not evaluate source-edit
selection, language-model reasoning, arbitrary repositories, or representative UI/UX accuracy.

## Trust, privacy, and resources

The workflow remains local and inherits the capture protocol's repository containment, offline
browser, one-frame, selected-node, redaction, and resource limits. The Rust binary receives only
the temporary Artifact IR path. No artifact content is uploaded. Child stdout/stderr are bounded
before interpretation; the workflow rejects non-JSON, unsupported, or internally inconsistent
reports rather than partially accepting them.

The fixture is fictional and repository-owned, has no external assets or personal data, and has no
independent redistribution grant until issue #33 resolves the project license. All annotations are
public smoke/development data visible to implementers, not a private holdout. A future claim about
agent generalization requires independently sampled tasks and leakage-controlled evaluation.

## Compatibility

`sightlint-web-check` and workflow report `0.1.0` are new alpha surfaces. Capture protocol `0.1.0`,
`org.sightlint.web@0.3.0`, Artifact IR `0.1.0`, CheckReport `0.3.0`, existing `sightlint-web`, and
Rust CLI behavior remain unchanged. The report schema rejects unknown fields and unsupported
versions. Any incompatible envelope or exit-semantics change requires a new version and retained
compatibility fixture or an explicit alpha migration decision.

## Alternatives considered

### Add browser launch to the Rust CLI

Rejected because it would couple an untrusted browser runtime to the trusted kernel and blur the
adapter boundary.

### Reimplement the three rules in Node

Rejected because two policy engines could disagree and the adapter would begin issuing verdicts.

### Emit only the existing CheckReport

Rejected because its evidence identifiers do not carry the native locator/source-bundle join an
agent needs for navigation. Changing the medium-neutral CheckReport solely for Web orchestration
would also introduce a medium-specific concern into a shared contract.

### Persist captures in a fixed repository directory by default

Rejected because reruns would require implicit overwrites and would leave screenshots or semantic
data behind. A future explicit diagnostic-export option may use caller-selected paths and a
separate overwrite policy.

### Have the product edit source automatically

Rejected for this slice. Source modification is performed by the coding agent in a temporary E2E
copy and governed by a reviewed edit oracle, not by SightLint.

## Non-goals

- arbitrary hosted URLs, arbitrary application frameworks, or interaction traces;
- model-based source selection or automated editing inside SightLint;
- exact source-line attribution from a DOM locator;
- promoting advisory Web rules to blocking;
- packaging, installation, license selection, or release (#33);
- MCP, GitHub Checks, or editor integrations (#31);
- representative agent success, real-world UI/UX accuracy, WCAG conformance, or a universal score.

## Verification

- Strictly validate the versioned workflow and annotation schemas, including rejection cases.
- Run the actual one-command Node process and built public Rust binary.
- Compare canonical JSON, human output, stderr, and exit codes across repeated identical runs.
- Verify the reviewed initial finding, exact node/locator/source-bundle join, reviewed source edit,
  disappearance of only the named finding, and absence of new failures.
- Verify the ambiguous-control `cantTell` case and existing hard-negative coverage remain
  conservative.
- Preserve generator drift, all Rust/CLI/PNG/image/product/Web corpora, rustdoc, Rust 1.85.0, and
  Linux/macOS/Windows gates.
