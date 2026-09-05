# ADR 0026 — Exact alpha-visible pixel observations

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0025 establishes bounded PNG-encoded RGBA8 pixels in final destination order. SightLint can now derive its first exact image-space geometry without guessing foreground colors, backgrounds, UI roles, or visual groups.

Transparency is an objective source property. A pixel with alpha greater than zero contributes visible source color under some compositing background; a pixel with alpha zero contributes none. This supports exact bounds for transparent assets and screenshots with transparent padding. It does not identify whitespace inside a fully opaque screenshot.

## Decision

Add a single-pass deterministic alpha-analysis stage over normalized RGBA8 pixels.

The stage records:

- bounds of all pixels with alpha greater than zero;
- bounds of all pixels with alpha equal to 255;
- visible, fully opaque, fully transparent, and translucent pixel counts;
- counts of visible pixels on each outer canvas edge;
- transparent top, right, bottom, and left insets outside the visible bounds;
- whether the image is entirely transparent;
- whether every pixel is visible by the alpha predicate.

Bounds use device-pixel edge coordinates. If visible pixels range from inclusive columns `min_x..=max_x` and rows `min_y..=max_y`, the rectangle is `(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)`.

When a visible bound exists, the PNG image node receives that rectangle as its Artifact IR `inkBox`, with exact local adapter evidence distinct from the source-header evidence. An entirely transparent image has no `inkBox`; absence is not replaced with a zero-area rectangle.

## Terminology and non-claims

The resulting `inkBox` is specifically **alpha-visible ink**. For a fully opaque screenshot it is the full canvas, even when humans would call large regions “blank” or “background.” SightLint must not subtract a corner color, dominant color, white, or any inferred background in this stage.

Background hypotheses, color-distance thresholds, compositing, and semantic whitespace require separate evidence and policy. They must not overwrite the exact alpha observations.

## Evidence and extension compatibility

Use a dedicated evidence record whose selector identifies the normalized RGBA8 alpha channel. The observation is deterministic and local; it carries no probabilistic confidence.

This slice also synchronizes the additive PNG extension contract by changing its pre-release extension version from `0.1.0` to `0.2.0`. The repository has not published a stable extension release. Future additions must update the extension version and corresponding E2E assertion in the same PR.

## Resource boundary

Alpha analysis allocates no image-sized secondary buffer. It scans the RGBA8 vector once with constant-size accumulators. It validates the expected four-byte-per-pixel layout before indexing.

## Verification

Tests must cover at least:

- a transparent border around opaque content;
- semitransparent pixels contributing to visible but not opaque bounds;
- disconnected visible pixels and internal transparent holes;
- a fully opaque image producing full-canvas visible and opaque bounds;
- an entirely transparent image producing no `inkBox`;
- exact transparent insets and edge-visible counts;
- Adam7 input after final-coordinate scattering;
- Artifact IR evidence linkage for `inkBox`;
- the PNG extension version contract;
- byte-identical Artifact IR and reports across repeated runs;
- inherited parser and normalization failures;
- Linux, macOS, Windows, and the repository MSRV.
