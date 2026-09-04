# ADR 0014 — Units and coordinate spaces are explicit

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

SightLint compares web CSS pixels, device pixels, Android dp, iOS points, slide points or EMUs,
PDF coordinates, normalized image regions, and transformed nested canvases. A numeric rectangle
without its unit, origin, scale, and coordinate-space identity is ambiguous and can produce
plausible but incorrect lint results.

## Decision

Every geometric observation declares:

- a coordinate-space identifier
- unit
- origin and axis convention
- bounds or extent of the referenced canvas when applicable
- transformation to its parent or canonical canvas when known
- numeric uncertainty for measured or inferred values when relevant

Native values are preserved. Normalized 0–1 coordinates and canonical coordinates are derived
views, not replacements for source geometry. Unit conversion and rounding policy belong in the
deterministic kernel and are versioned behavior.

## Consequences

- Rules can compare geometry safely across adapters and canvases.
- IR and geometry APIs are more explicit and somewhat verbose.
- Adapter authors must document transforms and pixel/device scale.
- Unknown transforms produce `cantTell` rather than implicit conversion.

## Alternatives considered

- Convert everything to pixels: loses physical and platform meaning.
- Store normalized coordinates only: loses source precision and typography-relevant units.
- Infer unit from artifact kind: fails for exports, nested canvases, and mixed-resolution data.

## Verification

M1 validation rejects geometry without a valid coordinate-space reference and explicit unit.
Property tests cover translation, scaling, round trips, and declared tolerance. Cross-adapter
fixtures preserve native and normalized views without overwriting either.
