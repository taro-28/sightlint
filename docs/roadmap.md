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
| M4 — Structured adapters | active | process-isolated file and managed-loopback Playwright capture, 23-case Atlas matrix, second four-case Harbor family, first advisory recommended Web pack, bounded three-case PPTX/PDF/Android/iOS processes, and strict external-holdout attestation contracts | independent review, an externally operated protected holdout, portability characterization, and broader representative evaluation |
| M5 — Optional perception | active | local protocol `0.1.0`, bounded wrapper/reference worker, typed family schema, three-state differential regression | real OCR/model acquisition, calibration, representative evaluation, downstream rule evidence |
| M6 — Interaction contracts | active | interaction extension `0.1.0`, controlled Playwright/Atlas traces, async-feedback and failure-recovery advisory rules, eight-case evaluation | offline/permission/stale/partial/destructive/focus/mobile traces and broader evidence |
| M7 — Ecosystem and release | complete for exit criteria | local CLI, bounded one-command Web agent workflow, managed target-repository server lifecycle, deterministic GitHub job-check projection, dual license, source-only alpha release | demand-led MCP/editor surfaces and binary/package channels |

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
accuracy. ADR 0051 and issue #72 later add a second fictional support-inbox family and a strict
holdout-admission contract. Both families still have maintainer-only review, visible development
labels, no operational protected holdout, no pixel-content identity, no complete hit regions, no
semantic peer inference, and no representative sampling. The three rules therefore remain
advisory. The scripted edit is not a claim about autonomous agent quality or arbitrary
repositories. Issue #25 provides the bounded comparison without admitting a broader segmentation
default. ADR 0040 and issue #26 add exact source-alpha geometry without admitting a padding rule.
ADR 0041 and issue #27 retain explicit unavailability for broader PNG formats because current
product evidence does not establish a coverage gap. ADR 0042 and issue #28 add the local
perception protocol foundation without semantic promotion or model-accuracy claims. ADRs
0043–0046 add bounded PPTX, PDF, instrumented Android, and UIKit/XCUITest iOS slices under #29.
They establish public regression paths, not representative medium accuracy.

### Current evidence expansion — #71 / #72 / #75 / #78 / #77 / #74

Issue #71 is the post-alpha evidence-first roadmap epic. Its first child, #72, adds ADR 0051 and an
additive multi-family Web registry without changing the historical `0.1.0` declared-IR corpus.
The new Harbor support-inbox family contributes four public cases through the existing isolated
Playwright and Rust command path: clean, one accessible-name mutation, one visually identical
`aria-labelledby` hard negative, and one ambiguous focusable surface. Acquisition and rule truth
remain separate, and results are grouped by family and split.

The public admission schema rejects an operational holdout claim without freeze, digest, separate
access authority, independent evaluator, leakage log, pinned execution, oracle-correction, and
reporting records. Current status is deliberately `notOperational`; independent human review and
an externally controlled bundle remain required follow-up work before holdout or maturity claims.

Issue #75 and ADR 0052 now supply the technical handoff boundary for that follow-up: strict
external bundle, separate oracle, invocation/environment, private-result, and sanitized public-run
schemas; an explicit `notRun` status; and a read-only checker with a fictional public conformance
chain. The chain covers all six case classes, digest/path/budget failures, private small-cell
suppression, and byte-stable diagnostics, but is tuning-visible and permanently ineligible as
holdout evidence. Issue #74 remains the next gate because only a real external authority,
independent evaluator, second verifier, protected freeze/exposure log, and actual execution can
make admission operational or create evidence.

Issue #78 and ADR 0053 add the intervening review-operation foundation for issue #77. A generated
packet embeds only the public Atlas/Harbor fixture source and capture requests, a strict submission
keeps acquisition and rule judgments separate, canonical finalization locks reviewer-authored
bytes, and a separate read-only process compares only after that lock. Fictional conformance data
exercises agreement, disagreement, unresolved, abstention, hard-negative, and all five outcome
states but is explicitly ineligible evidence. The tools cannot supply human judgment or verify
reviewer identity, qualification, independence, conflicts, or signatures. Issue #77 remains the
next human gate; issue #74 remains separately gated on protected data and external authorities.

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
- #27 — PNG format-demand and decoder strategy decision (complete; broader coverage and a decoder
  dependency were not admitted).

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

### Managed loopback Web capture — #62, focused child of #31

ADR 0048 adds capture protocol `0.2.0` for a caller-authorized development server in the target
repository. `sightlint-web` and `sightlint-web-check` require `--allow-server-command`, start a
shell-free argv on an unused loopback port, wait for final HTTP 2xx plus page readiness, constrain
browser traffic to that same origin, and stop the full process tree on every terminal path. Managed
captures emit adapter `0.4.0`, `org.sightlint.web@0.4.0`, and workflow report `0.2.0`; the `0.1.0`
repository-file path and output remain unchanged.

