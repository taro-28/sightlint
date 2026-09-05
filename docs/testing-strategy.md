# Testing strategy

SightLint is itself a quality gate, so its verification standard must exceed ordinary feature
code. Tests must prove the public behavior users and coding agents observe, not only execute
internal functions.

## Acceptance rule

Every user-visible command, serialized contract, rule, adapter, report, policy switch, and exit
code requires fixture-driven end-to-end coverage through the built `sightlint` binary. Unit tests
are necessary but are never sufficient for those changes.

The required CI path is:

```text
committed input bytes
  -> public CLI
  -> decoding and semantic validation
  -> normalization and queries
  -> rule execution
  -> evidence-linked report
  -> stdout, stderr, and exit code assertions
```

## Two independent verification questions

SightLint keeps conformance and product evaluation separate:

- **Conformance:** does the implementation obey its declared schemas, rule semantics, adapter
  boundaries, safety limits, reports, determinism, and exit codes?
- **Product evaluation:** do reviewed cases receive the outcomes that the intended visual or UX
  quality oracle says they should receive?

A green conformance suite can still encode the wrong product behavior. A green product evaluation
cannot replace parser, safety, compatibility, or malformed-input testing. Both are required, and
their results must not be collapsed into one score.

## Committed conformance fixture corpus

Synthetic conformance fixtures live under `fixtures/e2e/`. They are generated deterministically
by `tools/generate_e2e_fixtures.py`, committed for review, and checked for drift in required CI.
Generated files are not edited by hand.

The corpus contains these categories:

- `pass-*`: clean examples and medium-neutral coverage
- `fail-*`: targeted mutations that must be killed by one named rule
- `cant-tell-*`: missing or incomparable evidence that must cause conservative abstention
- `invalid-*`: malformed JSON or semantically invalid Artifact IR
- `inapplicable`: valid artifacts without a target for the rule
- ordering variants: semantically equivalent inputs with irrelevant collection order changed

A new rule must add passing and mutation fixtures. It must also add `cantTell` and
`inapplicable` cases whenever those outcomes are part of its contract. A new adapter must add
native-input-to-IR fixtures and, when possible, differential fixtures against another observation
source.

## Product evaluation corpus

The versioned product oracle lives under `evaluation/`. Its manifest records case IDs, media,
data sources, licenses, review status, splits, inputs, expected rule outcomes, and mutation
relations. The evaluation runner executes the same built `sightlint` binary users invoke.

The initial `0.1.0` corpus is a synthetic smoke suite over reviewed Artifact IR fixtures. It
establishes the harness early, but it is not evidence of real-world precision or UX benefit.
Future native and human-reviewed cases must be added before stronger accuracy claims or rule
maturity changes.

Evaluation splits have distinct purposes:

- `smoke`: small, deterministic, and blocking on every pull request
- `development`: reviewed data available during rule design and tuning
- `holdout`: frozen data not consulted while tuning the evaluated rule

Before a holdout split is introduced, its freeze, access, leakage-prevention, and release-reporting
process must be documented. Updating an expected outcome requires semantic review; snapshots are
not accepted merely because implementation output changed.

Every evaluation source records origin, license status, and review status. Public artifacts must
be redistributable and scrubbed of personal, customer, credential, and private information.
Human-reviewed sources additionally require annotation guidance, reviewer qualifications,
disagreement resolution, and known sampling limitations.

## Test layers

### Contract tests

Validate Artifact IR and report schemas, compatibility, canonical ordering, invalid-reference
rejection, and exact round trips.

### Unit tests

Cover geometry, units, tolerances, evidence propagation, selectors, applicability, and outcome
composition with small deterministic inputs.

### Property tests

Useful invariants include:

- containment is stable under translation
- gap is symmetric where its definition is symmetric
- overlap area is never negative
- coordinate normalization and denormalization remain within declared tolerance
- serialization order does not depend on insertion order
- rule results do not change under irrelevant node reordering

### Golden fixtures

Store human-readable input IR and expected reports. Golden updates require explanation, not a
blind snapshot refresh. Semantic assertions remain mandatory even when a golden report is used.

### Mutation fixtures

Each executable rule includes a transformation that injects its target defect into a valid
fixture. The rule must kill that mutation. Examples include:

- changing one peer gap
- introducing overlap
- moving observed bounds outside their canvas
- removing a pending indicator
- enabling duplicate submission
- hiding the affected count in a bulk action

Mutation kill rate measures whether the checker can actually detect the failures it claims to
cover. In the product evaluation manifest, every synthetic mutation identifies a clean baseline
and one target rule; required smoke CI verifies the baseline passes and the mutation fails that
same rule.

### Metamorphic tests

When there is no single correct screenshot, test relationships under transformations:

- slow or failing network
- larger data sets
- longer translations and right-to-left direction
- viewport and text-scale changes
- mouse, keyboard, and touch input
- permission and error-state changes

### Differential adapter tests

When native structure and pixels describe the same artifact, compare them. When two adapters
support the same fixture, verify compatible core observations and document expected loss.

### Binary end-to-end tests

Run the built CLI against committed data and cover at least:

- file and standard input
- human and canonical JSON reports
- pass, fail, `cantTell`, `inapplicable`, and malformed-input outcomes
- default and strict ambiguity policies
- stable exit codes 0, 1, and 2
- input safety limits and invalid UTF-8
- schema and version commands
- every supported artifact kind
- repeated byte-identical output
- canonical normalization idempotence
- invariance under irrelevant input ordering

The E2E suite runs on Linux, macOS, and Windows. Required CI also regenerates the conformance
fixture corpus in check mode before running tests.

### Product-evaluation E2E

Run every required smoke case through the public binary and verify:

- safe, repository-contained input paths and valid manifest references
- declared artifact medium and CLI exit code
- all required rule outcomes
- no undeclared failures, abstentions, or untested results when forbidden
- byte-identical output for the configured number of runs
- a passed baseline and failed mutant for every targeted mutation pair

Malformed files belong in conformance tests rather than product evaluation because product cases
represent valid artifacts whose quality verdict is being measured.

## Determinism testing

Run the same fixture repeatedly and compare canonical output bytes. Vary insertion order, locale,
and supported operating systems where possible. Any unstable field must be removed from the
canonical report or explicitly isolated as non-canonical metadata.

Product evaluation also repeats each smoke case independently. Determinism is a prerequisite for
measuring quality: a case that changes outcome between runs cannot contribute a trustworthy
precision or recall estimate.

## Precision and coverage

Rule quality is evaluated per rule, medium, evidence class, and dataset split, not only by a
global score:

- precision, recall, and false-positive rate
- run-to-run agreement
- correct abstention or `cantTell`
- accuracy at measured coverage
- mutation kill rate
- state-space and recovery-path coverage
- expert agreement where relevant
- real-user outcome validation for rules that claim behavioral value

SightLint must not collapse these measurements into a universal UX or design score. A rule begins
experimental or advisory and earns blocking eligibility through rule-specific evidence. A green
conformance suite proves conformance to the declared contract; a green synthetic smoke evaluation
proves only regression stability. Neither alone proves improved real-user outcomes.
