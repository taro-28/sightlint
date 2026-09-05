# Local Codex handoff

This document is the operational handoff for continuing SightLint in the local Codex app or
another local coding-agent environment. It is intentionally explicit: a new session must be able
to determine the trusted source of truth, current capability, next task, decision background,
quality gates, and historical traps without access to the chat that created the repository.

Last handoff preparation: 2026-09-06.

## Start here

The authoritative development line is the latest green commit on `main`.

At the start of the repository-settings documentation slice, the latest verified baseline was:

- commit: `2dbffba6802151842d4dfb7720b2367f589b6d1b`
- tree: `ed1b6ce7e492cf5ff069bbd43cc6d5c1c04e4d9e`
- merged PR: #43
- main CI: run 33997596273, all six jobs successful

The current focused branch records the already-applied GitHub settings for issues #19 and #32.
After it merges, its resulting `main` commit supersedes the hash above as the repository starting
point. Never hard-code this hash as a branch base. Verify the current `main` and its CI.

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

Issue #32 removed the 27 merged, duplicate, and obsolete remote branches retained by the
mobile/remote construction phase. Only `main` and intentional short-lived branches for current
pull requests remain. GitHub now deletes a head branch automatically after merge.

Historical PRs and Actions runs remain available as evidence, but their deleted branch names are
never valid development bases. Start from the latest green `main`, and preserve future intent in
issues and accepted documentation rather than recreating a historical branch.

### Hosting settings

Issue #19 is implemented by an active `Protect main` ruleset targeting the default branch. It
requires pull requests, up-to-date versions of the five exact CI contexts, linear history, and
resolved review conversations; it blocks force pushes and deletion with an empty bypass list.
Zero reviewer approvals remains appropriate for the single-maintainer workflow. Squash is the only
enabled merge method, GitHub suggests branch updates, and merged head branches are deleted
automatically.

The required contexts are exactly:

- `Format, lint, test, and docs`;
- `Minimum Rust 1.85.0`;
- `Test on ubuntu-latest`;
- `Test on macos-latest`;
- `Test on windows-latest`.

Private vulnerability reporting, the dependency graph, Dependabot alerts/security updates, and
advisory Rust CodeQL default setup are enabled. Actions default to read-only repository access,
cannot create or approve pull requests, require full-length action SHAs, and permit only the owner
plus the explicitly selected `actions/checkout` and `github/codeql-action` families. Secret
Protection and Push Protection remain enabled. GitHub Apps access is limited to the Codex
Connector for this repository. No self-hosted runner, webhook, deploy key, Pages site, or
Environment is configured. Discussions remains disabled and no committed issue-template link
points to it.

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

ADR 0032 and `evaluation/web/` provide the first issue #22 foundation. The committed
repository-owned dashboard has six reviewed declared-IR state/environment records:

- three required public-binary smoke cases for explicit peer spacing;
- one clean baseline and one targeted 16 CSS-pixel mutation;
- one intentional-grouping hard negative that excludes an adjacent promotion from the metric peer
  relation;
- development records for ambiguous peer intent, a narrow viewport, and 125% text scale;
- separate acquisition and rule annotation documents;
- explicit source ownership, pending-license, privacy, split, and holdout declarations.

The original runnable inputs use independently authored declared Artifact IR projections and do
not establish acquisition accuracy. Their browser fields remain `untested` rather than being
rewritten from implementation output. Metrics are small-corpus regression counts, not general
UI/UX accuracy.

### Playwright Web acquisition and evidence matrix

ADRs 0033–0035 and `adapters/playwright/` add an untrusted TypeScript/Node process with locked
Playwright/Chromium dependencies. Protocol `0.1.0` accepts a repository-contained local HTML
fixture and emits canonical Artifact IR plus an `org.sightlint.web@0.3.0` extension and a
synchronized PNG viewport screenshot. It records selected DOM hierarchy, locator-scoped
accessibility summaries, computed style, layout/render geometry, client/scroll overflow,
rectangular ancestor clipping, render-box-center hit samples, document and viewport canvases,
scroll translation, writing direction, environment/version provenance, privacy/network status,
bounded native/screenshot reconciliation, and explicit DOM/render/optional accessibility evidence
identifiers per selected node. Complete hit regions remain `cantTell`; a center sample is not
serialized as core `hitBox`.

