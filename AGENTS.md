# Instructions for coding agents

This file is normative for every coding agent working in this repository, including the local
Codex app. Read it before planning or editing code. The repository must remain sufficient context;
do not depend on an earlier chat, private scratchpad, branch name, or agent memory.

## Required reading order

1. `docs/handoff.md`
2. `docs/product-rationale.md`
3. `docs/decision-history.md`
4. `docs/vision.md`
5. `docs/principles.md`
6. `docs/architecture.md`
7. `docs/artifact-ir.md`
8. `docs/rules.md`
9. `docs/testing-strategy.md`
10. `docs/evaluation-strategy.md`
11. `docs/roadmap.md`
12. the selected GitHub issue and linked accepted files in `docs/decisions/`
13. `docs/development.md`

When instructions conflict, current `main`, accepted architecture decisions, and the hard
invariants below win. An issue may propose a future design; it does not override accepted ADRs or
implemented contracts until the corresponding change is merged.

## Source of truth

Use this order when determining what exists:

1. the latest green commit on `main` and its reachable public code;
2. tests and committed fixtures that execute that code through the real `sightlint` binary;
3. accepted ADRs indexed by `docs/decisions/README.md`;
4. this file, `docs/handoff.md`, and `docs/roadmap.md`;
5. current issues that explicitly target the latest `main`;
6. merged PR descriptions as historical evidence only;
7. closed PRs and legacy branches as non-authoritative reference material only.

A title, branch name, commit message, `mergeable` flag, old CI run, or previous agent report is not
proof of implementation. Inspect the source and execute the public path.

Never base new work on a historical branch. At handoff time, legacy Draft PRs #12–#17 were closed
as superseded, and their remaining value was moved into issues #22–#27. Do not reopen or merge
them. Issue #32 tracks deletion of their branches and other stale refs.

## Hard invariants

1. **The rule kernel is deterministic.** Given identical normalized input, configuration, rule
   versions, engine version, and declared compatibility environment, results must be identical.
2. **Probabilistic observations are not facts.** Preserve provenance, confidence, uncertainty,
   alternatives, and conflicts. Do not upgrade inferred values to exact values.
3. **Blocking results require sufficient evidence.** A free-form LLM opinion or heuristic-only
   semantic guess cannot block a build by default. Ambiguity becomes `cantTell`, not a guessed pass
   or failure.
4. **Pixels are the common floor, not the only source.** Prefer and reconcile native structures
   such as DOM, accessibility trees, PPTX nodes, PDF tags, and platform semantics when available.
5. **Adapters are untrusted sensors.** Parsing, browser automation, OCR, CV, and model inference
   stay outside the deterministic policy kernel.
6. **The core IR is medium-neutral.** Do not introduce web-, slide-, document-, or mobile-only
   concepts into mandatory core fields. Use versioned namespaced extensions.
7. **Rules are atomic and composable.** Broad principles belong in documentation; executable
   rules define exact applicability, required evidence, policy, units, tolerance, alternatives,
   and outcomes.
8. **Unknown is a valid result.** Preserve `inapplicable`, `cantTell`, and `untested` as first-class
   outcomes. Never count missing execution as a pass.
9. **Fact, applicability, policy, and judgment are separate.** A measured difference is not a
   defect until evidence establishes the target relation and a named expectation applies.
10. **Local-first is the default.** No core command transmits artifact content unless the user
    explicitly selects an external adapter or model with a documented data boundary.
11. **Unsafe Rust is forbidden in the trusted kernel** unless an accepted ADR defines a tightly
    bounded exception and verification plan.
12. **Public behavior requires fixture-driven binary E2E.** Unit tests alone cannot complete a
    command, rule, adapter, schema, report, policy, or exit-code change.
13. **Conformance is not product validity.** Changes affecting a visual or UX claim require an
    appropriate acquisition/rule evaluation corpus. Green synthetic tests do not establish
    real-world accuracy.
14. **No universal trusted quality score.** Outcome, severity, confidence, evidence strength,
    coverage, and rule maturity remain independent.
15. **Current `main` is the only branch base.** One focused issue, branch, and PR per coherent
    slice. Do not create placeholder/final/review/ready/bootstrap/repair/`v2` branch chains.
16. **No self-writing feature workflows.** GitHub Actions may verify code but must not assemble,
    format, repair, commit, or push feature implementation. Ordinary CI remains read-only.
17. **No unconnected implementation.** A source module or test for a new public API is incomplete
    until the actual public command/library path calls and verifies it.
18. **Evidence gates scope.** Do not expand codecs, generic CV, GUI, MCP, cloud, or packaging ahead
    of the earliest unblocked product/evaluation milestone merely because implementation is easy.

## Current product sequence

Issue #34 is the canonical execution epic for the first evidence-backed zero-setup web UI alpha.
Until it is complete, prefer the earliest unblocked step:

1. #22 — realistic, human-reviewed evaluation corpus and hard negatives;
2. #23 — isolated Playwright native/pixel web adapter;
3. #24 — evaluated recommended rule packs that work without per-project setup;
4. the local agent edit/check/fix/rerun loop defined by #34;
5. #33 — licensing, compatibility, packaging, and first alpha release.

Other work is preserved in #25–#32. It is not automatically higher priority because its code
already exists on a stale branch. Explain any deviation from the sequence in the issue and PR.

## Before editing

From the repository root:

```bash
git fetch --all --prune
git switch main
git pull --ff-only
git status --short --branch
git log -1 --format='commit=%H tree=%T subject=%s'
```

Then:

- confirm the latest `main` CI is green;
- search for an existing open PR or branch for the selected issue;
- read the required documents and current implementation/tests;
- write a concise plan that identifies the issue, user-visible claim, exact and inferred evidence,
  trust boundary, applicability, policy source, expected outcomes, fixtures, resource/privacy
  model, compatibility impact, and non-goals;
