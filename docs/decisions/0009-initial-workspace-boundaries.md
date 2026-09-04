# ADR 0009 — Initial workspace boundaries

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

The architecture anticipates many adapters, rule packs, report formats, integrations, and
optional perception workers. Creating a crate for every anticipated subsystem now would make
unproven boundaries expensive and encourage implementation breadth before the core contracts
are known.

At the same time, placing all code in one crate would blur the language-neutral IR, trusted
engine, and user-facing CLI boundaries that are already fundamental.

## Decision

Begin with exactly three Rust crates:

- `sightlint-ir`: serialized contracts, identifiers, evidence, validation, and compatibility
- `sightlint-engine`: deterministic geometry, queries, policy resolution, rules, and reports
- `sightlint-cli`: file/process orchestration and human or machine-facing command output

The dependency direction is `cli -> engine -> ir`. The IR crate does not depend on engine or
adapter concerns. Adapters remain outside the trusted engine and are added only when a roadmap
milestone requires them.

Create another crate only when at least one of these is true:

- the code has a distinct trust or process boundary
- it requires materially different dependencies or platform support
- it exposes a reusable compatibility surface
- measured compile-time or ownership pressure justifies separation

## Consequences

- The initial workspace is small enough to change while preserving critical boundaries.
- Geometry and reporting may start inside the engine and split later through an ADR.
- Image, browser, slide, document, mobile, MCP, and cloud packages are not scaffolded early.
- A future split must preserve public schema and rule semantics or document compatibility
  impact.

## Alternatives considered

- One monolithic crate: simpler initially but weakens architectural enforcement.
- A crate for every future subsystem: visually organized but speculative and high-maintenance.
- Separate repositories per adapter: stronger isolation but premature before protocols exist.

## Verification

The workspace manifest has three members and an acyclic dependency direction. CI checks the
whole workspace. New workspace members require an ADR or an explicit amendment to this one.
