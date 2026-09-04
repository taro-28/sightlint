# ADR 0020: Represent visual equivalence as explicit, evidenced contracts

- Status: Accepted
- Date: 2026-09-04

## Context

A deterministic engine can measure alignment, size, spacing, and typography exactly after data is
normalized. It cannot infer that two elements *ought* to align, have equal size, or use the same
font size from geometry alone without introducing a product-design assumption.

For example, sibling nodes may intentionally be staggered, a primary action may intentionally be
larger than secondary actions, and captions may intentionally use smaller text. Treating every
geometric outlier as a defect would be deterministic but not valid.

M1 already distinguishes primitive observations from declared `PeerSequence` and
`NonOverlapping` relationships. M2 extends that pattern.

## Decision

SightLint separates observed visual facts from expectations about visual equivalence.

- Geometry and typography observations live on nodes and retain evidence, units, confidence, and
  uncertainty.
- Pure measurements such as edges, centers, extents, containment, deviations, and overlap are
  derived by the deterministic query layer and are not redundantly serialized.
- Expectations that peers share an alignment anchor, extent, or font size are explicit relation
  variants with their own evidence and tolerance.
- Relation membership expresses a semantic or source-declared comparison set; it is not inferred
  by the trusted kernel.
- `start` and `end` alignment are resolved using the referenced canvas direction. The engine does
  not equate `start` with physical left or top unconditionally.
- Size consistency is defined one dimension at a time. Width and height are separate atomic
  obligations.
- A minimum font-size rule runs only when an explicit project, design-system, platform, or other
  declared contract supplies the threshold and target set. SightLint does not silently invent a
  universal minimum.
- Missing observations, incompatible units or coordinate spaces, and unresolved uncertainty become
  `cantTell` rather than pass or fail.
- All M2 visual-consistency rules remain experimental until rule-specific precision, mutation-kill,
  and false-positive evidence supports promotion.

Initial 0.2 relation contracts are:

- peer alignment: nodes, axis, start/center/end anchor, box kind, tolerance, evidence;
- peer extent: nodes, width or height, box kind, tolerance, evidence;
- peer font size: nodes, tolerance, evidence;
- minimum font size: nodes, threshold value and unit, evidence.

Parent containment is derived from the existing node hierarchy and node bounds. It remains
experimental because intentional overflow is possible; consumers may configure its policy later.

## Consequences

- Deterministic measurements do not masquerade as product intent.
- Adapters and design-system integrations can provide strong equivalence contracts when available.
- Image-only perception may later infer such contracts, but those relations remain inferred evidence
  and cannot become blocking merely because the rule calculation is deterministic.
- The serialized schema grows, but the rule engine remains medium-neutral.
- Authors must supply explicit peer contracts for strong conclusions; broad visual-outlier discovery
  can later exist as advisory analysis outside the trusted blocking path.

## Alternatives considered

### Automatically compare every sibling with the same primitive or role

Rejected as a default rule because same kind or role does not imply visual equivalence.

### Store precomputed alignment and size deviations in Artifact IR

Rejected under ADR 0017 because the values are deterministic derivatives of primitive observations.

### Use one generic free-form constraint expression

Rejected for the initial schema because it weakens validation, discoverability, fixture design, and
stable semantic versioning of individual rule meanings.

### Define a built-in universal minimum text size

Rejected because medium, viewing conditions, typography, density, role, and platform policy differ.
An explicit threshold is deterministic and auditable; an unqualified universal threshold is not.

## Verification

Each relation and rule has committed passing, targeted mutation, `cantTell`, inapplicable, boundary,
and malformed-input fixtures. Binary E2E verifies direction-aware alignment, per-dimension sizing,
font-size unit handling, evidence propagation, and deterministic ordering across supported artifact
kinds.
