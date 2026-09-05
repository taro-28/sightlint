# ADR 0025 — PNG encoded RGBA8 normalization

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0024 produces exact packed, unfiltered PNG samples in non-interlaced or Adam7 pass order. The data is still specific to PNG color types and bit depths: grayscale may be packed at 1, 2, or 4 bits; indexed pixels require `PLTE`; transparency may be represented by `tRNS`; 16-bit channels need an explicit quantization rule before the first common visual-analysis representation can be used.

SightLint needs a predictable pixel representation for alpha-aware geometry, color sampling, connected regions, and later differential adapter tests. It must not silently claim to reproduce final display appearance when a PNG carries gamma, chromaticity, sRGB, or ICC information that has not been applied.

## Decision

Add a deterministic sample-normalization stage that converts validated, reconstructed PNG samples to **PNG-encoded RGBA8**.

“PNG-encoded RGBA8” means:

- channels are expanded to red, green, blue, and alpha bytes in destination-pixel row-major order;
- sub-8-bit grayscale samples are scaled across the complete 0–255 range;
- 8-bit samples are preserved;
- 16-bit samples are deterministically rounded to the nearest 8-bit value using `(sample * 255 + 32767) / 65535`;
- grayscale is replicated to red, green, and blue;
- indexed samples are expanded through `PLTE`;
- `tRNS` is applied against original, unquantized source samples;
- grayscale-alpha and RGBA source alpha is quantized by the same rule;
- Adam7 samples are scattered into final image coordinates;
- no gamma, chromaticity, sRGB transfer, ICC transform, premultiplication, or compositing is applied.

The representation is therefore a deterministic transformation of source code values, not a declaration of color-managed visual appearance. The adapter records whether `gAMA`, `cHRM`, `sRGB`, or `iCCP` chunks are present and explicitly records that color management was not applied. Rules that require rendered colorimetry must not treat these bytes as sufficient evidence when an unresolved transform matters.

## Transparency and palette validation

The stage must validate the `tRNS` contract needed for exact expansion:

- at most one `tRNS`, before the first `IDAT`;
- grayscale `tRNS` contains exactly one 16-bit sample within the declared bit depth;
- truecolor `tRNS` contains exactly three 16-bit samples within the declared bit depth;
- indexed `tRNS` follows `PLTE`, is non-empty, and contains no more alpha entries than the palette;
- grayscale-alpha and RGBA reject `tRNS`;
- indexed sample values must address an existing palette entry.

Unspecified indexed alpha entries are opaque.

## Memory boundary

RGBA8 requires four bytes per destination pixel. Reject output larger than 128 MiB before allocation. The existing packed sample buffer remains alive during expansion, so this separate cap bounds the additional allocation. Typical interface screenshots are many orders of magnitude smaller; larger image-analysis strategies can later use tiles or streaming under a separate ADR.

The output is retained inside the adapter API for later deterministic pixel analysis but is not serialized into Artifact IR. The PNG extension records only bounded verification metadata:

- pixel encoding identifier;
- byte count and CRC-32;
- opaque, transparent, and translucent pixel counts;
- palette and transparency entry counts;
- presence of unresolved color-description chunks;
- `colorManagementApplied: false`.

## Boundary

This stage does not select a background, composite alpha, convert to linear light, calculate contrast, choose foreground/background, find ink bounds, segment components, detect text, or infer semantics.

## Verification

Tests must cover at least:

- grayscale at 1, 2, 4, 8, and 16 bits;
- RGB, indexed, grayscale-alpha, and RGBA source families at legal bit depths;
- deterministic 16-to-8 rounding endpoints and representative midpoint values;
- grayscale and truecolor `tRNS` matching against unquantized samples;
- indexed palette expansion, partial alpha tables, and out-of-range indices;
- forbidden, duplicate, misplaced, malformed, and out-of-range `tRNS` chunks;
- Adam7 scattering into final row-major destination order;
- the 128 MiB pre-allocation boundary;
- exact RGBA8 CRC and alpha-class counts through the public CLI;
- byte-identical Artifact IR and reports across repeated runs;
- inherited structure, inflation, and filter failures;
- Linux, macOS, Windows, and the repository MSRV.
