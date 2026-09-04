# Governance

SightLint is currently maintained by `@taro-28` and is in a pre-alpha design phase.

## Decision types

- **Implementation decisions** are made within a pull request when they do not change an
  accepted architectural invariant.
- **Architecture decisions** require an ADR under `docs/decisions/`.
- **Scope decisions** must update `docs/vision.md` or `docs/roadmap.md`.
- **Compatibility decisions** must identify affected schema, rule, CLI, or adapter versions.

Accepted ADRs are normative. Superseding one requires a new ADR that links to and explains
why the old decision no longer applies.

## Review standard

A change is ready to merge when:

- its behavior and evidence model are understandable without reading private discussion
- automated checks pass
- schema and rule compatibility are addressed
- the change does not blur the deterministic kernel and probabilistic perception boundary
- documentation reflects the implemented behavior

## Releases

There is no release cadence before the first executable vertical slice. Release automation,
versioning guarantees, and registry publication will be decided in a dedicated ADR.