Twenty-three independent browser requests and acquisition/rule oracles cover clean baselines; 11
targeted acquisition mutations for bounds, spacing, clipping, overflow, occlusion, peer dimension,
transformed text, programmatic names, and desktop/mobile positioning; hard negatives; responsive,
text-scale, RTL/vertical-writing, disabled/hidden/offscreen-control, scrollable clipping, and
ambiguous states. E2E invokes the actual Node process and built `sightlint` binary, validates
current and retained previous schemas, checks stable error/resource/recognized-extension behavior,
and compares repeated response, IR, screenshot, and report bytes on Linux. Accessibility snapshot
root parsing uses disjoint escaped/unescaped name branches and includes an adversarial repeated-
escape regression so untrusted names do not induce regex backtracking growth.

Current development-corpus metrics are 23/23 cases, 76 reviewed acquisition expectations, 45
reviewed acquisition abstentions, 11/11 acquisition mutations observed, 6/6 rule-eligible
mutations killed, 6/6 emitted failures matched, zero unexpected failures, and zero hard-negative
failures. These are single-family public regression counts, not real-world accuracy.

This slice supports exactly one main frame/page, local `file:` fixtures, and at most 200 selected
nodes. It does not infer semantic peer relations or compare pixel content; those aspects remain
`untested`/`cantTell`. Cross-platform screenshot byte identity and real-world UI/UX accuracy are
not claimed.

### Zero-setup recommended Web pack

ADR 0035 makes `sightlint:recommended` the additive default for `check` and `check-image`, with
`--profile base` as the explicit opt-out. When a strictly validated Web extension is present, the
Rust kernel runs three atomic rules:

- `web.accessibility.interactive-name@0.1.0`;
- `web.interaction.center-hit@0.1.0`;
- `web.interaction.ancestor-clip@0.1.0`.

Each CheckReport `0.3.0` result records policy source, profile, maturity, enforcement, and linked
evidence. Existing base/explicit rules are blocking; the three new Web rules are advisory and do
not cause exit 1. `--deny-cant-tell` remains an explicit stricter gate. The pack preserves
`cantTell` for incomplete platform semantics, intentional dialog overlays, transforms, incomplete
hit-region evidence, and scrollable clipping. It does not turn raw overflow, screenshots, or peer
dimensions into automatic defects.

For each recommended rule, the public browser E2E covers 5/5 contracted outcome-category entries,
1/1 matched failure, 2/2 reviewed abstentions, 1/1 killed targeted mutation, and zero hard-negative
failures. The corpus is public, fictional, maintainer-reviewed, and non-holdout; these counts do not
establish WCAG conformance, representative precision, or blocking maturity.

### One-command local Web agent workflow

ADR 0036 adds `sightlint-web-check` in the untrusted Node adapter package. One invocation captures
the repository-owned fixture into private temporary storage, calls the built public Rust binary
with `sightlint:recommended`, and emits either a canonical workflow report `0.1.0` or a stable
color-free human report. The envelope preserves the complete capture response and CheckReport and
joins node results to captured native selectors, source-bundle paths, and evidence identifiers.

Node does not compute or change outcomes. Exit 0/1 is preserved from the Rust blocking policy;
operational/contract failures exit 2. Temporary absolute paths are not serialized and the capture
files are removed. Native selectors are navigation hints, not exact source-line attribution.

The reviewed public smoke E2E runs the same command repeatedly on an isolated copy of the Atlas
unnamed-control mutation, applies only the human-authored edit in
`evaluation/web/annotations/agent-workflow.json`, and reruns it. It proves byte stability within
the declared environment, the reviewed finding-to-selector join, disappearance of the named
finding, zero new failures, and retained `cantTell` for an ambiguous control and intentional dialog
overlay. It does not prove autonomous edit selection, representative agent success, a protected
holdout, WCAG conformance, or general Web UI/UX accuracy.

