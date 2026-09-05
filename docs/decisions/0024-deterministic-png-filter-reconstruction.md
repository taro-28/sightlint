# ADR 0024 — Deterministic PNG filter reconstruction

- Status: Accepted
- Date: 2026-09-05

## Context

ADR 0023 establishes a bounded, checksum-validated PNG `IDAT` inflation boundary. The resulting bytes still contain one filter-type byte before each scanline and filtered sample bytes whose values depend on reconstructed neighbors. They are not yet exact packed sample bytes and therefore cannot safely support color, alpha, edge, whitespace, or ink analysis.

PNG filtering is deterministic but has format-specific details that are easy to implement incorrectly: the predictor byte width differs from packed row width, sub-byte samples still use a predictor width of one byte, arithmetic wraps modulo 256, the previous row is local to a pass, and Adam7 starts a new prediction history for every non-empty pass.

## Decision

Add a deterministic filter-reconstruction stage to `sightlint-adapter-png` and require it from `adapt_png` before Artifact IR is emitted.

The stage must:

- run only after bounded complete-stream validation and bounded zlib inflation;
- support PNG filter types `None` (0), `Sub` (1), `Up` (2), `Average` (3), and `Paeth` (4);
- reject every other filter byte with a stable structured error containing the original Adam7 pass index and row index;
- calculate packed row width as `ceil(pass_width * bits_per_pixel / 8)`;
- calculate filter bytes-per-pixel as `max(1, ceil(bits_per_pixel / 8))`, including one byte for 1-, 2-, and 4-bit packed samples;
- reconstruct with modulo-256 byte arithmetic;
- treat unavailable left, upper, and upper-left bytes as zero;
- reset previous-row state at the start of every non-empty Adam7 pass;
- retain Adam7 pass geometry needed by a later sample-unpacking and raster-scatter stage;
- produce the exact concatenation of reconstructed packed rows without filter bytes;
- reject any internal mismatch between the scanline layout declared by `IHDR` and the inflated input;
- expose only exact byte and pass counts in the PNG extension; raw sample bytes remain inside the adapter boundary.

The reconstructed representation remains packed. Palette indices, sub-byte samples, 16-bit sample values, `tRNS`, alpha, and Adam7 scattering are not interpreted in this slice.

## Evidence and determinism

Filter reconstruction is an exact transformation of source bytes already validated by ADRs 0022 and 0023. It performs no inference, network access, wall-clock reads, randomization, or floating-point computation. The transformation must return byte-identical output for identical input on every supported platform.

## Verification

Tests must include at least:

- independent forward encoders for all five filter types;
- first-pixel and first-row zero-neighbor behavior;
- multi-byte pixels and multi-row images;
- packed 1-, 2-, and 4-bit sample rows;
- 8- and 16-bit legal color/depth classes with predictor widths of 1, 2, 3, 4, 6, and 8 bytes;
- modulo-256 wraparound;
- Paeth tie and predictor cases;
- Adam7 pass geometry, empty passes, and previous-row resets at pass boundaries;
- invalid filter bytes through both the adapter API and the public CLI;
- exact reconstructed byte and non-empty pass counts in canonical IR;
- repeated byte-identical `adapt-image` and `check-image` output;
- Linux, macOS, Windows, and the repository MSRV.
