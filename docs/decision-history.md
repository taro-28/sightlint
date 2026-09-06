# Decision history and alternatives

This document preserves the reasoning that led to the current repository. Accepted ADRs remain
the normative architectural record. This history explains context, alternatives, reversals, and
superseded experiments so a future coding agent does not repeat the same investigation or infer
intent from branch names.

## Product origin

The starting observation was that AI-generated applications can be technically functional while
still exhibiting basic visual/UX defects. The desired solution was not another prompt template or
one-off screenshot critique. The requirement was a reusable checker that applies ordinary UI/UX
fundamentals even when the user gives no design-quality instructions.

That led to these product conclusions:

- the checker needs reusable rule/policy knowledge;
- rendered output matters, not only source code;
- image analysis should be usable beyond web pages;
- mobile apps, slides, documents, PDFs, and images should fit the same broad model;
- dynamic UX eventually requires traces, not one screenshot;
- probabilistic perception can help acquire structure but cannot be the final blocking judge;
- exact measurements and contextual applicability must be separated;
- the tool needs evaluation data early, because technically correct infrastructure can still
  become the wrong product.

## Name selection

The project name is **SightLint**, with repository and CLI spelling `sightlint`.

The desired naming properties were:

- communicate “linting what is seen,” not only source code;
- remain applicable to web, mobile, slides, documents, PDFs, and images;
- work naturally as a CLI command;
- retain `lint` so users understand the workflow;
- provide enough brand/search distinctiveness for an OSS project.

Alternatives considered included `VisualLint`, `vlint`, `ViewLint`, `CraftLint`, `PrismLint`,
`FrameLint`, `LensLint`, and several visual/verify-derived names. The direct names were often
already used by tools or research in closely related areas, and `vlint`/`ViewLint` had especially
confusing overlaps with visual UI linting. `SightLint` was selected as the best balance of meaning,
CLI readability, cross-artifact scope, and distinctiveness.

Do not casually rename the project. A rebrand would require renewed package/repository/product/
trademark research, compatibility planning, and migration documentation.

## Core versus adapters

### Decision

Use a deterministic Rust kernel surrounded by replaceable adapters and optional perception
workers.

### Why

The kernel owns stable contracts, validation, units, geometry, rule execution, outcomes, and
canonical reports. Rust supports local native binaries, explicit resource/error handling, and
memory-safe parsing of untrusted inputs.

The best adapter language varies by platform:

- TypeScript/Node for Playwright/browser automation;
- Kotlin for Android;
- Swift for iOS;
- Python for OCR/CV/model experimentation;
- Rust for bounded file parsers and deterministic image primitives.

Versioned process boundaries were preferred to an early in-process plugin ABI because they isolate
crashes, memory, dependencies, runtimes, and language choices.

### Alternatives not selected

- **All TypeScript:** convenient for web but weak as the cross-platform trusted image/file kernel
  and likely to pull browser/framework assumptions into core data.
- **All Python:** excellent for perception research but less suitable as a small deterministic
  distributable kernel and static contract boundary.
- **All Rust:** unnecessary and counterproductive for browser/mobile/model integrations.
- **In-process plugin ABI:** premature compatibility burden and weaker failure isolation.
- **Hosted service first:** conflicts with local/private artifacts and makes the core dependent on
  network availability and retention policy.

## Medium-neutral Artifact IR

### Decision

Create a language-neutral, versioned Artifact IR rather than defining the product around DOM/CSS
or raw screenshots.

### Model influences

The IR borrows ideas rather than copying one existing schema:

- scene graphs and design-tool node trees for canvases, nodes, bounds, hierarchy, and z-order;
- DOM/accessibility trees for roles, names, states, actions, and relationships;
- Figma-like node structures for cross-artifact visual objects and properties;
- platform accessibility/UI-automation hierarchies for mobile semantics and hit geometry;
- slide/document/PDF object and tag trees for pages, shapes, text, reading order, and native IDs;
- W3C ACT-style atomic/composite rule concepts and outcome distinctions;
- JSON Schema/versioned extensions for language-neutral validation and forward evolution;
- evidence/annotation models for provenance, selectors, confidence, and uncertainty.

No existing structure fully represented source geometry, rendered ink, hit targets, inferred
semantics, traces, evidence strength, and cross-medium extensions together. A SightLint-specific
IR was therefore justified, while retaining familiar concepts and serializable boundaries.