### Public commands

The current command families include:

```bash
sightlint check INPUT [--format human|json] [--deny-cant-tell] [--profile recommended|base]
sightlint normalize INPUT
sightlint schema
sightlint version
sightlint adapt-image INPUT
sightlint check-image INPUT [--format human|json] [--deny-cant-tell] [--profile recommended|base]
sightlint inspect-image INPUT [--format human|json]
```

Use `cargo run --locked -p sightlint-cli -- ...` until packaging is defined.

The Web adapter currently has a separate Node process surface:

```bash
node adapters/playwright/dist/src/cli.js \
  --request REQUEST.json \
  --repository-root REPOSITORY \
  --artifact-ir-out ARTIFACT-IR.json \
  --screenshot-out SCREENSHOT.png
```

The bounded combined agent surface is:

```bash
node adapters/playwright/dist/src/check-cli.js \
  --request REQUEST.json \
  --repository-root REPOSITORY \
  --sightlint-binary target/debug/sightlint \
  --format json
```

Omit `--format json` for human output. It performs capture plus the Rust check without persisting
temporary artifacts or moving verdict policy into Node.

For checks, exit codes are 0 for no blocking failure, 1 for a blocking failure or explicitly denied
`cantTell`, and 2 for usage/input/execution/recognized-extension validation errors. Advisory Web
failures remain visible with exit 0. `inspect-image` never exits 1 for a heuristic; observations or
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
- 23 companion browser captures with separate acquisition/rule oracles;
- synchronized DOM/accessibility/computed geometry, overflow/clipping/center-hit evidence, and
  viewport screenshot metadata;
- six rule-eligible mutation kills, 11 acquisition mutations, per-rule policy/profile/enforcement
  assertions, hard negatives, and explicit semantic/pixel/hit-region abstentions;
- one public reviewed agent workflow oracle covering the combined command, native source-target
  join, temporary fix/rerun, no-new-failure postcondition, JSON/human byte stability, and
  ambiguity/intentional-overlay controls;
- no representative screenshot corpus, private holdout, or general accuracy claim.

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
- arbitrary-URL, iframe, shadow-DOM, full accessibility-tree, or interaction capture;
- automatic semantic peer inference from Playwright output;
- PPTX, PDF/document, Android, or iOS adapters;
- baseline/semantic visual diff beyond current explicit contracts;
- blocking recommended Web rules, project overrides, or representative real-world rule evidence;
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

1. **#22 — human-reviewed realistic evaluation corpus (complete).** The
   ADR/schema/dashboard/oracle foundation and synchronized browser companion are present.
2. **#23 — Playwright web adapter (complete).** Protocol `0.1.0` plus
   `org.sightlint.web@0.3.0` supplies the bounded isolated local-fixture path and issue-required
   evidence matrix. Arbitrary applications and cross-platform screenshot identity remain
   explicit non-goals/non-claims.
3. **#24 — first recommended zero-setup Web pack (complete).** Three narrow advisory rules run by
   default with explicit policy provenance, conservative abstention, and per-rule regression
   metrics.
4. **#42 — agent loop within #34 (complete).** One local command, canonical report, targeted
   defect, reviewed source edit, and post-fix rerun through the same checker.
5. **#33 — alpha release gate (next).** Resolve license, packaging, compatibility, supply chain, and
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
- **#33:** licensing/release/compatibility/packaging gate.

Issues express future work, not implemented behavior. An issue body may contain design hypotheses;
accepted ADR plus tested code on `main` is required to make one normative.

Administrative issues #19 and #32 are complete. Their issue bodies and comments preserve the
verified policy and cleanup history; they are not product roadmap work.

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
- verify that GitHub deleted the merged head branch automatically.

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
- why the next product gate is the licensing, compatibility, packaging, and alpha distribution
  work in issue #33;
- why stale Draft branches are not a shortcut;
- which exact E2E proves the public claim;
- what remains unimplemented and what the PR must not claim.
