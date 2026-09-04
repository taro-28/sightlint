# ADR 0019: Version official Artifact IR extensions independently

- Status: Accepted
- Date: 2026-09-04

## Context

Artifact IR 0.1 deliberately provides a small medium-neutral core and versioned extension maps on
artifacts and nodes. M2 needs exact visual-style observations and source-declared expectations for
alignment, extent, and typography. These concepts are broadly useful but optional, and their design
will evolve faster than identity, evidence, canvases, nodes, and basic geometry.

Adding every optional visual concept directly to the core would force a core schema migration before
we have adapter experience. Leaving the data as undocumented arbitrary JSON would weaken validation,
canonicalization, generated schemas, and rule reproducibility.

## Decision

SightLint will define **official typed extensions** that are independently versioned while remaining
embedded in the existing Artifact IR `extensions` map.

- The M2 visual extension uses the key `org.sightlint.visual`.
- Its payload carries an explicit `extensionVersion`; the first version is `0.1.0`.
- Artifact IR remains at core schema version `0.1.0` for M2.
- Official extensions have Rust types, semantic validation, canonicalization, generated JSON Schema,
  fixture coverage, and stable error categories.
- Unknown extension keys remain opaque and are preserved by canonical serialization.
- An unsupported version of a recognized official extension is rejected rather than partially read.
- Official extension parsing occurs after core IR validation and before any dependent rule executes.
- The trusted engine consumes only the typed, validated, canonical extension model.
- Extension data retains the same evidence, unit, confidence, uncertainty, and reference discipline
  as core IR.
- A future decision may promote a mature extension field into the core. Promotion requires an
  explicit migration and compatibility plan; it does not happen silently.
- Core schema and each official extension schema are emitted separately by the CLI.

This is not a loophole for medium-specific dumping. An official extension must define a coherent
contract shared by multiple adapters or rule families. Adapter-private data remains in namespaced
unrecognized extensions and cannot affect trusted rules until normalized into a typed contract.

## Consequences

- M1 Artifact IR inputs remain valid without a core schema bump.
- Visual contracts can evolve independently and gain real adapter experience before entering core.
- Consumers must track both core and extension versions, which reports and schemas must expose.
- The engine needs a combined validation error boundary for core and official extensions.
- Canonicalization must understand recognized official extensions while preserving unknown ones.

## Alternatives considered

### Add visual fields directly to Artifact IR 0.1

Rejected because it would materially change a versioned contract without changing its version.

### Immediately introduce Artifact IR 0.2

Deferred. It would be valid but unnecessarily couples optional M2 concepts to the foundational core
before structured adapters exist.

### Store untyped JSON and let each rule interpret it

Rejected because rule implementations could disagree about shape, defaults, validation, ordering,
and evidence semantics.

### Put policy in a separate configuration file only

Rejected as the sole representation. External policy will be useful later, but adapter-derived and
source-declared contracts need to travel with their evidence in the artifact document.

## Verification

- Existing M1 fixtures continue to parse and produce the same rule outcomes.
- Visual extension fixtures validate through a generated extension schema and semantic checks.
- Unsupported or malformed visual extension payloads fail with exit code 2.
- Reordered official extension data normalizes and reports byte-identically.
- Unknown extension payloads survive normalization unchanged.
- CLI schema commands expose both the core and visual extension contracts.
