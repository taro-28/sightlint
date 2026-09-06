# Testing strategy

SightLint tests are part of the product specification. A linter that reports the wrong target,
uses the wrong evidence, silently guesses through ambiguity, or exposes a broken command is not
correct merely because its internal functions have unit coverage.

Read `docs/evaluation-strategy.md` for corpus governance, realistic annotations, metrics, hard
negatives, and holdout requirements. This document defines the executable engineering layers and
completion gates.

## Four independent verification questions

### 1. Implementation conformance

Does the implementation obey its written schema, algorithm, resource, compatibility, report, and
CLI contract?

Examples:

- parser and geometry arithmetic;
- schema validation and migration;
- canonical ordering and serialization;
- exact rule semantics;
- malformed inputs and stable diagnostics;
- time/memory/object/output limits;
- public command, stdin/file, format, and exit behavior;
- deterministic repeated output;
- supported OS/MSRV behavior.

### 2. Acquisition correctness

Did an adapter or perception worker recover the right observations from the artifact?

Examples:

- exact decoded pixels;
- region/text/node bounds;
- DOM/accessibility role and hierarchy;
- hit targets, clipping, transforms, and computed typography;
- native/pixel agreement and conflicts;
- explicit abstention for unsupported, ambiguous, or over-budget input.

An acquisition test must compare against an independent oracle or source. It must not generate
expected observations by running the implementation under test.

### 3. Semantic rule/product correctness

Given sufficient observations, applicability, and policy, did the correct rule produce the
expected outcome for the correct targets?

This requires:

- clean and targeted mutation pairs;
- valid alternatives and hard negatives;
- expected `cantTell`, `inapplicable`, and `untested` cases;
- exact policy source and applicability;
- per-rule precision, coverage, abstention, and mutation evidence.

A correctly measured difference is not automatically a semantic defect.

### 4. User outcome

Does the checker help humans or agents prevent/fix named defects without unacceptable noise?

This is later evidence, such as time-to-fix, accepted findings, recurrence reduction, and a coding
agent's ability to navigate, edit, and independently rerun the checker. It is not replaced by CI.

The four questions use different data and must not be collapsed into one green badge or aggregate
UX score.

## Completion rule

A public capability is incomplete unless the applicable evidence layers are present.

- Internal algorithm change: conformance may be sufficient when no public/evaluation claim
  changes.
- New adapter observation: conformance plus acquisition evaluation.
- New rule or policy: conformance plus acquisition evidence for required aspects plus semantic
  rule evaluation.
- Blocking maturity or accuracy claim: realistic hard negatives, measured quality, compatibility,
  and holdout evidence.
- Workflow/product claim: user-outcome evidence in addition to the above.

Every public command or adapter path must be exercised through the built binary/process. Library
unit tests alone cannot complete it.

## Layer 1 — Schema and protocol contract tests

Validate every versioned serialized boundary:

- Artifact IR;
- CheckReport and advisory observation reports;
- official namespaced extensions;
- adapter/perception process requests and responses;
- rule/profile configuration;
- evaluation manifests;
- CLI stdout/stderr and exit-code contracts.

Cover:

- valid minimal and representative documents;
- missing/unknown/malformed fields;
- duplicate and dangling IDs;
- hierarchy cycles;
- invalid units/coordinate spaces/directions;
- non-finite and negative-invalid geometry;
- confidence/uncertainty constraints;
- unknown extension preservation;
- old/current schema compatibility and migration;
- canonical and idempotent normalization;
- stable diagnostics.

A generated schema is checked into the repository when reviewable output is useful, and generator
drift fails CI.

## Layer 2 — Unit tests

Unit tests cover isolated deterministic behavior:

- rectangle and interval operations;
- containment, overlap, gap, alignment, and extent calculations;
- direction-aware logical geometry;
- policy precedence and tolerance/rounding;
- rule applicability and outcome composition;
- canonical ordering and identifiers;
- parser framing, checksums, and error classification;
- scanline/filter/raster transformations;
- resource accounting before allocation;
- advisory acquisition algorithms under explicit assumptions.

Table-driven tests should name boundary values. Error tests assert stable categories and relevant
locations, not incidental debug output.

