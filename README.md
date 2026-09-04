# SightLint

**Deterministic, evidence-backed visual linting for interfaces and artifacts.**

SightLint is an architecture-first project for finding visual and interaction-quality
problems in web interfaces, mobile applications, slides, documents, PDFs, and images.
It is designed for both humans and coding agents.

> **Status: pre-alpha.** The repository is establishing its contracts and verification
> boundaries before implementing broad artifact support. Do not depend on the current API.

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

See [Development](docs/development.md) and [Contributing](CONTRIBUTING.md).

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
