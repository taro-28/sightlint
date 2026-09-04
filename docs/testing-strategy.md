# Testing strategy

SightLint is itself a quality gate, so its verification standard must exceed ordinary feature
code.

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
blind snapshot refresh.

### Mutation fixtures

Each mature rule should include a transformation that injects its target defect into a valid
fixture. The rule must kill that mutation. Examples:

- remove a pending indicator
- change one peer gap
- clip essential text
- enable duplicate submission
- hide the affected count in a bulk action

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

### End-to-end tests

Exercise CLI input, rule execution, exit codes, report formats, and evidence references. Later
adapters should run in hermetic fixtures wherever possible.

## Determinism testing

Run the same fixture repeatedly and compare canonical output bytes. Vary hash seeds, insertion
order, locale, and supported operating systems where possible. Any unstable field must be
removed from the canonical report or explicitly isolated as metadata.

## Precision and coverage

Rule quality is evaluated per rule, not only by a global score:

- precision, recall, and false-positive rate
- run-to-run agreement
- correct abstention or `cantTell`
- accuracy at measured coverage
- mutation kill rate
- expert agreement where relevant
- real-user outcome validation for rules that claim behavioral value

A rule should begin advisory and earn blocking status through evidence.