### Key distinctions

- source/layout bounds, render/ink bounds, and hit bounds are different;
- observations and derived relations are different;
- exact facts and inferred values are different;
- evidence confidence and verdict outcome are different;
- rule severity and confidence are different;
- medium-specific fields belong in versioned extensions;
- IDs must not depend on input order or randomized hashes.

### Alternatives not selected

- **DOM as universal IR:** excludes non-web artifacts and encodes implementation details.
- **Accessibility tree as universal IR:** useful semantics but incomplete visual geometry and no
  guarantee of rendered visibility.
- **Screenshot pixel graph only:** universal input but weak meaning and expensive inference.
- **Figma node schema as universal IR:** useful design structure but not interaction traces or
  platform/runtime evidence.
- **One untyped JSON property bag:** easy initially, impossible to validate/version reliably.

## Evidence before verdict

### Decision

Every result must identify observed facts, targets, expected obligation, policy source, evidence,
and uncertainty. The trusted outcomes are `passed`, `failed`, `inapplicable`, `cantTell`, and
`untested`.

### Why

Visual/UX rules are often context-dependent. A numerical outlier can be an intentional group or
variant. Returning `cantTell` protects precision and makes coverage visible. `untested` prevents an
unexecuted check from looking like a pass.

### Alternatives not selected

- **Boolean pass/fail only:** hides missing evidence and encourages false certainty.
- **Model confidence as outcome:** confidence is not applicability or correctness.
- **Severity derived from confidence:** a low-confidence catastrophic issue and a high-confidence
  cosmetic issue are different dimensions.
- **One aggregate quality score:** conceals which obligations failed and cannot be a trusted gate.

## Policy instead of taste

### Decision

Executable rules verify narrow obligations. Policy precedence is project contract, exact
design-system/platform contract, inferred project norm, platform convention, then conservative
built-in baseline.

### Why

The product should work with no per-project setup, but there is no context-free universal answer
for every spacing, text size, hierarchy, density, or interaction choice. Built-in recommended
profiles are necessary, yet reports must identify their policy source and accept project
exceptions.

### Alternatives not selected

- **Require users to configure every value:** defeats the zero-instruction product requirement.
- **Hard-code one universal design system:** high false-positive risk and style lock-in.
- **Learn the current project as truth:** existing defects can become the inferred standard.
- **Pure aesthetic critique:** useful as advisory prose, not the deterministic core.

ADR 0035 implements the first issue #24 profile slice: `sightlint:recommended` is the additive
default, `--profile base` is the explicit opt-out, and policy provenance and enforcement are
separate CheckReport fields. The first three Web rules remain advisory because their evaluation is
public, single-family, and non-holdout.

## Playwright and rendering

### Decision

Playwright is not required to inspect every artifact, but it is the preferred first structured
adapter for web applications.

### Why

A screenshot is sufficient for pixel facts but not reliable semantic roles, DOM hierarchy,
computed styles, hit targets, clipping ancestors, or interaction state. A controlled browser can
provide native facts and the screenshot from the same session. The adapter remains outside the
kernel.

### Alternatives not selected

- **Make Playwright mandatory for core:** prevents image, mobile, slide, and document use.
- **Screenshot-only web analysis first:** forces expensive/uncertain reconstruction of information
  the browser already knows.
- **Static source analysis only:** misses final rendering, transforms, font/runtime differences,
  clipping, and occlusion.

ADRs 0033 and 0034 implement issue #23's bounded local-fixture adapter, deterministic capture
contract, and evidence matrix. ADR 0035 promotes the versioned Web payload to an official optional
extension consumed only after strict Rust validation; Playwright remains outside the kernel.
Complete hit regions and pixel-content identity remain explicitly unresolved rather than inferred.
ADR 0048 extends only the adapter boundary with explicit managed-server authorization, loopback-
only browser interception, bounded response identity, and process-tree cleanup. It preserves the
legacy file protocol and does not move process launch, source attribution, or verdict ownership
into Rust.

ADR 0049 corrects the managed evaluation-data boundary after #62: exact acquisition expectations
and abstentions now live in a strict acquisition oracle, while deterministic outcomes and false-
positive metrics live in a separate strict rule oracle. Public implementation output is never the
oracle, and the three visible Atlas cases remain non-holdout regression data.

## Image path decisions

### Initial question

Could a screenshot be converted into deterministic structural data and then checked like any
other artifact? The answer remains “partly, with explicit evidence and degraded coverage.”

