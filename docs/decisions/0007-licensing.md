# ADR 0007 — Dual MIT or Apache-2.0 licensing

- Status: Accepted
- Date: 2026-09-06
- Owners: @taro-28

## Context

The repository is public and intended to become an open-source developer tool, but license
choice affects contributor rights, patent grants, dependencies, and future commercial use.
Selecting a license is a legal and product decision that should not be implied by scaffolding.

## Decision

License SightLint under the SPDX expression `MIT OR Apache-2.0`. A recipient may use the project
under either license. The MIT option keeps reuse simple; Apache-2.0 adds an explicit patent grant
and patent-termination terms without imposing copyleft on integrations.

The grant covers repository-authored source, documentation, generated conformance fixtures,
evaluation schemas, and the fictional repository-owned Atlas fixture and screenshots unless a
file or directory carries a more specific notice. Third-party dependencies, browser downloads,
future external datasets, customer artifacts, and model weights retain their own licenses and do
not become covered merely by appearing in a workflow or manifest.

Intentional contributions accepted for inclusion are licensed under the same `MIT OR
Apache-2.0` terms unless the contributor and maintainer explicitly agree otherwise in writing.
No separate contributor license agreement or sign-off requirement is imposed for this alpha. The
policy may be reconsidered before accepting materially different ownership or patent risk.

The licenses do not grant trademark rights in the SightLint name beyond reasonable descriptive
use. A future hosted or commercial component may use separate terms only across an explicit
package/service boundary; that does not revoke the license of code already distributed here.

## Consequences

- Add the complete MIT and Apache-2.0 texts and SPDX metadata to each package.
- Keep Rust crates unpublished and the Node package private until a later channel-specific
  decision; open-source licensing does not require registry publication.
- Record fixture/data provenance separately from the code license. Future external fixtures,
  datasets, model weights, fonts, brands, or screenshots require explicit redistribution and
  privacy review before commit or release.
- Review all locked dependency license expressions before every release. Dependencies keep their
  upstream notices and terms.
- External contributors can understand the inbound license without a separate CLA, but repository
  maintainers must not merge a contribution marked with incompatible terms.

## Alternatives considered

- MIT only: lowest-friction text, but it lacks Apache-2.0's explicit patent grant.
- Apache-2.0 only: explicit patent terms, but needlessly removes the conventional MIT option used
  by much of the Rust dependency graph.
- Source-available or proprietary: conflicts with the stated open-source developer-tool adoption
  goal and contribution model.
- Defer again: leaves a public alpha unusable and non-redistributable despite release work.

## Verification

The accepting pull request adds both license texts, package metadata, contribution and data
guidance, a dependency-license check, and release documentation. The release gate must confirm
that locked Rust and Node dependencies declare reviewed compatible terms and that no external
fixture is silently relabeled as repository-owned.
