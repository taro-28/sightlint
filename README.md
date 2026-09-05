# SightLint

**Deterministic, evidence-backed visual linting for interfaces and artifacts.**

SightLint is an architecture-first project for finding visual and interaction-quality
problems in web interfaces, mobile applications, slides, documents, PDFs, and images.
It is designed for both humans and coding agents.

> **Status: pre-alpha.** The repository is establishing its contracts and verification
> boundaries before implementing broad artifact support. Do not depend on the current API.
> General screenshot-only UI/UX defect detection is not yet implemented. Image inspection
> supplies narrow, advisory-only region and gap observations, not semantic UX pass/fail verdicts.
> A green synthetic test suite is not evidence of real-world design-review accuracy.

## Why

AI can generate plausible-looking artifacts quickly, but basic quality failures still slip
through: inconsistent spacing, weak typography, clipping, overlap, missing states, unsafe
actions, inaccessible controls, and misleading feedback. A free-form AI critique is not a
reliable quality gate because its observations and judgments can vary between runs.

SightLint instead separates the problem into explicit stages:

```text
native structure / pixels / interaction traces
                    │
                    ▼
              artifact adapters
                    │
                    ▼
       evidence-backed Artifact IR
                    │
                    ▼
       deterministic queries and rules
                    │
                    ▼
 passed / failed / inapplicable / cantTell / untested
```

The intended invariant is:

> Given the same normalized IR, rule versions, configuration, and engine version,
> SightLint must produce the same rule results.

Probabilistic perception may help construct the IR, but it must not be disguised as fact.
Every inferred value carries provenance, confidence, and uncertainty where applicable.

## Product direction

SightLint will be local-first and cross-artifact. Pixels are the universal observation
layer; richer native structure is preferred whenever available.

Planned adapters include:

- JSON IR fixtures for deterministic engine development
- rendered images
- web pages through a browser adapter
- PowerPoint and other slide formats
- structured PDF and document formats
- Android and iOS semantic trees
- optional vision or language-model perception workers

The first functional milestone is deliberately narrow: load a versioned Artifact IR,
execute deterministic geometry rules, and emit evidence-linked results from a Rust CLI.
M2 extends that deterministic boundary with an official visual-contract extension for
explicit containment, alignment, peer extent consistency, peer typography consistency, and
project-supplied minimum font-size policies. These rules still consume declared,
evidence-backed observations rather than inferring semantics from pixels.

The current M3 PNG path validates header metadata, the complete bounded chunk stream, and
zlib/DEFLATE data carried by `IDAT`, then reconstructs all five PNG scanline filters with
pass-local Adam7 history. Eight-bit grayscale, RGB, grayscale-alpha, and RGBA samples without
tRNS can be expanded and scattered into row-major **PNG-encoded RGBA8**. These are unassociated
source samples, not display-corrected sRGB or linear-light colors. No color management or
alpha compositing is applied. The additional RGBA allocation is capped at 256 MiB.

The adapter reports explicit raster unavailability for palette, non-eight-bit, tRNS, animation
markers, or over-budget expansion. This is not a claim that unsupported ancillary semantics
have been fully validated. Raw pixels are available only through the native adapter API;
serialized IR contains versioned availability, counts, a regression checksum, and provenance.
No ink bounds, text, components, roles, peer groups, or automatic UX findings are inferred by
`adapt-image` or `check-image`. Later-stage draft PRs are not completed features.
See [ADR 0030](docs/decisions/0030-verified-staged-raster-and-corpus.md).

## Current deterministic command surface

The CLI accepts medium-neutral Artifact IR directly and can also validate and adapt deterministic
PNG source facts. Browser, slide, PDF, mobile, OCR, CV, and model-based acquisition remain
separate layers so that medium-specific sensing cannot leak into deterministic rule verdicts.

