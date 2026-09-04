# ADR 0007 — Licensing

- Status: Proposed
- Date: 2026-09-04
- Owners: @taro-28

## Context

The repository is public and intended to become an open-source developer tool, but license
choice affects contributor rights, patent grants, dependencies, and future commercial use.
Selecting a license is a legal and product decision that should not be implied by scaffolding.

## Proposed decision

Evaluate a conventional Rust ecosystem choice such as dual MIT OR Apache-2.0, with explicit
consideration of patent terms and contribution policy. Do not publish crates or accept
external code contributions until the maintainer accepts the decision and adds the license
texts.

## Consequences

- The foundation workspace uses `publish = false`.
- README and contributing guidance state that a license is pending.
- Dependency policy can be finalized after the project's own license is selected.

## Alternatives considered

- MIT only
- Apache-2.0 only
- dual MIT OR Apache-2.0
- source-available or proprietary licensing

## Verification

An accepted replacement for this ADR must add license files, workspace package metadata, and
updated contribution guidance in one pull request.
