# ADR 0027 — Deterministic opaque-border background candidates

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0026 derives exact ink only from alpha. That is intentionally conservative: most interface screenshots are fully opaque, so their alpha-visible bounds cover the complete canvas even when a visually uniform background surrounds the actual content.

SightLint needs evidence that can support later background-relative whitespace and region analysis without silently declaring a corner color to be “the background.” Candidate generation and policy judgment must remain separate.

## Decision

For images whose normalized pixels are all fully opaque, derive a deterministic list of exact source-code-value background candidates from the outer border.

Candidate seeds are:

- every distinct color sampled at the unique geometric corner positions; and
- the four most frequent exact RGBA colors among unique outer-edge pixels.

After deduplication, at most eight candidates remain. For each candidate, record:

- exact RGBA bytes and lowercase `#rrggbbaa` representation;
- occurrences among unique corner positions;
- exact outer-edge pixel count;
- exact whole-image pixel count;
- the total corner, edge, and image denominators needed to calculate ratios without serialized floating-point rounding;
- bounds of all pixels that are not exactly equal to the candidate, or no bounds when the whole image equals the candidate.

Sort candidates by the following stable tuple:

1. descending unique-corner occurrences;
2. descending outer-edge pixel count;
3. descending whole-image pixel count;
4. ascending packed RGBA value.

The first item is the **leading candidate**, not a verified background. No confidence score is invented and no threshold is applied in this slice.

## Applicability

Candidate generation applies only when every normalized pixel has alpha 255. For transparent or translucent images, alpha-derived observations are more direct and the source RGB values may not describe composited appearance. In those cases the analysis records `requiresFullyOpaquePixels` and emits no candidates.

The analysis operates in `pngEncodedRgba8`. It does not apply gamma, ICC, chromaticity, or sRGB transfer data. Exact equality and counts remain valid source-code-value facts, but visual similarity is not claimed.

## Geometry and non-claims

Candidate-relative non-matching bounds are exact for the stated candidate. They are not written to the node `inkBox`, because doing so would promote a hypothesis into a fact. ADR 0026 alpha-visible geometry remains unchanged.

This stage does not:

- choose a background automatically;
- merge near colors;
- account for antialiasing or gradients;
- composite alpha;
- infer whitespace, content, cards, controls, text, or semantic groups;
- emit a blocking rule result.

A later policy or perception layer may consume these candidates and must preserve the selected candidate and decision rationale as evidence.

## Resource boundary

The implementation scans the RGBA buffer without copying it. It stores exact counts only for outer-edge colors, whose unique pixel count is bounded by the perimeter, and at most eight fixed candidate accumulators. The whole-image pass is `O(pixels × candidates)` with a constant candidate cap and no image-sized secondary allocation.

Outer-edge pixels are unique. Corners are not double-counted, and degenerate one-row, one-column, or one-pixel images use each geometric position once.

The additive PNG extension contract advances from `0.2.0` to `0.3.0` in the same change.

## Verification

Tests must cover at least:

- a flat opaque background surrounding a contrasting content rectangle;
- an all-one-color image with no non-candidate bounds;
- deterministic ties among border colors;
- distinct corner colors being retained even when not among the four most frequent edge colors;
- one-row, one-column, and one-pixel edge/corner sampling;
- transparent and translucent images being explicitly inapplicable;
- unresolved color-description chunks without claiming color management;
- preservation of the alpha-derived full-canvas `inkBox` for opaque screenshots;
- exact candidate ordering, counts, denominators, and non-candidate bounds through the public CLI;
- extension version `0.3.0`;
- byte-identical Artifact IR and reports across repeated runs;
- Linux, macOS, Windows, and the repository MSRV.