- decide whether an ADR is required before implementation.

If the public claim, evidence, applicability, and evaluation cannot be stated, the task is not
implementation-ready. Prefer an ADR, benchmark, or corpus change over speculative production code.

## Change protocol

- Create one focused branch from the latest green `main` using `feat/`, `fix/`, `docs/`,
  `refactor/`, `test/`, or `chore/`.
- Link one primary issue and the roadmap milestone in the PR.
- Architectural, schema, trust-boundary, compatibility, policy-precedence, or public protocol
  changes start with an ADR. New ADR numbers continue at 0033 or later unless the index says
  otherwise.
- Implement the smallest user-visible path that reaches the real command and can be tested end to
  end.
- A new rule defines its problem, input aspects, applicability, expectation, policy source,
  tolerance/units, evidence threshold, valid alternatives, severity inputs, false positives,
  false negatives, maturity, and fixtures.
- A new adapter documents exact versus inferred observations, units/coordinate transforms,
  selectors, stable IDs, trust/privacy/network behavior, resource limits, compatibility,
  reconciliation, and native-input-to-IR E2E.
- Changes to a claimed capability update the applicable conformance, acquisition, and product
  oracle. Do not change an oracle merely because the implementation output changed.
- Update `docs/handoff.md` and `docs/roadmap.md` in the same PR whenever current capability,
  command surface, priority, accepted decision, evaluation claim, release/hosting state, or
  validation command changes.
- Keep raw artifacts private/local by default and review fixture licenses/provenance.

## Complete engineering gate

Run all currently applicable commands before considering a PR ready. At handoff time the complete
baseline is:

```bash
python3 tools/generate_e2e_fixtures.py --check
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_inspection_corpus.py --check
python3 tools/check_web_evaluation.py
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p sightlint-cli --test e2e
cargo test --locked -p sightlint-cli --test png_filter_e2e
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test evaluation_corpus
cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features --locked
```

Add a new generator or public E2E to this file, CI, the PR template, and the handoff in the same
change. Do not remove or bypass an existing suite to make a new feature pass.

Normal GitHub CI must also pass on:

- the exact final PR head;
- Linux, macOS, Windows, and the declared MSRV;
- the actual merged `main` commit.

A successful run on an earlier commit is not reusable evidence. Branch protection is separately
deferred in issue #19, so discipline is not enforced automatically yet.

## Required test design

Tests demonstrate behavior rather than merely execute code. Use, as applicable:

- unit tests for geometry, parsers, policies, and rule semantics;
- contract/schema and compatibility tests;
- golden fixtures with semantic assertions;
- targeted mutation fixtures proving the named rule detects its defect;
- `cantTell`, `inapplicable`, and `untested` cases;
- malformed input, boundary, allocation, and resource-limit cases;
- property tests for mathematical and canonicalization invariants;
- metamorphic tests for translation, scale, direction, viewport, text scale, state, and failure;
- differential tests when native structure and pixels observe the same artifact;
- public-binary E2E for file/stdin, human/JSON, reports, policies, and exit codes;
- separate acquisition and rule evaluation with reviewed provenance;
- byte-for-byte determinism across repeated runs and irrelevant input ordering;
- hard negatives and valid alternatives, not only clean/mutant positives;
- frozen holdout evaluation before strong accuracy or blocking-maturity claims.

Generated conformance fixtures remain committed for review and are not hand-edited. Change their
generator, regenerate, inspect the diff, and keep `--check` green. Reviewed product oracles require
a semantic explanation for every change.

## API and data-model discipline

- Version every serialized schema, protocol, extension, rule semantic, and report surface.
- Make units and coordinate spaces explicit; never store an unqualified numeric coordinate.
- Keep layout/source, render/ink, and hit geometry distinct.
- Stable identifiers must not depend on collection order or randomized hashes.
- Every report links to evidence or explains why evidence is unavailable.
- Preserve exact observations when adding inferred or candidate observations; never overwrite a
  fact with a hypothesis.
- Store medium-specific detail in extensions and map only truly shared semantics into core IR.
- Record runtime/model/adapter/browser versions and deterministic preprocessing/capture settings.
- Treat color-managed appearance, compositing, and encoded channel values as distinct concepts.
- Keep raw raster/document/page data out of serialized IR unless an accepted bounded protocol
  explicitly requires it.

## Pull request truthfulness

A PR description must state:

- issue and milestone;
- exact base and final head;
- reachable public behavior;
- evidence, assumptions, uncertainty, and policy source;
- fixture/oracle additions and what they do **not** prove;
- privacy, security, and resource effects;
- compatibility/version changes;
- explicit non-goals and remaining risks;
- exact final-head CI run and all required job results.

Do not describe code as complete when it is unconnected, only present on a Draft branch, or tested
only through library internals. Do not conflate an observation report with a trusted CheckReport.
Do not claim general UI/UX accuracy from synthetic data.

## After merge

- verify that `main` contains the intended tree and no temporary workflow or duplicate module;
- verify `main` CI on the exact merge/squash commit;
- update/close the issue and handoff as appropriate;
- delete the merged branch when repository settings permit;
- never leave a stale Draft PR as the project backlog; preserve future intent in issues/roadmap.

## Documentation discipline

Documentation is part of the executable contract. Update the relevant document and tests in the
same PR when changing an invariant, schema, command, rule, adapter, milestone, evaluated claim, or
release/hosting state. Explain deviations and rejected alternatives rather than hiding them.

Read `docs/decision-history.md` before reviving an idea from an old branch. A branch-only ADR whose
header says `Accepted` was never accepted by the repository unless it appears in the current ADR
index on `main`.