### Implemented sequence

The project deliberately proved one exact layer at a time:

1. PNG signature/IHDR and source metadata;
2. full chunk framing/order/CRC validation;
3. bounded IDAT inflation;
4. all five filter reconstructions and Adam7 pass layout;
5. bounded eight-bit RGBA raster for common screenshot formats;
6. committed exact pixel corpus;
7. conservative background-relative region/gap observations;
8. advisory-only unequal-gap reporting and negative controls.

Each stage is reachable through public commands and covered by E2E before the next is claimed.

### Important correction

Early remote work over-invested in building PNG details and created multiple speculative Draft
branches. The project advantage is not a custom PNG decoder. The current decision is:

- retain the verified narrow decoder as a trustworthy acquisition slice;
- expand codec coverage only when real product evidence requires it;
- explicitly evaluate a mature decoding library or isolated decoder rather than assuming custom
  code is always safer;
- move effort toward realistic evaluation, structured adapters, semantics, policy, and rules.

ADR 0041 resolves issue #27 for current evidence by retaining explicit unavailability and adding
no decoder dependency. A future product case may reopen exact formats through a new issue and ADR;
absence of a current signal is not a prevalence claim.

ADR 0042 resolves the protocol-v0 boundary in issue #28. A strict local process contract carries
typed region, text, role, hierarchy, and peer observations with explicit provenance, confidence
availability, alternatives, uncertainty, family coverage, preprocessing, and resource/privacy
declarations. Only model-free measured regions map into core IR; inferred semantics remain outside
the trusted kernel and cannot create a verdict. The reference worker uses an existing deterministic
segmentation report and the Atlas differential corpus, so real OCR/model quality and calibration
remain untested rather than being inferred from protocol conformance.

### Alpha-visible geometry

Alpha provides exact visible-source bounds for transparent assets but does not identify whitespace
inside opaque screenshots. The old implementation branch was not integrated. ADR 0040 implements
the bounded observation from current `main`: exact encoded-alpha geometry and evidence, without
compositing, semantic padding judgment, or a rule.

### Background candidates and components

Two approaches were explored:

- ranked exact corner/edge color candidates;
- a 95%-edge-qualified background with row-run/union-find components.

Current `main` uses a stricter unanimous-perimeter hypothesis and bounded flood fill because its
behavior is easier to state and test. The broader approaches may improve coverage but can mistake
headers, gradients, photos, overlays, or edge content for background. They are benchmark
candidates in issue #25, not accepted current behavior.

### Semantic boundary

The current image command can measure `[1,1]` versus `[1,2]`, but it cannot prove whether the third
item belongs to the same semantic group. That exact distinction is why the report says
`uxVerdict: cantTell` and remains nonblocking.

## Testing and evaluation decisions

### Public-binary E2E from the start

A linter can have perfect unit tests and a broken command, adapter wiring, report, or exit code.
Therefore every public behavior requires committed native input and execution through the built
`sightlint` binary.

The required case families include, where applicable:

- passing;
- targeted fail/mutation;
- `cantTell`;
- `inapplicable`;
- `untested`;
- malformed input;
- exact boundary and resource limit;
- ordering and metamorphic transformations;
- repeated byte-identical output;
- OS/MSRV compatibility.

### Conformance versus product validity

The repository keeps these distinct:

- parser/rule/CLI conformance;
- sensor acquisition correctness;
- intended product outcomes;
- eventual user benefit.

The initial product corpus and repository-owned Web application are valuable for regression, but
they cannot establish real-world UI review accuracy. ADRs 0032–0035 therefore keep acquisition and
rule truth separate and the first recommended Web rules advisory. Representative independent and
holdout evaluation remains a future evidence gate.

### Why mutation pairs matter

A clean fixture alone proves little. A targeted mutation should change one property and be killed
by the named rule. This helps verify that the checker detects the intended defect rather than
merely producing a stable report.

### Why holdout data is planned

A rule tuned against all known examples can memorize its development corpus. Future credible
accuracy claims need frozen holdout data, documented access/leakage policy, and reviewed oracle
changes.

## Cross-artifact scope

The architecture was required to support web, mobile, slides, documents, PDFs, and images. This is
an architectural constraint, not a schedule to implement everything at once.

The sequence is:

