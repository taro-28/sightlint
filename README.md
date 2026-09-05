# SightLint

**Deterministic, evidence-backed visual linting for interfaces and artifacts.**

SightLint is an architecture-first project for finding visual and interaction-quality
problems in web interfaces, mobile applications, slides, documents, PDFs, and images.
It is designed for both humans and coding agents.

> **Status: pre-alpha.** The repository is establishing its contracts and verification
> boundaries before implementing broad artifact support. Do not depend on the current API.
> Screenshot-only UI/UX defect detection is not yet implemented. A successful PNG adaptation
> or a green synthetic test suite is not evidence of real-world design-review accuracy.

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
pass-local Adam7 history. The reconstructed samples remain packed and pass-local. The adapter
emits source metadata and exact byte/pass counts, not decoded RGBA, ink bounds, text, components,
roles, peer groups, or automatic UX findings. Later-stage draft PRs are not completed features.

## Current deterministic command surface

The CLI accepts medium-neutral Artifact IR directly and can also validate and adapt deterministic
PNG source facts. Browser, slide, PDF, mobile, OCR, CV, and model-based acquisition remain
separate layers so that medium-specific sensing cannot leak into deterministic rule verdicts.

```bash
# Validate IR, run built-in atomic rules, and print a human report.
cargo run -p sightlint-cli -- check fixtures/e2e/pass-web.json

# Produce canonical machine-readable rule results.
cargo run -p sightlint-cli -- check fixtures/e2e/pass-web.json --format json

# Validate through PNG filter reconstruction, then emit canonical source-fact IR.
cargo run -p sightlint-cli -- adapt-image screenshot.png

# Adapt a PNG and immediately run the same deterministic rule engine.
cargo run -p sightlint-cli -- check-image screenshot.png --format json

# Binary stdin is supported for image adaptation too.
cat screenshot.png | cargo run -p sightlint-cli -- adapt-image -

# Make ambiguous cantTell outcomes fail an explicitly strict quality gate.
cargo run -p sightlint-cli -- check fixtures/e2e/cant-tell-missing-box.json --deny-cant-tell

# Normalize semantically valid IR into canonical JSON.
cargo run -p sightlint-cli -- normalize fixtures/e2e/pass-web-shuffled.json

# Emit the current Artifact IR schema.
cargo run -p sightlint-cli -- schema
```

`check-image` currently has only the whole-image source facts to inspect. Exit zero does not
mean that the screenshot has passed a comprehensive UI/UX review. Inspect the individual
`results` and their applicability; the report schema does not contain a `findings` field.

The public exit-code contract is:

| Code | Meaning |
|---:|---|
| `0` | No failed results; `cantTell` remains advisory unless explicitly denied |
| `1` | A rule failed, or strict policy denied a `cantTell` result |
| `2` | Usage, I/O, decoding, adapter validation, or semantic IR validation error |

## Executable verification and evaluation

`fixtures/e2e/` contains committed synthetic conformance data generated by
`tools/generate_e2e_fixtures.py`. The CI workflow verifies that the committed corpus is
reproducible and executes the real `sightlint` binary on Linux, macOS, and Windows.

`evaluation/` is a separate, versioned product oracle. Its smoke corpus runs the public
binary repeatedly, verifies declared rule outcomes, rejects undeclared failures or abstentions,
and requires targeted mutations to change the named rule from `passed` to `failed`. This prevents
a green contract suite from being mistaken for proof that the tool still behaves as intended.
The initial corpus is synthetic IR regression data, not evidence of real-world precision or
automatic structure acquisition from images.

The public-binary test suites additionally construct deterministic PNG byte streams and feed
them through binary stdin. Their zlib coverage combines an independent test-only stored-DEFLATE
encoder with fixed- and dynamic-Huffman streams generated by Python's zlib implementation, so
the production inflater is not validated only against data produced by one local fixture encoder.
Filter tests use independent forward encoders and known packed-byte answers. Public E2E also
compares direct `check-image` output against `adapt-image` followed by `check` byte for byte.

The corpus and generated binary cases include:

- clean web, mobile, slide, document, PDF, image, and other IR artifacts
- targeted spacing, overlap, out-of-canvas, containment, alignment, extent, and typography mutations
- missing and incomparable evidence that must return `cantTell`
- inapplicable rule cases
- zero and non-zero tolerance boundaries for declared visual contracts
- explicit right-to-left and vertical-up logical alignment cases
- malformed JSON, malformed official visual extensions, and semantically invalid IR
- valid PNG grayscale, RGB, indexed, grayscale-alpha, RGBA, and Adam7 variants
- complete PNG chunk ordering, CRC, palette, termination, and resource-limit cases
- valid zlib streams split across `IDAT` boundaries, including the Adler-32 trailer
- malformed DEFLATE, bad Adler-32, decoded-length mismatch, trailing compressed data, and decoded-size limits
- all five scanline filters, packed and multi-byte predictor widths, pass resets, and invalid selectors
- reordered but semantically equivalent documents and visual contracts
- preservation of unknown extensions while the official visual extension is validated and canonicalized
- standard-input, output-format, normalization, safety-limit, and exit-code cases
- repeated byte-for-byte determinism checks for both adapted IR and reports

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
```

Before public behavior is considered complete, CI must pass on the exact final commit,
including generated fixtures, public-binary E2E, and the product evaluation smoke corpus.
The normal CI workflow is read-only, rejects known stale integration files, and continues
independent checks after a failure without ignoring that failure.

Repository branch protection is a separate hosting setting, not enabled by this workflow.
[Issue #19](../../issues/19) tracks the required administrative action and API verification.
Do not infer enforced protection from the existence of a workflow or a green badge.

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
