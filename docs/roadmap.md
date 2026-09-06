# Roadmap

The roadmap controls scope and sequencing. Completing a milestone means satisfying its exit
criteria on the current `main`, not merely adding code, opening a Draft PR, or using a matching
branch name.

Read `docs/handoff.md` first for the factual repository state and `docs/product-rationale.md` for
the product model behind this sequence. Issue #34 is the canonical near-term execution epic.

## Current status

| Milestone | Status | Verified scope | Main remaining gap |
|---|---|---|---|
| M0 — Foundation | complete | architecture, ADRs, Rust workspace, local-first trust boundary, CI, hosting protection, license | ongoing governance and security review |
| M1 — Deterministic vertical slice | complete | Artifact IR, validation, canonicalization, evidence, rules, reports, CLI, binary E2E | continued compatibility discipline |
| M2 — Visual geometry rules | substantially implemented | explicit containment, overlap, spacing, alignment, extent, typography, minimum-size policy | acquisition of reliable applicability/evidence; contrast and semantic baseline work |
| M3 — Deterministic image adapter | active | bounded PNG path, exact common RGBA/source-alpha geometry, 43-case pixel corpus, five-case transparent-asset evaluation, narrow advisory region/gap inspection with 30 cases, first realistic Web evaluation foundation | representative realistic evaluation, an evaluated image/asset rule or reconciliation use case, evidence-gated broader acquisition |
| M4 — Structured adapters | active | process-isolated Playwright capture plus 23-case evidence/rule matrix and first advisory recommended Web pack | portability characterization, broader representative evaluation, then other media by demand |
| M5 — Optional perception | not started | isolation principles only | versioned OCR/CV/VLM worker protocol and calibration |
| M6 — Interaction contracts | not started | conceptual rule model only | actions, effects, states, traces, recovery, controlled E2E |
| M7 — Ecosystem and release | active | local CLI, bounded one-command Web agent workflow, dual license, source-only alpha release | MCP/GitHub/editor surfaces and demand-led package channels |

The project has a narrowly scoped source-only alpha. The kernel and structured-IR path are much
more mature than the ability to infer useful structure and applicability from an arbitrary
screenshot.

## Near-term product outcome

The first meaningful product target is not “support every artifact” or “complete PNG.” It is:

> A local coding agent can run one command against a repository-owned web UI, receive a small set
> of high-confidence basic UI/UX findings without manually describing every element, edit the
> source, rerun the same check, and verify the named finding is resolved.

Issue #34 established this bounded outcome. It does not establish arbitrary-site or general
screenshot UI/UX accuracy.

### Required sequence

1. **#22 — Realistic evaluation and annotation gate (complete)**
   - build deterministic repository-owned web fixtures with clean and targeted-mutant states;
   - capture or annotate native structure, rendered geometry, screenshots, peer/hierarchy
     relations, policy applicability, hard negatives, and ambiguity;
   - define acquisition and rule ground truth separately;
   - report precision, coverage, false positives, abstention, mutation kill rate, and a holdout
     process.
2. **#23 — Playwright structured web adapter (complete)**
   - isolate browser automation in TypeScript/Node;
   - capture DOM/accessibility/computed geometry and a synchronized screenshot;
   - preserve layout/render/hit geometry, selectors, units, browser environment, privacy, and
     resource limits;
   - reconcile native and pixel evidence instead of choosing one globally.
3. **#24 — Recommended zero-setup rule packs (complete for the first Web pack)**
   - the additive default `sightlint:recommended` profile admits three narrow Web rules with named
     policy provenance;
   - `--profile base` is the explicit opt-out, and profile/enforcement are canonical report data;
   - all three rules remain advisory and preserve `cantTell` for unresolved alternatives.
4. **#42 — Agent fix-and-rerun slice within #34 (complete)**
   - expose one local command and canonical machine report;
   - demonstrate Codex locating a source target, applying a focused edit, and rerunning SightLint;
   - verify the original finding disappears without hiding new failures.
