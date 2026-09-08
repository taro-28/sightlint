# ADR 0054 — Local Harbor review workbench and eight-judgment pilot

- Status: Accepted
- Date: 2026-09-07
- Issue: #80
- Parent: #71
- Human-review gate: #77
- Protected-holdout gate: #74
- Owners: @taro-28

## Context

ADR 0053 establishes a source-only packet, a strict reviewer-submission contract, immutable
finalization, and read-only comparison. Those boundaries prevent an implementation result or
existing oracle from becoming an independent review answer, but the general submission format
still exposes too much mechanical work to the first human reviewer. A reviewer must currently
copy identifiers, source references, viewport details, evidence boilerplate, rule metadata, and
JSON structure before the four-case Harbor pilot can be finalized.

That is not the minimum human contribution needed to review the current Harbor claim. The family
exists to test one advisory programmatic-name rule over four deliberately different source states.
For that pilot, the independent human must decide only one acquisition observation and one rule
verdict per case. Requiring unrelated geometry or manual serialization makes transcription errors
more likely without increasing the authority of those eight judgments.

The workbench must not solve the burden by generating an answer, opening an oracle, running
SightLint, or sending the review to a hosted service. The review remains source-first and public,
not blind or protected. The implementation can make the process smaller and safer, but it cannot
make the reviewer independent or turn a pilot into representative product evidence.

## Decision

Add a standard-library Python process, `run_web_review.py`, that serves a fixed local browser
workbench for the Harbor pilot. It constructs the existing reviewer-submission `1.0.0` format and
uses ADR 0053 finalization unchanged. It is an evaluation-operation tool outside the Rust kernel,
not a product adapter, rule engine, acquisition sensor, or remote service.

The first command surface is:

```text
python3 tools/run_web_review.py --scope harbor --draft PATH --final PATH
```

The host is fixed to IPv4 loopback. The port may be explicit or ephemeral. Browser opening is a
convenience only and can be disabled for automation. Draft resume is explicit rather than
silently trusting any existing file.

## Fixed answer-free questionnaire

Add a versioned, strict Harbor questionnaire with exactly four packet-bound cases and exactly two
questions per case. It is review input, not evidence, and contains no expected observation,
expected verdict, oracle value, SightLint output, captured Artifact IR, generated screenshot,
diagnostic, or suggested answer.

Every question names metadata that follows from the public source and executable rule contract:

- source selector: `[data-testid="reply-send"]`;
- stable node identifier: `web-reply-send`, derived by the documented Playwright locator-to-ID
  algorithm rather than copied from an oracle;
- acquisition aspect: native accessible `name`;
- rule: `web.accessibility.interactive-name@0.1.0`;
- rule target: node `web-reply-send` with no target aspect; and
- policy reference: WCAG 2.2 Name, Role, Value as already declared by the rule definition.

The questionnaire records the exact source-packet ID and digest, its own canonical SHA-256 digest,
the four case/request/source bindings, public fictional provenance, dual-license basis, local-only
processing requirement, and pilot limitations. Generator drift and strict validation bind the
tool to those reviewed bytes. The questionnaire does not classify a case as clean, mutation,
hard negative, or ambiguous for the reviewer; the generated submission uses a neutral case
context.

## Human-supplied and mechanical fields

The reviewer supplies the declaration once:

- stable project ID;
- qualification category and rationale;
- independence status and rationale;
- prior expected-label exposure status, affected case IDs, and rationale;
- conflict-of-interest status and rationale;
- actual review date; and
- affirmative confirmations of the ADR 0053 source-only boundary.

For each of the four cases, the reviewer supplies exactly two substantive judgment records:

1. acquisition status (`observed`, `cantTell`, or `untested`), an explicit observed-value kind,
   confidence, and a short factual rationale; and
2. rule outcome, required-evidence state, confidence, and a short rationale.

The workbench may derive only mechanically forced fields. It supplies IDs, bindings, rule and
policy identifiers, null coordinate units, the native-only question shape, generic documented
false-positive/false-negative cautions, neutral case context, and limitations. It maps a reviewer
rule outcome to the only compatible applicability value: pass/fail to `applicable`,
`inapplicable` to `inapplicable`, `cantTell` to `cantTell`, and `untested` to `untested`. It rejects
an incompatible required-evidence combination instead of repairing or guessing it.

