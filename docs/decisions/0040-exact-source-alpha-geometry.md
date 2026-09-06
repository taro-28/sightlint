# ADR 0040 — Exact source-alpha geometry for supported PNG assets

- Status: Accepted
- Date: 2026-09-06
- Issue: #26
- Builds on: ADRs 0013, 0015, 0016, 0021, 0030, and 0039

## Context

The PNG adapter exposes exact unassociated RGBA8 source samples for its supported subset, but the
image node still has only a full-canvas `renderBox`. Transparent padding can therefore hide a real
source-versus-ink offset in an icon, logo, or exported asset. Alpha is sufficient to locate encoded
samples that contribute nonzero source opacity. It is not sufficient to establish composited
display color, semantic whitespace, a contour, or a UI/UX defect.

The superseded PR #14 explored this area on an obsolete integration branch. Its code and branch
decision are not implementation inputs. Issue #26 restates the bounded behavior to implement from
the current verified raster path.

## Decision

For every available row-major `pngEncodedRgba8` raster, make one deterministic pass with constant-
size accumulators and define:

- a source-visible sample as `alpha > 0`;
- a source-opaque sample as `alpha == 255`;
- a source-translucent sample as `0 < alpha < 255`;
- a source-transparent sample as `alpha == 0`.

Record the half-open enclosing bounds of visible and opaque samples independently. Either bound is
`null` when its predicate has no matching sample. When visible bounds exist, record top, right,
bottom, and left transparent insets outside that rectangle. When no visible sample exists, insets
are `null`. Count total, visible, opaque, translucent, and transparent samples. For each outer edge,
record visible count and its own denominator; a corner contributes to both edges it belongs to.
Also record `entirelyTransparent` and `allPixelsVisible`.

Attach the visible bound to the image node as `inkBox`, in device pixels in the existing canvas,
using a dedicated `evidence:png-alpha` exact-source record whose selector is
`IDAT/encoded-rgba8-v1/alpha8`. If the image is entirely transparent, omit `inkBox`; do not invent a
zero-area rectangle. Keep the full-canvas `renderBox` and its IHDR evidence unchanged. Layout and
hit geometry remain absent.

Version the enclosing `org.sightlint.adapter.png` extension as `0.2.0` and add
`alphaGeometry@0.1.0`. The existing `encodedRgba8Raster@0.1.0` contract remains unchanged. The
strict extension schema is committed under `crates/sightlint-adapter-png/schemas/`. If raster
interpretation is unsupported, alpha geometry repeats that explicit unavailable reason, provides
no counts or bounds, and does not add alpha evidence or `inkBox`.

## Evidence and semantic boundary

Alpha geometry is exact for the validated encoded source samples under the predicates above. It is
not an exact statement about final displayed visibility. PNG color management, compositing onto a
background, CSS/object transforms, masks, filters, opacity, clipping, renderer behavior, and
occlusion are not applied. Hidden RGB under zero alpha is preserved in raster acquisition but has
no effect on alpha geometry.

The adapter does not create a rule, finding, peer relation, severity, or blocking decision. Whether
transparent padding is intentional, harmful, or compensated by layout remains `cantTell`; where
there is no visible content the proposed padding question is `inapplicable`. A future rule requires
its own policy/applicability contract and independently reviewed product evidence.

## Evaluation and data governance

Extend the procedural PNG conformance corpus with independently generated alpha oracles, including
transparent borders, translucent-only content, disconnected samples, internal holes, hidden RGB,
edge occupancy, fully opaque images, entirely transparent images, and Adam7 inputs.

Add a separate repository-owned Northstar transparent-asset family for product-path regression.
Its binary PNGs are deterministically generated from reviewed source primitives, while acquisition
and rule annotations remain human-authored files. The generator must never consume SightLint output
or rewrite the oracles. Record ownership, `MIT OR Apache-2.0`, fictional content, privacy, public
smoke/development/challenge splits, hard negatives, and the absence of a protected holdout. Public-
binary E2E must compare the acquired IR with those annotations and keep the rule oracle `untested`.

These public, maintainer-reviewed assets prove the bounded alpha predicate and product command
path. They do not estimate real-world prevalence, user benefit, rule precision, or general UI/UX
accuracy.

## Resource, privacy, and determinism boundary

Validate `width * height * 4` against the available raster length before indexing. Use checked
integer arithmetic and no image-sized secondary allocation. The existing 256 MiB raster allocation,
100,000,000 source-pixel, and input limits remain the governing upstream bounds.

Processing is local, sends no data, uses no network, clock, randomness, floating-point comparison,
model, OCR, CV, or VLM, and emits no raw pixels. Stable evidence IDs and canonical IR ordering make
file/stdin/API outputs byte-identical for the same declared input.

## Consequences

- Supported transparent assets gain exact source-alpha bounds, counts, insets, and edge occupancy.
- The core IR schema does not change because `inkBox` and evidence linkage already exist.
- Public `adapt-image` output and its PNG extension version change after alpha.2; compatibility and
  release notes must identify this as an unreleased surface change.
- Fully opaque supported inputs now carry an independently evidenced full-canvas `inkBox`.
- Unsupported rasters and entirely transparent assets remain explicit rather than receiving made-up
  geometry.

## Alternatives considered

### Keep alpha data only in a side report

Rejected. The core already models `inkBox`, and exact source-alpha geometry is useful to medium-
neutral geometry queries when its predicate and provenance are explicit.

### Treat `alpha == 255` as the only visible predicate

Rejected. That would erase semitransparent antialiasing, shadows, and artwork. Opaque bounds remain
a separate observation.

### Composite onto a default background

Rejected. There is no universally correct background, color space, transfer function, or later
application opacity. Such a result would be a different rendered observation.

### Emit zero bounds for an entirely transparent image

Rejected. A zero rectangle has an invented origin and can be confused with a real empty layout.
Absence plus exact counts states the evidence faithfully.

### Add an automatic transparent-padding finding

Rejected. The acquisition fact does not establish role, placement intent, policy, or harm.

## Verification

- strict schema validation for PNG extension `0.2.0` and alpha geometry `0.1.0`;
- generator drift for procedural and realistic assets;
- unit tests for predicates, bounds, insets, counts, edge denominators, hidden RGB, degenerate
  dimensions, invalid raster layout, and constant-space behavior;
- existing raster corpus cases plus targeted transparent, translucent, hole, disconnected, edge,
  fully opaque, fully transparent, and Adam7 cases;
- API, file, stdin, canonicalization, repeated-byte, `adapt-image`, and `check-image` E2E;
- separate realistic acquisition and rule oracles, hard negatives, mutation/metamorphic checks,
  abstention, and no unexpected blocking;
- all existing gates, Rust 1.85.0, Linux, macOS, Windows, exact PR head, and merged `main` CI.
