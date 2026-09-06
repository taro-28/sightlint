# ADR 0050 — Deterministic GitHub Actions report projection

- Status: Accepted
- Date: 2026-09-06
- Issue: #67
- Parent: #31
- Roadmap: M7
- Owners: @taro-28

## Context

SightLint has a local, deterministic Rust kernel, canonical `CheckReport`, a one-command Web
agent workflow, source/evidence navigation, and a verified source release. Issue #31 still lacks a
GitHub Check or MCP integration backed by that same kernel. A useful pull-request surface must not
reimplement rules, turn an advisory or ambiguous result into a blocking failure, invent source
causality, or upload screenshots and source merely because CI is enabled.

GitHub Actions jobs already appear as check runs. GitHub's workflow-command contract lets a local
process create `error`, `warning`, and `notice` annotations by writing escaped commands to standard
output, while `GITHUB_STEP_SUMMARY` is an explicit per-step environment file for a Markdown job
summary. Creating a separate check run through the REST Checks API instead requires GitHub App
write credentials, and the REST API accepts at most 50 annotations per request. The first bounded
slice does not need a hosted app or token-bearing network client.

The existing Web workflow report exposes a DOM selector and source-bundle list as navigation
hints. ADR 0036 explicitly says those fields are not exact source-line attribution. Treating a
bundle path as the cause of a finding, or defaulting to line one, would upgrade a useful hint into a
false exact fact.

Official platform references reviewed for this decision:

- <https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands>
- <https://docs.github.com/en/actions/reference/workflows-and-actions/variables>
- <https://docs.github.com/en/rest/checks/runs>

## Decision

### Separate projection crate and one public command

Add `sightlint-github-actions` outside the trusted engine. It consumes an already-produced
`CheckReport` and cannot acquire artifacts, run rules, alter outcomes, or access the network. The
public `sightlint github-check` command loads Artifact IR through the existing validation path,
calls the same `check_with_options` kernel entry point as `sightlint check`, and then invokes the
projection.

The command supports canonical JSON for agents and a `github-actions` representation for a GitHub
runner. Both are views of one independently versioned projection report. The command preserves the
existing exit contract:

- `0` when the underlying report has no blocking failure and `cantTell` was not explicitly denied;
- `1` for a blocking failed result or caller-selected `--deny-cant-tell` policy;
- `2` for usage, I/O, Artifact IR, source-map, source-location, projection, or summary-write error.

An integration error is never represented as a rule failure. All inputs and exact-source bindings
are validated before an annotation command is written, so rejected input cannot leave a partial
annotation stream.

### Independently versioned contracts

`GithubSourceMap` and `GithubActionsReport` each start at `0.1.0`. Their schemas are generated from
strict Rust types, committed for review, and checked for drift. They are independent of Artifact IR
`0.1.0`, CheckReport `0.3.0`, rule versions, and the package version.

The projection report contains the complete authoritative CheckReport. For every failed,
`cantTell`, or `untested` result, it also records a stable finding identity/key, unchanged outcome
and enforcement, evidence identifiers/classes, and one annotation disposition: emitted, exact
source unavailable, or omitted by the bounded annotation cap. Passed and inapplicable results
remain distinct in the embedded CheckReport and summary; they do not create annotations.

The source map identifies a report by artifact ID and joins entries by rule ID, rule version,
target kind, target ID, and optional aspect matched exactly. It deliberately contains no expected outcome
or enforcement. Source-location truth therefore cannot become rule-verdict truth. A declared
location contains:

- a repository-relative path with no traversal or cross-repository resolution;
- an inclusive one-based line range;
- one exact anchor line and UTF-8 anchor text inside that range;
- `declaredExactSourceLine` attribution;
- document-level declaration, implementation-oracle, and external-processing provenance.

Before projection, the command resolves the repository root and source path, rejects symlink escape
or a non-file target, bounds the declaration, validates the line range, and compares the exact
anchor. The stable anchor detects shifted/stale locations without requiring the edited defect text
itself to remain unchanged. Empty, duplicate, unsorted, dangling, mismatched-artifact, unsupported,
and unknown-field declarations fail closed. A result without an exact entry remains summary-only
with `sourceMapNotProvided` or `sourceLocationNotDeclared`; the projector never guesses.

The command accepts at most 1 MiB of source-map JSON and 512 entries. Each range spans at most 200
lines, each anchor is at most 4 KiB, each unique UTF-8 source file is at most 16 MiB, and no more
than 64 MiB of unique source text is inspected. Repeated locations share one validated in-memory
source copy rather than multiplying I/O by finding count.

### Outcome mapping, ordering, and limits

Annotation level is a deterministic projection of two existing fields, not a new verdict:

| Kernel outcome | Enforcement | Annotation | Default process effect |
|---|---|---|---|
| `failed` | `blocking` | `error` | exit 1 |
| `failed` | `advisory` | `warning` | exit 0 |
| `cantTell` | either | `notice` | exit 0 unless explicitly denied |
| `untested` | either | `notice` | exit 0 |
| `passed` | either | none | exit 0 |
| `inapplicable` | either | none | exit 0 |

