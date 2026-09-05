# ADR 0025 — Staged encoded RGBA8 raster availability

- Status: Accepted
- Date: 2026-09-05

## Context

ADRs 0021 through 0024 establish a strict deterministic path from PNG bytes to reconstructed packed sample rows. SightLint now needs an addressable pixel raster before it can derive image evidence such as alpha bounds, color samples, edges, regions, or whitespace.

Completing every PNG interpretation feature before using pixels would delay the product's actual visual-analysis work and turn SightLint into a general-purpose image codec project. At the same time, silently ignoring palettes, `tRNS`, sub-byte samples, 16-bit samples, or interlace placement would turn inferred or incomplete colors into false exact facts.

The common screenshot path is 8-bit grayscale, grayscale-alpha, RGB, or RGBA PNG. These formats can be expanded and Adam7-scattered deterministically from the data already produced by ADR 0024 without color-model guessing.

## Decision

Add a staged canonical-raster observation to `sightlint-adapter-png`.

The first stage supports exactly:

- bit depth 8;
- color types 0 (grayscale), 2 (RGB), 4 (grayscale-alpha), and 6 (RGBA);
- non-interlaced and Adam7 images;
- images without a `tRNS` chunk.

For supported inputs, produce an **encoded RGBA8 raster**:

- one row-major four-byte `R, G, B, A` pixel per canvas coordinate;
- grayscale replicated into `R`, `G`, and `B`;
- opaque alpha 255 for grayscale and RGB;
- source alpha retained for grayscale-alpha and RGBA;
- Adam7 pass samples scattered to their exact destination coordinates;
- no gamma, ICC, chromaticity, or display-profile transformation.

“Encoded” is part of the type and metadata name because byte values are exact PNG sample values, not yet a claim about a color-managed display value.

Legal inputs outside this first stage must continue to adapt successfully but report raster unavailability with a stable reason. Initial reasons are:

- indexed color;
- unsupported bit depth;
- presence of `tRNS`;
- canonical RGBA buffer exceeding the adapter memory budget.

Unsupported or resource-limited raster production is not malformed input and must not be converted into a parser error. Downstream pixel-dependent rules will eventually turn unavailable raster evidence into `cantTell` or `inapplicable` according to their applicability contract.

## Resource boundary

Cap the canonical RGBA8 allocation at 256 MiB. Calculate `width * height * 4` with checked integer arithmetic before allocation. The existing PNG input, pixel-count, inflated-byte, and reconstructed-layout limits continue to apply independently.

Do not serialize raw raster bytes into Artifact IR. The raster remains inside the adapter/acquisition boundary, while IR receives only exact availability metadata and observations derived from the raster. This avoids enormous JSON documents and keeps pixel storage replaceable.

## Explicit exclusions

This slice does not:

- expand indexed palettes;
- apply `tRNS` transparency;
- unpack 1-, 2-, or 4-bit samples;
- retain or scale 16-bit sample precision;
- perform gamma, sRGB, ICC, or chromatic adaptation;
- derive foreground, background, ink, whitespace, edges, connected regions, text, components, or semantic roles.

These exclusions are observable through the raster-availability status rather than hidden.

## Verification

Tests must include at least:

- exact grayscale, grayscale-alpha, RGB, and RGBA expansion;
- opaque-alpha insertion and source-alpha preservation;
- non-interlaced row-major placement;
- Adam7 scatter with coordinate-unique pixel values and every non-empty pass;
- exact pixel count and byte count;
- indexed, sub-byte, 16-bit, and `tRNS` unavailability without CLI failure;
- checked 256 MiB allocation classification without allocating the oversized raster;
- malformed reconstructed layout as a structured adapter error;
- canonical availability metadata through the public `adapt-image` and `check-image` commands;
- repeated byte-identical CLI output;
- Linux, macOS, Windows, and the repository MSRV.