## Layer 3 — Property and exhaustive tests

Use property or exhaustive tests for mathematical invariants and small finite domains:

- symmetric overlap;
- nonnegative extents;
- translation preserving relative gaps/alignment;
- scaling transforming values/tolerances consistently;
- normalization idempotence;
- collection-order invariance;
- stable identifiers independent of traversal order;
- interval/rectangle edge conventions;
- deterministic color/alpha/sample arithmetic;
- complete small-domain arithmetic checks where feasible;
- serialization round trips without semantic drift.

Randomized property tests must use explicit seeds and preserve failing cases. Randomness must not
enter production behavior.

## Layer 4 — Golden conformance fixtures

Golden fixtures encode reviewable declared behavior for one source/IR and expected structured
result.

Every rule should receive, where meaningful:

- clean pass;
- targeted fail/mutation;
- `cantTell` for missing/conflicting evidence;
- `inapplicable` target;
- `untested` acquisition/execution;
- zero and nonzero tolerance boundaries;
- direction/unit/coordinate variants;
- valid alternative solutions;
- malformed and compatibility cases.

Assertions should focus on semantic fields: rule ID/version, outcome, targets, observations,
policy, evidence, measurements, units, maturity, and blocking status. Do not rely only on a broad
whole-file snapshot that obscures why a result changed.

## Layer 5 — Mutation tests

A clean fixture alone is weak evidence. Targeted mutations should change one named property and be
killed by the intended rule/acquisition capability.

Examples:

- move one node outside a canvas;
- introduce one overlap;
- change one peer gap/alignment/extent;
- clip or truncate one text/control;
- reduce one exact text/hit target below a named policy;
- alter one relation/evidence source;
- remove one pending/recovery/safeguard trace event;
- corrupt one parser field/checksum/filter selector;
- exceed one resource boundary.

The baseline must satisfy the named obligation and the mutant must violate or make it ambiguous.
Unrelated outcomes should remain stable unless the mutation is intentionally cross-cutting.

## Layer 6 — Metamorphic tests

Metamorphic tests verify behavior across transformations whose expected effect is known:

- translation;
- uniform and nonuniform scale;
- viewport/device-pixel ratio/text-scale change;
- left-to-right, right-to-left, and vertical direction;
- recoloring where geometry should be invariant;
- input and map order permutation;
- equivalent selectors/serialization;
- platform/renderer compatibility changes;
- slow, offline, retry, failure, and recovery traces;
- image alpha/background or crop transformations with explicit expected effects.

A transformation can legitimately change applicability or coverage; encode that expectation rather
than forcing output equality.

## Layer 7 — Differential and reconciliation tests

When two sources observe the same artifact, compare them explicitly:

- native layout versus screenshot geometry;
- DOM/accessibility versus rendered visibility;
- PPTX/PDF source objects versus rendered pages;
- Android/iOS semantics versus screen pixels;
- deterministic CV/OCR/model output versus reviewed annotations;
- custom decoder versus an independent implementation;
- two renderer/browser/platform versions under a declared compatibility policy.

Record agreement, expected source loss, native-only, pixel-only, transform mismatch, clipping/
occlusion conflict, semantic conflict, and unresolved conflict. Do not overwrite one source to make
the test pass.

Differential tests can produce `cantTell`; that is often the correct result.

## Layer 8 — Public-binary and process E2E

End-to-end tests invoke the actual built `sightlint` binary and any real adapter process.

They verify:

- native bytes/files/URLs/fixtures entering the public command;
- process startup, timeout, limits, protocol, and failure handling;
- adapter to IR/advisory report wiring;
- IR validation and normalization;
- rule/query/report path;
- file and stdin behavior;
- human and canonical JSON formats;
- stdout/stderr separation;
- stable exit codes;
- source/evidence selectors;
- repeated byte-identical output where promised;
- direct versus composed command equivalence;
- unsupported and over-budget coverage;
- absence of partial/made-up output on failure or abstention.

A module tested only through its API is not proof that the CLI calls it. A test for an unexported or
unconnected API is not a completed feature.

## Layer 9 — Acquisition corpus evaluation

