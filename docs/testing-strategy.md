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

## Committed fixture corpus

Synthetic fixtures live under `fixtures/e2e/`. They are generated deterministically by
`tools/generate_e2e_fixtures.py`, committed for review, and checked for drift in required CI.
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
cover.

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

The E2E suite runs on Linux, macOS, and Windows. Required CI also regenerates the fixture corpus
in check mode before running tests.

## Determinism testing

Run the same fixture repeatedly and compare canonical output bytes. Vary insertion order, locale,
and supported operating systems where possible. Any unstable field must be removed from the
canonical report or explicitly isolated as non-canonical metadata.

## Precision and coverage

Rule quality is evaluated per rule, not only by a global score:

- precision, recall, and false-positive rate
- run-to-run agreement
- correct abstention or `cantTell`
- accuracy at measured coverage
- mutation kill rate
- state-space and recovery-path coverage
- expert agreement where relevant
- real-user outcome validation for rules that claim behavioral value

A rule begins experimental or advisory and earns blocking eligibility through rule-specific
evidence. A green E2E suite proves conformance to the declared contract; it does not by itself
prove that the contract improves real-user outcomes.
