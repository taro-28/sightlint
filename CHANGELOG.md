# Changelog

All notable release changes are recorded here. SightLint is pre-1.0; each entry also names the
independent compatibility surfaces changed by the release.

## Unreleased

- Added exact source-alpha geometry for supported PNG rasters under ADR 0040: half-open visible and
  opaque bounds, alpha-class counts, transparent insets, edge occupancy, dedicated exact-source
  evidence, and an evidence-linked `inkBox` without compositing or semantic rule claims.
- Versioned the PNG extension as `0.2.0` with `alphaGeometry@0.1.0`, preserving explicit
  unavailability for unsupported rasters and absence of `inkBox` for entirely transparent assets.
- Added a repository-owned five-case transparent UI asset evaluation with independent acquisition
  and rule annotations, provenance/license/privacy declarations, hard negatives, explicit
  abstention and non-holdout status, plus public-binary file/stdin determinism and mutation checks.
- Added the evaluation-only `benchmark-image-segmentation` command and report schema `0.1.0` to
  compare the unchanged strict perimeter policy with ranked exact-border and 95%-qualified
  row-run policies without producing a rule result or blocking a build.
- Added a repository-owned nine-case Web UI benchmark with separate human-authored acquisition and
  rule oracles, provenance/license/privacy declarations, hard negatives, abstention, targeted
  mutation, metamorphic checks, deterministic bytes, and resource-boundary coverage.
- Retained `inspect-image` as the strict default: the public corpus exposes unsafe ranked
  hypotheses and shadow-connected false grouping, so it does not support a production admission or
  a real-world UI/UX accuracy claim.

## 0.1.0-alpha.2 — 2026-09-06

First published source-only alpha.

- Added the deterministic Artifact IR/rule/report CLI, bounded PNG adapter, advisory image
  inspection, Playwright Web acquisition, the first advisory recommended Web pack, and the local
  Web agent workflow.
- Added versioned conformance and product evaluation corpora with independent acquisition/rule
  annotations, hard negatives, mutation coverage, abstention, privacy, and provenance records.
- Accepted dual `MIT OR Apache-2.0` licensing and the source-first release/compatibility contract.
- Added deterministic source packaging, checksums, locked dependency-license verification, and a
  tag-only cross-platform release gate.
- Preserved read-only verification by carrying exact prepublication bytes through a short-lived
  workflow artifact and comparing them with draft assets immediately before publication.

See `docs/releases/v0.1.0-alpha.2.md` for exact versions, supported environments, and non-claims.

## 0.1.0-alpha.1 — 2026-09-06 (unpublished)

The immutable tag records the first release-workflow attempt. Packaging succeeded, but read-only
verification jobs could not access draft release assets. No release was published. ADR 0038 and
issue #47 record the failure and alpha.2 recovery; the alpha.1 tag was not moved.
