# Local Codex handoff

This document is the operational handoff for continuing SightLint in the local Codex app or
another local coding-agent environment. It is intentionally explicit: a new session must be able
to determine the trusted source of truth, current capability, next task, decision background,
quality gates, and historical traps without access to the chat that created the repository.

Last handoff preparation: 2026-09-06.

## Start here

The authoritative development line is the latest green commit on `main`.

The branch for issue #75, the technical foundation required before issue #74 can operate a
protected holdout, started from
this verified green `main` baseline:

- commit: `6a7dfea4845f3bac49d56c1d71090218b0a13d14`
- tree: `82e76bc1687555508df11a830237d14d8e9c6ba6`
- merged PR: #73
- main CI: run 34043097106, all six jobs successful
- main CodeQL: run 34043096788, all four language jobs successful

Never hard-code the recorded baseline as a branch base; verify the latest `main`, its exact CI,
and the release page.

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
| #13 | full PNG sample/palette/`tRNS` normalization | reconsidered and not admitted by ADR 0041 without product need |
| #14 | exact alpha-visible geometry | superseded; current implementation is ADR 0040 / issue #26 |
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
plus the explicitly selected `actions/checkout`, `actions/upload-artifact`,
`actions/download-artifact`, and `github/codeql-action` families. Secret
Protection and Push Protection remain enabled. GitHub Apps access is limited to the Codex
Connector for this repository. No self-hosted runner, webhook, deploy key, Pages site, or
Environment is configured. Discussions remains disabled and no committed issue-template link
points to it.

The repository description identifies SightLint as deterministic, evidence-backed visual linting;
topics cover Rust, CLI, Playwright, accessibility, UI testing, and developer tooling. No separate
project homepage is currently appropriate. Dependabot PR #38 was closed because Node 26 type
definitions exceed the accepted Node 20–24 alpha compatibility range. The npm update configuration
therefore ignores semver-major `@types/node` version updates while retaining compatible minor and
patch updates in the existing dependency group.

### Release status

- repository: public;
- project status: narrow source-only alpha;
- workspace version: `0.1.0-alpha.2`;
- first published release: GitHub prerelease tag `v0.1.0-alpha.2`;
- artifact: deterministic tracked-source archive plus canonical SHA-256 record;
- license: `MIT OR Apache-2.0` for repository-owned source, documentation, schemas, and fixtures;
- Cargo crate publication: disabled; Node package: private;
- no prebuilt binary, registry package, installer, container, signature, or attestation.

The immutable `v0.1.0-alpha.1` tag is an unpublished failed release candidate, not a supported
release. Run 34000128047 created its draft assets but the read-only matrix could not access a draft
release. ADR 0038 keeps verification jobs read-only by using a short-retention workflow artifact
and requires the final write-enabled job to compare it with the draft assets byte-for-byte. Release
run 34000898691 published alpha.2 after every job passed. The public source archive is 367,534 bytes
with SHA-256 `67290954e7ed0e2e88bac59efe7e0e765c139e2e31580220afba4348d4ba5355`;
the asset digest, checksum sidecar, public download, and two local rebuilds agreed. The alpha.1
draft was then removed while its immutable tag and failure history were retained.

Accepted ADR 0007 defines the license boundary. ADR 0037 and `docs/release.md` define the first
release, supported environments, verification procedure, and explicit non-claims. Checksums detect
corruption; they do not authenticate the publisher.

## What is actually implemented

### Deterministic kernel and Artifact IR

The Rust workspace contains:

- `sightlint-ir`: versioned medium-neutral data contracts, validation, evidence, units, and
  canonicalization;
- `sightlint-engine`: deterministic geometry queries, atomic rules, result/report construction;
- `sightlint-adapter-png`: bounded PNG parsing, raster acquisition, advisory image inspection, and
  exact source-alpha geometry plus an evaluation-only segmentation comparison;
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
  -> exact encoded source-alpha geometry for supported rasters
