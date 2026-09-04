# ADR 0019: Use explicit forward-only Artifact IR migrations

- Status: Accepted
- Date: 2026-09-04

## Context

Artifact IR 0.1 established the medium-neutral geometry and evidence foundation. M2 needs exact
visual-style observations and new source-declared contracts for alignment, size, and typography.
Adding those fields and enum variants without changing `schemaVersion` would make a material
serialized-contract change invisible, contradicting ADR 0010.

Rejecting every 0.1 fixture immediately would also weaken reproducibility: reports created from the
first public vertical slice should remain reproducible while the project is pre-alpha.

## Decision

Artifact IR uses explicit, forward-only in-memory migration between supported schema versions.

- M2 introduces Artifact IR `0.2.0` as the current canonical schema.
- Artifact IR `0.1.0` remains a supported input version during the M2 compatibility window.
- Loading performs these deterministic stages:
  1. decode the declared input version;
  2. validate fields and variants permitted by that version;
  3. migrate one version at a time to the current model;
  4. validate the current normalized document;
  5. execute rules only against the current normalized model.
- Canonical normalization always emits the current schema version.
- A document that declares `0.1.0` while using a 0.2-only field or relation is rejected rather
  than silently accepted.
- Unknown versions are rejected with a structured validation error.
- Migration must not fabricate observations, confidence, evidence, relationships, or policy.
  New optional fields remain absent unless an older representation carries equivalent data.
- Each supported legacy version has committed input fixtures and binary E2E proving migration,
  canonical output, and rule behavior.
- Removing legacy input support requires a separate compatibility decision and release note.

The generated JSON Schema describes only the current canonical version. Supported legacy input
schemas and migration behavior are documented separately.

## Consequences

- Schema evolution remains visible and reproducible.
- Old valid M1 inputs continue to work without weakening the 0.2 contract.
- Parsers and the deterministic engine receive one normalized current model.
- Migration code and compatibility fixtures become permanent maintenance obligations while a
  legacy version is supported.
- Backward serialization is not provided; normalization is intentionally forward-only.

## Alternatives considered

### Add optional fields under schema 0.1

Rejected because old readers would reject or misinterpret documents while the version still claims
compatibility.

### Reject 0.1 immediately

Rejected because it would unnecessarily destroy reproducibility of the first executable milestone.

### Keep parallel version-specific engines

Rejected for now. It multiplies rule implementations and risks behavioral drift. A deterministic
migration to one current model is simpler and auditable at this stage.

## Verification

- 0.1 clean fixtures load and normalize to byte-stable 0.2 JSON.
- 0.1 documents using 0.2-only features fail with a version-specific issue.
- 0.2 fixtures round-trip canonically.
- unsupported versions fail with exit code 2.
- repeated migration and normalization are idempotent.
