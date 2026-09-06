# ADR 0047 — Add medium-neutral deterministic interaction contracts and controlled traces

- Status: Accepted
- Date: 2026-09-06
- Issue: #30
- Builds on: ADRs 0002–0006, 0010–0013, 0018–0019, 0024 (product evaluation),
  0032–0036, and 0042

## Context

Static structure and pixels cannot prove what happens after an action. Pending feedback, effect
completion, failure, and recovery are temporal obligations. Browser timing, network requests,
application instrumentation, accessibility state, DOM state, and screenshots also have different
authority. Collapsing them into one inferred verdict would violate the deterministic-kernel and
untrusted-adapter boundaries.

Issue #30 needs a first vertical slice that can represent and evaluate these obligations without
claiming broad interaction coverage. The slice must preserve all five outcomes, accept multiple
valid recovery designs, and avoid wall-clock-dependent results.

## Decision

Add the optional official `org.sightlint.interaction@0.1.0` extension. It is medium-neutral and
contains:

- stable actions linked to core target nodes;
- caller- or application-declared latency and recovery contracts backed by
  `declaredContract` evidence;
- exactly one captured or explicitly `untested` trace per action, with viewport/unit, locale,
  timezone, color-scheme, reduced-motion, network, clock, and external-processing facts;
- one-based canonical event order, stable attempt IDs, and optional causal event links;
- visible pending, optimistic, success, and failure states;
- success/failure effect resolution and retry/save-draft recovery alternatives;
- explicit agreement or retained conflict evidence.

The extension contains no timestamps. Sequence is assigned from the controlled script, and effect
latency is a declared categorical contract rather than a duration inferred from host scheduling.
Captured events require `interactionTrace` evidence. Rendered screenshots, platform semantics,
and declared instrumentation may accompany that evidence but do not replace it.

The first Playwright interaction protocol remains an untrusted local sensor. It supports only a
repository-contained fixture, a fixed viewport/environment, denied external network, a bounded
controlled step script, and an explicit fixture harness. Each named state acquisition records DOM
state, accessibility state, screenshot bytes/digest, and viewport extent in one ordered adapter
step. The adapter records the acquisition as sequential and non-atomic; disagreement is retained
as conflict instead of selecting one source. Declared effect events establish only what the
instrumented fixture reported. Pixels never establish an invisible effect.

The Rust kernel adds two `sightlint:base` rules:

- `interaction.async-feedback@0.1.0` applies only to a declared observably latent effect and
  requires a pending or optimistic visible-state observation between activation and resolution;
- `interaction.failure-recovery@0.1.0` applies to an executed failure path with a required
  recovery contract and accepts any declared alternative that is offered, activated, resolves
  successfully, and reaches visible success.

Both rules start at advisory maturity and advisory enforcement. Their failures never change the
default exit code. An unexecuted trace is `untested`; an immediate action or unexercised failure
path is `inapplicable`; a retained cross-source conflict is `cantTell`. Missing required evidence
in a completed, agreeing controlled trace is a failure.

The report schema is unchanged. Until a separately evaluated need justifies a report migration,
interaction results target the artifact and identify the stable action in the target aspect as
`interaction.action:<action-id>`.

## Evaluation and governance

Add a repository-owned Atlas account-settings fixture with controlled success, failure, and
recovery states. Acquisition annotations and rule-verdict annotations are separate strict
documents. Neither adapter output nor CheckReport output is an oracle.

The public corpus contains clean traces, targeted mutations, a valid alternative-recovery hard
negative, conflict, inapplicable, and untested cases. Public smoke/development/challenge labels are
for regression organization only. All fixture source, scripts, annotations, and expected outcomes
are visible to implementers, so no protected holdout exists. A passing corpus is not evidence of
representative Web interaction or UI/UX accuracy.

Metrics record acquisition fact coverage, evaluated-case coverage, failure precision,
false-positive rate, abstention retention, and mutation kill rate. Expected output may change only
after an explained human review of fixture intent or the versioned rule contract.

Fixture content is fictional, repository-owned, and distributed under `MIT OR Apache-2.0`. The
adapter performs no external processing. DOM text, screenshots, selectors, and instrumentation can
still be sensitive in real projects; this slice supports only repository-contained fixtures and
does not persist screenshot bytes in the evaluation oracle.

## Resource, security, and compatibility limits

The adapter bounds request size, steps, actions, events, viewport axes, screenshot bytes, output
bytes, and execution timeout. It rejects unknown fields, unsupported versions/steps, duplicate
identifiers, path traversal/symlink escape, external requests, missing harness functions, malformed
state observations, and output overwrite. It uses argument arrays for the public Rust binary.

Artifact IR remains `0.1.0`, CheckReport remains `0.3.0`, and the interaction extension and
Playwright interaction request/response protocols start at `0.1.0`. Incompatible changes require
new surface versions. Existing static fixtures and `inspect-image` remain unchanged.

## Non-goals

This decision does not add general-purpose browser scripting, arbitrary live-site capture,
real-network timing, offline/permission/stale-data coverage, duplicate-submit or destructive-action
rules, mobile interaction adapters, OCR/VLM verdicts, automatic source edits, blocking interaction
policy, a universal UX score, or a representative holdout.

## Alternatives considered

### Infer behavior from screenshots

Rejected. A screenshot can establish rendered pixels at one state, not the action, effect, causal
ordering, or invisible completion.

### Put Playwright events directly in the Rust kernel

Rejected. Browser execution is an untrusted adapter responsibility and would make the kernel
nondeterministic and platform-dependent.

### Use elapsed milliseconds as the rule threshold

Rejected. Host scheduling and wall-clock measurements are unstable. The first rule consumes a
declared observable-latency class and ordered events.

### Require retry as the only recovery

Rejected. Save-draft is a valid reviewed alternative in this slice, and future extension versions
may admit other explicit alternatives without pretending all recovery designs are equivalent.

## Verification

- generated interaction schema and strict request/response/evaluation schemas compile;
- Rust validation rejects malformed versions, references, evidence classes, ordering, causal
  links, duplicates, and invalid captured/untested combinations;
- public-binary E2E covers pass, targeted fail, `cantTell`, `inapplicable`, `untested`, and malformed
  extension cases;
- Playwright E2E drives controlled slow success, failure/retry, and save-draft alternative flows;
- evaluation E2E checks separate acquisition/rule annotations, false positives, abstention,
  mutation kills, and byte-stable repeated results;
- existing generator, Rust, CLI, image, Web, structured-adapter, documentation, MSRV, and
  cross-platform gates remain green.
