# Changelog

All notable release changes are recorded here. SightLint is pre-1.0; each entry also names the
independent compatibility surfaces changed by the release.

## Unreleased

- Accepted ADR 0045 and added strict Android capture/request/response and
  `org.sightlint.android@0.1.0` surfaces with a dependency-free untrusted local file adapter that
  keeps exact View allocation, platform accessibility, and PNG render evidence separate.
- Added a realistic repository-owned API-35 account/settings fixture application, instrumentation
  capture runner, and three digest-pinned native/PNG cases with separate acquisition/rule truth,
  one targeted off-canvas mutation, one offscreen hard negative, explicit provenance/license/
  privacy/non-holdout records, and public-process E2E through `adapt-image`, `normalize`, and
  `check` on supported CI systems.
- Kept Android coverage explicitly partial: live device acquisition, Compose, arbitrary apps and
  devices, touch regions, dynamic behavior, occlusion/ink, rendered node identity, protected
  holdout evidence, Android-specific rules, and general mobile/UI/UX accuracy remain
  unimplemented, `untested`, or `cantTell`.
- Accepted ADR 0044 and added bounded PDF request/response and `org.sightlint.pdf` extension
  `0.1.0` surfaces backed by exactly hash-locked `pypdf==6.17.0` in an untrusted local process.
- Added deterministic repository-owned PDF pages and reviewed page renders with separate
  acquisition/rule annotations, one source-only off-page Link mutation, one `QuadPoints` hard
  negative, explicit provenance/license/privacy/non-holdout records, and public-process E2E
  through `adapt-image`, `normalize`, and `check` on the supported CI systems.
- Kept PDF coverage explicitly partial: text, tags, paint/ink, reading order, actions, forms,
  viewer hit testing, rendered annotation identity, protected holdout evidence, and general
  PDF/document-quality accuracy remain unimplemented or untested.
- Accepted ADR 0043 and added a bounded local PPTX process protocol/extension `0.1.0` that maps
  directly declared unrotated slide shapes/groups and exact source EMU layout geometry while
  preserving separate digest-pinned PNG extent evidence and rendered-node `cantTell`.
- Added deterministic repository-owned PPTX fixtures, reviewed LibreOffice-derived renders,
  separate acquisition/rule annotations, a targeted off-slide mutation, an asymmetric hard
  negative, explicit provenance/license/privacy/non-holdout records, metric contracts, parser
  safety tests, and public-process E2E through `adapt-image`, `normalize`, and `check`.
- Kept PPTX coverage explicitly partial: master/layout/theme resolution, other DrawingML objects,
  full text, rendered ink/identity, PPTX-specific rules, protected holdout evidence, and general
  presentation-quality accuracy remain unimplemented or untested.
- Accepted ADR 0042 and added local perception process protocol `0.1.0` with strict typed
  region/text/role/hierarchy/peer observations, bounded resources, explicit confidence and
  alternatives, worker/model/runtime/input provenance, and nonblocking `untested` rule status.
- Added a dependency-free reference region worker and public Node wrapper that validates,
  canonicalizes, hashes worker provenance, and maps only model-free measured regions through the
  public Rust normalizer; inferred semantics remain outside core IR.
- Added conformance/resource/error fixtures and a three-state Atlas differential evaluation with
  separate acquisition/rule oracles, native/pixel conflict retention, acquisition mutation, hard
  negative, abstention, license/privacy/non-holdout governance, and byte-stability gates. No OCR,
  model-calibration, semantic-rule, blocking, or real-world UI/UX accuracy claim is introduced.
- Accepted ADR 0041 after a versioned PNG format-demand assessment found no current product gap:
  retained explicit unavailability for indexed, sub-byte, 16-bit, `tRNS`, and animated inputs,
  added no decoder dependency, and made no format-prevalence or broader product-accuracy claim.
- Added a repository PNG inventory drift check and pinned-browser PNG-header assertion while
  keeping unsupported conformance controls separate from product-demand evidence.
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
