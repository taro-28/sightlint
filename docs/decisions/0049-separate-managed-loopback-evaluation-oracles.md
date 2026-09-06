# ADR 0049 — Separate managed-loopback acquisition and rule oracles

- Status: Accepted
- Date: 2026-09-06
- Issue: #65
- Parent: #31
- Follow-up to: #62 and ADR 0048
- Roadmap: M7
- Owners: @taro-28

## Context

ADR 0048 introduced managed-loopback Web capture and a three-case product evaluation. Its first
annotation document combines case classification with expected rule results, while acquisition
expectations and metric calculations remain embedded in the E2E test. The implementation keeps
adapter observations and Rust verdicts separate at runtime, but the reviewed evaluation data does
not expose the same boundary clearly enough.

This matters because successful server startup, redirect traversal, response acquisition, DOM
capture, screenshot production, and cleanup are not proof that a rule verdict is correct. Likewise,
a correct rule verdict cannot be used as the oracle for what the browser acquired. Reviewers need
to be able to change or challenge either authority without implicitly changing the other.

## Decision

The current managed-loopback evaluation uses two independently versioned, strict documents:

- `managed-loopback-acquisition.schema.json` and
  `annotations/managed-loopback-acquisition.json` describe exact acquisition expectations,
  explicit acquisition abstentions, provenance, split status, and acquisition coverage;
- `managed-loopback-rule.schema.json` and `annotations/managed-loopback-rules.json` describe
  deterministic rule outcomes, false-positive risks, non-claims, and rule metrics.

The original combined `managed-loopback.schema.json` remains as the strict historical `0.1.0`
shape, but its combined annotation is removed from the current evaluation path. It is not an
authority for new results.

Both current documents name the same three case and request identifiers. The E2E validates both
schemas, requires a one-to-one case join, and rejects classification or request disagreement. It
then checks acquisition expectations only against capture/Artifact IR/lifecycle evidence and rule
expectations only against the public Rust report.

### Acquisition expectations

The acquisition oracle records protocol and adapter versions, requested query-free route, bounded
response-count evidence, digest binding, viewport screenshot extent, document state, required
native node identifiers, unavailable source attribution, redaction, and port release. Exact facts
use typed comparison operators. Pixel-content identity, complete hit regions, and source-file
causality remain explicit `cantTell` observations rather than inferred facts.

Acquisition coverage is reported as matched reviewed expectations over all reviewed expectations.
Abstention coverage is reported separately. These counts do not include rule outcomes.

### Rule expectations and metrics

The rule oracle records the expected exit code, failure counts, and named results for the clean,
targeted-mutation, and hard-negative cases. It defines counts for:

- executed reviewed cases;
- matched reviewed failures over all emitted failures (failure precision);
- matched reviewed abstentions over reviewed abstentions;
- unexpected emitted failures (false positives);
- killed reviewed rule-eligible mutations;
- failures emitted for reviewed hard negatives.

The E2E reports integer numerators and denominators. It does not introduce an aggregate or
universal quality score, and it does not publish a percentage with a zero denominator.

### Provenance, leakage, license, and privacy

Both documents are maintainer-authored from the repository-owned Atlas source and existing rule
contracts. Captured Artifact IR, screenshots, reports, and other implementation output are not
copied into either oracle. Oracle changes require semantic review rather than adjustment merely to
make a test pass.

All data is fictional and repository-owned under `MIT OR Apache-2.0`; no customer data, secrets,
third-party assets, or external processing are permitted. The three labels are public and visible
to implementers. They are smoke/development/challenge regression cases, not a protected holdout,
independent review, or representative sample. A future holdout requires a separate decision that
defines its freeze point, access controls, evaluator, leakage controls, and correction process.

## Consequences

- Evaluation review can distinguish a transport/acquisition regression from a rule regression.
- Metric denominators are derived from reviewed data rather than duplicated test constants.
- Schema strictness prevents rule fields from being inserted into acquisition truth and vice
  versa.
- The public corpus remains intentionally small and cannot support general Web-accuracy claims.
- The protocol, Playwright implementation, fixture application, Rust kernel, rules, reports, and
  compatibility surfaces do not change.

## Alternatives considered

### Keep the combined annotation and improve test comments

Rejected because comments do not create independently reviewable authorities or strict boundaries.

### Generate expectations from current capture and rule output

Rejected because that makes the implementation its own oracle and can preserve regressions as
expected data.

### Add a private holdout now

Rejected because this focused governance correction does not yet have an independent data owner,
access policy, evaluator, or representative sampling plan. Public CI must not depend on private
raw artifacts.

### Expand the fixture family or add rules

Rejected for this PR. Broader usefulness and accuracy require separately reviewed evidence and
should not be hidden inside an evaluation-contract correction.

## Non-goals

- adapter, server-lifecycle, network, protocol, extension, report, or kernel behavior changes;
- new or promoted rules, blocking policy, or a universal UX score;
- representative accuracy, WCAG conformance, source causality, or whole-application coverage;
- treating the tabisaifu dogfood report as reviewed ground truth.
