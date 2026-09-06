# ADR 0041 — Retain bounded PNG coverage until product evidence shows a format gap

- Status: Accepted
- Date: 2026-09-06
- Issue: #27
- Builds on: ADRs 0024 (product evaluation), 0030, 0031, 0039, and 0040

## Context

The PNG adapter intentionally expands eight-bit grayscale, RGB, grayscale-alpha, and RGBA images
without `tRNS`. Indexed, sub-byte, 16-bit, `tRNS`, animation, and over-budget rasters remain
explicitly unavailable. Issue #27 permits broader normalization only after evaluation data or real
use establishes a meaningful coverage gap and requires a decoder strategy decision first.

The current repository-owned product evidence does not establish that gap:

- all five committed transparent UI assets are non-interlaced eight-bit RGBA;
- all three later ADR 0043 PPTX differential renders are non-interlaced eight-bit RGB and are
  inventoried by the same assessment checker;
- the pinned Playwright/Chromium segmentation evaluation produces non-interlaced eight-bit RGB
  PNGs and already exercises them through the public image command;
- indexed, packed, 16-bit, `tRNS`, and animation inputs exist only as explicit conformance
  controls, not failed product cases or user reports;
- SightLint collects no customer artifact telemetry from which to infer format prevalence.

The superseded PR #13 and branch ADR 0025 are design references only. Their code and workflow are
not implementation inputs.

## Decision

Select issue #27 strategy 4 for the current milestone: keep unsupported formats explicitly
unavailable and require an upstream conversion chosen by the caller. Do not add a decoder
dependency or extend the custom normalization path in this issue.

Commit a versioned format-demand assessment and a read-only checker. It inventories every
repository-tracked PNG, records the ephemeral browser screenshot contract exercised by product
evaluation, distinguishes product inputs from synthetic unavailable controls, and fails when a
new committed PNG bypasses review. Browser E2E must assert the PNG header family before passing
the screenshot to the public image command.

This decision closes the optional work as “not admitted for lack of product need,” not as “all PNG
formats supported.” A future observed gap requires a new issue and ADR rather than silently
changing this decision.

## Strategy comparison

### 1. Extend the current narrow decoder

This offers direct control over validation order, sample semantics, allocation budgets, stable
errors, and the existing `EncodedRgba8Raster`. It also makes SightLint maintain palette indexing,
packed row tails, 16-bit conversion, original-sample `tRNS` matching, and APNG ambiguity in an
untrusted-input parser. The implementation and security review burden grows without improving a
currently failing product case. Differential testing would still need an independent decoder.

Status: rejected until a demonstrated gap is both narrow and cheaper to maintain locally than a
library boundary.

### 2. Use the mature Rust `png` crate inside the adapter

The leading candidate reviewed on 2026-09-06 is `png` 0.18.1. Its upstream metadata declares Rust
1.73 and `MIT OR Apache-2.0`, compatible with SightLint's current MSRV and license policy. Upstream
documents no unsafe code in the default crate, OSS-Fuzz use, APNG decoding, configurable
transformations, and best-effort decoder limits. `EXPAND` covers palettes, packed grayscale, and
`tRNS`; `STRIP_16` is available, but its exact semantics must not be assumed to equal SightLint's
previously proposed rounded 16-to-8 formula.

A future adoption must pin features, avoid the optional `zlib-rs` feature unless a separate unsafe
dependency decision accepts it, impose SightLint's own input/output/allocation limits in addition
to library limits, canonicalize errors and output, define first-frame/APNG behavior, and run the
same corpus through both the old and new paths. Upstream fuzzing and maintenance are useful
evidence, not proof of an absence of vulnerabilities; the exact dependency graph must pass the
project's license, Dependabot, RustSec/GHSA, CodeQL, MSRV, and cross-platform gates at admission.

Status: preferred candidate if broad PNG demand is later established; not added now.

### 3. Isolate decoding behind a process or library protocol

This provides the strongest fault and resource boundary and could support several image formats
without placing decoding code in the deterministic kernel. It also introduces a versioned binary
protocol, process lifecycle, packaging/runtime availability, byte-transfer cost, platform
portability, dependency licensing, and more operational failure modes. A process boundary does not
make inferred color management or frame semantics exact.