```

The supported raster subset is eight-bit grayscale, RGB, grayscale-alpha, and RGBA without
`tRNS`. Palette/indexed, sub-byte, 16-bit, `tRNS`, animation, and over-budget cases are explicitly
unavailable rather than guessed. Raw pixels remain inside the adapter API. Serialized IR stores
bounded metadata, evidence, and a regression checksum rather than the raw raster.

The samples are PNG-encoded channel values, not color-managed display values. No gamma/ICC/
chromaticity transformation or alpha compositing is applied. They are insufficient by themselves
for a trusted contrast/colorimetric verdict.

ADR 0040 versions the PNG extension as `0.2.0` and adds `alphaGeometry@0.1.0`. One bounded pass
records half-open visible (`alpha > 0`) and opaque (`alpha == 255`) bounds, exact alpha-class
counts, transparent insets, and visible edge occupancy. A dedicated exact-source evidence item
links nonempty visible bounds to the image node's device-pixel `inkBox`; entirely transparent
images omit the box, and unsupported rasters repeat their explicit unavailable reason without
alpha evidence. No compositing, semantic whitespace judgment, alpha rule, or blocking result is
implemented.

ADR 0041 resolves the optional broader-format decision without expanding the decoder. A versioned
assessment inventories five source-alpha assets plus three PPTX, three PDF, three Android, and
three iOS renders and verifies the nine pinned-browser product captures as current-subset inputs;
indexed, packed, 16-bit, `tRNS`,
and animation cases remain
synthetic unavailable controls rather than product-demand evidence. No decoder dependency,
automatic conversion, telemetry, protected holdout, prevalence claim, command/schema change, or
broader accuracy claim is introduced. A caller-selected conversion proves facts about the
converted bytes only. A future observed gap requires a new issue and ADR.

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

ADR 0039 keeps that strict default unchanged and adds `benchmark-image-segmentation` solely for
research comparison. Its `0.1.0` canonical report compares strict flood fill, ranked exact-border
flood fill, and 95%-qualified corner row-run/union-find. Candidate backgrounds are unconfirmed,
semantic applicability is `cantTell`, rule outcome is `untested`, and the command never blocks.
The nine-case Northstar corpus has separate source-authored acquisition and rule oracles, public
smoke/development/challenge labels, hard negatives, targeted mutation, metamorphic relations, and
bounded resource refusal. Screenshots and implementation reports remain temporary.

The qualified policy recovers one edge-contaminated acquisition case and correctly abstains on two
required hard negatives; ranked selection observes both hard negatives unsafely. Realistic shadows
merge multiple dashboard surfaces under all three exact-color policies. The corpus therefore does
not admit a new default, downstream rule, blocking result, holdout claim, or real-world accuracy
claim.

### Isolated perception process protocol

ADR 0042 and `adapters/perception/` add strict request, response, run-report, and
`org.sightlint.perception` extension `0.1.0` surfaces. The dependency-free Node wrapper runs one
caller-selected local worker without a shell, validates input/worker/model/backend identity,
canonicalizes results, bounds time, standard streams, input, observations, text, hierarchy, and
geometry, and sends mapped IR through the public Rust `normalize` command. Remote execution,
external processing, transmitted fields, telemetry, and retention are rejected in v0.

Typed observation families represent regions, text, roles, hierarchy, and peer groups with source
links, confidence availability, alternatives, uncertainty, and run-agreement metadata. Only
model-free `visionMeasured` region observations map into core `other` nodes. Inferred regions and
all semantic families remain in the separately written canonical response and extension summary;
they create no core role, name, parent, relation, rule result, or blocking authority. Conformance
fixtures cover the five family shapes, calibrated and unavailable confidence, explicit acquisition
unavailability, byte stability, timeout, output overflow, malformed output, identity mismatch,
and inferred-family non-promotion.

The local reference worker exposes regions from one request-selected
`benchmark-image-segmentation` policy and never falls back. Three Atlas Web states run the actual
Playwright capture, Rust segmentation, perception wrapper, Rust normalization, and check paths.
They preserve a reviewed 16±1 CSS-pixel native layout/render conflict, observe one pixel
acquisition mutation, and produce zero semantic claims and zero hard-negative failures. Because
Atlas has dark and light top-level edge surfaces, qualified and strict policies abstain; the corpus
selects ranked only to exercise mapping and keeps its background hypothesis unconfirmed.

This is a protocol/process/evidence-boundary result, not OCR or model evaluation. The public cases
are one fictional maintainer-authored application family with no independent review or protected
holdout. OCR/text/role/hierarchy/peer precision/recall, calibration, backend sensitivity, latency
distributions, downstream rule accuracy, and real-world UI/UX accuracy remain `untested`. Process
isolation is not an OS sandbox or generic memory ceiling; third-party workers remain able to use
the caller's local privileges unless separately sandboxed.

### Bounded PPTX source adapter

ADR 0043 and `adapters/pptx/` add the first non-Web structured process adapter. Strict `0.1.0`
request/response and `org.sightlint.pptx@0.1.0` extension surfaces bind local source/render paths,
SHA-256 identity, digest-only text handling, resource limits, runtime provenance, and explicit
partial coverage. Python standard-library ZIP/XML parsing remains outside the Rust kernel.

Supported direct slide facts are slide size/order, shapes/groups, native IDs, parentage, local
z-order, placeholders, text digest/count, and unrotated/group-transformed source `layoutBox`
geometry in EMUs. Optional PNG renders are validated through public `adapt-image`, retained as a
separate device-pixel canvas, and reconciled only at slide extent. Candidate IR is accepted through
public `normalize`; public `check --profile base` reuses exact-source canvas containment and kills
the off-slide mutation without a PPTX-specific kernel branch.

The process always reports partial source coverage. It does not map master/layout objects,
theme-resolved styles, full text, pictures/charts/tables/media, unsupported transforms, rendered
ink/text layout, or shape-to-pixel identity. The three public synthetic cases have maintainer-only
review and no protected holdout, so their perfect regression metrics are not real-world accuracy.

### Bounded PDF source adapter

ADR 0044 and `adapters/pdf/` add the second non-Web structured process adapter. Strict `0.1.0`
request/response and `org.sightlint.pdf@0.1.0` extension surfaces bind repository-contained source
and page-render paths, SHA-256 identity, geometry/type-only privacy, resource limits, parser/runtime
provenance, and explicitly partial coverage. The only package is the universal
`pypdf==6.17.0` wheel, locked by SHA-256 and license-reviewed as BSD-3-Clause; parsing remains
outside the Rust kernel and is not an OS sandbox.

The process iteratively walks the raw page tree with cycle detection so inherited page properties
are not silently promoted. It maps only explicit integral unrotated MediaBox/CropBox geometry and
indirect rectangular internal Link annotations with zero flags and no `QuadPoints`/`Path` to exact
source `pdfPoint` canvases and `hitBox` nodes. It records tag-tree presence without interpreting
it, does not follow destinations/actions, and serializes no document text, URI, metadata, content
stream, or pixels. Optional PNG pages pass public `adapt-image` and remain separate device-pixel
canvases with extent-only agreement/conflict and node identity `cantTell`.

The three public repository-owned cases recover eight exact link rectangles, kill one source-only
off-page mutation whose rendered bytes are unchanged, and retain one `QuadPoints` link as an
abstention with no unexpected failure. Acquisition and rule truth are separate, implementation
output is not an oracle, provenance/license/privacy/public split/no-holdout status are explicit,
and perfect regression metrics do not establish representative PDF, accessibility, interaction,
or document-quality accuracy. Text, paint, tags, viewer hit testing, broader annotation/actions,
and PDF-specific rules remain unimplemented or untested.

### Bounded Android capture adapter

ADR 0045 and `adapters/android/` add the third non-Web structured process slice. Strict `0.1.0`
request/response, capture, and `org.sightlint.android@0.1.0` extension surfaces bind a local
instrumentation manifest and paired PNG by repository-contained paths and SHA-256 identity. The
dependency-free Python 3.9+ adapter does not operate `adb`, boot or mutate a device, install an
APK, perform accessibility actions, or use the network.

The repository-owned Atlas API-35 fixture application records classic View hierarchy/allocation
and `AccessibilityNodeInfo` facts separately before a sequential `UiAutomation` screenshot. Only
shown, globally visible, identity-transform Views with unique resource IDs and nonempty allocation
become exact-source device-pixel `layoutBox` nodes. Accessibility rectangles remain
`platformSemantics`, the PNG stays on a separate exact-render canvas, and reconciliation is limited
to display extent. Clipped accessibility geometry does not repair source allocation, while
offscreen and invalid platform bounds remain extension evidence without core geometry.

The three public cases match 114 reviewed acquisition facts, kill one Save-allocation mutation,
retain one offscreen-scroll hard-negative abstention, and emit no clean/hard-negative failure.
Acquisition and rule truth remain separate, implementation output is not an oracle, and
provenance/license/privacy/public split/no-holdout status are explicit. This proves regression
behavior only. Live capture, Compose, arbitrary applications/devices, touch regions, dynamic
behavior, occlusion/ink, rendered node identity, Android-specific rules, and representative
mobile/UI/UX accuracy remain unimplemented, `untested`, or `cantTell`.

### Bounded iOS capture adapter

ADR 0046 and `adapters/ios/` add the fourth non-Web structured process slice. Strict `0.1.0`
request/response, capture, and `org.sightlint.ios@0.1.0` extension surfaces bind a local paired
UIKit/XCUITest manifest and PNG by repository-contained paths and SHA-256 identity. The
dependency-free Python 3.9+ adapter does not operate Xcode or `simctl`, boot a simulator,
install/launch an app, execute an XCUI action, parse an `.xcresult`, or use the network.

The repository-owned Atlas fixture application records UIKit hierarchy/allocation facts, a
pre-query screenshot, and independently queried XCUITest platform semantics on Xcode 26.3 with
an iOS 26.3.1 iPhone 17 Pro simulator. Only attached, visible, identity-transform UIKit Views with
unique identifiers, nonempty allocations, and nonempty window intersections become exact-source
point-valued `layoutBox` nodes. A direct clipped scroll-content container and fully offscreen
Views remain extension-only. XCUITest frames remain `platformSemantics`; source/XCUI disagreement
remains conflict evidence; the PNG stays on a separate exact-render device-pixel canvas with
extent-and-scale-only reconciliation.

The three public cases match 122 reviewed acquisition facts, kill one Save-allocation mutation,
retain four hard-negative exclusions, and emit no clean/hard-negative failure. Acquisition and
rule truth remain separate, implementation output is not an oracle, and capture-order/provenance/
license/privacy/public-split/no-holdout status are explicit. This proves regression behavior only.
Live capture, SwiftUI, arbitrary applications/devices, activation regions, dynamic behavior,
occlusion/ink, rendered node identity, iOS-specific rules, and representative mobile/UI/UX
accuracy remain unimplemented, `untested`, or `cantTell`.

### Realistic Web evaluation foundation

ADR 0032 and `evaluation/web/` provide the first issue #22 foundation. The committed
repository-owned dashboard has six reviewed declared-IR state/environment records:

- three required public-binary smoke cases for explicit peer spacing;
- one clean baseline and one targeted 16 CSS-pixel mutation;
- one intentional-grouping hard negative that excludes an adjacent promotion from the metric peer
  relation;
- development records for ambiguous peer intent, a narrow viewport, and 125% text scale;
- separate acquisition and rule annotation documents;
- explicit source ownership, dual-license, privacy, split, and holdout declarations.

The original runnable inputs use independently authored declared Artifact IR projections and do
not establish acquisition accuracy. Their browser fields remain `untested` rather than being
rewritten from implementation output. Metrics are small-corpus regression counts, not general
UI/UX accuracy.

### Multi-family Web evaluation and holdout admission

ADR 0051 and issue #72 add `evaluation/web/evaluation-v1.json` as an additive registry over the
existing Atlas browser oracle and a second repository-owned Harbor support-inbox family. It records
family context, public exposure/tuning status, provenance, dual license, privacy, reviewer roles,
agreement/adjudication state, oracle joins, split inventory, metric dimensions, and non-claims.
The historical Web corpus and browser schemas remain unchanged compatibility surfaces.

Harbor has four public cases: a clean named send control, an `aria-label` removal mutation, a
visually identical `aria-labelledby` hard negative, and a focusable generic send surface whose
role/name remain `cantTell`. Separate acquisition and rule documents are authored independently
from output. The existing local Playwright process and built Rust binary repeat capture response,
Artifact IR, screenshot, CheckReport, diagnostics, and exit bytes. Current results are 4/4 selected
acquisition expectations, 9/9 reviewed rule results, 6/6 reviewed abstention results, 1/1 failure
precision, 1/1 mutation kill, and zero false-positive or hard-negative failures.

`holdout-admission.json` is public admission metadata only. It rejects an operational claim
without an exact freeze/digest, separately administered access, independent evaluator, exposure
log, tuning exclusion, pinned execution, correction procedure, and reporting plan. Its current
status is `notOperational`; both families remain public maintainer-authored tuning data. No
independent-review agreement, representative accuracy, holdout performance, WCAG conformance, or
blocking maturity is claimed.

ADR 0052 and issue #75 add strict `1.0.0` contracts for a protected external bundle, separately
authored acquisition/rule oracle, pinned invocation/environment, private result, and sanitized
public attestation. `holdout-run.json` truthfully remains `currentStatus`/`notRun`, bound to the
current non-operational admission record. The committed six-case `conformance/holdout/` chain is
fictional, tuning-visible, dual-purpose protocol test data: it covers clean, targeted mutation,
hard negative, ambiguity, malformed input, and resource boundary, but is always
`evidenceEligible: false`.

`tools/check_web_holdout_foundation.py` is a read-only, standard-library checker. It enforces
canonical SHA-256 projections, contained relative paths, byte limits, manifest joins, pinned
public argv, acquisition/rule oracle separation, integer metric arithmetic, explicit 0/0,
small-cell suppression at denominator 5, distinct evaluator/verifier declarations, leakage
redaction, and byte-stable exit-2 diagnostics. It does not run SightLint, contact a protected
store, validate a person's identity or qualification, verify detached signatures, or establish
real holdout performance. Those operational facts remain issue #74.

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

Current Atlas development-corpus metrics are 23/23 cases, 76 reviewed acquisition expectations, 45
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

### Managed loopback Web capture

Issue #62, a focused child of #31, is implemented by ADR 0048. Capture protocol `0.2.0` accepts
`managedLoopbackHttp`, a path/query, state, readiness selector, a bounded direct argv with one
`{port}` placeholder, port/startup bounds, and `sameOriginLoopback`. Both `sightlint-web` and
`sightlint-web-check` require bare `--allow-server-command` before spawning. The canonical target
repository is the working directory and the caller environment is inherited; no shell is used.

The adapter verifies that the port is initially free, waits for TCP listen, final HTTP 2xx,
`load`, the readiness selector, and fonts, and then stops the server on success, capture/kernel
failure, SIGINT, or SIGTERM. POSIX cleanup uses the process group with TERM, a five-second grace,
then KILL; Windows validates the PID and calls `taskkill.exe /T /F`. Combined stdout/stderr is
drained to 1 MiB and never serialized. Early exit, timeout, port conflict, and log overflow have
stable operational diagnostics.

Browser-side traffic is restricted to `http://127.0.0.1:<port>` on the same origin. External
HTTP(S) fails capture; WebSocket and service-worker attempts are blocked and counted. Request
bodies, individual responses, aggregate bytes, and response count are bounded. Source identity is
derived from method, query-hiding target and request-body digests, status, byte count, and buffered
response bytes. Raw queries, bodies, variable headers, environment values, server output, and PIDs
are absent from reports. The command itself is not sandboxed and its outbound traffic remains
uncontrolled.

