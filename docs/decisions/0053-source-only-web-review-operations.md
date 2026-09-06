# ADR 0053 — Source-only Web review packets and immutable comparison

- Status: Accepted
- Date: 2026-09-06
- Issue: #78
- Parent: #71
- Related gates: #77, #74
- Roadmap: M2/M4 evaluation governance
- Owners: @taro-28

## Context

ADRs 0032 and 0051 keep Web acquisition truth separate from rule-verdict truth and identify the
public Atlas and Harbor annotations as maintainer-authored, tuning-visible development data.
ADR 0052 defines externally governed protected-holdout manifests without pretending that the
holdout is operational. Issue #77 now asks a real human to review the public annotations before
seeing SightLint output or the existing expected values.

The issue template is a useful protocol, but manual collection has avoidable failure modes. A
reviewer can accidentally open an oracle, include a screenshot or captured Artifact IR, collapse
an acquisition observation into a rule verdict, overwrite an uncertain judgment, or change an
answer after seeing the expected value. A free-form comparison can also hide the denominator,
lose disagreement rationale, or treat a fictional conformance response as human evidence.

This repository can reduce those operational risks, but it cannot generate human judgment or
prove a reviewer's identity, qualifications, independence, prior exposure, or conflicts of
interest. It also cannot make the external protected holdout in issue #74 operational.

## Decision

Add two standard-library Python processes outside the Rust kernel:

1. `prepare_web_review.py` deterministically assembles and validates a source-only review packet,
   emits a blank submission template, and finalizes reviewer-authored submission bytes; and
2. `compare_web_review.py` verifies a finalized submission digest before it opens the current
   public acquisition and rule oracles, then emits a read-only comparison report.

The tools are evaluation operations, not product adapters or policy engines. They do not launch a
browser, run SightLint, infer a review answer, adjudicate a disagreement, or modify an oracle.

## Source-only packet boundary

Packet version `1.0.0` has a fixed inventory of the current repository-owned Atlas and Harbor
fixture families. Its variable inputs are only:

- the six reviewed HTML, CSS, and JavaScript fixture source files; and
- the 27 versioned Playwright capture requests named by the public review protocol.

The inventory, review questions, non-claims, and governance declarations are protocol constants,
not values copied from an oracle. Each allowed file is embedded as UTF-8 text with its
repository-relative POSIX path, kind, byte length, and raw SHA-256 digest. Capture requests remain
identifiable as requests, not capture results. Files must be regular, non-symlinked files below the
repository root. The generator accepts no arbitrary include path, URL, or network input.

A packet validator rejects an unlisted file, duplicate path or case ID, wrong kind, path escape,
digest or byte-length mismatch, unknown field, and a request whose declared local/offline/privacy
contract differs from the admitted capture-request schema. In particular, a packet cannot add an
existing acquisition or rule oracle, expected verdict, SightLint report or diagnostic, captured
Artifact IR, generated screenshot, or another implementation output. The allowlist rather than a
filename heuristic is the authority, so an innocent word such as `report` in fixture prose does
not bypass or trigger the boundary.

The packet records public provenance, `MIT OR Apache-2.0` redistribution, fictional/no-personal-
data review, no external assets or processing, tuning visibility, and source-first/not-blind
limitations. It is a review input, never evidence by itself. The committed packet and blank
template are generator outputs checked byte for byte for drift; they contain no expected answer.

## Reviewer submission authority

Reviewer submission version `1.0.0` is distinct from packet and oracle data. It has separate arrays
for acquisition judgments and rule judgments. Acquisition judgments record a stable subject and
aspect, actual availability (`observed`, `cantTell`, or `untested`), a reviewed value only when
observed, confidence, rationale, native evidence, pixel evidence, native/pixel relationship, and
unavailable evidence. Rule judgments separately record rule/version/target, applicability,
evidence sufficiency, one of `passed`, `failed`, `cantTell`, `inapplicable`, or `untested`, policy
basis, valid-alternative or hard-negative rationale, false-positive and false-negative risks,
confidence, and rationale.

The schema does not permit an unavailable measurement to be filled by an estimate. Native/pixel
conflict remains an explicit relationship and neither side overwrites the other. `cantTell`,
`inapplicable`, and `untested` remain distinct from pass/fail and from comparison disagreement.

Every submission records a stable project reviewer ID, qualification category and rationale,
declared independence from annotation authors, prior expected-label exposure, conflicts of
interest, and review date. It also declares whether SightLint output, existing oracles, generated
capture evidence, or implementation-authored answers were used before finalization. A submission
with a prohibited declaration cannot be finalized. These are declarations only:
`identityOrQualificationVerifiedBySightLint` and `signatureVerifiedBySightLint` are always false.

`recordPurpose` is either `humanReviewCandidate` or `fictionalConformance`. Conformance records are
permanently ineligible for review evidence. A human candidate is still not automatically accepted
as independent evidence; issue #77 and repository governance must review the actual person,
declarations, scope, and exposure.

## Finalization and canonical bytes

A draft has no submission digest. Finalization performs strict structural and packet-binding
validation, changes only the lifecycle field and digest, and emits the result to standard output;
it never supplies, changes, or removes a judgment. The reviewer must store the emitted bytes before
opening an oracle.

