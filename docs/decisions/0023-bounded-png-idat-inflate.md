# ADR 0023 — Bounded PNG IDAT inflation

- Status: Accepted
- Date: 2026-09-04

## Context

ADR 0022 guarantees that a PNG presented to the public image adapter has a bounded, CRC-valid chunk stream with consecutive `IDAT` chunks. The bytes inside those chunks are still attacker-controlled zlib/DEFLATE data. Structural validity alone does not prove that the compressed stream is valid, that its Adler-32 checksum matches, or that decompression terminates within a safe memory budget.

The next M3 step needs exact decompressed scanline bytes before PNG filters, pixel samples, colors, or ink can be interpreted. This boundary must be deterministic and must not allocate according to an unbounded decompressed size.

## Decision

Add a bounded IDAT inflation stage to `sightlint-adapter-png` and require it from `adapt_png` before Artifact IR is emitted.

The stage must:

- run only after the complete chunk stream passes ADR 0022 validation;
- feed consecutive `IDAT` payload slices to one zlib stream in file order without concatenating the compressed payload into a second large buffer;
- validate the zlib wrapper and Adler-32 checksum;
- require the zlib terminator to consume all non-empty bytes in the concatenated `IDAT` stream, rejecting trailing compressed payload;
- calculate the exact expected decompressed scanline byte count from `IHDR`, including one filter byte per scanline;
- support both non-interlaced data and all seven Adam7 passes when calculating the expected byte count;
- reject an expected decompressed stream larger than 256 MiB before allocating it;
- allocate the exact expected scanline byte count plus one sentinel byte so valid split trailers can complete and any decoded-byte overflow is observable;
- reject compressed streams that fail inflation, produce the sentinel byte, or produce fewer bytes than expected;
- expose the exact validated decompressed byte count in the namespaced PNG extension;
- keep the decompressed scanline bytes inside the adapter boundary for later deterministic filter reconstruction.

Use `miniz_oxide` as the initial inflater. It is a pure-Rust DEFLATE/zlib implementation. SightLint uses its low-level decompressor rather than the convenience slice helper so that consumed compressed bytes remain observable. Pin the direct workspace dependency exactly and keep it outside the deterministic rule kernel.

## Scanline sizing

For a pass width `w`, calculate packed row bytes as `ceil(w * bits_per_pixel / 8)`, then add one filter byte for each non-empty row. Non-interlaced images contain one pass covering the whole image.

Adam7 sizing uses the standard seven passes:

1. start `(0, 0)`, step `(8, 8)`
2. start `(4, 0)`, step `(8, 8)`
3. start `(0, 4)`, step `(4, 8)`
4. start `(2, 0)`, step `(4, 4)`
5. start `(0, 2)`, step `(2, 4)`
6. start `(1, 0)`, step `(2, 2)`
7. start `(0, 1)`, step `(1, 2)`

Empty passes contribute zero bytes.

## Boundary

This decision does **not** reconstruct PNG filters, unpack sub-byte samples, expand palettes, apply transparency, normalize to RGBA, infer foreground/background, calculate ink bounds, or expose semantic claims. Inflation establishes only that the compressed byte stream exactly matches the byte count required for the declared raster geometry.

## Verification

Tests must cover at least:

- valid zlib-wrapped scanline data through the public `adapt-image` and `check-image` paths;
- a zlib stream split across multiple consecutive `IDAT` chunks, including a split immediately before Adler-32;
- empty `IDAT` chunks around an otherwise complete stream where structurally legal;
- rejection of non-empty bytes after the complete zlib stream, both within one `IDAT` and in a following `IDAT`;
- each PNG color type and legal bit-depth class used by current M3 fixtures;
- Adam7 expected-size calculation, including small images with empty passes;
- corrupt DEFLATE data and corrupt Adler-32;
- decompressed output shorter and longer than the exact expected byte count;
- the 256 MiB decoded-byte budget without attempting an oversized allocation;
- byte-identical adapted IR and reports across repeated runs;
- Linux, macOS, Windows, and the repository MSRV.