- prove the kernel and exact IR rules;
- prove image acquisition and uncertainty boundaries;
- build realistic evaluation;
- add Playwright as the first rich adapter;
- establish recommended rules;
- establish the optional perception process/provenance boundary;
- then add PPTX/PDF/mobile adapters according to demand and fixture quality;
- add interaction traces only through isolated protocols;
- add MCP/GitHub/editor/UI surfaces after the core produces useful evidence.

ADR 0042 completes the protocol-v0 perception boundary. ADR 0043 then selects PPTX as the first
post-Web structured medium and proves a bounded direct-shape/source-geometry slice without moving
OOXML into the kernel. ADR 0044 follows with a hash-locked untrusted PDF parser process and maps
only exact page and rectangular internal-Link activation geometry; a `QuadPoints` case proves that
uncertain activation regions are not promoted. ADR 0045 then selects a bounded file adapter over a
repository-owned Android instrumentation capture: exact View allocations, accessibility
semantics, and PNG pixels stay distinct, and offscreen/invalid platform geometry is not promoted.
ADR 0046 completes the bounded first-slice sequence with paired UIKit source, XCUITest platform,
and PNG evidence; clipped scroll content, offscreen source facts, and source/platform conflicts are
preserved without geometry promotion. Issues #30 and #31 preserve the later stages.

## Local-first and privacy

### Decision

Core commands must run locally and transmit nothing unless the user explicitly selects an
external adapter/model.

### Why

Source code, screenshots, documents, and app states may contain credentials, customer data, or
unreleased designs. Local operation also improves repeatability and CI use.

### Deferred decisions

Remote perception, telemetry, hosted history, and organization policy services may exist later,
but require explicit endpoint, retention, transmitted-data, and opt-in decisions. They cannot be
a requirement of core linting.

## Auto-fix

### Decision

Do not begin with broad automatic correction. Start with evidence-linked findings and rerun the
same rule after a coding agent proposes an edit.

### Why

A layout change can affect responsive behavior, design-system constraints, semantics, and other
rules. A generative model should not both make the change and declare success without an
independent rerun.

Narrow deterministic fixes can be introduced later with preconditions, source ownership,
post-fix verification, and mutation tests.

## Remote-development incident and lessons

The mobile/remote phase demonstrated several process failures:

- multiple branch names for one logical task;
- stale Draft PRs appearing more advanced than verified `main`;
- write-enabled temporary workflows used to assemble or repair code;
- duplicate modules waiting for later wiring;
- tests that referenced APIs the public library did not expose;
- success reports based on old or partial CI;
- confusion between PR mergeability and actual green checks.

PR #18 recovered the integration baseline. Later work used read-only normal CI and exact-head
verification. The permanent lessons are encoded in `AGENTS.md`, `docs/handoff.md`, issue #32, and
the PR template:

- one current branch per task;
- current `main` only as the base;
- no self-writing feature workflows;
- no unconnected implementation;
- exact final-head and post-merge CI;
- stale work moves to issues/docs and is closed.

## Historical PR disposition

| PR | Outcome |
|---|---|
| #1 | project foundation merged |
| #2 | deterministic vertical slice and E2E merged |
| #3 | visual geometry/typography contracts merged |
| #4 | first deterministic PNG adapter merged |
| #5 | full chunk validation merged |
| #6 | bounded IDAT inflation merged |
| #7 | product evaluation harness merged |
| #8–#9 | placeholder/redundant PRs closed during recovery |
| #10 | verified filter reconstruction merged |
| #11 | staged common RGBA raster experiment merged into the then-current line; later recovered/current path is defined by #18/#20 |
| #12 | superseded duplicate filter branch, closed |
| #13 | superseded broad PNG normalization; ADR 0041 did not admit it without product need |
| #14 | superseded alpha geometry branch; current implementation is ADR 0040 / #26 |
| #15 | superseded background candidate branch, research in #25 |
| #16 | superseded image corpus branch, real evaluation in #22 |
| #17 | superseded component branch, current slice #21 and research #25 |
| #18 | integration and CI recovery merged |
| #20 | verified raster plus 38-case pixel corpus merged |
| #21 | advisory region/gap inspection plus 30-case corpus merged |
| #35 | authoritative local handoff and evidence-gated roadmap merged |
| #36 | realistic Web evaluation foundation merged |
| #37 | first reviewed Playwright acquisition slice merged |
| #39 | repository protection/settings evidence merged; issues #19/#32 resolved |
| #40 | complete bounded Playwright evidence matrix merged |
| #41 | first advisory recommended Web pack merged |
| #43 | one-command local Web agent workflow merged |
| #45 | accessibility snapshot parser ReDoS repair merged |