Managed captures emit adapter `0.4.0`, `org.sightlint.web@0.4.0`, and workflow report `0.2.0`.
Runtime locators do not prove source locations, so managed source targets declare
`sourceAttribution: "unavailable"` and `sourceFiles: []`. Rust explicitly dispatches Web extension
`0.3.0` and `0.4.0`; Artifact IR `0.1.0`, CheckReport `0.3.0`, the three advisory rule versions,
maturity, enforcement, and the legacy capture/workflow bytes remain unchanged.

ADR 0049 and issue #65 separate the committed three-case managed Atlas acquisition oracle from its
rule oracle. The clean, unnamed-control mutation, and intentional-overlay hard-negative cases now
report 54/54 exact acquisition expectations, 9/9 acquisition abstentions, 1/1 failure precision,
4/4 reviewed rule abstentions, zero unexpected failures, 1/1 mutation kill, and zero hard-negative
failures. Those denominators come from reviewed data rather than duplicated E2E constants. The
public E2E also covers redirect, same-origin POST, authorization, lifecycle/network/resource
failures, redaction, repeated bytes, direct capture, kernel dispatch, and process-tree cleanup.
This is public single-family regression evidence, not general UI/UX accuracy, WCAG conformance,
whole-application coverage, source causality, or blocking maturity. The tabisaifu
`/test/login?next=%2Fentries%2Fnew` run is dogfood only and does not become committed oracle data or
modify that target repository.

