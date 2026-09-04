# ADR 0005 — ACT-inspired rule outcomes

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Binary pass/fail is insufficient when a rule does not apply, evidence is ambiguous, or a
required state was never exercised. Accessibility Conformance Testing rules provide a proven
pattern of atomic/composite rules, applicability, expectations, and explicit non-binary
outcomes.

## Decision

Adopt an ACT-inspired model with atomic and composite rules. The core outcomes are:

- `passed`
- `failed`
- `inapplicable`
- `cantTell`
- `untested`

Rules declare input aspects, applicability, expectations, assumptions, evidence requirements,
and versioned semantics.

## Consequences

- Reports can distinguish lack of applicability from lack of evidence.
- CI can enforce policy on high-risk unknowns without pretending they are failures.
- Rule authors must write more precise contracts and fixtures.
- SightLint remains free to extend visual and interaction-specific metadata beyond ACT.

## Alternatives considered

- Boolean rules: too lossy.
- pass/fail/unknown: conflates inapplicable, ambiguous, and unexecuted cases.
- free-form evaluator text: not composable or reliably machine-actionable.

## Verification

The engine API and report schema use the five outcomes. Rule tests include at least passing,
failing, and non-applicable or unknown fixtures as appropriate.