5. **#33 — First alpha release gate (complete)**
   - resolve license, compatibility surfaces, packaging, supply-chain checks, install, and release
     documentation only after the product path above has evidence.

Do not skip #22 to tune a broad screenshot heuristic. Do not skip #23 by putting browser or model
logic into the Rust kernel. Do not skip #24 by presenting raw measurements as a complete UI/UX
reviewer.

### Completed #22–#24 evaluation/acquisition/rules and #42 agent path

ADR 0032 establishes an independently versioned Web evaluation contract and one repository-owned
dashboard fixture. Six reviewed records separate acquisition annotations from rule verdicts. Three
smoke cases exercise the existing explicit peer-spacing rule through the public binary with a clean
baseline, targeted mutation, and intentional-grouping hard negative; three declared-IR development
cases keep ambiguous, narrow-viewport, and text-scale acquisition explicitly untested.

ADRs 0033 and 0034 add a separate browser companion without overwriting those projections.
The isolated Node process captures selected DOM/accessibility structure, computed geometry,
client/scroll overflow, rectangular ancestor clipping, render-box-center hit samples, writing
direction, a synchronized viewport screenshot, and explicit native/screenshot reconciliation.
ADR 0035 evolves that companion to 23 cases and the official optional
`org.sightlint.web@0.3.0` extension, adds explicit per-node evidence references, and admits three
advisory deterministic rules for programmatic names, one center-hit sample, and non-scrollable
ancestor clipping. Human-authored acquisition and rule oracles cover 11 targeted acquisition
mutations, 6 rule-eligible mutation kills, hard negatives, ambiguity, responsive layout, text
scale, and 45 explicit acquisition abstentions. The actual capture and built Rust binary are
exercised together on Linux with byte-stability, profile override, malformed-extension, and
per-rule metric checks.

ADR 0036 adds one `sightlint-web-check` invocation that keeps browser orchestration in Node and
rule verdicts in the public Rust binary. Workflow report `0.1.0` retains capture/runtime
provenance and the complete CheckReport while joining node results to native locators and
source-bundle navigation hints. A public reviewed E2E performs the unnamed-control mutation,
human-authored source edit in an isolated copy, and post-fix rerun; it checks byte stability,
removal of the named finding, no new failure, and retained ambiguous/intentional-overlay
`cantTell` behavior.

ADR 0037 and accepted ADR 0007 complete the release gate with dual `MIT OR Apache-2.0` licensing,
surface-specific alpha compatibility, dependency-license checks, and a deterministic source
archive verified on the supported hosted systems. The first release deliberately publishes no
prebuilt binary, Cargo crate, npm package, installer, container, signature, or attestation.
ADR 0038 keeps the cross-platform verification jobs read-only by carrying the exact unpublished
bytes through a short-retention workflow artifact and comparing them with draft assets before
publication. The immutable alpha.1 attempt was not published; alpha.2 is the first supported tag.

This completes the bounded #22–#24, #42, and #33 sequence, but it is not evidence of general Web
accuracy. The corpus has one fictional application family, maintainer-only review, visible
development labels, no private holdout, no pixel-content identity, no complete hit regions, no
semantic peer inference, and no representative sampling. The three new rules therefore remain
advisory. The scripted edit is not a claim about autonomous agent quality or arbitrary
repositories. Issue #25 provides the bounded comparison without admitting a broader segmentation
default. ADR 0040 and issue #26 add exact source-alpha geometry without admitting a padding rule.
The earliest remaining implementation gate is #27.

## Scope-selection rules

When multiple tasks are possible, apply this order:

1. restore a broken verified contract or CI baseline;
2. add missing evaluation evidence required by the current claim;
3. complete the smallest vertical user-visible path for the current milestone;
4. improve precision, abstention, and hard-negative behavior;
5. expand medium/input coverage only when product evidence shows the need;
6. optimize or polish integrations after usefulness is demonstrated.

