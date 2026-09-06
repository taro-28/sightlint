# ADR 0039 — Evaluate broader background hypotheses without changing the strict default

- Status: Accepted
- Date: 2026-09-06
- Issue: #25
- Builds on: ADRs 0016, 0024, 0030, 0031, 0032, and 0033

## Context

`inspect-image` deliberately accepts only an opaque raster with one exact perimeter color. It then
uses bounded four-connected flood fill. The policy is easy to explain and safely abstains on the
repository-owned Atlas dashboard screenshot because navigation and application surfaces reach the
viewport edge. It therefore has low realistic coverage, but a wider exact-color guess can mistake
headers, sidebars, gradients, illustrations, overlays, or other edge content for a canvas
background.

Superseded PRs #15 and #17 proposed ranked corner/edge colors and a 95%-qualified corner candidate
with row-run union-find. Their branches and decisions are not accepted implementation sources.
Issue #25 retains only the hypotheses to evaluate from current `main`.

A segmentation benchmark also needs two separate truths. Pixel acquisition can be compared with
human-authored visible-surface regions. Whether those regions are semantic peers or constitute a
UI/UX defect is a different rule question. The implementation being measured must never write its
own region oracle.

## Decision

Keep `inspect-image` and `uniform-perimeter-four-connected-v1` unchanged as the product default.
Add a separate, experimental `benchmark-image-segmentation` command and native adapter API. They
compare exactly three named acquisition policies over the same locally decoded PNG raster:

1. `strict-uniform-perimeter-flood-v1`: the accepted ADR 0031 opaque/unanimous-perimeter policy and
   four-connected flood fill;
2. `ranked-exact-border-flood-v1`: candidates are distinct corner colors plus the four most
   frequent exact colors on unique perimeter pixels, deduplicated and capped at eight; ranking is
   descending corner count, edge count, and whole-image count, then ascending packed RGBA; the
   leading candidate is segmented with the same flood fill;
3. `qualified-corner-95-row-runs-v1`: an opaque canvas of at least 3 by 3 pixels must have a corner
   color supported by at least 95% of unique perimeter pixels, using the integer test
   `edge_count * 100 >= perimeter_count * 95`; the leading qualifying candidate is segmented with
   maximal horizontal runs and deterministic union-find under four-connectivity.

Every candidate remains an unconfirmed hypothesis. Ranked observation is not evidence that the
leading color is semantically a background. The command is advisory-only, never emits a rule
result, never exits 1, and cannot affect `check`, `check-image`, or `inspect-image`. Its report has
an independent `0.1.0` schema and records candidate denominators, selection, algorithm, resource
limits, deterministic work counters, exact regions, and explicit unavailability. It records no
wall-clock duration in canonical output.

The benchmark corpus is a repository-owned Web UI fixture captured by the existing isolated
Playwright adapter. Human-authored acquisition annotations define visible-surface targets and
whether a single global exact background is usable for each case. A separate rule-oracle document
records `untested` when no executable rule exists and `cantTell` or `inapplicable` semantic
applicability. Captured screenshots and implementation reports are temporary test outputs, not
stored oracles.

## Resource, privacy, and determinism boundary

All policies retain the 4,194,304-pixel inspection limit. Flood-fill policies retain the 1,024
region limit. Row-run segmentation adds a 250,000-run limit and also retains the 1,024 emitted
region limit; this is intentionally lower than the historical 50,000-component proposal because
canonical JSON output must remain bounded. Any exceeded limit discards partial regions.

Candidate counting, ranking, run creation, union, component aggregation, and output sorting use
integers and stable coordinate/color order. Reports include deterministic work counters rather
than unstable timing. Tests compare repeated bytes and verify flood/run equivalence where both use
the same candidate. Coarse wall-clock measurements may be reported as environment-specific
diagnostics but are not a correctness oracle.

The browser remains an untrusted process sensor and the PNG adapter remains outside the trusted
rule kernel. Inputs and temporary screenshots stay local, network access is denied, and no full DOM
or arbitrary page text is added to the segmentation corpus. The fixture contains fictional data,
no external assets, and is covered by `MIT OR Apache-2.0`. No private/customer/personal data is
permitted.

## Evaluation and admission criteria

The corpus covers a uniform realistic dashboard, one-pixel edge contamination, recoloring,
translation, device-scale variation, a modal surface, split panes, a gradient/illustration edge,
and checkerboard stress. Public labels are smoke/development/challenge data, not a private holdout.
The contract reports, per policy:

- usable-case observation coverage;
- one-to-one region precision and recall plus bounds error for annotated surfaces;
- unsafe-background observations and correct abstentions on hard negatives;
- fragmentation and false grouping;
- deterministic agreement and logical work/run/component counts;
- metamorphic stability for declared translation, recoloring, and device-scale relations.

No broader policy may replace the strict `inspect-image` default merely because it gains coverage.
Admission needs representative independently reviewed data, acceptable unsafe-background and
downstream rule false-positive rates, and a separate ADR. The initial corpus is one fictional app
family with maintainer-authored labels and cannot establish those conditions.

## Consequences

- Candidate ideas become executable and comparable without being promoted to product truth.
- The ranked policy is expected to expose why coverage alone is misleading on multi-surface and
  textured edge cases.
- The qualified policy can demonstrate narrow recovery from minor edge contamination while still
  abstaining on many realistic layouts.
- Row-run behavior and worst-case refusal become testable without replacing the reviewed flood-fill
  implementation.
- Browser capture adds a Linux product-evaluation path; Rust unit and synthetic tests remain
  cross-platform.

## Alternatives considered

### Replace `inspect-image` with the highest-coverage candidate

Rejected. The public development corpus is too small and visible, and an observed component is not
a semantic UI object or a rule verdict.

### Merge or cherry-pick superseded PR #15 or #17

Rejected. Their branches predate the recovered architecture, evaluation contracts, current
resource model, and protected `main`.

### Put candidate selection in the deterministic rule kernel

Rejected. Background selection and segmentation are untrusted acquisition concerns. Only reviewed
evidence with explicit applicability may feed blocking rules.

### Store captured screenshots and generated reports as expected data

Rejected. That would couple the oracle to browser/implementation output and conceal regressions.

### Add color clustering, morphology, contours, or a vision model now

Deferred to issue #28 or a later evaluated adapter change. Exact-color candidates are sufficient
to answer the bounded #25 comparison without starting a general computer-vision subsystem.

## Non-goals

- changing the trusted kernel, CheckReport, rule outcomes, or blocking policy;
- identifying semantic peers, text, cards, controls, hierarchy, or design intent from pixels;
- claiming real-world segmentation or UI/UX accuracy;
- establishing a private holdout or independent reviewer agreement;
- color management, alpha compositing, lossy-image robustness, cross-platform screenshot byte
  identity, or arbitrary-site support;
- making ranked or 95%-qualified candidates the zero-setup default.
