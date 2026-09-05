# ADR 0030 — Verified staged PNG raster and byte corpus

- Status: Accepted
- Date: 2026-09-05
- Baseline: c85e1eedaaf1c680a9cea8fdb8a752d731989188 (recovery PR #18)

## Context

The verified public adapter reconstructs PNG filters but emits only source facts. Earlier
raster experiments are not evidence of reachable or tested behavior. This slice connects one
bounded raster implementation to the recovered public pipeline and adds committed input bytes
with independently specified expected pixels. No self-writing bootstrap workflow is permitted.

## Decision

For eight-bit grayscale, grayscale-alpha, RGB, and RGBA PNGs without tRNS, expand source samples
to unassociated, row-major encoded RGBA8. Scatter nonempty Adam7 passes using their documented
canvas coordinates. Keep the 256 MiB additional RGBA allocation cap from the staged experiment;
check it before allocation and use fallible allocation. No palette expansion, sub-byte unpacking,
16-bit quantization, color transformation, compositing, OCR, or semantic grouping is added.

The decoded values are **PNG-encoded samples, not display-corrected sRGB or linear light**.
Do not use them for a contrast or colorimetric verdict. The representation preserves RGB values
of transparent pixels; zero alpha does not license erasing hidden sample values.

The versioned `org.sightlint.adapter.png` extension gains `encodedRgba8Raster`, whose own version
is `0.1.0`. It contains availability, encoding, byte count, dimensions, and a CRC-32 of the
row-major sample bytes when available. CRC-32 is a regression checksum, not a security digest or
identity claim. Raw pixels remain in the adapter API, never in serialized Artifact IR. Add a
separate exact-source evidence record for this deterministic transformation rather than cite
IHDR as evidence for decoded samples. Existing source fields and core/report schemas remain
unchanged. All adapters remain outside the rule kernel.

Unsupported palette/depth/tRNS cases return explicit raster-unavailable reasons while retaining
source-level adaptation. Unavailable is not an assertion that every PNG feature has been
validated, nor is it a passed UX check. The decoder does not validate all unsupported ancillary
semantics. APNG control/data chunks cause explicit raster unavailability until frame selection
is supported. The normal parser and inflation/filter validators still run before a result is
emitted. A budget refusal is separate from an allocator failure and malformed input.

## Executable corpus

Commit a compact JSON corpus containing hex-encoded PNG input bytes and exact expected RGBA
bytes (or stable unavailable/error expectations). Hex keeps small binary fixtures reviewable
without a binary dependency. Tests materialize those exact bytes as temporary PNG files and
also feed them to binary stdin. A standard-library Python generator creates the corpus without
using SightLint and `--check` verifies byte-for-byte reproducibility in normal CI.

The corpus must exercise channel expansion, alpha preservation, each filter, multi-row images,
Adam7 scatter including empty passes, small/degenerate canvases, unavailable formats, and
malformed input. Verify native API pixels byte for byte, public CLI sample checksum/metadata,
source evidence, direct-versus-two-step command equivalence, stderr, exit codes, normalization,
and repeated output bytes. Include a synthetic clean/mutated card layout as **future** spacing
ground truth. Neither successful decoding nor identical empty findings proves spacing detection;
no unsupported product capability may be counted as passed. Existing IR product smoke evaluation
remains separate and unchanged.

## Scope and acceptance

This slice makes source pixels addressable and testable; it is not screenshot-only UI/UX lint.
The next useful step is pixel observations/regions against clean and targeted-mutant examples,
not implementing further image codecs. Native-structure acquisition and optional perception
remain planned alternatives to making semantic guesses from raw components.

Read-only CI must pass on the exact final candidate: corpus reproducibility, fmt, Clippy, tests,
public binary corpus, existing product smoke, rustdoc, Rust 1.85, Linux, macOS, and Windows. Check
the actual merged tree and main CI before reporting integration complete. Branch protection is
explicitly deferred by the maintainer and is not a blocker for this slice; PR-and-CI discipline
still applies. No claim is made that hosting enforcement has been enabled.

## Sources

- W3C PNG Third Edition, sections 6.1, 6.2, 8, 9, 11.2.2, and 11.3.1.1:
  https://www.w3.org/TR/png-3/
- Repository principles, Artifact IR, testing strategy, ADR 0024, and docs/recovery.md.
