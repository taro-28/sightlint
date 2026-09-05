# ADR 0025 — Deterministic PNG filter reconstruction

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0023 establishes a bounded, validated zlib stream whose decompressed length exactly matches
the filtered scanline layout declared by `IHDR`. Those bytes are not yet the original PNG row
data: every non-empty scanline begins with a filter-type byte, and the remaining bytes may be
deltas from reconstructed bytes to the left, above, or upper-left.

SightLint needs the original serialized row bytes before it can safely unpack sub-byte samples,
expand palettes, normalize colors, calculate alpha-aware ink bounds, or derive pixel evidence.
Skipping this boundary would make every later image observation depend on unverified filter
handling.

## Decision

Add a deterministic filter-reconstruction stage to `sightlint-adapter-png` and require it from
`adapt_png` after bounded inflation.

The stage must:

- support PNG filter method 0 types `None` (0), `Sub` (1), `Up` (2), `Average` (3), and `Paeth` (4);
- reject any other per-scanline filter byte with the pass and zero-based row in the error;
- apply filters to serialized bytes rather than interpreted pixels;
- calculate filter bytes-per-pixel as `max(1, ceil(channels * bit_depth / 8))`;
- use unsigned wrapping arithmetic modulo 256 for reconstructed output bytes;
- calculate the Average predictor without eight-bit overflow;
- calculate the Paeth predictor exactly and preserve the specification's left, above, upper-left
  tie-breaking order;
- treat missing left, above, and upper-left bytes as zero;
- reset the previous-row reference at the start of the image and at the start of every Adam7 pass;
- consume no filter byte for an empty Adam7 pass;
- preserve packed sub-byte samples and row padding exactly as serialized;
- compact reconstructed row data in the existing inflated allocation rather than retaining a
  second full-size raster allocation;
- retain explicit pass geometry and byte offsets for later sample unpacking and deinterlacing;
- expose exact filter counts, scanline count, non-empty pass count, reconstructed byte count, and
  reconstructed-data CRC-32 in the adapter-private PNG extension;
- bump that adapter-private extension payload from version `0.1.0` to `0.2.0`.

## In-place reconstruction

The inflated stream has one filter prefix before each non-empty row. Reconstructed data is written
forward into the same allocation while the read cursor remains ahead by at least the number of
filter bytes already consumed. Each source byte is copied to a local value before writing the
corresponding earlier destination byte. Previous reconstructed rows and left bytes therefore remain
available without an additional raster-sized buffer.

The output byte sequence concatenates reconstructed rows in PNG transmission order. For an Adam7
image, that means pass order 1 through 7, with no bytes for empty passes. It is not yet the final
full-image pixel order.

## Boundary

This decision does **not**:

- unpack 1-, 2-, or 4-bit samples;
- interpret 8- or 16-bit channel values;
- expand `PLTE` indices;
- apply `tRNS` or alpha semantics;
- deinterlace Adam7 samples into full-image order;
- normalize pixels to RGBA;
- infer foreground, background, regions, text, components, or roles;
- calculate ink bounds or whitespace.

The reconstructed-data CRC is exact adapter metadata useful for reproducibility and E2E oracles.
It is not a visual-quality finding and does not enter the trusted rule kernel.

## Verification

Unit and public-binary E2E coverage must include at least:

- all five filter types with non-trivial left, above, and upper-left predictors;
- Average arithmetic that would overflow an eight-bit intermediate;
- Paeth tie-breaking and predictor edge cases;
- filter bytes-per-pixel for packed sub-byte data and multi-byte pixels;
- the first row's zero prior row;
- Adam7 pass boundaries, empty passes, and prior-row reset between passes;
- an invalid filter type after otherwise valid structure and inflation;
- exact filter counts, pass counts, byte count, and reconstructed-data CRC;
- split `IDAT` input and byte-identical repeated CLI output;
- the existing conformance corpus, product evaluation smoke corpus, MSRV, Linux, macOS, and Windows.

The filter formula and pass-reset behavior follow the W3C PNG specification. Test fixture values
must be independently derived or hard-coded; production reconstruction code must not be used to
produce its own expected output.