The digest domain is UTF-8 JSON with duplicate keys rejected, finite JSON numbers only, object keys
sorted by Unicode code point, array order retained and separately required to be stable, ASCII JSON
escaping, no insignificant whitespace, and no trailing newline. `submissionDigest` is SHA-256 over
the canonical object with only that top-level field removed. The finalized serialized document is
the same canonical object including the digest. Packet and comparison digests use the same
projection rule. This deliberately bounded convention is not claimed to implement all of RFC 8785.

## Read-only comparison

Comparison begins by validating the packet, the submission-to-packet binding, finalized lifecycle,
and submission digest. Only then may the process open `evaluation-v1.json` and the separately
referenced acquisition and rule oracle files. It resolves acquisition subjects/aspects and rule
keys without copying one authority into the other.

Each comparison row preserves the reviewer value/status/rationale, the existing oracle
value/source/rationale, and one of `agreement`, `disagreement`, or `unresolved`. A missing or
ambiguous comparison key is unresolved. A differing comparable value is a disagreement and remains
unresolved until a separately responsible human adjudicates it; this version never adjudicates.
Consequently the `adjudicated` count is present and zero for tool-produced reports. Later support
for signed adjudication input requires a new version and decision rather than editing a finalized
reviewer submission.

Reports contain separate integer counts for acquisition agreement, rule agreement, all
disagreements, unresolved comparisons, adjudicated comparisons, and agreement on `cantTell` or
`untested` abstentions. Counts are not percentages and are not combined into a score. Rule
`inapplicable` agreement remains a rule agreement, not an abstention. The report binds the exact
packet, submission, registry, and oracle byte digests and labels fictional input as
`evidenceEligible: false`.

The comparison command writes canonical JSON only to stdout. It has no output-path option and does
not modify packet, submission, registry, oracle, source, or request files. Invalid input produces
empty stdout, one stable categorized diagnostic on stderr, and exit code 2. Success uses exit code
0 and empty stderr; agreement is not a process pass/fail gate.

## Resource, privacy, and compatibility limits

Version `1.0.0` enforces:

- 1,048,576 bytes per source, request, schema, submission, registry, or oracle input file;
- 8,388,608 bytes for a generated or validated packet and comparison output;
- exactly the current 33 allowlisted packet files and 27 packet cases, at most 27 submitted cases,
  and 512 total judgments/comparison rows; the 512 bound keeps a minimally populated exact-count
  submission below the independent 1 MiB input limit;
- at most 4,096 UTF-8 bytes for an ordinary submitted string, with smaller identifier/path limits;
- finite JSON numbers and no byte-order mark, duplicate key, absolute/backslash/parent path, URL
  input, symlink, or repository escape.

Every independently controllable maximum is accepted and the exact one-over value is rejected in
process E2E; fixed repository-owned file sizes and inventory counts are checked for exact drift.
The public
fixtures remain fictional, repository-owned, local, and free of customer data, personal data,
credentials, third-party assets, and external processing. Review submissions must not contain
private paths, URLs, credentials, protected membership, private labels, or private artifact bytes.
Public CI validates only public packets and wholly fictional conformance submissions.

These additive `1.0.0` evaluation schemas do not change Artifact IR, CheckReport, the capture
protocol, Web extension, rule IDs or semantics, profiles, enforcement, CLI exits, or existing
oracles. Incompatible review-operation changes require new schema files and an explicit migration
or coexistence plan.

## Alternatives considered

### Let the issue form remain the only review record

Rejected for the operational foundation because it cannot enforce source-only inputs, immutable
finalization, duplicate/unknown-field rejection, canonical digests, or reproducible separate
counts. The issue remains the human coordination record.

### Generate a draft answer from current annotations or SightLint output

Rejected because it would bias the reviewer and turn the implementation or existing oracle into
the source of the supposedly independent judgment.

### Compare before finalization and add the digest afterward

Rejected because expected values could influence or silently change the recorded first pass.

### Automatically resolve disagreements

Rejected because equality checking does not establish which authority is correct. Both sides and
their rationale remain available for independent adjudication.

### Put the workflow in the Rust kernel or normal product command

Rejected because review operations consume untrusted human/public evaluation documents and do not
belong to deterministic product policy or the medium-neutral IR.

## Non-goals

- generating, supplementing, or impersonating human judgment;
- proving reviewer identity, qualification, independence, declarations, or signatures;
- completing or closing issue #77 without a real reviewed submission;
- making issue #74 operational or creating protected-holdout evidence;
- blind evaluation, representative accuracy, WCAG conformance, blocking maturity, or a universal
  UI/UX score;
- changing a fixture oracle, product result, rule, profile, capture, IR, or report contract;
- private-data upload, remote processing, a database, credential store, or self-writing workflow.

## Verification

- JSON Schemas and process validators accept the current packet, blank template, finalized
  fictional conformance submission, and comparison report, and reject unknown or mixed-authority
  fields.
- Generator `--check` detects packet/template drift without writing.
- Process E2E covers clean agreement, disagreement, unresolved keys, `cantTell`, `inapplicable`,
  `untested`, a hard negative, duplicate IDs/keys, unknown fields, malformed and stale digests,
  privacy/leakage attempts, every exact limit, and every exact one-over limit.
- Repeated generation, finalization, comparison, stdout, stderr, and exit codes are byte stable,
  and before/after hashes prove comparison does not mutate inputs.
- Existing public Web evaluation and all CLI, PNG, image, interaction, perception, medium-adapter,
  release, documentation, MSRV, and cross-platform gates remain green.
