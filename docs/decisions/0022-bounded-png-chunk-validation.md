# ADR 0022 — Bounded full-stream PNG chunk validation

- Status: Accepted
- Date: 2026-09-04

## Context

ADR 0021 established a deterministic PNG header boundary by validating the signature and first `IHDR` chunk. That was enough to prove the binary-input → Artifact IR → deterministic engine path, but a header-only byte sequence is not a complete PNG image. Future pixel decoding must not receive a stream whose header is trustworthy while later chunk framing, checksums, ordering, or termination is malformed.

PNG image data is chunked and attacker-controlled. Before any decompression is introduced, the adapter needs a bounded structural pass that does not allocate according to untrusted chunk lengths and does not interpret compressed image samples.

## Decision

Keep `inspect_png_header` as the explicit low-level API for callers that only need a validated `IHDR`. Add a full-stream structural validator and require it from `adapt_png` and therefore from the public `adapt-image` / `check-image` CLI paths.

The full-stream validator must:

- enforce the adapter-level 64 MiB input limit independently of the CLI read limit;
- walk chunks by offsets into the input slice without copying chunk payloads;
- cap the total number of chunks at 10,000;
- reject chunk lengths that exceed remaining input or overflow offset arithmetic;
- require four ASCII alphabetic chunk-type bytes and the PNG reserved bit to be valid;
- validate CRC-32 for every chunk, not only `IHDR`;
- allow only the standard critical chunk types `IHDR`, `PLTE`, `IDAT`, and `IEND`; unknown ancillary chunks remain opaque and are accepted when structurally valid;
- require exactly one `IHDR`, first;
- require at least one `IDAT` chunk;
- require all `IDAT` chunks to be consecutive;
- require `PLTE`, when present, to appear before the first `IDAT` and at most once;
- require `PLTE` for indexed-color PNG (color type 3);
- reject `PLTE` for grayscale PNG (color types 0 and 4);
- validate `PLTE` length as non-zero, divisible by three, no more than 256 entries, and no more entries than the indexed bit depth can address;
- require one zero-length `IEND`, last;
- reject trailing bytes after `IEND`;
- record exact structural counts (chunk count, `IDAT` chunk count, compressed `IDAT` byte count, palette presence) in the namespaced PNG adapter extension.

This pass deliberately does **not** decompress or validate the zlib/DEFLATE stream inside `IDAT`. A structurally valid PNG can therefore still contain invalid compressed image data. Pixel decoding requires a later slice with explicit decompression-bomb and decoded-byte budgets.

## Security and determinism

No chunk payload is copied solely for structural validation. Chunk lengths are converted and added with checked arithmetic before slicing. The validator is deterministic, local-only, and performs no network access, wall-clock reads, randomization, or semantic inference.

The crate-level 64 MiB limit exists even though the CLI already caps binary input, because the adapter is a reusable library boundary and must remain safe when called outside the CLI.

## Verification

Public-binary and unit tests must include at least:

- complete RGB/RGBA/grayscale/indexed PNG structures;
- multiple consecutive `IDAT` chunks;
- valid unknown ancillary chunks;
- missing `IDAT` and missing `IEND`;
- duplicate `IHDR`, duplicate `PLTE`, and duplicate `IEND` situations;
- non-consecutive `IDAT` chunks;
- `PLTE` after `IDAT`;
- required/forbidden `PLTE` cases by color type;
- invalid palette lengths and indexed palette cardinality;
- unknown critical chunks;
- invalid chunk-type bytes and reserved bit;
- bad CRC on non-`IHDR` chunks;
- truncated/oversized chunk framing;
- trailing bytes after `IEND`;
- chunk-count and adapter-level input-size boundaries;
- exact structural metadata in canonical IR;
- repeated byte-identical adapter and report output.
