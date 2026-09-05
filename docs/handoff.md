# Local Codex handoff

This document is the operational handoff for continuing SightLint in the local Codex app or
another local coding-agent environment. It is intentionally explicit: a new session must be able
to determine the trusted source of truth, current capability, next task, decision background,
quality gates, and historical traps without access to the chat that created the repository.

Last handoff preparation: 2026-09-05.

## Start here

The authoritative development line is the latest green commit on `main`.

At the start of this handoff work, the last functional baseline was:

- commit: `583136ac965a342526c6bbc100250cd9d7ce3a0c`
- tree: `c74368a152c2cfc65ae82c8db05735f014ae273a`
- merged PR: #21
- main CI: run 33972019767, all five jobs successful

This handoff PR changes documentation and workflow instructions, not the product kernel. After it
merges, its resulting `main` commit supersedes the hash above as the repository starting point.
Never hard-code the old hash as a branch base. Verify the current `main` and its CI.

Use this source-of-truth order:

1. current `main` source and tests;
2. accepted ADRs indexed in `docs/decisions/README.md`;
3. `AGENTS.md`, this handoff, and `docs/roadmap.md`;
4. open issues that explicitly target current `main`;
5. merged PR descriptions only as historical evidence;
6. closed PR branches only as non-authoritative reference material.

A PR title, branch name, commit message, old CI badge, or prior agent statement is never proof that
a capability is implemented. Inspect reachable code and run the public E2E.

## Current repository state

### Pull requests

There should be no open legacy implementation PR after this handoff. Draft PRs #12–#17 were
closed as superseded after their useful intent was transferred to current issues and documents.
They all came from obsolete integration lines and their final recorded CI runs failed.

Do not reopen or merge them:

| PR | Historical idea | Current disposition |
|---|---|---|
| #12 | alternative PNG filter reconstruction | already implemented and verified through #10/#18; no remaining task |
| #13 | full PNG sample/palette/`tRNS` normalization | optional remaining scope is issue #27 |
| #14 | exact alpha-visible geometry | reimplement from current main under issue #26 |
| #15 | ranked opaque border/background candidates | research candidate under issue #25 |
| #16 | layered image evaluation corpus | replaced by current corpora; real-data work is issue #22 |
| #17 | scalable background-relative components | current slice is #21; alternative algorithm is issue #25 |

### Branches

Many merged, duplicate, and obsolete remote branches remain from the mobile/remote construction
phase. The current connector cannot delete branch refs or change repository settings. Issue #32
tracks manual/local cleanup and automatic branch deletion.

Until cleanup is complete:

- never select a branch because its name sounds more advanced;
- never build new work on a legacy branch;
- start from the latest green `main` only;
- compare any historical branch to `main` before extracting an isolated idea;
- do not merge a historical branch wholesale.

### Hosting settings

Branch protection is not enabled. Issue #19 records the intended ruleset and required check names.
The maintainer explicitly deferred that administrative action. A green workflow does not mean
GitHub prevents an unverified merge, so local/PR discipline remains mandatory.

Repository settings also retained merge commits and did not automatically delete branches at
handoff time. Issue #32 tracks cleanup. Do not describe either setting as fixed until an API or
GitHub UI check confirms it.

### Release status

- repository: public;
- project status: pre-alpha;
- workspace version: `0.1.0-alpha.1`;
- releases: none at handoff time;
- license: not selected;
- crate publication: disabled.

Public visibility is not an OSS license. Issue #33 is the release gate.

## What is actually implemented

### Deterministic kernel and Artifact IR

The Rust workspace contains:

- `sightlint-ir`: versioned medium-neutral data contracts, validation, evidence, units, and
  canonicalization;
- `sightlint-engine`: deterministic geometry queries, atomic rules, result/report construction;
- `sightlint-adapter-png`: bounded PNG parsing, raster acquisition, and advisory image inspection;
- `sightlint-cli`: public command surface and exit-code behavior.

The trusted kernel can consume structured Artifact IR and emit deterministic, evidence-linked
results. It distinguishes:

- `passed`;
- `failed`;
- `inapplicable`;
- `cantTell`;
- `untested`.

It keeps layout, render/ink, and hit geometry separate. Outcome, severity, evidence strength,
confidence, coverage, and blocking policy are separate concepts.

### Structured visual rules

Current rules and contracts cover high-confidence geometry/typography when their required facts
and peer/policy declarations are already present in IR. The implemented areas include:

- bounds within a canvas;
- declared non-overlap;
- declared peer spacing consistency;
- parent containment;
- alignment contracts;
- peer width/height consistency;
- peer typography consistency;
- project-supplied minimum font-size policies;
- direction, unit, coordinate-space, tolerance, evidence, and ambiguity handling.