Status: reserve for a multi-format demand pattern or a decoder whose in-process risk cannot be
bounded acceptably.

### 4. Retain explicit unavailability and request conversion

This preserves current deterministic behavior, zero new dependencies, the 256 MiB RGBA allocation
cap, Rust 1.85.0 portability, and honest unknown coverage. It does not serve callers whose only
artifact is an unsupported encoding. Conversion also changes the artifact: SightLint can make
exact claims about the converted PNG only, not the original source samples.

Status: accepted for current evidence.

## Conversion and evidence boundary

SightLint does not select or invoke a converter in this decision. A caller that needs the current
image path may explicitly convert to non-interlaced or Adam7 eight-bit grayscale, RGB,
grayscale-alpha, or RGBA without `tRNS`, while retaining the original and the conversion tool,
version, options, and digest in its own provenance record.

The resulting `evidence:png-raster` and `evidence:png-alpha` are exact for the converted PNG bytes.
They are not exact evidence for the original palette, 16-bit samples, `tRNS`, animation frames,
color management, or composited display. A future official conversion adapter must version its
protocol and transformation provenance and remain outside the deterministic rule kernel.

## Re-admission gate

Broader decoding may be reconsidered only when at least one of these is recorded without storing
private artifact content:

- a repository-owned or redistributable product fixture cannot exercise a useful admitted rule
  because its PNG encoding is unsupported;
- opt-in aggregate user reports show a material unsupported-format rate and document sampling;
- another accepted adapter requires the original source encoding and conversion would erase facts
  needed by an evaluated capability;
- a security or maintenance finding makes the current custom decoder less appropriate than a
  bounded library/process replacement.

The new issue must identify exact formats, expected product benefit, privacy-safe sampling,
acquisition and rule ground truth, hard negatives, and non-goals. If implementation is admitted,
the normalization requirements preserved in issue #27 become candidates for a new versioned
contract; they do not silently modify `encodedRgba8Raster@0.1.0` or `alphaGeometry@0.1.0`.

## Security, licensing, privacy, and holdout

- No dependency or external executable is added, so the locked dependency/license inventory is
  unchanged.
- Existing parser, decompression, pixel, and allocation limits remain unchanged.
- The assessment reads only repository files in normal CI and performs no network access.
- No artifact telemetry, filenames, pixels, or customer data are collected.
- The assessment is public development evidence, not a protected holdout or prevalence study.
- Absence of a known demand signal is not proof that users never have unsupported PNGs.

## Consequences

- Current product paths, including the later PPTX differential renders, remain covered without
  increasing codec scope.
- Unsupported inputs keep stable explicit reason codes and are not relabeled passed or failed.
- Users needing unsupported encodings must opt into conversion and understand the evidence scope.
- `org.sightlint.adapter.png@0.2.0`, current allocation limits, and command/exit behavior do not
  change.
- The next roadmap gate becomes issue #28; broader PNG work requires new evidence and a new issue.

## Verification

- strict schema validation of the format-demand assessment;
- drift check over all eight committed `.png` files, their digests, dimensions, depth, color type,
  and interlace method;
- explicit linkage to the five unsupported conformance cases and their stable reason codes;
- nine-case Playwright segmentation E2E assertion that generated screenshots are eight-bit RGB
  PNG before the public binary consumes them;
- all existing PNG/API/file/stdin/malformed/determinism/product E2E;
- Rust 1.85.0 and Linux/macOS/Windows on the exact PR head and merged `main`.

## Sources reviewed

- W3C PNG Third Edition: https://www.w3.org/TR/png-3/
- image-rs `png` repository and security/maintenance claims:
  https://github.com/image-rs/image-png
- `png` 0.18.1 manifest, including license, Rust version, dependencies, and feature warnings:
  https://github.com/image-rs/image-png/blob/master/Cargo.toml
- `png` decoder limits and transformation API:
  https://docs.rs/png/0.18.1/png/struct.Decoder.html
- `png` transformation semantics:
  https://docs.rs/png/0.18.1/png/struct.Transformations.html
- RustSec advisory database and audit model:
  https://github.com/RustSec/advisory-db
