# ADR 0011 — Evidence grading and CI policy

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

A visual or UX finding can be supported by exact source data, rendered measurements,
interaction traces, declared contracts, probabilistic inference, or human evidence. Treating
all findings as equivalent would either create noisy CI or falsely present model judgments as
proof.

## Decision

Keep outcome, evidence strength, confidence, severity, and CI policy as separate dimensions.

The initial evidence grades are:

- `provenStatic`
- `provenRender`
- `provenTrace`
- `provenDeclared`
- `inferred`
- `empirical`
- `advisory`

A rule result may block CI only when the project's policy marks that rule as blocking and the
rule's minimum evidence requirement is satisfied. Model-only inferred or advisory findings do
not block by default. Strict project policy may fail on high-risk `cantTell` outcomes when an
explicit declaration was required.

SightLint's trusted API does not expose one universal quality score as the release decision.

## Consequences

- High-confidence inference is still distinguishable from deterministic proof.
- Teams can adopt rules gradually and promote them after validation.
- Reports require more structured fields than a conventional warning list.
- Presentation layers may summarize results, but they must retain access to individual
  evidence and outcomes.

## Alternatives considered

- Confidence threshold alone: confidence does not identify the kind or authority of evidence.
- Severity alone: severe guesses are not proof.
- Every failure blocks: unusable for probabilistic and evolving rules.
- One aggregate score: hides incompatible failure modes and policy choices.

## Verification

M1 result types represent all five dimensions independently. Tests prove that an inferred
failure cannot satisfy a rule requiring proven rendered evidence and that policy resolution is
deterministic.