The 2026-09-06 dogfood used tabisaifu commit
`94269ee4452ea4b96ea27c7499eea8375303c8f5` with explicit Wrangler test vars and no `.dev.vars`
change. It reached the final `<main>`, captured 47 nodes including 「支払いを追加」 and
「支払いを記録」 at a 412×839 CSS-pixel viewport, DPR 2, `ja-JP`, UTC, light theme, and reduced
motion, emitted valid JSON and human `0.2.0` reports, and released port 4173. The JSON report
exited 1 with 12 existing blocking `visual.bounds.within-canvas` results across six nested
details/form targets; this is a dogfood finding, not a managed-lifecycle failure. One guarded
capture compared the complete tracked diff immediately before and after and found identical bytes.
Another active task changed unrelated tabisaifu files during the broader session, so only that
single-invocation paired comparison is claimed. No tabisaifu request, capture, screenshot, log, or
source change is committed here.

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
sightlint github-check INPUT [--source-map FILE] [--repository-root PATH] [--format json|github-actions] [--write-step-summary] [--deny-cant-tell] [--profile recommended|base]
```

For a checkout, use `cargo run --locked -p sightlint-cli -- ...`. The source alpha documents how to
verify and build the same CLI from its release archive; no registry or prebuilt binary is claimed.

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

The GitHub Actions surface is a separate deterministic projection of the same Rust report:

```bash
target/debug/sightlint github-check \
  evaluation/web/inputs/dashboard-peer-spacing-mutant.json \
  --source-map evaluation/github-actions/source-maps/dashboard-peer-spacing-mutant.json \
  --repository-root . \
  --format github-actions \
  --write-step-summary