This does **not** mean SightLint can infer all of those facts and peer relations from an arbitrary
screenshot. The rule kernel and the acquisition problem are distinct.

### PNG source and raster path

The verified PNG path performs, with explicit resource limits:

```text
PNG signature and IHDR validation
  -> complete chunk framing/order/CRC validation
  -> bounded IDAT zlib/DEFLATE inflation
  -> all five scanline-filter reconstructions
  -> non-interlaced or Adam7 pass handling
  -> staged row-major PNG-encoded RGBA8 for supported inputs
```

The supported raster subset is eight-bit grayscale, RGB, grayscale-alpha, and RGBA without
`tRNS`. Palette/indexed, sub-byte, 16-bit, `tRNS`, animation, and over-budget cases are explicitly
unavailable rather than guessed. Raw pixels remain inside the adapter API. Serialized IR stores
bounded metadata, evidence, and a regression checksum rather than the raw raster.

The samples are PNG-encoded channel values, not color-managed display values. No gamma/ICC/
chromaticity transformation or alpha compositing is applied. They are insufficient by themselves
for a trusted contrast/colorimetric verdict.

### Advisory image inspection

`inspect-image` is an opt-in acquisition experiment, separate from the trusted CheckReport path.
Under one narrow policy—fully opaque raster and one identical perimeter color—it:

- treats that perimeter color as an unconfirmed background candidate;
- extracts bounded four-connected non-candidate regions;
- records exact region bounds and pixel counts;
- proposes groups of at least three same-size, same-color solid rectangles in one row/column;
- rejects a group when another region intersects its intervening strip;
- reports exact device-pixel gaps and `uniform`/`unequal` measurement patterns;
- emits an advisory for unequal gaps;
- keeps `uxVerdict: cantTell` and `blocking: false`.

The clean/mutated synthetic card pair is measured as `[1, 1]` versus `[1, 2]`. This proves the
narrow measurement path, not that unequal spacing is semantically wrong in a real interface.
Identical pixels with an “intentional grouping” annotation receive the same output.

### Realistic Web evaluation foundation

ADR 0032 and `evaluation/web/` provide the first issue #22 foundation without implementing a Web
adapter. The committed repository-owned dashboard has six reviewed state/environment records:

- three required public-binary smoke cases for explicit peer spacing;
- one clean baseline and one targeted 16 CSS-pixel mutation;
- one intentional-grouping hard negative that excludes an adjacent promotion from the metric peer
  relation;
- development records for ambiguous peer intent, a narrow viewport, and 125% text scale;
- separate acquisition and rule annotation documents;
- explicit source ownership, pending-license, privacy, split, and holdout declarations.

All browser-derived DOM/accessibility snapshots, computed render/hit geometry, screenshots, and
native/pixel reconciliation remain `untested` for issue #23. The runnable inputs use independently
authored declared Artifact IR projections and do not establish acquisition accuracy. Metrics are
small-corpus regression counts, not general UI/UX accuracy.

### Public commands

The current command families include:

```bash
sightlint check INPUT [--format human|json] [--deny-cant-tell]
sightlint normalize INPUT
sightlint schema
sightlint version
sightlint adapt-image INPUT
sightlint check-image INPUT [--format human|json] [--deny-cant-tell]
sightlint inspect-image INPUT [--format human|json]
```

Use `cargo run --locked -p sightlint-cli -- ...` until packaging is defined.

For checks, exit codes are 0 for no denied failures, 1 for a failed/strictly denied result, and 2
for usage/input/execution errors. `inspect-image` never exits 1 for a heuristic; observations or
explicit unavailable coverage exit 0, malformed/usage/execution errors exit 2.

## Continuous verification currently in `main`

SightLint treats tests as part of the product specification.

### Conformance and rule fixtures

- deterministic generated Artifact IR corpus under `fixtures/e2e/`;
- public-binary tests for pass, fail/mutation, `cantTell`, `inapplicable`, malformed inputs,
  canonicalization, ordering invariance, stdin/file paths, formats, limits, and exit codes;
- versioned product smoke evaluation under `evaluation/` with targeted rule mutations.

### Image fixtures

- 38-case PNG raster corpus with exact input bytes and independent expected pixels,
  explicit unavailable cases, malformed inputs, filters, Adam7, alpha values, and a future
  semantic spacing pair;
- 30-case image-inspection corpus with independently declared region/gap oracles,
  19 observed cases, nine explicit unavailable cases, and two malformed inputs;
- negative controls for blockers, differing sizes/colors, holes, mixed regions, touching and
  diagonal components, border variation, transparency, translation, scaling, recoloring, and
  intentional unequal grouping;
- API/file/stdin/human/JSON/repeated-byte checks and actual budget-boundary unit tests.

