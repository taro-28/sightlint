# ADR 0001 — Product scope and name

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

The project began from a need to make AI-generated application interfaces follow basic UI
and UX principles without repeating those principles in every prompt. The same visual-quality
checks can apply to mobile apps, slides, documents, PDFs, and images.

A web-only or screenshot-only name would prematurely constrain the product.

## Decision

The product and CLI are named **SightLint** and `sightlint`. The repository is `sightlint`.

SightLint is a deterministic, evidence-backed linting system for interfaces and visual
artifacts. The architecture is cross-artifact, while implementation proceeds through narrow
milestones.

## Consequences

- Core terminology must work for more than web pages.
- Medium-specific behavior belongs in extensions and rule packs.
- Product messaging may use “visual linting,” but the architecture may later include
  interaction contracts.
- Scope expansion requires roadmap discipline.

## Alternatives considered

- `VisualLint`: direct but strongly collides with existing tools and Android terminology.
- `vlint`: short but already used by closely related visual-linting projects.
- `ArtifactLint`: precise but less memorable and less approachable.

## Verification

Repository, binary, documentation, and serialized metadata use the SightLint name. Core
schema reviews reject mandatory web-only concepts.