```bash
# Validate IR, run built-in atomic rules, and print a human report.
cargo run -p sightlint-cli -- check fixtures/e2e/pass-web.json

# Produce canonical machine-readable rule results.
cargo run -p sightlint-cli -- check fixtures/e2e/pass-web.json --format json

# Validate PNG source stages and report staged raster availability in canonical IR.
cargo run -p sightlint-cli -- adapt-image screenshot.png

# Adapt a PNG and immediately run the same deterministic rule engine.
cargo run -p sightlint-cli -- check-image screenshot.png --format json

# Observe region and gap candidates without a blocking UX verdict.
cargo run -p sightlint-cli -- inspect-image screenshot.png --format json

# Binary stdin is supported for image adaptation too.
cat screenshot.png | cargo run -p sightlint-cli -- adapt-image -

# Make ambiguous cantTell outcomes fail an explicitly strict quality gate.
cargo run -p sightlint-cli -- check fixtures/e2e/cant-tell-missing-box.json --deny-cant-tell

# Normalize semantically valid IR into canonical JSON.
cargo run -p sightlint-cli -- normalize fixtures/e2e/pass-web-shuffled.json

# Emit the current Artifact IR schema.
cargo run -p sightlint-cli -- schema
```

`check-image` currently has only whole-image source facts to inspect; the rule engine does not
consume raster samples as semantic nodes or peer groups. Exit zero does not mean that the
screenshot has passed a comprehensive UI/UX review. Inspect individual `results` and their
applicability; the report schema does not contain a `findings` field.

The public exit-code contract for checks is:

| Code | Meaning |
|---:|---|
| `0` | No failed results; `cantTell` remains advisory unless explicitly denied |
| `1` | A rule failed, or strict policy denied a `cantTell` result |
| `2` | Usage, I/O, decoding, adapter validation, or semantic IR validation error |

## Advisory image inspection

`inspect-image` is a separate, opt-in acquisition experiment, not a new blocking rule. Given
an entirely opaque raster with a single-color perimeter, it hypothesizes that color as the
background and measures four-connected regions that differ from it. Same-size, single-color
solid rectangles aligned in a row or column can form repeated-shape candidates. A foreign
region intersecting the intervening strip prevents that grouping.

The report includes exact source-device-pixel bounds and gaps, evidence links, the background
hypothesis, uncalibrated semantic confidence, and `blocking: false`. The committed card pair
has observed gaps `[1, 1]` and `[1, 2]`. The second receives an unequal-gap advisory, but both
retain `uxVerdict: cantTell`: identical pixels could also represent intentional grouping.
The unchanged old future semantic-spacing oracle remains `untested`.

No options are needed to select candidate shapes. JSON output has `inspectionSchemaVersion`
0.1.0, independently of Artifact IR and CheckReport. Human output says advisory-only. Exit 0
means the inspection ran or returned explicit unavailable coverage, not that a design is good.
Exit 2 indicates input, usage, I/O, or execution failure. This command never exits 1 and does
not accept `--deny-cant-tell`; heuristic observations cannot silently block a build.

Inspection is limited to 4,194,304 pixels and 1,024 connected regions. Exceeding either limit,
nonopaque pixels, a varying border, or unavailable source pixels produces an explicit reason
and no partial region/group output. These limits are checked after the existing bounded PNG
acquisition. Rounded cards, text, shadows, gradients, photographs, and complex layouts are
not generally supported; this prototype is not a general screenshot design reviewer.
See [ADR 0031](docs/decisions/0031-advisory-image-region-inspection.md) and the
[observation evaluation contract](evaluation/image-inspection.md).

## Executable verification and evaluation

`fixtures/e2e/` contains committed synthetic conformance data generated by
`tools/generate_e2e_fixtures.py`. The CI workflow verifies that the committed corpus is
reproducible and executes the real `sightlint` binary on Linux, macOS, and Windows.

[The native pixel corpus](fixtures/png-raster/README.md) contains 38 committed PNG inputs and
independent expected-pixel, unavailable, or malformed-input outcomes. It checks the native API's
actual pixels, the CLI checksum and evidence, file/stdin equivalence, normalization, direct and
two-step command paths, and repeated output bytes. A separate generator check detects fixture
drift. Its clean and mutated card layouts retain future semantic spacing ground truth as
**untested**; successfully decoding them is not counted as detecting a UX defect.

