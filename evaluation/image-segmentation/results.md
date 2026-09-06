# Image-segmentation benchmark results

These results belong to ADR 0039 and corpus `0.1.0`. They are regression evidence for one
repository-owned fictional application family, not representative real-world accuracy.

## Reviewed acquisition metrics

The public-process E2E captures nine states with Linux Chromium and invokes the built
`sightlint benchmark-image-segmentation` command twice per screenshot. Region matches use the
human-authored source rectangles and tolerances in `annotations/acquisition.json`.

| Policy | usable coverage | region precision | region recall | false groups | unsafe hard-negative hypotheses | correct required abstention | edge mutation observed |
|---|---:|---:|---:|---:|---:|---:|---:|
| strict uniform perimeter + flood fill | 5/6 | 1/5 | 1/21 | 4 | 0/0 | 3/3 | 0/1 |
| ranked exact border + flood fill | 6/7 | 2/7 | 2/27 | 5 | 2/2 | 0/0 | 1/1 |
| 95%-qualified corner + row runs | 6/7 | 2/7 | 2/27 | 5 | 0/0 | 2/2 | 1/1 |

The denominator for usable coverage differs because strict selection correctly declines the
edge-contaminated case before selecting a background. Unsafe observations are not counted as useful
coverage. Ranked selection measures both split-pane and gradient hard negatives, but its selected
color is not a globally valid background in either case.

The modal surface and 20-by-1 edge indicator are the only annotated surfaces matched within their
source-derived edge tolerances. In the dashboard cases, antialiased shadows connect the header,
cards, and activity panel into one exact non-background component. That component intersects
multiple targets and is recorded as false grouping, not quietly credited as a card or container.
Row-run union-find and flood fill produce identical four-connected regions whenever they use the
same qualified candidate.

There is no executable downstream rule. Rule outcome and rule mutation kill rate are `untested`,
with zero eligible rule mutations. A correct component would still not prove semantic peers,
spacing intent, or a defect.

## Coverage and bounded refusal examples

- A one-pixel-high edge status indicator makes strict acquisition unavailable; the 95%-qualified
  policy recovers the source-authored canvas candidate because integer perimeter support exceeds
  95%.
- Split panes and a gradient/illustration produce no 95%-qualified corner candidate, so the
  qualified policy abstains before segmentation.
- Ranked selection observes 25 regions for the split pane and 580 for the gradient, demonstrating
  why a result count is not evidence that the chosen background is valid.
- The checkerboard has an exact white perimeter. Flood fill reaches `regionBudgetExceeded`; row-run
  processing reaches `runBudgetExceeded` at the 250,001st attempted run. Every policy returns no
  partial regions.
- The uniform dashboard has 691,200 input pixels. The qualified implementation records 612 row
  runs and one merged 568,028-pixel component.

## Performance diagnostic

Wall-clock and resident-memory values are diagnostics, not canonical report fields or CI
thresholds. One local arm64 macOS run used Rust 1.90.0, Node 22.23.2, Playwright 1.63.0, and
Chromium 153.0.8010.12. The optimized command evaluates all three policies sequentially, including
PNG validation and decoding:

| Screenshot | five warm real/user runs | one maximum resident-set observation |
|---|---:|---:|
| uniform dashboard, 960×720 | 0.04 s / 0.03 s each | 12,697,600 bytes |
| checkerboard stress, 960×720 | 0.03 s / 0.03 s each | 13,582,336 bytes |

These figures are not cross-platform promises and do not isolate one policy. Deterministic report
counters and hard limits are the regression gate: 4,194,304 pixels, 1,024 regions, and 250,000 row
runs. A future performance claim requires a dedicated benchmark runner, named hardware, repeated
statistics, and per-policy isolation.

## Decision

Neither broader policy replaces `inspect-image`. The qualified policy gains one narrow acquisition
case without selecting either reviewed hard negative, but its semantic region precision/recall is
still poor because exact connected components merge realistic shadowed surfaces. Ranked selection
adds the same useful case while selecting both unsafe hard negatives. More exact-color tuning would
not solve the layer/semantic problem demonstrated here.

The comparison command remains evaluation-only and nonblocking. Any production admission requires
new representative independently reviewed evidence, downstream rule precision, and a separate ADR.
Color clustering, alpha compositing, contours, and learned perception remain outside this slice.