Adapter/perception acquisition uses committed inputs and independently specified observations.

Current examples:

- the 38-case PNG raster corpus compares complete decoded pixels and unavailability/errors;
- the 30-case image-inspection corpus compares region bounds, groups, gaps, abstentions, evidence,
  and errors under the strict perimeter hypothesis;
- the nine-case image-segmentation benchmark captures one realistic repository-owned application,
  validates separate acquisition/rule annotations, runs three policies through the built binary,
  and measures region matching, false grouping, hard-negative abstention, mutation, metamorphic
  behavior, deterministic bytes, and bounded refusal.

The Playwright evidence matrix uses 23 repository-owned states with selected
DOM/accessibility/computed geometry, overflow/clipping/center-hit reconciliation, and synchronized
screenshots. Its acquisition and rule oracles are independently authored, and previous/current
strict schemas remain distinguishable. The built binary also runs the default recommended and
explicit base profiles, rejects malformed recognized Web extensions, and reports rule-specific
contract coverage, failure precision, reviewed abstention, mutation kill rate, and hard-negative
failures. Broader representative Web applications and future OCR/CV/VLM outputs still require
comparison with independent reviewed annotations.

Required acquisition controls include:

- positive coverage;
- unsupported/ambiguous/over-budget abstention;
- fragmentation and false merging;
- coordinate/scale transforms;
- hard negatives;
- repeated-run agreement;
- native/pixel conflicts;
- resource and latency behavior;
- version/runtime compatibility.

Acquisition results remain separate from semantic rule outcomes.

## Layer 10 — Rule/product corpus evaluation

The versioned product corpus executes the public command repeatedly and asserts reviewed rule
outcomes, target relations, policy sources, and mutation behavior.

It must:

- reject undeclared failures or abstentions when configured;
- require targeted baseline/mutant obligations;
- preserve valid alternatives and hard negatives;
- record source/provenance/license/review status;
- separate smoke, development, holdout, and challenge data;
- report precision, coverage, correct abstention, mutation kill rate, and errors per rule/medium/
  evidence class;
- avoid one universal quality score;
- never derive expected outcomes from SightLint itself.

The original rule smoke corpus is synthetic Artifact IR regression data. ADR 0032 adds the first
repository-owned realistic Web fixture foundation: six reviewed case records, separate acquisition
and rule oracles, three runnable public-binary smoke cases, one targeted mutation, one intentional-
grouping hard negative, and explicit deferred abstentions. ADRs 0033–0035 add the isolated browser
path and three narrow recommended Web rules. All three remain advisory because this public
single-application corpus does not support strong UI/UX claims or blocking maturity.

ADR 0036 adds the first agent-workflow product-path regression: one public combined command,
canonical and human byte stability, exact node-to-native-locator joining, a human-authored edit in
an isolated fixture copy, named-finding removal, no new failure, and retained ambiguous/dialog
`cantTell` controls. Because the edit and oracle are visible, it does not estimate autonomous
agent selection or real-world success.

## Layer 11 — Performance and resource tests

Test bounded behavior at and around declared limits:

- binary and JSON input size;
- PNG dimensions, pixels, chunks, compressed and decoded bytes;
- adapter nodes, frames/pages, text, relations, output bytes, and process duration;
- perception tiles/objects/hierarchy depth;
- interaction trace events and duration;
- memory allocations and fallible allocation paths;
- worst-case component/run/graph shapes;
- deterministic timeout/error/untested classification.

The exact boundary should succeed when promised; one unit beyond should return the specified
outcome without partial results or unbounded allocation.

Performance benchmarks are separate from correctness tests but must use versioned fixtures and
record environment. An optimization cannot change canonical semantics silently.

## Layer 12 — Security, privacy, and fuzz testing

Untrusted parser/adapter surfaces require:

- fuzzing and malformed corpus coverage;
- recursion/object/output limits;
- path/URL/scheme and network policy tests;
- process isolation and timeout tests;
- secret/PII fixture exclusion;
- remote transmission opt-in and declared payload checks;
- crash/panic resistance and stable errors;
- dependency advisory/license review;
- no write-enabled feature workflows or leaked credentials.