### Web evaluation fixtures

- source-digest and reference validation for the repository-owned dashboard fixture;
- six reviewed case records with separate acquisition/rule oracles;
- three repeated public-binary smoke executions;
- one killed peer-spacing mutation and one nonfailing intentional-grouping hard negative;
- three explicit deferred abstentions for ambiguous/responsive/text-scale acquisition;
- no claimed browser acquisition, screenshot corpus, or holdout.

Synthetic success is regression evidence, not real-world accuracy evidence.

### Required normal CI

The normal workflow is read-only and includes:

- generated fixture drift checks;
- rustfmt;
- Clippy with warnings denied;
- full workspace tests;
- explicit public-binary E2E and evaluation targets;
- rustdoc with warnings denied;
- Rust 1.85.0 MSRV check;
- full tests on Linux, macOS, and Windows.

A change is not complete until the exact final PR head and then the merged `main` commit are green.
A successful run on an earlier head is not reusable evidence.

## What is not implemented

Do not infer these capabilities from the architecture or closed experimental branches:

- general screenshot-only UI/UX review;
- rounded-card, shadow, gradient, photo, text, hierarchy, or semantic component understanding;
- OCR, CV-model, or VLM workers;
- automatic peer-role or design-intent inference;
- trusted spacing failures derived only from image grouping;
- color management, compositing, or trusted contrast from PNG samples;
- Playwright/DOM/accessibility capture;
- PPTX, PDF/document, Android, or iOS adapters;
- baseline/semantic visual diff beyond current explicit contracts;
- dynamic interaction traces, pending/error/recovery/destructive-action rules;
- MCP, GitHub Checks annotations, editor extension, browser extension, or local GUI;
- broad automatic fixes;
- packaged installation or public release.

## Product hypothesis and largest remaining risks

The core hypothesis remains viable:

> Convert native structures, pixels, and traces into evidence-backed observations; resolve a
> narrow policy; then make the final obligation deterministic and inspectable.

The largest risks are now product/evidence risks rather than basic kernel feasibility:

1. **Structure acquisition** — identifying the right objects, hierarchy, peer groups, and text
   from screenshots or imperfect native structures.
2. **Applicability** — deciding when a general UI/UX principle applies without turning valid
   variation into a false positive.
3. **Policy quality** — supplying useful defaults with no user prompt while distinguishing
   project policy, platform policy, inferred norms, and conservative baselines.
4. **Evaluation quality** — avoiding synthetic self-confirmation, benchmark leakage, weak labels,
   and unrepresentative examples.
5. **Cross-source reconciliation** — preserving conflicts between native structure and rendered
   pixels rather than normalizing them away.
6. **Scope drift** — spending the project on codecs, generic computer vision, GUI, MCP, or release
   polish before proving useful findings.

## Canonical next sequence

Issue #34 is the execution epic for the first evidence-backed zero-setup web UI alpha. Until it is
complete, select the earliest unblocked item in this sequence:

1. **#22 — human-reviewed realistic evaluation corpus.** The ADR/schema/dashboard/oracle foundation
   is present. Continue with reviewed synchronized native/pixel capture, broader cases/review, and
   representative metrics without treating the visible development data as holdout.
2. **#23 — Playwright web adapter.** Capture DOM/accessibility/computed layout and synchronized
   screenshot in an isolated TypeScript/Node process, then reconcile evidence into IR.
3. **#24 — recommended zero-setup rule packs.** Admit a small set of high-confidence rules only
   after evidence demonstrates acceptable precision/coverage/abstention.
4. **Agent loop within #34.** One local command, canonical report, targeted defect, Codex fix, and
   post-fix rerun through the same checker.
5. **#33 — alpha release gate.** Resolve license, packaging, compatibility, supply chain, and
   distribution after product evidence exists.

Do not skip #22 to tune a broad screenshot heuristic. Do not skip #23 by placing browser/model
logic in the Rust kernel. Do not skip #24 by calling raw measurements a complete product.

## Other preserved backlog

- **#25:** compare strict/current background policy with ranked-border candidates and scalable
  row-run/union-find segmentation against realistic hard negatives.
- **#26:** exact alpha-visible geometry for transparent assets.
- **#27:** optional PNG palette/sub-byte/16-bit/`tRNS` support, only if product evidence justifies
  more codec maintenance and after an explicit library-versus-custom decision.
- **#28:** isolated OCR/CV/VLM perception-worker protocol and calibration.
- **#29:** structured PPTX, PDF/document, Android, and iOS adapter roadmap.
- **#30:** interaction actions/effects/states/traces and recovery contracts.
- **#31:** CLI packaging, Codex/MCP/GitHub/editor/local-UI ecosystem.
- **#19:** branch protection/ruleset administration.
- **#32:** legacy branch cleanup and automatic branch deletion.
- **#33:** licensing/release/compatibility/packaging gate.

