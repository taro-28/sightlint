# Roadmap

The roadmap controls scope. Completing a milestone means satisfying its exit criteria, not
merely adding code with a matching name.

## M0 — Project foundation

**Goal:** make architectural drift difficult before feature work starts.

Deliverables:

- vision, principles, architecture, threat model, and testing strategy
- accepted initial ADRs
- coding-agent instructions and contribution workflow
- Rust workspace ownership boundaries
- stable/MSRV/cross-platform CI
- no external runtime dependencies

Exit criteria:

- the workspace builds and tests in CI
- a new contributor or coding agent can explain the trusted boundary
- unresolved legal and release decisions are explicit rather than implicit

## M1 — Deterministic vertical slice

**Goal:** prove the core pipeline without image recognition or browser automation.

Deliverables:

- versioned Artifact IR types and validation
- language-neutral JSON schema and canonical serialization
- evidence and selector model
- deterministic geometry primitives and query context
- atomic rule trait and ACT-inspired outcomes
- JSON IR adapter
- CLI command that checks a fixture and emits human and JSON reports
- initial rules for invalid bounds, overlap, and explicit peer consistency

Exit criteria:

- repeated runs produce byte-identical canonical JSON
- invalid or ambiguous inputs result in structured errors or `cantTell`
- mutation fixtures demonstrate each initial rule
- Linux, macOS, Windows, and MSRV checks pass

## M2 — Visual geometry rule pack

**Goal:** cover high-confidence visual defects before probabilistic semantics.

Candidate areas:

- containment, clipping, occlusion, and safe margins
- alignment clusters and outliers
- explicit repeated-group spacing
- color and contrast primitives
- typography values when supplied exactly
- baseline comparison and semantic diff

Exit criteria include per-rule quality fixtures and documented tolerances.

## M3 — Deterministic image adapter

**Goal:** make pixels a common input without pretending image semantics are exact.

Candidate extraction:

- canvas metadata and color spaces
- connected regions and edges
- ink bounds, whitespace, overlap, and color sampling
- optional text-region detection without semantic role claims

The adapter must emit uncertainty and declare what cannot be verified from pixels alone.

## M4 — Structured adapters

Add one adapter at a time according to demand and fixture quality:

- Playwright/web
- PPTX/slides
- structured PDF/document
- Android semantics
- iOS accessibility hierarchy

Each adapter requires an ADR, threat analysis, differential fixtures, and unit conversion plan.

## M5 — Optional perception

Introduce OCR, component detection, hierarchy reconstruction, or VLM classification as
isolated workers. Model output remains inferred evidence and is not a blocking verdict.

## M6 — Interaction contracts

Add action effects, state machines, traces, temporal obligations, mutation testing, and
metamorphic tests for loading, failure, recovery, focus, destructive actions, and partial
success.

## M7 — Ecosystem

Potential integrations:

- MCP server for coding agents
- GitHub checks and annotated evidence
- editor and browser extensions
- local desktop or browser UI through a compiled kernel
- optional organization policy and history service

Cloud services remain optional and must not be required for core linting.