For the acquisition record, an observed value kind distinguishes `text` from `absent`. `text`
requires a non-empty reviewer string. `absent` serializes to an observed null value, representing
native evidence that no accessible name exists; it is not an evidence-insufficient `cantTell`.
`cantTell` and `untested` require no value kind and serialize a null value while reusing the
reviewer's rationale as the unavailable-evidence explanation. Observed states map native evidence
to available. Pixel evidence is `notApplicable` and the native/pixel relationship is
`notCompared`; the programmatic name is not inferred from pixels. These mappings are serialization
consequences of the human choice, not additional observations.

The generated submission remains version `1.0.0`. The existing validator and comparator remain
the authorities, and handwritten/general submissions remain supported. The workbench records its
questionnaire version and digest in the submission limitations without adding a new schema field.

## Local origin and source boundary

The process serves only:

- fixed workbench HTML, CSS, and JavaScript assets;
- a bounded same-origin JSON API;
- the three Harbor fixture files read from the already validated packet's embedded UTF-8 bytes;
  and
- the four fixed fixture states from the questionnaire.

It never reads the repository fixture files to render a case after packet validation, so the
displayed source is exactly what the packet digest binds. Before finalization it must not import or
open comparison code, evaluation registries, acquisition/rule oracle files, captured output,
screenshots, reports, diagnostics, or a SightLint command. Finalization does not compare. After a
successful lock the UI displays the separate local comparison command, which the reviewer may run
deliberately after leaving the source-only phase.

The server accepts no arbitrary URL, source include, command, executable, repository-relative
read path, or client-selected filesystem destination. Draft and final paths come only from the
launching CLI and must resolve outside the repository. Their existing parent directories must be
real, non-symlinked directories. Draft and final paths must differ. Existing paths must be regular,
non-symlinked files; a final path is never overwritten.

## HTTP and local-write safety

The HTTP server is single-process and loopback-only. It uses fixed methods and routes, strict Host
and same-origin checks, no CORS, JSON-only mutation requests, a per-process random capability token
held in the browser fragment/in-memory request header, restrictive cache/referrer/content-type and
Content-Security-Policy headers, bounded request bodies, duplicate-key rejection, and stable
fail-closed JSON errors. Neither request nor response bodies are logged.

The fixture response uses a separate restrictive policy that permits only its same-origin CSS and
JavaScript and prevents network connections or top-level navigation. The workbench does not claim
to control browser extensions, browser synchronization, operating-system telemetry, or developer
tools. A reviewer seeking an external-processing-free record remains responsible for using a
clean local browser profile with such features disabled; the UI records that limitation and
requires the existing declaration.

Draft writes use a closed same-directory temporary file, flush, file synchronization, and atomic
replacement. Resume is allowed only with an explicit option and only when a draft validates,
matches the current packet/questionnaire/scope, and can be losslessly projected back into the
minimal form. Final bytes are canonical, written through an exclusive same-directory promotion,
and never replace an existing target. After finalization the process locks mutation endpoints.
Temporary files are cleaned after failures where the platform permits; a crash may leave an
obviously named temporary file but never a silently accepted finalized record.

## Bounds and lifecycle

The workbench adds a smaller HTTP body limit than the existing 1 MiB submission limit. The exact
limit is a protocol constant and is tested at the boundary and one byte over. Strings retain the
ADR 0053 bounds, case and question inventories are fixed, JSON numbers are finite, and unknown
fields, duplicate keys, duplicate cases, invalid enum combinations, invalid dates, guessed
unavailable values, privacy leakage, URL/path material, and credential-like content fail closed.

A strict workbench-state schema represents incomplete local progress without pretending that it is
a reviewer submission. It binds the packet and questionnaire, keeps exactly four answer slots,
allows null or empty draft fields, carries a canonical state digest, and is never evidence. This
separate draft contract prevents placeholder values or half-entered answers from being promoted
into the existing submission schema.