Annotation priority is `error`, then `warning`, then `notice`; equal-priority entries sort by a
percent-encoded stable finding key. Identical result/source inputs therefore produce byte-identical
canonical JSON and workflow commands. The integration emits at most 50 annotations per step, keeps
the complete CheckReport, and records every omitted disposition and count. It does not hide excess
findings or replace them with a score.

Workflow-command data escapes `%`, carriage return, and newline; property values additionally
escape `:` and `,`, matching the GitHub runner contract. This prevents a report identifier,
message, title, or path from injecting another command. The first slice uses whole-line locations
only and does not invent columns.

The command never discovers or writes `GITHUB_STEP_SUMMARY` implicitly. The caller must select an
explicit flag, the GitHub runner must supply the path, and the command refuses to exceed the
platform's 1 MiB per-step summary limit. The path must already be a regular non-symlink file and
the integration does not create it. The summary lists outcome, enforcement, rule/policy,
evidence, source disposition, and gate exit independently. Screenshot, overlay, raw artifact,
source excerpt, token, and credential data are not written.

### Evaluation and GitHub execution

The versioned product corpus uses two repository-owned Atlas fixture families. Its main clean,
targeted spacing mutation, and intentional-grouping hard negative come from the realistic dashboard
corpus. Existing interaction projections add an advisory missing-feedback mutation, a valid
save-draft alternative, native/declaration conflict (`cantTell`), and unexecuted trace (`untested`).

Rule truth remains in the existing Web and interaction rule-oracle documents. New source maps and
projection annotations are independently authored from fixture source and this contract; neither
copies current implementation output. The E2E joins those authorities and reports integer
numerator/denominator evidence for execution coverage, failure precision, exact-source annotation
coverage, abstention preservation, summary-only abstention coverage, false-positive failures,
mutation kill rate, and hard-negative failures. It produces no aggregate score.

All fixtures are fictional, repository-owned, licensed `MIT OR Apache-2.0`, and contain no customer
or personal data, secrets, external assets, or external processing. Labels and source locations are
public smoke/development data visible to implementers. They are not a protected holdout,
independent review, representative sample, GitHub usability study, or real-world UI/UX accuracy
estimate. Oracle changes require semantic review and cannot be made merely to match output.

A read-only CI step executes the clean `github-actions` representation and explicitly writes its
job summary. The runner transports the output into its existing job check. The workflow receives no
write token, uploads no artifact, and never writes, formats, commits, or pushes repository source.

## Consequences

- Coding agents and CI can consume a strict JSON projection or native GitHub job annotations from
  the same verified kernel.
- Exact source annotations are possible where a repository supplies independently reviewable
  source truth; all other results remain visible without fabricated attribution.
- Advisory failures and abstentions stay nonblocking by default, while coverage remains visible.
- A GitHub App, hosted service, token, telemetry channel, and second rule engine are unnecessary.
- Source maps require maintenance when their anchor line moves; stale declarations fail loudly.
- Workflow-command annotations use the runner's existing job check and do not expose an independent
  REST check-run conclusion or cross-run API lifecycle.

## Alternatives considered

### Create check runs through the REST API

Deferred. Write access requires a GitHub App installation token and introduces credential,
permission, fork, webhook, network, retry, and hosted-lifecycle scope. The local job-check surface
provides the required first integration without those risks. A later REST publisher may consume the
same canonical projection behind a separate opt-in trust and privacy decision.

### Emit SARIF

Rejected for this slice. SARIF would target code scanning rather than the requested check surface,
and source causality would still need an exact independent mapping.

### Annotate every result at a bundle file or line one

Rejected because a DOM selector/source-bundle join is not exact source causality and would produce
misleading annotations.

### Reimplement report interpretation in a JavaScript action

Rejected because another implementation could collapse outcomes or drift from enforcement and
policy semantics. The Rust projection is the single implementation used by JSON, workflow commands,
and E2E.

### Upload screenshots automatically

Rejected because it silently expands retention and privacy scope. This slice carries evidence IDs
and explicit unavailability but no artifact upload.

## Non-goals

- GitHub App installation, REST publishing, webhook handling, or a hosted service;
- MCP, editor/browser extension, local GUI, project-directory discovery, or package-channel work;
- screenshot/overlay upload, history storage, telemetry, or artifact retention;
- new rules, rule promotion, automatic remediation, source-map inference, or exact DOM-to-source
  causality;
- representative UX accuracy, WCAG conformance, autonomous-agent success, or a universal score.

## Verification

- Generate both public schemas from Rust and fail CI on drift.
- Validate source-map versions, provenance, ordering, duplicates, containment, anchors, limits, and
  mismatched/dangling identities before output.
- Exercise JSON and GitHub Actions representations through the built public binary with file and
  standard input.
- Prove blocking/advisory/`cantTell`/inapplicable/`untested` mapping and strict `cantTell` policy.
- Test exact source navigation, summary-only fallback, 50-annotation priority/cap, command escaping,
  1 MiB summary boundary, malformed inputs, stable diagnostics, and byte-identical reruns.
- Join independent rule, source-location, and projection oracles for clean/mutation/hard-negative
  and abstention cases, including the clean rerun after each targeted mutation.
- Preserve all existing generator, Rust/CLI/PNG/image/Web/product/adapter, documentation, MSRV, and
  Linux/macOS/Windows gates, then verify exact-head and post-merge `main` CI.
