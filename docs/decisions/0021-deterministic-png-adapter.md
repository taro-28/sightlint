# ADR 0021 — Deterministic PNG adapter boundary

- Status: Accepted
- Date: 2026-09-04

## Context

M3 introduces pixels as a common SightLint input. The first adapter must prove the binary-input → evidence-backed Artifact IR → deterministic engine path without smuggling probabilistic semantics into the trusted kernel.

A PNG header contains useful facts that can be extracted exactly without decoding image samples: dimensions, bit depth, color type, compression method, filter method, and interlace method. These facts are sufficient to establish a real image coordinate space and provenance boundary before connected-region, edge, ink, whitespace, or color-sampling work begins.

## Decision

Create a Rust crate named `sightlint-adapter-png`. The first version is dependency-light and parses only the PNG signature and first `IHDR` chunk.

The adapter must:

- require the standard eight-byte PNG signature;
- require `IHDR` to be the first chunk and its data length to be exactly 13 bytes;
- validate the `IHDR` CRC-32 before trusting its fields;
- reject zero dimensions;
- reject dimensions above 100,000 pixels on either axis and images above 100,000,000 total pixels before later decoding work can allocate based on them;
- validate the PNG bit-depth/color-type combinations defined by the PNG specification;
- require compression method 0 and filter method 0;
- accept only interlace method 0 or 1;
- emit one `Image` artifact, one device-pixel canvas, and one full-raster image node;
- mark header-derived observations as `ExactSource` evidence from the local `sightlint-adapter-png` adapter;
- expose exact PNG header metadata under the namespaced artifact extension `org.sightlint.adapter.png`;
- validate the produced Artifact IR before returning it.

The first version does **not** decode `IDAT`. Therefore it does not claim visible ink bounds, connected components, text regions, semantic roles, foreground/background colors, transparency-adjusted content bounds, or perceptual meaning. The full raster rectangle is a render/source extent, not an ink extent.

## Determinism

Stable identifiers are fixed as `artifact`, `canvas`, `image`, and `evidence:png-header`. Coordinates use `devicePixel`, increase rightward and downward, and originate at `(0, 0)`. Canonical Artifact IR serialization remains the responsibility of `sightlint-ir`.

No network, wall-clock time, random identifier, locale-dependent parsing, image codec library, OCR, CV model, or VLM is involved in this slice.

## CLI surface

Add:

- `sightlint adapt-image <png>` — emit canonical Artifact IR;
- `sightlint check-image <png>` — adapt the image, run the same deterministic rule engine, and emit the same human/JSON report contract as `check`.

Both accept `-` for binary standard input. Binary image input has a separate 64 MiB read limit; parser-level dimension and pixel-count limits remain independent defense-in-depth controls.

## Verification

Unit and public-binary E2E coverage must include at least:

- valid RGB, RGBA, grayscale, and indexed PNG headers;
- Adam7 interlace metadata;
- invalid signature;
- truncated header/chunk;
- non-`IHDR` first chunk;
- invalid `IHDR` length;
- CRC mismatch;
- zero dimensions;
- excessive dimension and pixel count;
- invalid color type and invalid bit-depth/color-type pair;
- invalid compression, filter, and interlace methods;
- canonical IR containing exact provenance and no invented ink/semantic observations;
- `check-image` exercising the real engine and report path;
- repeated byte-identical adapter and report output.

Later pixel-analysis work requires a separate decision or an explicit extension to this ADR because decoding and region extraction introduce new resource, interpretation, and quality boundaries.
