# Vision and scope

## Mission

SightLint turns visual and interaction-quality expectations into explicit, inspectable,
evidence-backed contracts that can run during creation, review, and continuous integration.

The project exists so that a person or coding agent should not need to remember every basic
UI, UX, typography, spacing, accessibility, and artifact-quality principle in every prompt.
Those expectations should live in reusable rules and verified components of the development
process.

## Problem

Existing tools are fragmented:

- static linters understand code but often not the rendered result
- visual regression tools detect pixel changes but not whether the result is better or worse
- accessibility tools cover an important subset but not general visual and interaction UX
- design-system checks compare tokens but not broader relationships or state transitions
- free-form AI reviews can be useful but are difficult to reproduce and audit
- user research provides ground truth but cannot run on every edit or pull request

SightLint addresses the gap between deterministic structural checks and contextual UX
reasoning without pretending that all human judgment can be formalized.

## Product promise

SightLint aims to provide:

1. A medium-neutral Artifact IR with provenance and uncertainty.
2. Deterministic geometry, structure, style, and interaction queries.
3. Atomic and composite rules with explicit applicability and evidence requirements.
4. First-class `cantTell`, `inapplicable`, and `untested` outcomes.
5. Local-first execution and optional, isolated perception workers.
6. The same kernel in a local CLI, CI, agent integration, and future user interfaces.
7. Reports that explain what was observed, what was expected, and why a verdict was reached.

## Artifact scope

The architecture must be able to represent, over time:

- web interfaces
- Android and iOS interfaces
- slide decks
- documents and reports
- PDF pages
- screenshots and exported images
- diagrams, charts, email, and other visual artifacts
- time-based or interactive artifacts through optional extensions

Supporting all of these is not the first milestone. Cross-artifact compatibility is an
architectural constraint, not permission to implement every adapter at once.

## Quality domains

The eventual rule ecosystem may include:

- geometry: containment, overlap, clipping, alignment, spacing, bounds
- typography: scales, role consistency, line length, legibility, overflow
- color: contrast, palette conformance, semantic use, distinguishability
- hierarchy and grouping: repetition, proximity, reading order, density
- platform behavior: hit targets, focus, keyboard, responsive transformations
- interaction contracts: pending states, results, recovery, destructive effects
- medium-specific rules: slide rhythm, document headings, chart labels, safe areas

## Non-goals

SightLint will not:

- prove that an artifact is universally beautiful, persuasive, or usable
- replace representative user research or production behavior data
- use a single opaque score as a trusted release gate
- force an exact layout when multiple valid designs satisfy the same contract
- make inferred semantics look exact
- require a hosted service for core checks
- make a large language model the final authority for blocking decisions

## Success criteria

The project is succeeding when:

- identical normalized inputs produce identical reports
- users can trace each result to its evidence and rule version
- adapters can be replaced without rewriting the rule kernel
- image-only analysis degrades by returning less coverage, not fabricated certainty
- new artifact types reuse the core IR and rule concepts without web-specific hacks
- rules demonstrate both low false-positive rates and mutation-detection ability
- coding agents can verify their own changes without being trusted to grade themselves