[The image-inspection corpus](fixtures/image-inspection/corpus.json) adds 30 observation cases:
19 observed, nine unavailable, and two malformed. It verifies the real public binary's acquired
bounds and gaps against independently specified oracles. Controls include horizontal/vertical
patterns, translation, scaling, recoloring, blockers, different sizes/colors, holes, diagonal
contact, alpha, and intentional unequal grouping. API/file/stdin/JSON/human output, evidence
links, determinism, and preserved check-image behavior are exercised. Actual pixel/component
budget boundaries also have direct raster tests. CI runs its generator check and E2E explicitly.

`evaluation/` is a separate, versioned product oracle. Its existing rule smoke corpus runs the
public binary repeatedly, verifies declared rule outcomes, rejects undeclared failures or
abstentions, and requires targeted mutations to change the named rule from `passed` to `failed`.
The inspection acquisition oracle remains separate from that rule schema so measured patterns
are not misrepresented as semantic rule outcomes. Both suites are synthetic regressions, not
evidence of real-world precision. New claims require appropriate acquisition and rule oracles.

The public-binary test suites additionally construct deterministic PNG byte streams and feed
them through binary stdin. Their zlib coverage combines an independent test-only stored-DEFLATE
encoder with fixed- and dynamic-Huffman streams generated by Python's zlib implementation.
Inspection fixtures use Python's independent fixed-Huffman encoder and explicit shape/gap
oracles; they do not derive expected regions by running SightLint. Generator drift fails CI.
Filter tests use independent forward encoders and known packed-byte answers. Public E2E also
compares direct `check-image` output against `adapt-image` followed by `check` byte for byte.

A new public rule or adapter is not complete without corresponding pass, fail/mutation,
ambiguity, inapplicable, malformed-input, boundary, resource-limit, determinism, and product
oracle coverage where those outcomes apply. See [the evaluation corpus](evaluation/README.md).

## Architecture

The repository is organized around a small Rust kernel and replaceable sensors:

```text
Rust kernel       deterministic IR, geometry, query, rule execution, reporting
Adapters          browser, image, PPTX, PDF, Android, iOS, and other sensors
Perception        optional OCR/CV/VLM processes; never part of the trusted verdict kernel
Integrations      CLI, CI, MCP, editor extensions, and future local UI
```

Read these documents before changing the architecture:

- [Vision and scope](docs/vision.md)
- [Project principles](docs/principles.md)
- [Architecture](docs/architecture.md)
- [Artifact IR](docs/artifact-ir.md)
- [Rule model](docs/rules.md)
- [Testing strategy](docs/testing-strategy.md)
- [Threat model](docs/threat-model.md)
- [Roadmap](docs/roadmap.md)
- [Architecture decisions](docs/decisions/README.md)
- [Integration recovery and evidence discipline](docs/recovery.md)

Coding agents must also follow [AGENTS.md](AGENTS.md).

## Development

The repository uses Rust 2024 with an explicit minimum supported Rust version. Common
commands are available through Cargo aliases:

```bash
cargo check-all
cargo lint
cargo test-all
cargo docs
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_inspection_corpus.py --check
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
```

Before public behavior is considered complete, CI must pass on the exact final commit,
including generated fixtures, public-binary E2E, and the product evaluation smoke corpus.
The normal CI workflow is read-only, rejects known stale integration files, and continues
independent checks after a failure without ignoring that failure.

Repository branch protection is a separate hosting setting, not enabled by this workflow.
[Issue #19](../../issues/19) tracks the administrative action, explicitly deferred by the
maintainer. PR-and-CI verification still applies; do not infer enforced protection from the
existence of a workflow or a green badge.

See [Development](docs/development.md), [Testing strategy](docs/testing-strategy.md),
[Product evaluation](evaluation/README.md), and [Contributing](CONTRIBUTING.md).

## What SightLint is not

SightLint is not intended to:

- assign a magical universal UX score
- let an LLM make opaque blocking decisions
- replace user research, usability testing, or product judgment
- require uploading private artifacts to a hosted service
- force every medium into a web-specific DOM model

## License

No open-source license has been selected yet. The workspace is intentionally marked
`publish = false` until the maintainer accepts a licensing decision. See
[ADR 0007](docs/decisions/0007-licensing.md).