A pre-existing stale implementation does not outrank an earlier evidence gate. Future intent lives
in issues, not long-lived Draft branches.

## M0 — Project foundation

**Goal:** make architectural drift difficult before feature work starts.

### Delivered

- vision, principles, architecture, threat model, testing strategy, and accepted ADRs;
- coding-agent instructions and contribution workflow;
- Rust 2024 workspace and Rust 1.85.0 MSRV;
- deterministic/local-first trusted-kernel boundary;
- formatting, Clippy, tests, rustdoc, MSRV, and Linux/macOS/Windows CI;
- explicit license and release decisions, now resolved by accepted ADR 0007 and ADR 0037.

### Administrative status

- #19 — complete: active `Protect main` ruleset and required checks;
- #32 — complete: legacy branches removed and automatic branch deletion enabled;
- #33 — complete: dual license, compatibility policy, source packaging, and first alpha release.

Hosting protection does not authorize bypassing local, PR-head, or post-merge CI discipline.

## M1 — Deterministic vertical slice

**Goal:** prove the trusted pipeline without image recognition or browser automation.

### Delivered

- versioned medium-neutral Artifact IR and JSON schema;
- semantic validation, stable identifiers, evidence/selectors, confidence and uncertainty;
- explicit units and coordinate spaces;
- distinct layout, render/ink, and hit geometry;
- deterministic canonical serialization and normalization;
- geometry/query context and atomic rule execution;
- ACT-inspired outcomes `passed`, `failed`, `inapplicable`, `cantTell`, and `untested`;
- human and canonical JSON reports;
- stable CLI exit codes;
- generated fixtures and public-binary E2E across supported systems.

### Ongoing exit invariant

Every later milestone must preserve byte-stable normalized behavior, compatibility/versioning, and
conservative ambiguity handling. A new adapter may supply observations but cannot weaken the
kernel's evidence contract.

## M2 — Visual geometry and typography rules

**Goal:** cover high-confidence, mechanically testable visual defects when sufficient observations
and applicability are known.

### Delivered areas

- canvas bounds;
- declared non-overlap;
- explicit peer spacing consistency;
- parent containment;
- logical start/center/end alignment;
- peer width/height consistency;
- peer typography consistency;
- exact project-supplied minimum font-size policies;
- direction, tolerance, evidence, unit, coordinate-space, and `cantTell` behavior;
- cross-artifact synthetic fixtures and targeted mutations.

### Remaining candidate areas

- broader clipping and occlusion beyond the first conservative Web-control slice;
- safe areas and hit-target relationships;
- text overflow/truncation;
- responsive transformations;
- color and contrast after color-management/compositing evidence is defined;
- baseline and semantic diff;
- additional recommended policies and inferred project norms.

### Gate

Do not add a broad rule because it sounds like a best practice. The rule must define:

- exact target/applicability;
- observed facts and evidence grade;
- policy source, units, tolerance, and valid alternatives;
- pass/fail/mutation/ambiguity/inapplicable cases;
- hard negatives and false-positive risks;
- real or sufficiently realistic evaluation before recommended/blocking maturity.

ADR 0035 supplies the first #24 recommended-pack slice; further admissions still depend on the
#22 evaluation gate and #23 evidence source.

## M3 — Deterministic image adapter

**Goal:** make pixels a common evidence source without pretending that screenshot semantics are
exact.

### Verified current slice

The public PNG path performs:

```text
signature/IHDR validation
  -> bounded full chunk framing/order/CRC validation
  -> bounded zlib/DEFLATE inflation
  -> None/Sub/Up/Average/Paeth reconstruction
  -> non-interlaced/Adam7 layout handling
  -> bounded row-major PNG-encoded RGBA8 for supported eight-bit inputs
  -> exact encoded source-alpha geometry and evidence
```

Supported raster inputs are eight-bit grayscale, RGB, grayscale-alpha, and RGBA without `tRNS`.
Palette, sub-byte, 16-bit, `tRNS`, animation, and resource-limit cases return explicit
unavailability rather than fabricated pixels.

