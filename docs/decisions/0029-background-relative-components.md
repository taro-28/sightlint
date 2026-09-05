# ADR 0029 — Bounded background-relative component hypotheses

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0027 produces ranked exact border-color candidates but deliberately does not choose a verified background. The layered corpus in ADR 0028 includes a clean dashboard and a targeted peer-gap mutation whose card regions are not yet recoverable from pixels.

The next step toward useful screenshot linting is deterministic region acquisition under an explicit, inspectable hypothesis. It must abstain when the border evidence is weak and must not turn a heuristic background choice into exact Artifact IR nodes or blocking findings.

## Decision

Add an experimental background-relative segmentation stage using policy
`opaque-border-components-v1`.

The policy qualifies the leading ADR 0027 candidate only when:

- candidate analysis is applicable to fully opaque pixels;
- the canvas is at least 3 × 3 device pixels;
- the candidate occurs at every unique geometric corner; and
- the candidate occupies at least 95% of unique outer-edge pixels, evaluated with integer
  arithmetic as `candidate_edge_count * 100 >= edge_sample_count * 95`.

A qualified candidate remains a hypothesis. The stage records its exact color and support counts.
It never rewrites the alpha-derived `inkBox`.

## Segmentation

Treat every pixel that is not exactly equal to the qualified candidate in `pngEncodedRgba8` as
foreground for this hypothesis. Extract maximal four-connected components using row runs and a
deterministic union-find:

- horizontal adjacency is represented by one maximal run;
- runs in adjacent rows are joined only when their half-open x intervals overlap;
- diagonal contact alone does not connect components;
- no near-color threshold, antialias merge, dilation, erosion, or gap closing is applied.

For each component, record:

- stable index after canonical sorting;
- device-pixel bounds;
- exact foreground pixel count;
- run count;
- whether it touches an outer canvas edge.

Sort components by top, left, height, width, pixel count, then run count. This order is independent
of union-find root identity.

## Outcomes and abstention

The stage has explicit statuses:

- `available`
- `requiresFullyOpaquePixels`
- `imageTooSmall`
- `noQualifiedBackgroundCandidate`
- `runLimitExceeded`
- `componentLimitExceeded`

Abstention and resource limits do not reject an otherwise valid PNG. They emit no components and
preserve prior exact observations.

## Resource boundary

Use at most 250,000 row runs and 50,000 final components. The run vector, union-find parent vector,
and previous/current row indexes are bounded by these limits. No full-size foreground mask or
second RGBA buffer is allocated.

The implementation is linear in pixels plus run-overlap operations. A checkerboard-like image can
exceed the run limit; this becomes `runLimitExceeded`, not an adapter error.

## Evidence and non-claims

Component metadata must include the policy identifier and selected candidate support. It remains
inside the namespaced PNG extension and is not emitted as core `Node` or `Relation` data yet.
Therefore existing deterministic visual rules do not consume it and CI cannot block on it.

This stage does not infer cards, buttons, text, repeated groups, reading order, spacing obligations,
or UX quality. A later promotion step requires evaluation evidence, explicit provenance, and an IR
contract for inferred structures.

The additive pre-release PNG extension advances from `0.3.0` to `0.4.0`.

## Verification

Tests must cover at least:

- clean and spacing-mutated dashboard corpus cases producing navigation plus three card components;
- exact component bounds and horizontal gaps matching corpus ground truth;
- a unanimous-corner but insufficient-edge candidate causing abstention;
- the multi-color border tie causing abstention;
- transparent/translucent input causing explicit inapplicability;
- one- and two-pixel dimensions causing `imageTooSmall`;
- four-connectivity keeping diagonal pixels separate;
- union across overlapping adjacent-row runs;
- run and component limit outcomes through reduced internal test limits;
- stable component ordering independent of union roots;
- unchanged alpha-derived `inkBox`;
- extension version `0.4.0`;
- byte-identical Artifact IR and reports across repeated runs;
- Linux, macOS, Windows, and the repository MSRV.
