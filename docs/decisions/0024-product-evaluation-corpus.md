# ADR 0024 — Separate conformance tests from product evaluation

- Status: Accepted
- Date: 2026-09-05

## Context

SightLint is a quality gate. Its existing fixture-driven binary E2E suite proves that schemas,
rules, adapters, reports, safety limits, and exit codes behave according to their declared
contracts. That is necessary, but it can still produce a perfectly conforming implementation of
the wrong product behavior.

The original product goal is stronger: without repeated prompting, SightLint should detect
general visual and interaction-quality failures that people expect a competent UI/UX review to
catch. That claim requires a reviewed oracle and measurements of precision, recall, abstention,
coverage, and mutation detection. Mixing those claims into parser or API tests would make both
sets of evidence difficult to interpret.

## Decision

Maintain two distinct, required verification systems:

1. `fixtures/e2e/` and its tests are the **conformance corpus**. They verify executable contracts,
   malformed-input handling, safety boundaries, serialization, determinism, and platform
   compatibility.
2. `evaluation/` is the **product evaluation corpus**. It verifies that the public binary's
   observable rule outcomes match a versioned, reviewed oracle.

A green conformance suite must never be described as proof of real-world rule quality. A product
evaluation failure must not be hidden by changing an oracle without reviewing whether the product
behavior or the oracle was wrong.

## Manifest contract

The product corpus uses a versioned, language-neutral JSON manifest and schema. Each case records:

- a stable case identifier and medium;
- a `smoke`, `development`, or `holdout` split;
- a repository-relative input and input kind;
- a source with origin, license, and review status;
- the expected public CLI exit code;
- required rule outcomes;
- whether undeclared failures, abstentions, or untested results are forbidden;
- an optional clean-baseline and target-rule relation for a synthetic mutation.

Version `0.1.0` accepts Artifact IR inputs only. This establishes the runner and oracle semantics
using the existing reviewed synthetic corpus. Native image, browser, slide, PDF, and mobile inputs
require a compatible schema extension after their data-governance and annotation contracts are
defined.

## Required smoke gate

Required CI executes the built `sightlint` binary for every smoke case. It must:

- validate manifest identifiers, references, allowed fields, and repository-contained paths;
- run each case at least twice and require byte-identical stdout, stderr, and exit code;
- assert every declared rule outcome;
- reject undeclared `failed`, `cantTell`, or `untested` outcomes when forbidden by the case;
- require each mutation baseline to pass its target rule;
- require each mutation to fail that same target rule;
- execute on all operating systems through the existing workspace test matrix.

Malformed input remains in conformance E2E rather than product evaluation. Product cases represent
valid artifacts whose quality outcome is being evaluated.

## Dataset splits and leakage

- `smoke` is small and blocking on every change.
- `development` may be used while designing and tuning rules.
- `holdout` is frozen and must not be consulted while tuning the evaluated rule.

The initial corpus contains only smoke cases. Before adding a holdout set, the project must
document its freeze process, access policy, and release reporting. Public availability does not
remove the obligation to avoid deliberately tuning against holdout labels.

## Metrics and maturity

Evaluation is reported per rule, medium, evidence class, and split. Appropriate measurements
include precision, recall, false-positive rate, abstention rate, accuracy at measured coverage,
mutation kill rate, run-to-run agreement, and reviewer agreement.

SightLint does not derive a universal UX score from these measurements. Rule maturity and blocking
eligibility remain rule-specific decisions under ADR 0011 and ADR 0012.

The synthetic smoke corpus is a regression oracle, not evidence of real-world precision. Rules
remain experimental until representative human-reviewed data supports stronger claims.

## Data governance

Every source must declare origin, license status, and review status. External artifacts may be
committed only when redistribution is permitted. Private, customer, credential, or personal data
must not enter the public corpus. Human-reviewed corpora must document annotation guidance,
reviewer qualifications, disagreement resolution, and known sampling bias.

## Consequences

- Product intent becomes executable and reviewable early rather than being deferred until after
  adapters and models are built.
- Rule regressions can be distinguished from parser or report regressions.
- Pull requests incur a small additional runtime from repeated public-binary evaluation.
- The initial synthetic corpus provides no basis for marketing accuracy claims.
- Future native and human-reviewed data requires deliberate licensing and annotation work rather
  than ad hoc screenshot collection.
