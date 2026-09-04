# ADR 0015 — Layout, render, and hit bounds are distinct

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

One generic rectangle cannot represent all relevant geometry. Source layouts allocate space,
rendering produces visible pixels that may include transforms and effects, and interactive
platforms expose hit regions that may be larger or smaller than visible content.

Conflating these leads to incorrect findings. A small icon may have an adequate touch target;
a node with valid source bounds may be visually clipped; transparent image padding may make a
mathematically aligned element appear offset.

## Decision

Artifact IR models these concepts separately:

- `layoutBox`: space allocated by a source layout system
- `renderBox` or `inkBox`: visible output under an explicitly defined effects policy
- `hitBox`: region that can receive interaction

Each box has independent provenance, coordinate space, unit, and uncertainty. Missing boxes
remain missing; the normalizer does not synthesize equality between them without explicit
evidence.

Rules declare which geometry kind they require. Reports identify the geometry source used in
the verdict.

## Consequences

- Source-vs-rendered and visible-vs-interactive mismatches become testable.
- Static artifacts can omit hit geometry cleanly.
- Adapters need clear definitions of what their render bounds include.
- Geometry queries must not accept an ambiguous generic rectangle.

## Alternatives considered

- One `box` field: compact but semantically unsafe.
- Layout and render only: insufficient for touch-target and occlusion behavior.
- Derive every box from pixels: impossible for interaction and loses source intent.

## Verification

M1 geometry types expose separate optional bounds with no implicit conversion. Tests verify
that rules requesting hit geometry return `untested` or `cantTell` when only render geometry is
available, rather than reusing it silently.
