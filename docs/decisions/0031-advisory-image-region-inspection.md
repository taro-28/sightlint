# ADR 0031 — Advisory image-region inspection

- Status: Accepted
- Date: 2026-09-05
- Builds on: ADR 0030 and the verified PR #20 raster boundary

## Problem and decision

The next experiment must test useful structure acquisition, not add another image codec. A
source-raster test cannot prove spacing detection. Conversely, different measured gaps do not
prove a UX defect: repeated shapes can have different meanings or intentional grouping.

Add `sightlint inspect-image INPUT [--format human|json]`. It uses the same validated PNG and
RGBA acquisition API, then emits a separately versioned, advisory-only observation report.
Existing `adapt-image`, `check-image`, Artifact IR, CheckReport, and their exit policies are
unchanged. This avoids smuggling a heuristic verdict into the trusted kernel or changing the
meaning of existing reports. Inspection needs no per-image user configuration.

## Observation contract

Version 0.1.0 supports one deliberately conservative hypothesis: the entire perimeter has
one identical opaque RGBA value and the complete raster is opaque. That perimeter color is
a background *candidate*, not confirmed background. Transparency, varying borders, unsupported
raster formats, and exceeded inspection budgets produce explicit unavailable reasons.

Under that hypothesis:

1. Identify four-connected components of pixels different from the candidate color.
2. Record their half-open bounds, pixel counts, stable coordinate-derived IDs, and whether
   they are solid, single-color rectangles. Bounds are in source device pixels.
3. Propose groups of at least three such rectangles with identical size and color, sharing
   the same horizontal row or vertical column. Compare geometry, not semantic roles.
4. Reject a proposed group if another component's bounds intersect the intervening strip.
5. Report ordered gaps, minimum, maximum, and spread. `uniform` and `unequal` are exact
   descriptions of these integer measurements, with zero measurement tolerance. They are
   not `passed` or `failed` UX outcomes.

Every report identifies source/raster evidence, algorithm version, candidate assumptions,
uncalibrated semantic confidence, coordinate units, and external-processing=false. Confidence
is not invented as a numeric probability. The UX verdict remains `cantTell` when observations
are available and `untested` when acquisition is unavailable. Unequal gaps receive an advisory
asking whether the proposed repetition should be uniform. No font-size, card/button role,
reading-order, contrast, universal quality score, or component hierarchy is claimed.

## Resource and privacy boundary

Inspection is local and performs no network calls. At most 4,194,304 pixels and 1,024 components
are analyzed. Visited and flood-fill buffers are bounded by the pixel budget and use fallible
allocation. Unsupported/over-budget analysis returns no partial regions or groups. Malformed
PNG or allocation/internal-layout failure remains an input/execution error. Region grouping
uses stable sorting and bounded collections; it does not depend on traversal randomness,
wall-clock time, floating point, or locale.

This budget is additional to existing PNG source and raster limits. It does not replace them,
and checking it after raster acquisition does not claim to avoid the earlier decoder costs.

## CLI and evaluation

Inspection exits 0 for observations or explicit unavailable coverage, never 1 for a heuristic
advisory. Exit 2 means invalid input, usage, I/O, or execution error. Human output prominently
says advisory-only. Machine output has its own report version, not the CheckReport version.

The existing committed card pair must yield regions matching its independently declared
bounds and gaps [1,1] versus [1,2]. Its original future UX oracle remains untested: this slice
measures the pattern but does not establish semantic applicability of a spacing rule.

Commit native PNG input bytes and separately authored expected regions/groups. Test horizontal
and vertical patterns, translation, scaling, recoloring, multiple groups, blockers, differing
sizes/colors, one/two components, nonrectangles, touching/diagonal components, uniform images,
alpha, ambiguous borders, malformed inputs, unsupported rasters, and budgets. Public-binary E2E
must verify file/stdin/API equality, human/JSON output, exact gaps, deterministic repetitions,
exit codes, no false blocking, and preserved ordinary check-image results. Run the new corpus
and its reproducibility check in normal read-only CI on every PR; keep the existing suites.

## Limitations and next evidence

This is a flat-artwork prototype. Rounded cards, shadows, text, photographs, antialiasing,
intentional nonuniform grouping, and unknown screenshot scale can defeat the hypothesis.
Synthetic successes are not real-world accuracy. Before automatic spacing failures or broader
claims, reconcile native/semantic evidence, add realistic annotated artifacts and negative
controls, and evaluate precision/coverage. Native adapters and isolated perception remain the
planned routes to richer structure. Do not expand this into a hand-written general vision model.