PR numbers not listed as merged capabilities must not be inferred from sequence alone.

## Accepted, historical, and proposed ADRs

The index in `docs/decisions/README.md` is authoritative.

Important historical detail:

- two different ADR files were assigned number 0024 during remote development;
- branch-only ADRs 0025–0029 described experiments that were never accepted into current `main`;
- ADR 0030 re-established the verified raster boundary;
- ADR 0031 defines current advisory image inspection;
- ADR 0035 defines the first recommended Web profile and advisory enforcement;
- ADR 0036 defines the one-command local Web orchestration and source-navigation report;
- accepted ADR 0007 selects dual `MIT OR Apache-2.0` licensing;
- ADR 0037 defines the source-only first alpha release and surface-specific compatibility policy;
- ADR 0038 preserves read-only verification and immutable public tags after the unpublished
  alpha.1 workflow-transport failure;
- ADR 0039 compares broader exact-color segmentation hypotheses without replacing the strict
  default after the realistic corpus exposes unsafe selection and false grouping;
- ADR 0040 defines exact source-alpha geometry, its PNG extension/evidence contract, and the
  separate acquisition-versus-rule evaluation boundary;
- ADR 0041 compares PNG decoder strategies and retains the bounded subset until product evidence
  establishes a gap;
- ADR 0042 defines the local perception-worker protocol, typed observation families, and
  non-promotion boundary;
- ADR 0043 defines the bounded PPTX source-geometry process, separate rendered extent evidence,
  and public acquisition-versus-rule evaluation boundary;
- ADR 0044 defines the bounded PDF page/Link-annotation process, exact hash-locked parser,
  conservative non-rectangular abstention, and separate acquisition/rule evaluation boundary;
- ADR 0045 defines the bounded Android instrumented-capture file adapter, separate View/platform/
  render evidence, offscreen abstention, and public acquisition/rule evaluation boundary;
- ADR 0046 defines the bounded iOS paired UIKit/XCUITest capture adapter, separate source/platform/
  render evidence, clipped-scroll/offscreen abstention, and public acquisition/rule evaluation
  boundary;
- ADR 0047 defines medium-neutral deterministic interaction contracts, controlled traces, and
  separate acquisition/rule evidence;
- ADR 0048 defines opt-in managed loopback Web capture, server lifecycle ownership, browser-side
  same-origin enforcement, digest/redaction rules, and unavailable source attribution;
- ADR 0049 defines separate managed-loopback acquisition and rule evaluation authorities;
- new ADRs should continue at 0050 or later rather than silently reusing historical numbers.

Historical branch ADRs are useful design references only. Their `Status: Accepted` header applied
inside an unmerged branch and does not make them accepted repository decisions.

## Decisions still open

- representative corpus expansion and protected holdout operation beyond the public #22 slice;
- broader arbitrary-project and cross-platform Playwright compatibility beyond the bounded
  managed-loopback #62 slice;
- project/profile override syntax and future rule-promotion thresholds beyond ADR 0035;
- exact unsupported PNG formats and decoder admission if future product evidence establishes a
  gap after ADR 0041;
- real OCR/CV/VLM model selection, calibration, and representative perception evaluation after
  the issue #28 protocol foundation;
- representative/broader PPTX, PDF, Android, and iOS coverage after the bounded issue #29 slices;
- broader interaction scenarios beyond the bounded issue #30 slice;
- MCP/GitHub/editor/local UI and later package channels: issue #31;
- signing/attestation identity and any future registry or prebuilt-binary channel after the
  source-only ADR 0037 alpha.

Open issues are hypotheses and work contracts. They become normative only through accepted ADRs,
verified implementation, and merged `main`.

Administrative issues #19 and #32 are resolved, and the bounded issue #34 alpha sequence is
complete. GitHub now enforces the documented `main`
ruleset, squash-only merging, and automatic head-branch deletion; the legacy remote refs were
removed after their unique intent was preserved in issues and this history.

Release run 34000128047 is negative operational evidence: alpha.1 packaging and draft creation
succeeded, but read-only verification jobs could not see draft release assets. Issue #47 and ADR
0038 preserve the failure instead of widening every job to write access or moving the public tag.
