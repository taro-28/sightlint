# Architecture

## Overview

SightLint follows a ports-and-adapters architecture around a deterministic Rust kernel.
Language and process boundaries are intentional trust boundaries.

```text
                 artifact sources
   ┌───────────────┼────────────────┐
   │               │                │
 pixels      native structure   interaction trace
   │               │                │
   └───────────────┼────────────────┘
                   ▼
               adapters
      exact extraction / perception
                   │
                   ▼
        validation and normalization
                   │
                   ▼
             Artifact IR
                   │
       ┌───────────┼───────────┐
       ▼           ▼           ▼
    geometry     queries    baselines
       └───────────┼───────────┘
                   ▼
            deterministic rules
                   │
                   ▼
        evidence-linked report model
                   │
       ┌───────────┼──────────────┐
       ▼           ▼              ▼
      CLI          CI           agent/UI
```

## Trusted kernel

The trusted kernel is Rust code responsible for:

- validating and versioning Artifact IR
- unit and coordinate normalization
- deterministic spatial and structural queries
- baseline and policy resolution
- atomic and composite rule execution
- result, evidence, coverage, and compatibility models
- deterministic serialization and report generation

The kernel must not:

- fetch network resources
- run a browser
- execute an OCR, CV, VLM, or LLM model
- interpret an unknown artifact format directly
- infer domain semantics without marking them as inferred
- rely on wall-clock time, random seeds, locale defaults, or iteration-order accidents

## Adapters

Adapters acquire observations and emit IR fragments. They may be written in the language that
best matches the platform:

- TypeScript for browser automation
- Kotlin for Android semantics
- Swift for iOS accessibility and UI automation
- Python for OCR, computer vision, and model experiments
- Rust for native file parsers and deterministic image primitives

Adapters communicate through a versioned, language-neutral schema. Process isolation is
preferred over an in-process plugin ABI during early development because it preserves crash,
resource, dependency, and language boundaries.

## Perception workers

Perception is optional and untrusted. A worker may classify a region as a heading, button,
caption, group, or other semantic role. Its output must include:

- model and version
- exact input reference or digest
- confidence or calibrated probability when available
- uncertainty or alternatives when relevant
- deterministic preprocessing parameters
- whether remote transmission occurred

Perception may supply observations, but the deterministic engine supplies outcomes.

## Data flow and reconciliation

When both native structure and rendered pixels are available, they are not merged by blindly
choosing one source. Reconciliation records agreement and conflict.

Examples:

- CSS declares 14 px text, but a transform renders approximately 11.2 px.
- A platform semantics tree exposes a button, but rendered pixels show it is fully occluded.
- A slide shape has a shared layout position, but transparent image padding shifts its ink.

These are valuable findings, not noise to normalize away.

## Workspace boundaries

The initial Rust workspace contains:

- `sightlint-ir`: versioned data contracts and validation
- `sightlint-engine`: deterministic queries and rule execution
- `sightlint-cli`: local command-line entry point

Future crates must justify a stable ownership boundary. Likely candidates include geometry,
reporting, image primitives, adapter protocol, and rule packs, but they are not created until
there is code pressure to separate them.

## Compatibility

Compatibility is tracked separately for:

- Artifact IR schema
- adapter protocol
- rule identifiers and rule semantics
- configuration schema
- CLI behavior
- report schema

A single package version must not conceal incompatible changes across these surfaces. The
release strategy will define how they are versioned together.

## Determinism contract

For deterministic output, the kernel will eventually specify:

- stable node and evidence ordering
- canonical floating-point tolerances and comparison rules
- explicit unit conversion and rounding
- locale-independent formatting
- stable map/set representations or sorted serialization
- fixed rule evaluation ordering
- content-addressed evidence references where practical

These details are part of correctness, not implementation trivia.
