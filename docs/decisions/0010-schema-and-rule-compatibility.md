# ADR 0010 — Schema and rule compatibility are separate contracts

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

SightLint will expose several compatibility surfaces: Artifact IR, adapter exchange, rule
meaning, configuration, CLI behavior, and reports. Treating the package version as the only
version risks silently changing a rule's meaning while serialized inputs still parse.

## Decision

Version the Artifact IR schema and executable rule semantics explicitly and independently.

- Serialized documents carry a schema version.
- Rules have stable identifiers and semantic versions.
- A rule identifier must not silently acquire a materially different applicability condition
  or expectation.
- Reports identify the engine, schema, rule ID, rule version, and resolved policy source.
- Unknown future fields are handled according to an explicit compatibility policy rather than
  being accepted or rejected accidentally by a serializer.

Detailed compatibility ranges and migration tooling will be defined during M1, before the
first complete serialized schema is merged.

## Consequences

- Reproducing an old result remains possible after unrelated rule changes.
- Report consumers can distinguish parsing compatibility from behavioral compatibility.
- Schema and rule changes require more explicit fixtures and release notes.
- Early data types must not assume one global version is sufficient.

## Alternatives considered

- Use only the Cargo package version: simple but too coarse for adapters and reports.
- Version only the schema: does not capture changed rule meaning.
- Treat all pre-1.0 behavior as unversioned: fast but undermines the project's auditability.

## Verification

M1 types and reports expose separate schema and rule versions. Golden compatibility fixtures
prove supported inputs, rejected inputs, and stable rule behavior across revisions.