Issues express future work, not implemented behavior. An issue body may contain design hypotheses;
accepted ADR plus tested code on `main` is required to make one normative.

## Local Codex session procedure

### 1. Establish the truth before planning

```bash
git fetch --all --prune
git switch main
git pull --ff-only
git status --short --branch
git log -1 --format='commit=%H tree=%T subject=%s'
```

Then inspect:

- `AGENTS.md`;
- this file;
- `docs/product-rationale.md`;
- `docs/decision-history.md`;
- `docs/roadmap.md`;
- the selected issue and linked accepted ADRs;
- current source/tests, not a historical branch implementation.

Check GitHub for an existing active PR/branch for the selected issue. Do not duplicate current work.

### 2. Select one issue and define the evidence

Before editing, write a concise plan that states:

- issue/milestone advanced;
- user-visible claim being added or changed;
- exact/inferred evidence and trust boundary;
- applicability and expected outcomes;
- pass, mutation/fail, `cantTell`, inapplicable, malformed, boundary, resource, determinism, and
  product-evaluation cases that apply;
- explicit non-goals;
- whether an ADR is required.

If those questions cannot be answered, the task is not implementation-ready.

### 3. Create one focused branch from `main`

```bash
git switch -c feat/<focused-name>
```

Use one branch and one PR per coherent issue slice. Do not create placeholder, final, review,
ready, repair, bootstrap, or `v2` branch chains. If a design changes, update the same branch or
close it with an explanation.

### 4. Implement vertically

Prefer the smallest user-visible path that reaches the real command and can be tested end to end:

```text
native input
  -> adapter/acquisition
  -> validated observations/IR
  -> deterministic query/rule or advisory report
  -> public CLI
  -> committed fixture/oracle
```

Do not accumulate unconnected modules or tests for APIs that the public path does not call.

### 5. Run the complete local gate

At minimum, from repository root:

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

Add new generator/E2E commands to `AGENTS.md`, the PR template, CI, and this handoff when a new
public corpus or adapter is introduced.

### 6. Open a truthful PR

The PR must identify:

- exact base and head;
- reachable public behavior;
- evidence and uncertainty;
- fixture/oracle additions;
- security/privacy/resource implications;
- compatibility changes;
- explicit non-claims;
- exact CI run for the final head.

Do not claim a feature from code that is unconnected, a Draft branch, or tests that do not invoke
the real binary. Do not weaken an oracle merely to match implementation output.

### 7. Verify after merge

After merge:

- confirm `main` points to the intended merge/squash commit;
- confirm the source tree contains the expected files and no temporary workflow;
- confirm `main` CI succeeds on that exact commit;
- update this handoff and roadmap when current capability or priority changed;
- close/supersede the issue as appropriate;
- delete the merged branch when repository settings permit.

## Practices that are specifically prohibited

The remote construction phase exposed failure modes that must not recur:

- self-writing GitHub Actions used to assemble, format, repair, or commit feature code;
- temporary workflows with `contents: write` left in a branch or `main`;
- duplicate source files such as `foo.rs` and `foo_next.rs` awaiting later wiring;
- tests targeting an API that the library does not export or public path does not call;
- reopening/retargeting historical PRs to imply current completion;
- merging based on `mergeable: true` rather than green exact-head checks;
- reporting success from an old workflow run;
- treating synthetic decoding/measurement as real-world UX accuracy;
- allowing an LLM or heuristic-only semantic guess to block by default;
- pushing feature work directly to `main`;
- adding broad dependencies, hosted services, databases, GUI, MCP, or more codecs because they
  are convenient rather than required by the current evidence-gated milestone.

## Handoff maintenance rule

This document is not a diary. Update it whenever one of these changes:

- verified public commands or supported inputs;
- rule/evidence/report semantics;
- current milestone or issue sequence;
- accepted/superseded architecture decision;
- evaluation corpus or supported accuracy claim;
- hosting/release/licensing status;
- known stale branches/PRs that could mislead the next session;
- canonical local validation commands.

A PR that changes those facts is incomplete until the handoff is updated in the same PR.

## Definition of a successful continuation

A local Codex session is continuing SightLint correctly when it can explain, before editing:

- why SightLint separates acquisition, evidence, policy, applicability, and judgment;
- why native structure and pixels are reconciled rather than one replacing the other;
- why uncertainty is a result rather than an error to hide;
- why the next product gate is realistic evaluation and a structured web adapter;
- why stale Draft branches are not a shortcut;
- which exact E2E proves the public claim;
- what remains unimplemented and what the PR must not claim.
