# ADR 0018: Require fixture-driven binary end-to-end tests

- Status: Accepted
- Date: 2026-09-04

## Context

SightLint is intended to make deterministic, evidence-backed claims about artifacts. Unit tests
can prove individual geometry functions and rule branches, but they do not prove that a real user
can pass bytes to the distributed CLI and receive the intended report and exit code.

The product also spans several boundaries that can drift independently: serialized Artifact IR,
semantic validation, normalization, query logic, rule selection, report serialization, command-line
parsing, standard input, file I/O, and CI policy. A mistake at any boundary can produce a tool that
compiles while violating the product contract.

Development is often reviewed from a mobile device and executed remotely. Committed, inspectable
test data and reproducible GitHub Actions runs are therefore part of the verification model, not
an optional convenience.

## Decision

Every user-visible behavior must be protected by an end-to-end test that executes the built
`sightlint` binary. Calling library functions alone is insufficient for acceptance.

The repository will maintain a committed synthetic fixture corpus with these properties:

1. Fixtures are generated deterministically from a dependency-free generator when practical.
2. Generated fixture files remain committed so humans and coding agents can inspect exact inputs.
3. Required CI regenerates the corpus in check mode and fails on drift.
4. Each executable rule has a clean passing fixture, a targeted mutation fixture that the rule
   must kill, and `cantTell` or `inapplicable` fixtures when those outcomes are meaningful.
5. Schema and validation changes include malformed and semantically invalid fixtures.
6. CLI changes exercise file input, standard input, human output, machine output, and documented
   exit codes.
7. Deterministic output is checked byte-for-byte across repeated runs and irrelevant input order.
8. Medium-neutral behavior is exercised with representative web, mobile, slide, document, PDF,
   image, and other artifact kinds.
9. E2E tests run on Linux, macOS, and Windows through required CI checks.

A rule or adapter is not complete merely because its happy path passes. Its fixture set must
represent the relevant pass, fail, abstention, inapplicable, malformed-input, and regression cases.

Tests should assert semantic outcomes, evidence references, and stable public contracts. Large
snapshots may supplement those assertions but cannot be the only oracle.

## Consequences

- New behavior requires more deliberate test-data design before merge.
- The fixture corpus becomes a reviewable executable specification of SightLint's intent.
- False confidence from isolated unit tests is reduced.
- Mutation fixtures measure whether a rule can detect the defect it claims to cover.
- Cross-platform and serialization drift is caught early.
- Fixture generation and E2E execution add CI work, but correctness takes priority over minimizing
  a small amount of early-project runtime.

## Alternatives considered

### Add E2E only after adapters exist

Rejected. The first JSON-to-report vertical slice already crosses enough boundaries to drift, and
delaying the harness makes later regressions harder to diagnose.

### Test only Rust library APIs

Rejected. This would not verify argument parsing, byte limits, standard input, reports, or exit
codes experienced by users and coding agents.

### Generate fixtures only during CI

Rejected. Uncommitted fixtures are harder to review, cite as evidence, compare in pull requests,
and reuse when debugging regressions.