Current evaluation includes:

- 43 committed source-raster cases with independent exact pixel/alpha/unavailable/error oracles;
- five realistic transparent UI assets with separate acquisition/rule annotations, one targeted
  mutation, two hard negatives, explicit abstention, and no protected holdout;
- 30 committed image-inspection cases with independent region/gap/abstention/error oracles;
- nine realistic image-segmentation benchmark cases with separate acquisition/rule annotations,
  public split declarations, targeted mutation, hard negatives, metamorphic variants, and bounded
  resource refusal;
- API, file, stdin, human/JSON, malformed, limits, metamorphic, and repeated-byte E2E;
- a narrow `inspect-image` policy for fully opaque images with one exact perimeter color;
- four-connected regions and simple same-size/same-color solid-rectangle row/column candidates;
- exact gaps and nonblocking `uniform`/`unequal` observations;
- `uxVerdict: cantTell` because image geometry alone does not prove semantic peer intent.

ADR 0039 compares the strict policy with ranked exact-border flood fill and a 95%-qualified
corner/row-run policy. Qualified selection recovers the edge mutation and abstains on the two
required hard negatives, while ranked selection observes both hard negatives unsafely. All three
policies false-group realistic shadow-connected surfaces, so none is admitted as a semantic or
blocking product path.

### Explicit limitations

- no general text, role, hierarchy, card, button, rounded shape, shadow, gradient, photo, or
  antialias understanding;
- no trusted spacing failure from an image-only candidate group;
- no display color management, compositing, or trusted contrast;
- no real-world accuracy claim from synthetic corpora;
- no reason to extend every image codec before product evidence exists.

### Active follow-ups

- #22 — realistic UI acquisition/rule corpus (complete foundation);
- #25 — broader background and scalable segmentation benchmark (complete; no admission);
- #26 — exact source-alpha geometry for transparent assets (complete; no rule admitted);
- #27 — optional palette/sub-byte/16-bit/`tRNS` support after a custom-vs-library strategy and
  demonstrated product need.

### M3 exit criteria

M3 is complete enough to move into M4 when:

- the exact pixel path remains robust and bounded;
- image-derived observations clearly state hypotheses and abstain safely;
- realistic evaluation quantifies what screenshot-only acquisition can and cannot recover;
- at least one useful image/asset rule or reconciliation use case is evaluated;
- adding more codec/CV breadth has a documented evidence-based reason.

M3 completion does not require solving general semantic vision.

## M4 — Structured adapters

**Goal:** obtain richer meaning and source navigation from native artifact structures while using
pixels to verify rendered reality.

### First adapter: Playwright/web — #23

Capture protocol `0.1.0` with `org.sightlint.web@0.3.0` captures, from one controlled local session:

- DOM and frame hierarchy;
- accessibility roles/names/states;
- computed style and typography;
- layout/render rectangles, client/scroll overflow, rectangular ancestor clipping,
  render-box-center hit samples, transforms, scroll offsets, direction, and viewport; full hit
  rectangles remain explicitly `cantTell`;
- device-pixel ratio and deterministic browser/capture environment;
- synchronized screenshot and evidence reference;
- stable selectors, adapter/browser versions, privacy/network status, and resource limits;
- bounded agreement/conflict between native geometry and screenshot extent.

The browser runs in an isolated TypeScript/Node adapter process. It is not linked into the trusted
Rust kernel. Start with repository-owned local fixtures rather than arbitrary network URLs.

The #23/#24 matrix includes overlap/occlusion, clipping, overflow, visual/interactive extent,
hidden/disabled/offscreen, peer-dimension, transformed-text, responsive desktop/mobile,
RTL/vertical-writing, named/unnamed/ambiguous controls, scrollable clipping, and intentional-overlay
fixtures. The default recommended profile evaluates three narrow advisory obligations in Rust;
raw acquisition measurements remain non-verdicts. Characterizing macOS and Windows browser output,
and later support for iframes, shadow DOM, interaction, and arbitrary projects, remain
future compatibility/capability work rather than part of the bounded local-fixture protocol.
Pixel-content identity, complete hit regions, and semantic peer relations remain abstentions until
independently evaluated evidence exists.