The public three-case Atlas slice plus lifecycle matrix prove startup, redirect, same-origin API,
capture/check, redaction, unavailable source attribution, deterministic bytes, bounded failures,
and cleanup. The tabisaifu `/test/login?next=%2Fentries%2Fnew` dogfood is a real-application smoke
run, not committed evaluation data. It does not establish whole-app quality, source causality,
representative accuracy, WCAG conformance, or blocking maturity.

### Other adapters — #29

Add one at a time according to demand and fixture quality:

1. PPTX/slides — first bounded source-geometry slice implemented through ADR 0043;
2. structured PDF/document — bounded page/Link-annotation slice implemented through ADR 0044;
3. Android semantics/accessibility plus screenshot — first bounded instrumented-capture slice
   implemented through ADR 0045;
4. iOS XCUI/accessibility plus screenshot — first bounded paired UIKit/XCUITest capture slice
   implemented through ADR 0046.

This order is provisional. Every adapter needs an ADR, native fixture corpus, unit/coordinate plan,
trust/privacy/resource model, compatibility strategy, rendered differential tests, and public E2E.

The PPTX `0.1.0` process maps direct unrotated slide shapes/groups, native IDs, hierarchy, z-order,
text digests, and exact EMU layout boxes. It retains a separately evidenced PNG canvas and extent
agreement while rendered node identity stays `cantTell`. Its clean/mutation/hard-negative corpus
proves shared canvas containment without kernel medium-specific logic. Master/layout/theme
resolution, other DrawingML objects, representative files, a protected holdout, and PPTX-specific
recommended rules remain later slices; the implementation therefore reports partial coverage.

The PDF `0.1.0` process uses an exact hash-locked pypdf wheel outside the Rust kernel. It maps only
explicit integral unrotated page boxes and rectangular internal Link activation regions to
`pdfPoint` canvases and exact-source `hitBox` nodes. Optional PNG pages remain separate
device-pixel canvases with extent-only reconciliation. Its clean/source-only-mutation/QuadPoints
hard-negative corpus proves the shared containment rule and conservative abstention. Text, tags,
paint, actions, viewer hit testing, node-to-pixel identity, broader documents, a protected holdout,
and PDF-specific recommended rules remain unimplemented or untested.

The Android `0.1.0` process consumes a strict digest-pinned instrumentation capture and paired PNG
without operating a device. It maps only supported shown/globally-visible classic View allocations
to exact-source device-pixel `layoutBox` nodes. Accessibility rectangles remain separate platform
semantics, and the PNG remains a separate exact-render canvas with extent-only reconciliation. Its
clean/off-canvas-mutation/offscreen-hard-negative corpus proves the shared containment rule,
conflict retention, and conservative exclusion. Live capture, Compose, arbitrary applications,
touch regions, dynamic behavior, node-to-pixel identity, representative devices, a protected
holdout, and Android-specific recommended rules remain unimplemented or untested.

The iOS `0.1.0` process consumes a strict digest-pinned paired UIKit/XCUITest capture and PNG
without operating Xcode or a simulator. It maps only supported attached/visible identity-transform
UIKit View allocations with nonempty window intersections to exact-source point-valued
`layoutBox` nodes. XCUITest frames remain separate platform semantics, and the PNG remains a
separate exact-render device-pixel canvas with extent-and-scale-only reconciliation. Its
clean/off-canvas-mutation/offscreen-scroll-hard-negative corpus proves the shared containment
rule, source/XCUI conflict retention, and conservative exclusion. Live capture, SwiftUI, arbitrary
applications, activation regions, dynamic behavior, node-to-pixel identity, representative
devices, a protected holdout, and iOS-specific recommended rules remain unimplemented or untested.

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

### Protocol foundation — #28 (complete)

ADR 0042 defines strict request, response, run-report, and perception-extension `0.1.0` surfaces.
A dependency-free Node wrapper runs a caller-selected local worker without a shell, bounds time,
standard streams, input, observations, text, hierarchy, and geometry, validates worker/model/input
identity, canonicalizes results, and asks the public Rust normalizer to validate mapped IR. Typed
family records cover regions, text, roles, hierarchy, and peer groups, but only model-free
`visionMeasured` regions may become core `other` nodes; inferred semantics remain outside core IR
and no worker can create a trusted verdict.

