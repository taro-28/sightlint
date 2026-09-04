# ADR 0017 — Derive geometric relations instead of redundantly storing them

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Relations such as containment, overlap, gap, aligned edges, same row, and nearest neighbor can
usually be computed deterministically from geometry. Serializing every derived relation would
inflate Artifact IR, create conflicting sources of truth, and make adapter outputs harder to
compare.

Some relations, such as semantic peers, repeated groups, captions, or visual grouping, may
come from native source structure or probabilistic inference and cannot always be recomputed
from boxes alone.

## Decision

Store primitive observations and source-declared relationships in Artifact IR. Compute purely
geometric relationships in deterministic query APIs. Store or reference semantic/inferred
relations only when they carry independent evidence and provenance.

Derived-query behavior is versioned with the engine and identifies tolerance policy. Reports
may materialize the relation used by a rule as evidence without making it an authoritative IR
fact.

## Consequences

- IR remains smaller and avoids redundant geometric truth.
- Rule behavior depends on an explicit engine and tolerance version.
- Semantic grouping can coexist with deterministic geometry without being confused with it.
- Reports remain self-explanatory by recording the derived relation used.

## Alternatives considered

- Serialize all relations: easy consumption but duplication and inconsistency risk.
- Store no relations: loses exact source hierarchy and inferred grouping evidence.
- Let each rule calculate ad hoc geometry: creates inconsistent semantics and tolerances.

## Verification

M1 places geometry relations behind shared query APIs. Reordering or omitting derived caches
does not change canonical rule results. Source-declared and inferred relations retain separate
evidence classes.
