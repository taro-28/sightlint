# Rule model

## Goal

SightLint does not execute broad advice such as “make the hierarchy clear.” It executes
narrow, testable obligations with explicit evidence and applicability.

The rule model is inspired by W3C ACT concepts while extending them to visual artifacts and
interaction traces.

## Rule kinds

### Atomic rule

An atomic rule evaluates one narrow expectation against one or more targets. Examples:

- a rendered node is within its canvas bounds
- equivalent peer gaps do not exceed a configured tolerance
- essential text remains visible after a viewport transformation
- an asynchronous action exposes a pending or optimistic state

### Composite rule

A composite rule combines atomic outcomes and may permit multiple valid solutions. For an
irreversible action, a composite obligation might accept a review step, typed confirmation,
capability token, or another explicitly approved safeguard.

## Required rule metadata

Every rule must define:

- stable identifier and semantic version
- title and user-visible problem
- rule kind
- required input aspects
- applicability selector
- expectation
- accepted tolerances and units
- evidence requirements
- assumptions
- possible outcomes
- severity derivation inputs
- passing, failing, inapplicable, and cantTell fixtures
- known false-positive and false-negative risks

## Outcomes

The trusted result set is:

- `passed`: the applicable expectation is satisfied by sufficient evidence
- `failed`: the applicable expectation is violated by sufficient evidence
- `inapplicable`: the target does not meet the rule's applicability conditions
- `cantTell`: required meaning or evidence cannot be established safely
- `untested`: the required observation or execution was not performed

`cantTell` and `untested` are different. The first means evidence was considered but remains
ambiguous; the second means the necessary test did not run.

## Input aspects

Rules declare exactly what they consume, for example:

- semantic tree
- source geometry
- rendered geometry
- pixels
- typography
- color
- project policy
- inferred project baseline
- interaction trace
- action effects
- accessibility tree
- temporal state model

If an input aspect is unavailable, the rule returns `inapplicable`, `cantTell`, or `untested`
according to its contract instead of guessing.

## Evidence strength

Reports distinguish verdict outcome from evidence strength. Planned evidence grades include:

- `provenStatic`
- `provenRender`
- `provenTrace`
- `provenDeclared`
- `inferred`
- `empirical`
- `advisory`

A result may be a high-confidence inference and still be non-blocking because it is not proof.

## Severity

Severity is derived from explicit factors rather than a model's free-form label. Candidate
factors include:

- user or data harm
- affected scope
- exposure or frequency
- likelihood
- recoverability
- task criticality

The precise model belongs in a future ADR. Severity must not be inferred from confidence.

## Policy precedence

Expectations resolve in this order:

1. explicit project rule or exception
2. exact design-system or platform contract
3. inferred project norm with visible confidence
4. platform convention
5. conservative universal baseline

The chosen source is evidence in the report.

## CI policy

The initial CI integration should block only on:

- sufficiently evidenced failures in rules configured as blocking
- schema or engine errors
- high-risk `cantTell` outcomes when strict policy explicitly requires declaration

Advisory AI critique, aesthetics, and a single aggregate score must not block by default.

## Example conceptual rule

```yaml
id: visual.spacing.peer-consistency
version: 0.1.0
kind: atomic
inputAspects:
  - semantic-tree
  - rendered-geometry
applicability:
  repeated peers with equivalent roles and variants
expectation:
  corresponding gaps differ by no more than the resolved tolerance
outcomes:
  - passed
  - failed
  - inapplicable
  - cantTell
  - untested
```

The executable implementation may be Rust rather than YAML. The metadata and semantics must
remain serializable and inspectable.