The reference worker exposes one request-selected image-segmentation benchmark policy. Three
public Atlas states preserve synchronized native and pixel outputs, a reviewed 16±1 CSS-pixel
layout/render conflict, an independently measured mutation, and an intentional-grouping hard
negative. Qualified and strict policies abstain on Atlas's two edge surfaces; the evaluation names
ranked only to exercise mapping while retaining the unconfirmed hypothesis. OCR, role, hierarchy,
peer, calibration, backend sensitivity, latency distributions, downstream rule accuracy, and
protected-holdout evidence remain `untested`.

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

- one reference local worker and conformance suite (complete for protocol v0);
- differential evaluation against native/annotated data from #22/#23 (complete for the measured
  region boundary);
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

### First bounded slice

ADR 0047 implements the medium-neutral `org.sightlint.interaction@0.1.0` contract and a local
Playwright process over the repository-owned Atlas settings fixture. The adapter assigns canonical
controlled-step order without raw timestamps, denies external network, captures native state,
accessibility, screenshot digest/extent, and declared effect events separately, and preserves
disagreement as conflict evidence.

Two advisory base-profile rules cover observable-latency feedback and declared failure recovery.
Eight public cases include clean slow success, failure/retry, missing-pending and missing-recovery
mutations, a save-draft alternative hard negative, `cantTell`, inapplicable, and `untested` paths.
Acquisition and rule truth remain separately authored with no protected holdout. This slice does
not complete offline, permission, stale-data, partial-success, destructive safeguard, focus,
navigation, or mobile interaction coverage.

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

Issue #62 completes managed loopback protocol `0.2.0`, letting the same commands own one target-
repository development-server lifecycle behind explicit command authorization. Issue #65 and ADR
0049 make its acquisition and rule oracles independently reviewable and derive precision,
coverage, abstention, false-positive, and mutation-kill counts from those documents. Neither slice
adds MCP, hosted processing, source edit, existing-server attachment, remote URL, or package
channel.

Issue #67 and ADR 0050 complete the remaining M7 integration exit criterion with a deterministic
`github-check` projection into the existing GitHub Actions job check. Exact file/line annotations
require separately reviewed anchored source declarations; missing locations stay summary-only.
The command preserves kernel outcome/enforcement/evidence/policy, bounds output at 50 annotations,
escapes runner commands, and performs no network request, upload, source edit, or REST check-run
creation. Its eight public Atlas cases separate rule, source, and projection truth and report
integer precision/coverage/abstention/false-positive/mutation evidence without a holdout or
aggregate score.

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

Do not add MCP, GUI, or installers to compensate for weak product evidence. Those optional surfaces
remain demand-led after the bounded local-agent and GitHub job-check paths met M7's exit criteria.

### M7 exit criteria

- one stable local install path on supported desktop systems;
- agent-friendly canonical command/protocol backed by the same kernel;
- source/evidence navigation and post-fix rerun demonstrated;
- GitHub Actions job-check integration backed by the same kernel;
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

- New architecture decision numbers continue at 0054 or later.
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
| #27 | completed PNG format-demand and decoder strategy decision; broader coverage not admitted |
| #28 | completed local OCR/CV/VLM perception protocol foundation; model quality remains untested |
| #29 | PPTX, PDF, Android, and iOS first slices implemented |
| #56 | bounded Android instrumented-capture adapter slice |
| #60 | bounded iOS UIKit/XCUITest capture adapter slice |
| #30 | completed bounded interaction extension/trace/async-feedback/recovery slice; broader M6 scenarios remain evidence-gated |
| #31 | completed M7 exit criteria through local agent and GitHub job-check surfaces; optional MCP/editor/local UI remains demand-led |
| #62 | managed loopback target-repository Web capture and `/entries/new` dogfood |
| #65 | separate managed-loopback acquisition and rule evaluation oracles |
| #67 | deterministic GitHub Actions job-check projection and exact-source annotations |
| #71 | post-alpha evidence-first product maturity epic |
| #72 | second realistic Web fixture family and protected-holdout admission contract |
| #75 | external holdout manifests, sanitized run attestation, and conformance verifier foundation |
| #78 | deterministic source-only public Web review packet, finalized submission, and read-only comparison foundation |
| #77 | real independent human review of the public Atlas and Harbor annotations; remains a human gate |
| #74 | independent human review and externally operated protected holdout; remains externally gated |
| #32 | completed legacy branch and repository-setting cleanup |
| #33 | completed license, compatibility, source packaging, and alpha release gate |
| #34 | completed first evidence-backed zero-setup web UI alpha execution epic |
| #47 | completed read-only release-artifact transport and immutable-tag recovery |

Issues define work and evidence requirements. They do not make proposed capabilities current until
an accepted ADR, tested implementation, and successful merge establish them.
