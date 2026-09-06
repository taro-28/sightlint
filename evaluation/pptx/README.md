# PPTX evaluation contract

This directory is the public, repository-owned evaluation boundary for ADR 0043 and the first
focused PPTX slice of issue #29.

- `corpus.schema.json` separates smoke, development, and challenge cases and records source and
  rendered artifact digests.
- `acquisition-annotation.schema.json` describes exact native IDs, hierarchy, EMU geometry, text
  digests, rendered extent reconciliation, and explicit acquisition abstentions.
- `rule-annotation.schema.json` separately describes rule applicability, policy, target outcomes,
  false-positive risk, and blocking authority.

Fixture specifications and annotations are authored before adapter output is inspected. Adapter
or `sightlint check` output is never copied into either oracle. The committed rendered slides are
reviewed differential evidence from a named renderer, not canonical output and not regenerated in
ordinary CI.

All data in this directory is fictional and owned by the SightLint project under
`MIT OR Apache-2.0`. It contains no personal, customer, credential, external asset, or remotely
processed data. The public cases are visible development data with maintainer-only review and no
protected holdout. Passing them cannot establish general presentation-quality accuracy.

Protocol v0 proves source slide/object acquisition, one exact source-geometry mutation, and
slide/render extent reconciliation. Shape-to-pixel identity, ink bounds, text layout, font
substitution, effects, and slide-specific rule quality remain `cantTell` or `untested`.
