# Native PNG raster conformance corpus

`corpus.json` contains 38 small, procedural native-input cases. Hex strings are committed PNG
bytes, not instructions to synthesize input from the implementation under test. Available cases
also contain independently calculated RGBA bytes and a CRC-32 regression checksum. The tests
materialize the exact PNG bytes as files and feed them to stdin; there is no runtime encoder or
network dependency in the Rust corpus tests.

## What is checked

- 20 cases: four eight-bit color families times five filters
- 8 cases: Adam7 across color families and degenerate/8×8 dimensions
- 1 case: source samples remain unchanged by unprocessed gamma metadata
- 2 cases: clean and spacing-mutated synthetic card layouts
- 5 cases: explicit palette, depth, tRNS, and animation-marker unavailability
- 2 cases: malformed scanline filter and CRC

Every case reaches the native adapter API and both public image commands. Supported pixels are
compared byte for byte to the independent oracle; the CLI checks its encoded-sample checksum,
metadata, evidence, canonical normalization, file/stdin equivalence, two-step/direct command
paths, stderr, exit code, and repeated report bytes. Resource-bound classification is separately
unit tested before allocation. CRC-32 is not a cryptographic identity or security guarantee.

## Reproduction

```bash
python3 tools/generate_raster_corpus.py --check
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
```

To update fixtures, change `tools/generate_raster_corpus.py`, run it without `--check`, and review
both PNG and oracle changes. Do not accept changed expected values merely because output changed.
The generator uses standard-library Python, explicit forward filter encoding and stored DEFLATE,
CRC-32 and Adler-32; it does not call SightLint. The normal read-only CI runs `--check`, and the
public-binary tests run on all three supported operating systems.

## Product evaluation boundary

These are synthetic source-raster conformance cases, not representative application screenshots,
expert reviews, usability studies, or measured real-world precision. All assets are procedural;
there are no third-party images, fonts, personal data, or private application content.

The two card cases retain future `peer-spacing` ground truth: gaps `[1,1]` versus `[1,2]`, with
matching peer bounds and a baseline link. **Detection remains `untested`.** The test verifies that
the oracle is internally consistent; it does not award a successful UI/UX detection for decoding
these images. The existing `evaluation/corpus.json` remains the separate synthetic IR rule smoke
oracle. Native semantic evaluation can only be promoted when the actual acquisition-to-rule path
has independently reviewed expectations and executable tests.

No display color correction, alpha compositing, inferred background, ink bounds, roles, or peer
groups are supplied by this slice. Unsupported ancillary semantics are not claimed to be fully
validated. Animation-marker unavailability does not certify a complete valid APNG stream.

See [ADR 0030](../../docs/decisions/0030-verified-staged-raster-and-corpus.md).