### Other adapters — #29

Add one at a time according to demand and fixture quality:

1. PPTX/slides;
2. structured PDF/document;
3. Android semantics/accessibility plus screenshot;
4. iOS XCUI/accessibility plus screenshot.

This order is provisional. Every adapter needs an ADR, native fixture corpus, unit/coordinate plan,
trust/privacy/resource model, compatibility strategy, rendered differential tests, and public E2E.

### M4 exit criteria

- one stable versioned process-adapter protocol pattern;
- Playwright supplies enough evidence for evaluated zero-setup rules;
- native and pixel conflicts are represented rather than erased;
- medium-specific information stays in extensions;
- shared rules run without DOM-specific hacks in the kernel;
- at least one additional medium has an evidence-backed plan or implementation based on demand.

## M5 — Optional perception

**Goal:** recover useful inferred observations when native structure is absent or incomplete,
without moving model authority into the kernel.

Issue #28 owns the protocol for OCR, deterministic CV, component/hierarchy detection, and optional
VLM workers.

### Required boundary

- isolated process with versioned request/response schema;
- exact input digest, preprocessing, model/runtime/version, local/remote status, and resource
  limits;
- observations with geometry, candidates, confidence or calibrated probability, alternatives,
  uncertainty, and evidence links;
- canonical ordering before the kernel;
- measured run-to-run agreement separate from confidence;
- conflicts retained against native and pixel facts;
- local-first default and explicit remote transmission policy.

### M5 exit criteria

- one reference local worker and conformance suite;
- differential evaluation against native/annotated data from #22/#23;
- measured acquisition precision, coverage, abstention, and downstream rule impact;
- no model-only blocking verdict;
- model updates cannot hide behind unchanged evidence versions.

## M6 — Interaction contracts

**Goal:** verify user-visible behavior that cannot be established from a static artifact.

Issue #30 owns actions, preconditions, effects, scope, pending/optimistic/success/failure/partial
states, recovery, safeguards, focus, navigation, and deterministic traces.

### Candidate first obligations

- pending/optimistic feedback for latent actions;
- duplicate-submit prevention or idempotency;
- visible success/failure distinction;
- retained input and retry without duplicate effects;
- itemized partial success;
- scope and safeguards for destructive actions;
- focus visibility and predictable focus movement.

### M6 exit criteria

- versioned medium-neutral trace/effect extension;
- one controlled adapter and deterministic fixture application;
- atomic and composite obligations with multiple valid solutions;
- clean/mutation/`cantTell`/inapplicable/`untested` trace E2E;
- controlled slow, offline, failure, permission, stale-data, and recovery scenarios;
- static screenshots never claim invisible dynamic behavior.

## M7 — Ecosystem, agent workflow, and release

**Goal:** expose the same verified kernel through useful local and collaboration surfaces without
creating another source of truth.

Issue #31 covers:

- stable local CLI and canonical agent output;
- local Codex workflow;
- MCP or equivalent protocol;
- GitHub Checks and evidence annotations;
- editor/browser/local UI;
- optional policy/history service that does not replace local core.

Issue #33 resolved license, independent compatibility surfaces, source packaging, release
provenance, supply-chain checks, install documentation, and the first public alpha. It selected a
source-only release and deferred binary/crate/npm channels until demand and channel-specific
contracts justify them.

### First bounded slice

- `sightlint-web-check` composes local Playwright capture with the existing public Rust check;
- canonical workflow report `0.1.0` provides source/evidence navigation without changing verdicts;
- a separately reviewed public oracle fixes one targeted mutation in a temporary fixture copy and
  requires the named finding to disappear with no new failure;
