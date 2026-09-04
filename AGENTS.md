# Instructions for coding agents

This file is normative for every coding agent working in this repository. Read it before
planning or editing code.

## Required reading order

1. `docs/vision.md`
2. `docs/principles.md`
3. `docs/architecture.md`
4. `docs/artifact-ir.md`
5. `docs/rules.md`
6. `docs/testing-strategy.md`
7. relevant files in `docs/decisions/`
8. `docs/roadmap.md`

When instructions conflict, accepted architecture decisions and the hard invariants below
win. Do not silently reinterpret them.

## Hard invariants

1. **The rule kernel is deterministic.** Given identical normalized input, configuration,
   rule versions, and engine version, results must be identical.
2. **Probabilistic observations are not facts.** Preserve provenance, confidence, and
   uncertainty. Do not upgrade inferred values to exact values.
3. **Blocking results require sufficient evidence.** A free-form LLM opinion cannot block a
   build. Ambiguity must become `cantTell`, not a guessed pass or failure.
4. **Pixels are the common floor, not the only source.** Prefer native structures such as an
   accessibility tree, DOM, PPTX node, PDF tag tree, or platform semantics when present.
5. **Adapters are untrusted sensors.** Keep parsing, browser automation, OCR, and model
   inference outside the deterministic policy kernel.
6. **The core IR is medium-neutral.** Do not introduce web-only concepts into the mandatory
   core. Medium-specific information belongs in versioned extensions.
7. **Rules are atomic and composable.** Broad principles belong in documentation; executable
   rules must have explicit applicability, required evidence, and expectations.
8. **Unknown is a valid result.** Preserve `inapplicable`, `cantTell`, and `untested` as
   first-class outcomes.
9. **Local-first is the default.** No core command may transmit artifact content unless the
   user explicitly selects an external adapter.
10. **Unsafe Rust is forbidden in the trusted kernel** unless an accepted ADR defines a
    tightly bounded exception and its verification plan.
11. **Public behavior requires fixture-driven binary E2E.** Unit tests alone cannot complete a
    command, rule, adapter, schema, report, or exit-code change. Execute the built `sightlint`
    binary against committed pass, fail, ambiguity, and malformed-input fixtures as applicable.

## Change protocol

- Work through a focused branch and pull request. Do not push feature work directly to
  `main`.
- Describe architectural changes in an ADR before implementing them.
- A new rule must define its input aspects, applicability, expected outcome, evidence,
  false-positive risks, and fixtures.
- Every executable rule must include a passing fixture and a targeted mutation fixture. Add
  `cantTell` and `inapplicable` fixtures whenever those outcomes are meaningful.
- A new adapter must document trust level, failure modes, units, coordinate transforms,
  evidence mapping, privacy behavior, and native-input-to-IR E2E fixtures.
- Do not expand scope merely because a library makes it easy. Follow milestone exit criteria
  in `docs/roadmap.md`.
- Do not add an LLM, hosted service, database, GUI, or plugin runtime to solve a problem that
  the current milestone does not require.

## Engineering requirements

Before considering a change complete, run or make required CI run:

```bash
python3 tools/generate_e2e_fixtures.py --check
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p sightlint-cli --test e2e
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
```

Tests must demonstrate behavior, not merely execute code. Prefer:

- unit tests for geometry and rule semantics
- golden fixtures for serialized IR and reports
- property tests for mathematical invariants
- mutation fixtures proving a rule can detect its target defect
- differential tests when two adapters observe the same artifact
- public-binary E2E for file and standard input, reports, policies, and exit codes
- byte-for-byte determinism checks across repeated runs and irrelevant input ordering

Generated fixtures remain committed for review. Do not hand-edit them. Change the generator,
regenerate the corpus, inspect the diff, and keep `--check` green.

## API and data-model discipline

- Version serialized schemas explicitly.
- Make units explicit; never store an unqualified numeric coordinate or size.
- Keep layout bounds, rendered/ink bounds, and hit bounds distinct.
- Stable identifiers must not depend on collection order or randomized hashes.
- Every report must point back to evidence or state why evidence is unavailable.
- Severity, confidence, and outcome are separate concepts. Never derive one implicitly from
  another.
- Avoid a single aggregate quality score in the trusted API.

## Documentation discipline

Update the relevant document and tests in the same PR when changing an invariant, schema,
rule contract, command, or milestone. Explain deviations rather than hiding them.