Fuzz-discovered regressions should become minimized committed cases when licensing/privacy permit.

## Determinism testing

Determinism is tested at multiple boundaries:

- canonical IR bytes;
- adapter protocol and advisory output;
- rule result ordering;
- reports and exit codes;
- normalized selectors and identifiers;
- repeated perception agreement metadata;
- fixture generation;
- cross-platform behavior where the contract claims equality.

Tests vary irrelevant insertion order and execute identical inputs repeatedly. Platform/runtime
versions and capture/preprocessing settings must be recorded when byte equality cannot reasonably
span environments.

Do not hide model/browser variability. Version and measure it before the deterministic kernel.

## Current committed suites

At handoff time, normal CI requires:

```bash
python3 tools/generate_e2e_fixtures.py --check
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_inspection_corpus.py --check
python3 tools/check_web_evaluation.py
npm --prefix adapters/playwright ci --ignore-scripts
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:e2e
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p sightlint-cli --test e2e
cargo test --locked -p sightlint-cli --test png_filter_e2e
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test image_segmentation_benchmark_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test evaluation_corpus
cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features --locked
```

The GitHub workflow also runs full tests on Linux, macOS, and Windows. New generators/process
adapters/public corpora add explicit CI steps and update `AGENTS.md`, the PR template, handoff, and
development guide.

## Fixture and oracle governance

### Generated conformance data

- generator is the source of truth;
- output is committed for review;
- `--check` fails on drift;
- generated files are not hand-edited;
- deterministic inputs avoid clocks, network, random defaults, and platform fonts unless the
  environment is explicitly part of the contract.

### Reviewed acquisition/rule ground truth

- expected data is authored independently of the implementation;
- record provenance, license, privacy, annotation guide, reviewer status, and ambiguity;
- oracle changes require a semantic explanation;
- do not snapshot-bless output after an implementation change;
- update related baseline/mutant/hard-negative cases coherently;
- protect holdout data from tuning leakage;
- report sampling limitations and reviewer disagreement.

### Real artifact privacy

Do not commit customer/private screenshots, credentials, personal data, or unlicensed product
copies. Prefer repository-owned deterministic fixture applications and explicitly approved/
permissively licensed artifacts.

## Rule maturity and CI blocking

A new rule begins experimental or advisory. Blocking eligibility requires rule-specific evidence:

- stable versioned semantics and compatibility;
- reliable acquisition and target applicability;
- real-case precision and acceptable false-positive cost;
- useful measured coverage and conservative abstention;
- valid alternatives and hard negatives;
- mutation detection;
- deterministic kernel behavior;
- clear source/evidence/policy explanation and scoped exception behavior;
- no unresolved privacy/security/platform risk.

Model-only or heuristic-only semantic inference cannot silently block. Severity, confidence,
evidence strength, maturity, and CI policy remain separate.

## Pull request and merge gate

Before a PR becomes ready:

- complete all applicable local layers;
- confirm fixtures/oracles and docs are updated;
- review privacy/security/resource/compatibility effects;
- push the final head;
- verify every required CI job on that exact head;
- review the final changed-file list and trust/evidence boundaries;
- ensure no self-writing/temporary workflow or duplicate/unconnected code remains.

After merge:

- verify the expected `main` commit/tree;
- verify `main` CI on that exact commit;
- update/close issues and handoff/roadmap as needed;
- verify that the merged head branch was deleted automatically.

The active `Protect main` ruleset requires the documented five CI contexts on an up-to-date head,
but hosting enforcement does not weaken any local, exact-head, or post-merge completion gate.

## Prohibited shortcuts

- declaring public behavior from unit tests only;
- creating expected outputs with the implementation under test;
- weakening an oracle to make a branch green;
- counting unsupported/untested coverage as a pass;
- treating synthetic success as real-world accuracy;
- using one aggregate score to hide rule errors;
- reporting CI from an older commit;
- merging because a PR is technically mergeable while required checks fail;
- using self-writing GitHub Actions to repair feature code;
- reviving stale branches instead of implementing from current `main`;
- allowing a model or heuristic semantic guess to become blocking without evaluated evidence.