```

The runner turns escaped `error`/`warning`/`notice` workflow commands into annotations on its
existing job check. Exact paths and lines require a separately authored, anchored `0.1.0` source
map; missing mappings stay summary-only. The command never infers source from selectors or bundle
paths and performs no network request, token use, artifact upload, source edit, or independent REST
check creation. `--write-step-summary` is explicit and requires the runner-provided
`GITHUB_STEP_SUMMARY` path.

The perception wrapper is a separate local public process:

```bash
node adapters/perception/src/cli.mjs \
  --request REQUEST.json \
  --worker-program "$(command -v node)" \
  --worker-argument adapters/perception/src/reference-worker.mjs \
  --worker-source adapters/perception/src/reference-worker.mjs \
  --sightlint-binary target/debug/sightlint \
  --response-out RESPONSE.json \
  --artifact-ir-out ARTIFACT-IR.json
```

Success and explicit partial/unsupported/ambiguous acquisition exit 0 with a canonical nonblocking
run report whose rule outcome is `untested`; operational/protocol/resource/mapping failures exit 2.
The wrapper never exits 1.

The first non-Web structured adapter is a separate local Python process:

```bash
python3 adapters/pptx/sightlint_pptx.py \
  --request evaluation/pptx/requests/atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/atlas-clean.ir.json
```

It exits 0 with canonical response bytes and explicitly partial Artifact IR, or 2 for bounded
request/path/archive/XML/resource/Rust-validation failures. A subsequent public `sightlint check`
owns rule outcomes and exit 1. The process is an untrusted adapter, not a Rust-kernel dependency.

The bounded PDF adapter is also a separate local Python process and requires the exact locked
parser first:

```bash
python3 -m venv .venv-sightlint-pdf
.venv-sightlint-pdf/bin/python -m pip install --require-hashes -r adapters/pdf/requirements.txt
export PATH="$PWD/.venv-sightlint-pdf/bin:$PATH"
python3 adapters/pdf/sightlint_pdf.py \
  --request evaluation/pdf/requests/atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/atlas-clean-pdf.ir.json
target/debug/sightlint check /tmp/atlas-clean-pdf.ir.json --profile base --format json
```

It exits 0 with a canonical partial response and normalized IR, or 2 for dependency, request,
path, digest, parser, encryption, resource, Rust-validation, or output errors. The subsequent
trusted check owns exit 1. The adapter never follows PDF destinations/actions or performs external
processing.

The bounded Android adapter is a dependency-free local file process over a previously captured
manifest and PNG:

```bash
python3 adapters/android/sightlint_android.py \
  --request evaluation/android/requests/android-atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/atlas-android-clean.ir.json
target/debug/sightlint check /tmp/atlas-android-clean.ir.json --profile base --format json
```

It exits 0 with a canonical partial response and normalized IR, or 2 for request, capture, path,
digest, resource, extent, Rust-validation, or output errors. The subsequent trusted check owns
exit 1. Device acquisition is an explicit maintainer fixture operation, not part of this adapter
command.

The bounded iOS adapter is likewise a dependency-free local file process over a previously
captured paired UIKit/XCUITest manifest and PNG:

```bash
python3 adapters/ios/sightlint_ios.py \
  --request evaluation/ios/requests/ios-atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/atlas-ios-clean.ir.json