- ambiguity and intentional-overlay controls preserve `cantTell`;
- temporary artifacts stay local and are removed; no hosted processor or automatic source editor
  is introduced.

### First alpha release

- `v0.1.0-alpha.2` is a source-only GitHub prerelease under `MIT OR Apache-2.0`;
- a deterministic tracked-source archive and canonical SHA-256 record are verified before
  publication;
- the extracted Rust workspace is tested on Ubuntu x64, macOS arm64, and Windows x64;
- the extracted full Playwright product path is tested on Linux with Node 20–24 and pinned
  Chromium;
- checksums detect corruption but do not authenticate the publisher; signing, attestations,
  registries, installers, and prebuilt binaries remain deferred and unclaimed.
- the immutable `v0.1.0-alpha.1` attempt remains unpublished; ADR 0038 records its draft-download
  failure and the verified alpha.2 recovery without moving the old tag.

### Sequencing rule

Do not polish MCP, GitHub annotations, GUI, or installers to compensate for weak product evidence.
The minimum local agent loop in #34 may precede broad ecosystem work; full M7 follows evaluated
usefulness.

### M7 exit criteria

- one stable local install path on supported desktop systems;
- agent-friendly canonical command/protocol backed by the same kernel;
- source/evidence navigation and post-fix rerun demonstrated;
- privacy/offline behavior tested;
- license and compatibility policy resolved;
- release artifacts verified and documented;
- no universal score or hidden cloud requirement.

## Cross-cutting quality gates

Every public behavior must satisfy the applicable matrix:

- exact specification/ADR before architecture or protocol changes;
- pass and targeted mutation/fail cases;
- `cantTell`, inapplicable, and `untested` cases when meaningful;
- malformed input and stable diagnostics;
- boundary, allocation, time, and resource limits;
- deterministic repeated output and irrelevant-order invariance;
- metamorphic and differential tests;
- public binary/process E2E, not only unit APIs;
- conformance, acquisition, and semantic product evaluation kept separate;
- hard negatives and valid alternative designs;
- documented privacy, provenance, licensing, compatibility, and non-claims;
- exact final-head and post-merge `main` CI on stable, MSRV, Linux, macOS, and Windows where
  supported.

## Repository and decision hygiene

- New architecture decision numbers continue at 0041 or later.
- Historical branch-only ADRs 0025–0029 are design references, not accepted decisions.
- Closed PRs #12–#17 are superseded and must not be reopened as implementation shortcuts.
- Start every task from current green `main`.
- Keep one focused branch and PR per coherent issue slice.
- Store future work in issues and this roadmap rather than a chain of Draft implementation
  branches.
- Never use self-writing feature workflows.
- Keep the `Protect main` ruleset active and verify automatic deletion after each merge.
- Update `docs/handoff.md` and this roadmap whenever current facts or priorities change.

## Issue map

| Issue | Role |
|---|---|
| #19 | completed branch protection and required checks |
| #22 | realistic human-reviewed UI evaluation gate |
| #23 | Playwright native/pixel web adapter |
| #24 | zero-setup recommended rule packs |
| #25 | completed background/segmentation benchmark research; strict default retained |
| #26 | completed exact source-alpha transparent-asset geometry; no rule admitted |
| #27 | optional broader PNG format coverage and decoder strategy |
| #28 | OCR/CV/VLM perception-worker protocol |
| #29 | PPTX, PDF/document, Android, and iOS adapter roadmap |
| #30 | interaction states/effects/traces/recovery |
| #31 | Codex, MCP, GitHub Checks, editor/local UI ecosystem |
| #32 | completed legacy branch and repository-setting cleanup |
| #33 | completed license, compatibility, source packaging, and alpha release gate |
| #34 | completed first evidence-backed zero-setup web UI alpha execution epic |
| #47 | completed read-only release-artifact transport and immutable-tag recovery |

Issues define work and evidence requirements. They do not make proposed capabilities current until
an accepted ADR, tested implementation, and successful merge establish them.