A draft can be saved while incomplete. Finalization requires complete reviewer declarations and
all eight judgments, converts the minimal state to a reviewer submission, validates it, and calls
the existing canonical finalizer. Actual workbench sessions use `humanReviewCandidate` and remain
`requiresGovernanceReview`. Process E2E uses only explicitly selected fictional-conformance mode,
which produces `fictionalConformance` / `ineligibleConformance` and can never satisfy issue #77.

## Consequences

The Harbor pilot no longer requires handwritten JSON, identifiers, policy metadata, digest work,
or manual metric counting. The reviewer still must inspect browser-native evidence and provide the
eight judgments; reducing clicks does not reduce epistemic responsibility.

The first slice is intentionally not a generic review-form builder and does not review Atlas. The
full public-annotation gate therefore remains staged:

1. Phase A uses this workbench for the four-case Harbor pilot; and
2. Phase B reviews the Atlas corpus under a later evidence-backed workload decision.

Completing the tool does not complete Phase A. Completing Phase A does not by itself establish
full independent review, permit a protected-holdout claim, or promote a rule.

## Alternatives considered

### Continue with handwritten reviewer-submission JSON

Rejected as the preferred Harbor path because it gives humans mechanical work that adds no review
authority and creates avoidable transcription failures. The general JSON path remains supported.

### Collect answers in a hosted AI chat

Rejected for eligible review because it introduces external processing and model-mediated
authoring before finalization. A chat can provide non-evidentiary rehearsal only.

### Pre-fill answers from the current oracle or SightLint output

Rejected because it destroys independence and turns the compared authority into the answer source.

### Generate browser accessibility observations for the reviewer

Rejected from this slice because a generated capture would become another untrusted sensor and is
explicitly outside the current source-only answer boundary. The reviewer inspects native browser
information directly and can abstain when it is unavailable.

### Review all 27 public cases in the first workbench

Rejected because the immediate problem is excessive human burden. The four Harbor states cover the
pilot's clean, targeted-change, valid-alternative, and insufficient-evidence situations without
pretending to complete the Atlas audit.

### Put the workbench in the Rust CLI or kernel

Rejected because it is a local evaluation operation over untrusted human input, not deterministic
medium-neutral policy execution.

## Non-goals

- generating, suggesting, supplementing, or adjudicating a human answer;
- proving reviewer identity, qualification, independence, conflict status, or signatures;
- performing the real Harbor review or committing a human response;
- reviewing Atlas or designing a general dynamic questionnaire system;
- changing an oracle, rule, rule version, profile, maturity, enforcement, capture protocol,
  Artifact IR, CheckReport, or product exit behavior;
- launching SightLint or generated acquisition before finalization;
- remote hosting, external processing, telemetry, a database, private data, or holdout storage;
- completing issue #77 or making issue #74 operational; or
- representative accuracy, WCAG conformance, blocking maturity, or a universal UI/UX score.

## Verification

- Strict schema and process validation prove the questionnaire has exactly four cases, exactly two
  questions per case, fixed metadata, packet binding, no answer fields, and no prohibited paths or
  expected values.
- Generator drift checks bind committed questionnaire bytes.
- Browser process E2E starts on an ephemeral loopback port, renders all four fixture states from
  packet bytes, exercises the fixed UI and API, and proves no oracle or SightLint dependency is
  needed before finalization.
- Negative process tests cover non-loopback binding, bad Host/Origin/token/method/content type,
  traversal, unknown and duplicate fields, exact and one-over body limits, incomplete declarations,
  all inconsistent status/value/evidence/outcome combinations, invalid exposure/date/privacy data,
  unsafe output paths, incompatible resume, final overwrite, and edits after locking.
- Fictional E2E proves atomic draft/resume/finalization, stable canonical bytes and digests,
  validation by `prepare_web_review.py`, and opt-in post-finalization comparison without claiming
  human evidence.
- Existing generator, CLI, Web, image, product-evaluation, interaction, perception, medium-adapter,
  release, documentation, MSRV, cross-platform, and CodeQL gates remain green.
