# ADR 0024 — Deterministic PNG filter reconstruction

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0023 establishes a bounded, checksum-validated zlib stream whose decompressed length exactly matches the scanline layout declared by `IHDR`. Those bytes are not yet pixel samples: every non-empty PNG scanline begins with a filter selector and the remaining bytes may be transformed by one of five reversible PNG filters.

SightLint needs exact packed sample bytes before it can unpack sub-byte samples, expand palettes, apply transparency, normalize colors, or measure visual regions. Filter reconstruction must remain a deterministic parser operation, not a visual inference step.

## Decision

Add a deterministic filter-reconstruction stage to `sightlint-adapter-png` and require it from `adapt_png` before Artifact IR is emitted.

The stage must:

- consume only scanline bytes that already passed the bounded structure and inflation gates;
- implement PNG filter methods 0 through 4: None, Sub, Up, Average, and Paeth;
- reject every other filter selector with a stable error that identifies the pass and row;
- calculate filter bytes-per-pixel as `max(1, ceil(bits_per_pixel / 8))`, including packed 1-, 2-, and 4-bit samples;
- reset the previous-row state at the beginning of every Adam7 pass;
- reconstruct bytes with PNG's modulo-256 arithmetic;
- compact the inflated buffer in place so filter bytes are removed without allocating a second raster-sized buffer;
- preserve packed samples exactly as encoded by the PNG color type and bit depth;
- record active pass layouts and exact filter-use counts inside the adapter boundary;
- expose the reconstructed packed byte count and a deterministic CRC-32 of those bytes in the namespaced PNG extension so independent E2E fixtures can verify the complete public path.

For a non-interlaced image, the reconstruction contains one logical pass. For Adam7, only non-empty passes are represented, while each descriptor retains its original pass number, origin, step, dimensions, packed row width, byte offset, and byte length.

## Memory and trust boundary

The inflater already allocates the exact filtered scanline size plus one sentinel byte and rejects decoded data above 256 MiB. Reconstruction reuses that buffer with separate read and write cursors. The write cursor always trails the unread source because every row removes one filter byte, so reconstruction does not overwrite future source bytes. After all rows are reconstructed, the buffer is truncated to the exact packed-sample length.

No semantic role, foreground/background classification, text, component hierarchy, ink bounds, or design judgment is produced at this stage. All emitted measurements remain exact parser observations.

## Verification

Tests must cover at least:

- independently encoded examples for all five filters;
- multiple rows so Up, Average, and Paeth depend on reconstructed prior rows;
- modulo-256 underflow and overflow behavior;
- filter bytes-per-pixel for packed, 8-bit, and 16-bit color/depth classes;
- all legal PNG color-type families already accepted by the adapter;
- Adam7 pass geometry and previous-row reset between passes;
- invalid filter selectors in non-interlaced and Adam7 streams;
- exact reconstructed byte count, pass metadata, filter histogram, and packed-byte CRC through the public binary;
- byte-identical Artifact IR and reports across repeated runs;
- inherited decompression and resource-limit failures;
- Linux, macOS, Windows, and the repository MSRV.
