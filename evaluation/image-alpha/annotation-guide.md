# Source-alpha annotation guide 0.1.0

This guide separates the exact acquisition question “which encoded PNG samples have nonzero or
fully opaque alpha?” from the rule question “does transparent padding constitute a defect?” The
first can be answered from supported source samples. The second needs role, placement intent,
policy, and alternatives that this corpus does not provide.

## Acquisition oracle

Annotate from the fixture generator's source primitives and review the rendered asset, never from a
SightLint report.

- `visibleBounds` encloses every sample whose alpha is greater than zero using integer
  `[x, y, width, height]` half-open device-pixel geometry.
- `opaqueBounds` independently encloses samples whose alpha equals 255.
- A missing predicate population is `null`; do not invent a zero rectangle.
- Insets are the transparent rows/columns outside visible bounds in top/right/bottom/left order.
- Counts partition total pixels into alpha 0, alpha 255, and alpha 1–254.
- Each edge count has its own width/height denominator. Corner samples count on both applicable
  edges.
- Hidden RGB under alpha zero must not affect any alpha observation.
- `expectedInkBox` equals `visibleBounds` when present and is otherwise `null`.

These annotations do not describe contours, disconnected-object count, internal holes, composited
appearance, semantic whitespace, alignment quality, or user harm.

## Rule oracle

No alpha-padding rule exists in this slice. Record `expectedOutcome: untested` and
`blockingAllowed: false` for every case. Use:

- `cantTell` when visible content exists but intent/policy is absent;
- `inapplicable` when there is no visible content to which a padding or alignment obligation could
  apply.

List false-positive risks such as optical compensation, animation frames, intentionally flush
badges, export canvases, shadows, and layout compensation. Do not reinterpret exact geometry as a
failure.

## Splits, provenance, and holdout

- `smoke` protects the public command and ordinary asset path.
- `development` exposes targeted mutations and metamorphic invariants.
- `challenge` contains hard negatives that tempt zero-box or asymmetry findings.
- No protected holdout exists. All source, labels, and generator logic are public and visible to
  implementers.

Assets are fictional, repository-owned, dependency-free, and licensed `MIT OR Apache-2.0`. They
must contain no customer content, personal data, third-party marks, or external resources.

## Oracle changes

The deterministic asset generator may regenerate PNG bytes, but it must not write acquisition or
rule annotations and must never consume SightLint output. Change an oracle only after a source or
semantic review explains why the expected fact changed. A failing implementation is not a reason
to weaken the oracle.
