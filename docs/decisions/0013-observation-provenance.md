# ADR 0013 — Observation provenance is mandatory

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

The same apparent property can come from a source declaration, platform semantics, rendered
pixels, OCR, a vision model, a user contract, or an interaction trace. A bare value such as
`fontSize: 14` cannot support an auditable verdict because its authority, units, uncertainty,
and source location are unknown.

## Decision

Every observation used by a rule carries a resolvable evidence reference. Evidence identifies:

- source class and adapter
- artifact and source selector or region
- native unit and coordinate space where relevant
- acquisition or model version
- confidence, bounds, or alternatives for inferred values
- content digest or stable source identity where practical
- whether external processing or transmission occurred

Exact facts and probabilistic inference use the same observation envelope but different
evidence classes. Exact observations do not receive artificial confidence scores merely to
fit a probabilistic model.

## Consequences

- Reports can explain and reproduce their inputs.
- Conflicting native and rendered observations can coexist.
- IR is larger than a minimal scene graph.
- Adapters must do provenance work rather than returning anonymous nodes.

## Alternatives considered

- Provenance only on final findings: loses how intermediate values were obtained.
- Confidence on every field: conflates exact authority with probabilistic belief.
- Adapter-level provenance only: too coarse when one adapter combines several sensors.

## Verification

M1 validation rejects observations whose required evidence reference cannot be resolved.
Golden reports link each outcome to observations and source selectors. Inferred observations
cannot omit model or uncertainty metadata required by their evidence class.