target/debug/sightlint check /tmp/atlas-ios-clean.ir.json --profile base --format json
```

It exits 0 with a canonical partial response and normalized IR, or 2 for request, capture, path,
digest, resource, compatibility, extent/scale, Rust-validation, or output errors. The subsequent
trusted check owns exit 1. Xcode/simulator acquisition is an explicit pinned maintainer fixture
operation, not part of this adapter command.

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

- 43-case PNG raster corpus with exact input bytes and independent expected pixels/alpha geometry,
  explicit unavailable cases, malformed inputs, filters, Adam7, alpha values, and a future
  semantic spacing pair;
- five-case repository-owned source-alpha evaluation with separate acquisition/rule annotations,
  one targeted mutation, hidden-RGB metamorphism, two hard negatives, public splits, and explicit
  provenance/license/privacy/abstention/non-holdout declarations;
- 30-case image-inspection corpus with independently declared region/gap oracles,
  19 observed cases, nine explicit unavailable cases, and two malformed inputs;
- nine-case realistic segmentation benchmark with separate acquisition/rule oracles, three named
  policies, hard negatives, targeted mutation, metamorphic variants, and checkerboard limits;
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
- one additive two-family registry plus strict review/exposure/holdout-admission contracts;
- four Harbor support-inbox captures with separate acquisition/rule oracles, clean/mutation/
  hard-negative/ambiguity coverage, 4/4 selected acquisition expectations, 9/9 rule results, 6/6
  reviewed abstentions, 1/1 failure precision and mutation kill, and byte-stable local reruns;
- semantic-only mutation and valid alternative-name states whose screenshot bytes equal clean;
- one public reviewed agent workflow oracle covering the combined command, native source-target
  join, temporary fix/rerun, no-new-failure postcondition, JSON/human byte stability, and
  ambiguity/intentional-overlay controls;
- no representative screenshot corpus, independent review, operational protected holdout, or
  general accuracy claim.

### GitHub Actions integration fixtures

- accepted ADR 0050 plus strict `github-source-map@0.1.0` and
  `github-actions-report@0.1.0` generated schemas;
- a separate Rust projector outside the kernel and a public `sightlint github-check` file/stdin
  path using the existing `check_with_options` entry point;
- independently authored rule, exact-source, and projection authorities across eight public Atlas
  dashboard/settings cases: two clean, two targeted mutations, two hard negatives, two abstention
  states, and no protected holdout;
- exact blocking/error and advisory/warning annotations, exact-source `cantTell`/notice
  conformance, summary-only abstentions, strict `cantTell` gating without outcome coercion, and
  passed/inapplicable suppression;
- repository containment, symlink resolution, stable anchors, declaration ordering/uniqueness,
  artifact/finding joins, provenance, UTF-8, range, 50-annotation, command-injection, input, and
  1 MiB summary bounds tested through the public binary;
- byte-stable file/stdin/rerun output and paired clean reruns after both reviewed mutations;
- zero unexpected or false-positive failures/annotations in the public corpus, reported only as
  integer regression evidence rather than real-world precision or a universal score;
- no network, GitHub App/token, REST publisher, artifact/screenshot upload, telemetry, source
  excerpt, source edit, or oracle generation from implementation output.

### Perception protocol fixtures

- strict JSON Schemas for request, response, run report, extension, corpus, and annotations;
- small hand-authored protocol inputs plus full typed-family conformance data, separate from
  product ground truth;
- four public-process E2E cases for repeated bytes, explicit unavailable acquisition, inferred
  family non-promotion, timeout, output overflow, malformed output, and identity mismatch;
- three realistic Atlas states with synchronized Playwright native IR and screenshots, Rust pixel
  benchmark input, separate acquisition/rule oracles, one mutation, one hard negative, and retained
  native/pixel conflict;
- public non-holdout development data, no committed captures/implementation output as oracle, no
  semantic/model-accuracy or blocking claim.

### PPTX source-adapter fixtures

- ADR 0043 plus strict request, response, and `org.sightlint.pptx@0.1.0` extension schemas;
- a Python 3.9+ standard-library process that bounds and validates local transitional OOXML ZIP/XML
  input, maps direct shapes/groups/native IDs/hierarchy/z-order and exact source EMU `layoutBox`
  geometry, and never executes Office or follows external relationships;
- three deterministic repository-owned PPTX packages and three reviewed LibreOffice-derived
  960×540 renders with source/render digests, fictional-data ownership, dual-license, privacy, and
  renderer provenance;
- separate acquisition and rule annotations for a clean baseline, one off-slide mutation, and an
  asymmetric hard negative, plus explicit public splits and no protected holdout;
- public-process E2E through `adapt-image`, `normalize`, and `check`, including repeated-byte,
  malformed/archive, resource, digest, output-collision, mutation-kill, false-positive, and
  abstention assertions;
- source geometry coverage remains `partial`; master/layout/theme resolution, full text, ink,
  rendering fidelity, and shape-to-pixel identity are not claimed.

### PDF source-adapter fixtures

- ADR 0044 plus strict request, response, `org.sightlint.pdf@0.1.0`, dependency-lock, corpus,
  acquisition-annotation, rule-annotation, and metric schemas;
- three deterministic fictional PDF 1.7 report pages and three reviewed Poppler 26.05.0
  612×792 RGB renders with exact source/render/request digests and dual-license/privacy provenance;
- separate acquisition and rule truth for a clean page, one source-only off-page Link mutation,
  and one asymmetric `QuadPoints` abstention hard negative across public smoke/development/
  challenge splits with no protected holdout;
- an exact hash-locked pypdf 6.17.0 wheel record, parser/geometry/page-tree unit tests, static drift
  and governance checks, and no network/action following;
- public-process E2E through `adapt-image`, `normalize`, and `check`, including repeated-byte,
  digest, object-budget, dependency, malformed, encrypted, output-collision, mutation-kill,
  false-positive, non-leakage, and abstention assertions;
- page/link coverage remains `partial`; text, tags, paint/ink, reading order, actions, forms,
  viewer behavior, and rendered annotation identity are not claimed.

### Android capture-adapter fixtures

- ADR 0045 plus strict request, response, capture, `org.sightlint.android@0.1.0`, corpus,
  acquisition-annotation, rule-annotation, and metric schemas;
- one realistic repository-owned classic-View account/settings fixture application and a
  dependency-free instrumentation capture runner pinned to API 35, Pixel_8, Gradle 8.13, Android
  Gradle Plugin 8.10.1, and Java 17;
- three committed native manifests and RGB screenshots with exact source/request/render digests,
  fictional-data ownership, dual-license/privacy provenance, and tool/device/build provenance;
- separate acquisition and rule truth for clean, targeted off-canvas mutation, and offscreen
  scroll hard-negative cases across public smoke/development/challenge splits with no protected
  holdout;
- public-process E2E through `adapt-image`, `normalize`, and `check`, including 114 acquisition
  facts, repeated bytes, digest/node/output/path/schema/extent/output-collision boundaries,
  mutation kill, false-positive, non-leakage, and abstention assertions;
- coverage remains `partial`; accessibility bounds do not become hit/render geometry, and live
  device capture, Compose, dynamic behavior, arbitrary apps/devices, or general UI/UX accuracy are
  not claimed.

### iOS capture-adapter fixtures

- ADR 0046 plus strict request, response, capture, `org.sightlint.ios@0.1.0`, corpus,
  acquisition-annotation, rule-annotation, and metric schemas;
- one realistic repository-owned UIKit account/settings fixture application and paired
  source/XCUITest capture target pinned to Xcode 26.3, iOS Simulator 26.3.1, iPhone 17 Pro,
  Swift 6.2.4, and a fixed light/locale/content-size/orientation profile;
- three committed native manifests and RGB screenshots with exact source/request/render digests,
  fictional-data ownership, dual-license/privacy provenance, explicit non-atomic capture order,
  and tool/device/build provenance;
- separate acquisition and rule truth for clean, targeted off-canvas mutation, and offscreen
  scroll hard-negative cases across public smoke/development/challenge splits with no protected
  holdout;
- public-process E2E through `adapt-image`, `normalize`, and `check`, including 122 acquisition
  facts, repeated bytes, digest/node/output/path/schema/extent/output-collision boundaries,
  mutation kill, false-positive, non-leakage, source/XCUI conflict, and abstention assertions;
- coverage remains `partial`; XCUITest frames do not become layout/hit/render geometry, and live
  capture, SwiftUI, dynamic behavior, arbitrary apps/devices, activation geometry, or general
  UI/UX accuracy are not claimed.

Synthetic success is regression evidence, not real-world accuracy evidence.

### Interaction contract and controlled-trace slice

ADR 0047 and issue #30 add:

- optional medium-neutral `org.sightlint.interaction@0.1.0` actions, declared effect latency,
  recovery alternatives, captured/`untested` traces, controlled environment, ordered events,
  attempt/causal IDs, and retained conflicts;
- the local `sightlint-interaction` Playwright process over a realistic repository-owned Atlas
  account-settings app, with fixed controlled steps, denied external network, and bounded output;
- sequential DOM, accessibility, and screenshot capture for each named step plus separate
  app-declared effect events; screenshot bytes are ephemeral and pixels never prove invisible
  effects;
- advisory `interaction.async-feedback@0.1.0` and
  `interaction.failure-recovery@0.1.0` base-profile rules;
- eight public cases with separate acquisition/rule truth, 35 reviewed acquisition facts, two
  killed mutations, a save-draft hard negative, `cantTell`, inapplicable, and `untested` paths;
- no protected holdout, independent review, representative product accuracy, blocking policy,
  real-network timing, or general application support.

### Required normal CI

The normal workflow is read-only and includes:

- generated fixture drift checks;
- release-tag/package metadata, locked dependency-license, and source-archive safety/determinism
  checks;
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
- real OCR, learned CV, or VLM worker implementations and their model-quality evaluation;
- automatic peer-role or design-intent inference;
- trusted spacing failures derived only from image grouping;
- color management, compositing, or trusted contrast from PNG samples;
- arbitrary-URL, iframe, shadow-DOM, full accessibility-tree, or arbitrary interaction capture;
- automatic semantic peer inference from Playwright output;
- broad PDF/document parsing beyond explicit page boxes and rectangular internal Link annotations,
  including text/tags/paint/actions/forms/viewer behavior;
- broad Android support beyond the repository-owned classic-View capture, including live-device
  acquisition, Compose, multiple windows, touch regions, dynamic behavior, and representative
  device/application evaluation;
- broad iOS support beyond the repository-owned paired UIKit/XCUITest capture, including live
  acquisition, SwiftUI, custom accessibility containers, multiple windows, activation geometry,
  dynamic behavior, focus navigation, and representative device/application evaluation;
- broad PPTX coverage beyond direct unrotated shapes/groups, or a PPTX-specific recommended rule;
- baseline/semantic visual diff beyond current explicit contracts;
- blocking recommended Web rules, project overrides, or representative real-world rule evidence;
- independent Web annotation review or an operational, externally controlled protected holdout;
- broad interaction support beyond the controlled Atlas slice, including offline, permission,
  stale-data, partial success, destructive safeguards, undo, focus/navigation, and mobile traces;
- MCP, an independent REST-published GitHub App check, editor extension, browser extension, or
  local GUI;
- broad automatic fixes;
- prebuilt binaries, Cargo/npm publication, package-manager installers, signed artifacts, or
  attestations.

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
4. **Evaluation quality** — expanding beyond two public maintainer-authored families, obtaining
   independent review, operating a leakage-controlled holdout, and avoiding synthetic
   self-confirmation or weak labels.
5. **Cross-source reconciliation** — preserving conflicts between native structure and rendered
   pixels rather than normalizing them away.
6. **Scope drift** — spending the project on codecs, generic computer vision, GUI, MCP, or release
   polish before proving useful findings.

## Canonical next sequence

Issue #71 is now the canonical post-alpha execution epic. Its evidence-first order begins with
fixture-family diversity, independent review, and protected-holdout operation before admitting
broader recommended rules, Web acquisition breadth, interaction breadth, perception models,
additional-medium breadth, or more distribution surfaces.

1. **#72 — second Web family and holdout admission (complete).** ADR 0051 adds
   the additive registry, Harbor support-inbox clean/mutation/hard-negative/ambiguity slice,
   separate acquisition/rule truth, split/family metrics, and strict admission metadata. The
   protected holdout remains honestly `notOperational` and independent review remains absent.
2. **#75 — external manifest and public attestation foundation (implemented in this change).**
   ADR 0052 defines the private/public boundary, frozen digest chain, disclosure threshold,
   lifecycle, current `notRun` record, conformance fixtures, and read-only verifier. It does not
   claim an operational holdout.
3. **#74 — independent review and protected holdout operation (next, externally gated).** Use a
   separately controlled bundle, qualified independent evaluator, second verifier, exposure log,
   and detached signatures; do not relabel public fixtures or fictional identities as evidence.
4. **Later focused children — rule and adapter expansion.** Select one evidence-backed candidate
   at a time only after the earlier evaluation gate supplies its applicability and false-positive
   evidence.

The completed issue #34 sequence remains the verified bounded alpha baseline:

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
5. **#33 — alpha release gate (complete).** Dual licensing, surface-specific compatibility,
   source-only packaging, dependency checks, cross-platform source verification, and the first
   prerelease are present.
6. **#62 — managed loopback Web capture (complete).** Protocol `0.2.0` adds explicit server-command
   authorization, one target-repository page, loopback-only browser traffic, bounded HTTP evidence,
   unavailable source attribution, and owned process-tree cleanup without changing `0.1.0`.
7. **#65 — managed-loopback evaluation authority split (complete).** Strict,
   independently versioned acquisition and rule oracles replace the combined current annotation;
   metrics are derived from reviewed expectations and preserve explicit abstention and non-claims.
8. **#67 — deterministic GitHub Actions job check (complete).** The public Rust
   command projects the same authoritative report into escaped job annotations and an explicit
   bounded summary, using independently reviewed exact-source declarations and preserving
   unavailable source, abstention, evidence, enforcement, and exit semantics.

Do not skip #22 to tune a broad screenshot heuristic. Do not skip #23 by placing browser/model
logic in the Rust kernel. Do not skip #24 by calling raw measurements a complete product.

## Other preserved backlog

- **#25 (complete):** compared strict/current background policy with ranked-border and
  95%-qualified row-run candidates; neither broader policy was admitted.
- **#26 (complete):** exact source-alpha geometry for transparent assets; no rule admitted.
- **#27 (complete):** PNG format-demand and decoder strategy decision; ADR 0041 retains the
  explicit unavailable boundary and adds no decoder dependency.
- **#28 (complete for protocol v0):** isolated local perception process, typed OCR/CV/VLM
  observation families, bounded deterministic reference wrapper, non-promotion boundary, and
  three-state differential regression. Real model calibration/accuracy remains `untested`.
- **#29 (PPTX, PDF, Android, and iOS slices implemented):** ADRs 0043–0046 provide bounded local
  source/capture adapters and separate regression corpora.
- **#30 (complete for the bounded first slice):** medium-neutral actions/effects/states,
  deterministic controlled traces, async feedback, and declared recovery alternatives.
- **#31 (complete for its exit criteria):** children #62, #65, and #67 provide managed local
  capture, independent evaluation authority, and a same-kernel GitHub job-check surface. MCP,
  editor/browser/local UI, and later package channels remain demand-led non-goals.

The bounded first slices inside #29 and #30 are implemented. Issue #71 now owns later expansion,
which still requires newly scoped, evidence-gated child issues in dependency order. The completed
alpha, benchmark, adapter, and integration slices do not make stale branches authoritative.

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
python3 tools/generate_github_actions_schemas.py --check
python3 tools/check_github_actions_evaluation.py
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_alpha_assets.py --check
python3 tools/generate_inspection_corpus.py --check
python3 tools/check_alpha_evaluation.py
python3 tools/check_png_format_demand.py
python3 tools/check_web_evaluation.py
python3 tools/check_web_evaluation_v1.py
python3 tools/check_web_holdout_foundation.py
python3 tools/check_web_holdout_foundation.py --conformance-dir evaluation/web/conformance/holdout
python3 tools/check_perception_evaluation.py
python3 tools/generate_pptx_fixtures.py --check
python3 tools/check_pptx_evaluation.py
python3 -m unittest adapters/pptx/tests/test_adapter.py
python3 -m venv .venv-sightlint-pdf
.venv-sightlint-pdf/bin/python -m pip install --disable-pip-version-check --require-hashes -r adapters/pdf/requirements.txt
export PATH="$PWD/.venv-sightlint-pdf/bin:$PATH"
python3 tools/generate_pdf_fixtures.py --check
python3 tools/check_pdf_evaluation.py
python3 -m unittest adapters/pdf/tests/test_adapter.py
python3 tools/generate_android_fixtures.py --check
python3 tools/check_android_evaluation.py
python3 -m py_compile adapters/android/sightlint_android.py
python3 tools/generate_ios_fixtures.py --check
python3 tools/check_ios_evaluation.py
python3 -m py_compile adapters/ios/sightlint_ios.py
python3 tools/release.py validate-tag --tag v0.1.0-alpha.2
python3 tools/check_dependency_licenses.py
python3 -m unittest tools/test_release.py
npm --prefix adapters/playwright ci --ignore-scripts
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run check
npm --prefix adapters/perception ci --ignore-scripts
npm --prefix adapters/perception run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/perception run test:e2e
npm --prefix adapters/playwright run test:e2e
npm --prefix adapters/playwright run test:managed-e2e
npm --prefix adapters/playwright run test:server-e2e
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p sightlint-cli --test e2e
cargo test --locked -p sightlint-cli --test github_actions_e2e
cargo test --locked -p sightlint-cli --test png_filter_e2e
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test alpha_geometry_evaluation_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test image_segmentation_benchmark_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test evaluation_corpus
cargo test --locked -p sightlint-cli --test github_actions_evaluation_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test pptx_evaluation_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test pdf_evaluation_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test android_evaluation_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test ios_evaluation_e2e -- --nocapture
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
- why issue #25 retained the strict baseline after broader background hypotheses failed realistic
  hard-negative and false-grouping evidence, why source-alpha geometry does not establish a UX
  defect, and why ADR 0041 did not admit broader PNG decoding without a product gap;
- why stale Draft branches are not a shortcut;
- which exact E2E proves the public claim;
- what remains unimplemented and what the PR must not claim.
