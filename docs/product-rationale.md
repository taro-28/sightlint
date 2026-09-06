# Product rationale and problem model

This document preserves the product intent and reasoning that existed before implementation. It
explains what SightLint is trying to solve, why the architecture has its current boundaries, how
the problem was abstracted, and which claims require future evidence. It is not a statement that
all described capabilities already exist.

## Original problem

AI-assisted application development can generate usable code quickly, yet rendered results often
still feel generic, inconsistent, or unfinished. Common examples include:

- spacing values that drift across nominally equivalent components;
- text that is too small or visually weak for its role;
- inconsistent typography, radii, density, alignment, or component dimensions;
- clipping, overlap, accidental overflow, and responsive breakage;
- generic card grids and hierarchy that look plausible in isolation but do not form a coherent
  system;
- missing loading, empty, error, success, disabled, focus, and recovery states;
- controls whose visual bounds, semantic roles, and hit targets disagree;
- destructive or asynchronous behavior without adequate feedback or safeguards;
- slide/document artifacts with weak rhythm, margins, reading order, or visual consistency.

The important requirement is not “provide another design critique prompt.” It is:

> A coding agent should apply ordinary UI/UX and artifact-quality fundamentals even when the
> user did not remember to enumerate those fundamentals in every request.

That implies reusable policy, acquisition, evaluation, and enforcement—not only better prompting.

## Why free-form AI review is insufficient

A vision/language model can notice useful problems, but an unconstrained response is difficult to
use as a release gate:

- observations and wording can change between runs;
- the model may confuse style preference with a defect;
- it may infer roles or relationships that are not present;
- it rarely exposes a complete chain from source evidence to applicability and expectation;
- model/version updates can silently change behavior;
- confidence language is not calibrated evidence;
- a prose answer is hard to compare, suppress narrowly, version, or test through mutations;
- the same model that generated an artifact should not be trusted to grade itself without an
  external oracle.

SightLint can use AI as one sensor, but it cannot make an opaque model opinion the final trusted
judge.

## The abstracted problem

Visual/UX linting is not one algorithm. It is a sequence of different questions with different
trust levels:

```text
artifact source
  -> acquisition sensors
  -> observations
  -> reconciliation and uncertainty
  -> semantic applicability
  -> policy/expectation resolution
  -> deterministic obligation
  -> evidence-linked outcome
  -> explanation and optional remediation
```

Keeping these stages separate is the central architectural decision.

### 1. Artifact source

Examples:

- browser document and screenshot;
- Android/iOS hierarchy and screen capture;
- PPTX DrawingML plus rendered slide;
- PDF tags/drawing operations plus page image;
- standalone image;
- interaction trace and screenshots at named states.

### 2. Acquisition sensors

A sensor extracts or infers data:

- native parsers and platform APIs;
- Playwright/browser automation;
- screenshot/raster analysis;
- OCR and deterministic computer vision;
- optional VLM/classifier workers;
- application-declared action/effect instrumentation.

Sensors can be wrong, incomplete, version-dependent, or probabilistic. They are adapters, not the
trusted kernel.

### 3. Observations

An observation states what was measured or inferred, with units and provenance:

- canvas/page/screen size;
- layout/render/ink/hit rectangle;
- style and typography values;
- text region/content where permitted;
- role, name, state, and action;
- parent/child, peer, reading-order, alignment, overlap, or containment relation;
- visible pixels, colors, edges, and connected regions;
- interaction events, effects, pending/failure/recovery states;
- confidence, uncertainty, alternatives, and conflicts.

An exact rectangle and a guessed “button” role are different kinds of evidence and must remain so.

### 4. Reconciliation

Multiple sources can disagree:

- CSS reports 14 px, but a transform renders approximately 11.2 px;
- an accessibility node says “button,” but it is clipped or fully occluded;
- a slide shape is aligned by source bounds, but transparent image padding shifts visible ink;
- OCR sees text that has no native node;
- a native node exists, but no matching visual region can be found.

SightLint should preserve agreement and conflict rather than selecting one source globally. A
conflict may itself be a finding or may force `cantTell`.

### 5. Semantic applicability

A mathematical difference is not yet a design defect.

For example, gaps `[8, 8, 16]` may represent:

- one accidental spacing mutation;
- two intentionally separated groups;
- a card plus a call-to-action;
- mixed variants that should not be peers;
- an editorial composition where uniform spacing is not expected.

The rule can fail only when evidence establishes that the compared objects are equivalent for the
relevant obligation. Otherwise the result is advisory or `cantTell`.

### 6. Policy resolution

Facts and expectations are separate.

```text
fact: rendered body-text size is 11 device-independent units
policy: selected web/mobile/project profile requires at least X for this role
result: compare the fact to the named policy with explicit units and evidence
```

Policy precedence is:

1. explicit project contract or exception;
2. exact design-system/platform contract;
3. statistically inferred project norm with visible confidence;
4. platform convention;
5. conservative built-in baseline.

The user should receive useful defaults without configuration, but the report must say where each
expectation came from. A built-in default must not masquerade as a universal law.

### 7. Deterministic obligation

After observations, applicability, policy, units, and tolerance are normalized, the final check
should be narrow and reproducible:

- bounds are inside a canvas;
- declared peers do not overlap;
- equivalent gaps stay within a declared tolerance;
- a known text role satisfies a selected minimum;
- a trace includes one of several accepted recovery paths.

The obligation can be atomic or a composite of valid alternatives.

### 8. Outcome

The trusted outcome set is:

- `passed`;
- `failed`;
- `inapplicable`;
- `cantTell`;
- `untested`.

Unknown is not noise. `cantTell` means the relevant evidence was considered but meaning remains
ambiguous. `untested` means acquisition or execution did not occur. Both protect users from false
certainty.

### 9. Explanation and remediation

A useful report answers:

- what was observed;
- where it came from;
- which targets were compared;
- why the rule applied;
- which policy and tolerance supplied the expectation;
- what conflicted or remained uncertain;
- why the result is advisory or blocking;
- which narrow remediation options are valid.

A suggested fix is not proof. The same rule must be rerun after the edit.

## What “deterministic” means here

The project does not claim that all perception is deterministic. The contract is:

> For fixed normalized observations, rule versions, configuration, engine version, and declared
> compatibility environment, the kernel produces the same canonical result.

Determinism also requires:

- canonical ordering and serialization;
- explicit units and coordinate spaces;
- finite numbers and documented rounding/tolerance;
- no locale, wall-clock, random iteration, or hidden network dependence;
- versioned adapter/model/runtime evidence;
- stable rule evaluation order and exit semantics.

A perception worker may still vary. Its variation is measured as evidence uncertainty before the
kernel, not hidden inside the verdict.

## Why Rust is the kernel language

The implementation language was selected for the tool rather than the maintainer's existing web
stack. Rust fits the trusted local kernel because it offers:

- predictable native binaries and local-first distribution;
- memory-safe systems programming for untrusted formats;
- explicit data models and error handling;
- cross-platform performance for geometry/image primitives;
- minimal runtime requirements;
- strong control over determinism and resource limits.

Rust is not required for every adapter. TypeScript/Node is appropriate for Playwright, Kotlin for
Android, Swift for iOS, and Python for model/CV experiments. Versioned process boundaries preserve
language freedom and isolate crashes/dependencies.

## Why native structure and pixels are both needed

### Native structure strengths

Native sources can provide:

- roles and names;
- hierarchy and peer relationships;
- exact source/computed style values;
- hit targets and actions;
- text content and reading order;
- project/design-system identifiers.

### Pixel strengths

Pixels reveal:

- final visual clipping and occlusion;
- transforms and renderer differences;
- transparent padding and visible ink;
- visual output that native structure omitted;
- a common floor across web, mobile, slides, documents, and images.

### Combined strategy

Native structure is normally better for meaning; pixels are better for rendered reality. The
strongest checks reconcile both. Screenshot-only analysis is allowed to have lower coverage; it
must degrade to uncertainty instead of inventing exact semantics.

## Why Playwright is not universally required

A standalone image, exported slide, or mobile screenshot can be inspected without Playwright.
The current PNG path proves that. Playwright becomes valuable when the target is a web
application because it supplies native structure, computed geometry, accessibility data, and a
controlled rendered screenshot. It is an adapter for one medium, not a requirement of the core
architecture.

## Cross-artifact data model

The common model should represent stable concepts while preserving medium-specific extensions.
Core concepts include:

- artifact;
- canvas/page/screen/frame;
- node/object;
- hierarchy and relations;
- source, layout, render/ink, and hit geometry;
- style/typography/color observations;
- evidence and selectors;
- actions, states, effects, and traces through extensions;
- policy and rule results.

Examples of native mappings:

| Medium | Native structure | Pixel/render source |
|---|---|---|
| Web | DOM, accessibility tree, computed style | browser screenshot |
| Android | accessibility/semantics/UI automation | device/emulator capture |
| iOS | XCUI/accessibility hierarchy | device/simulator capture |
| Slides | PPTX shapes, groups, text, theme, z-order | rendered slide |
| PDF/document | tags, text/image/paint geometry | rendered page |
| Image | metadata and deterministic raster primitives | source pixels |

The core must not gain a mandatory CSS, DOM, PowerPoint, or mobile-only field. Rich native data
belongs in versioned namespaced extensions and maps to shared concepts only when semantics match.

## The zero-setup product wedge

The intended first useful experience is:

```text
coding agent edits a local web project
  -> SightLint captures the page in a controlled environment
  -> native structure and screenshot are reconciled
  -> recommended high-confidence rules run automatically
  -> report names source targets, observations, policy, and uncertainty
  -> agent applies a narrow fix
  -> the same check is rerun and the finding disappears
```

The user should not manually provide every selector, peer group, spacing value, or typography rule
for the common case. Explicit project contracts remain the strongest source when present.

This wedge is captured by issues #22, #23, #24, and execution epic #34.

## Recommended rule packs

ADR 0035 establishes the first composable default: `sightlint:recommended` is additive over
`sightlint:base`, and `--profile base` is the initial explicit opt-out. Future web, mobile, slide,
document, organization, and project overlays still require evidence and a follow-up ADR.

A rule enters a recommended pack only after it has:

- narrow applicability and evidence requirements;
- a named policy source;
- valid alternatives and hard negatives;
- pass/fail/mutation/ambiguity/inapplicable fixtures as relevant;
- real or sufficiently realistic reviewed evaluation;
- measured precision, coverage, abstention, determinism, and mutation kill rate;
- a declared maturity and blocking policy.

High-confidence first-wave candidates are geometry, clipping, overflow, peer consistency, exact
text/hit-target policy checks, and responsive loss. Broad visual hierarchy or aesthetics should
remain advisory until evidence and evaluation are stronger.

## Inferred project norms

A future layer may learn a project's spacing scale, typography roles, radii, density, or component
patterns. This can reduce configuration, but it creates a risk that existing defects become the
inferred standard.

Guardrails:

- explicit contracts override inferred norms;
- show sample count, clusters/modes, confidence, and exceptions;
- do not force multi-modal design systems into one value;
- use robust baselines and targeted mutation tests;
- evaluate on a frozen holdout;
- inferred norms do not become exact facts;
- changing the baseline must be reviewable and versioned.

## Image-only structure

The image path now starts with exact encoded source-alpha geometry under ADR 0040. Longer-term
structure acquisition may combine it with:

- deterministic region/edge/color primitives;
- OCR;
- component detectors;
- hierarchy/peer heuristics;
- optional VLM classification;
- project/native references when available.

No single method is expected to solve the complete problem. Every observation carries its source,
confidence, uncertainty, and alternatives. The current `inspect-image` implementation is a
controlled proof of the data-flow idea, not the planned general algorithm.

Issue #25 preserves broader background/segmentation experiments. ADR 0041 retains explicit
unavailability for broader PNG formats because current product inputs establish no coverage gap;
future format work requires new evidence and a new issue. ADR 0042 implements issue #28's local
perception protocol foundation while leaving real OCR/model accuracy and calibration untested.
ADR 0043 selects a bounded PPTX source-geometry process as the first post-Web structured medium;
it deliberately leaves rendered node identity, master/layout/theme resolution, and representative
slide-quality evaluation unresolved.
Issue #22 remains the evaluation basis before promoting broad heuristics.

## Interaction quality

Static checks cannot verify many UX fundamentals. The eventual interaction model needs:

- actions and preconditions;
- expected effects and affected scope;
- pending/optimistic/success/failure/partial states;
- duplicate-submission and idempotency behavior;
- retained input and recovery;
- confirmation, undo, trash, version history, or other valid safeguards;
- focus and navigation behavior;
- controlled traces with time, order, causal links, and environment.

Composite obligations must permit multiple valid solutions. Issue #30 preserves the detailed
interaction roadmap.

## Evaluation philosophy

SightLint is itself a quality gate, so its evaluation cannot be self-referential.

Maintain separate layers:

- **conformance:** implementation obeys schemas, limits, algorithms, determinism, and public CLI;
- **acquisition evaluation:** sensors recover the intended observations/relations;
- **rule evaluation:** applicable artifacts receive the intended outcomes;
- **user outcome evidence:** rules actually reduce recurring defects or improve workflows.

Use synthetic fixtures for exact boundaries and mutations, realistic repository-owned fixtures
for product development, and frozen holdouts before stronger claims. Measure per rule, medium,
evidence class, and split:

- precision and false-positive rate;
- recall at measured coverage;
- correct abstention;
- run-to-run agreement;
- mutation kill rate;
- native/pixel reconciliation;
- reviewer agreement;
- downstream user outcomes where appropriate.

Do not collapse these into one universal UX score.

## Why broad auto-fix is deferred

A reliable fix requires more than locating a symptom. It must know source ownership, design-system
constraints, valid alternatives, responsive effects, and whether the change introduces new
regressions.

The safe initial model is:

1. report evidence and narrow remediation options;
2. let a coding agent propose an edit;
3. rerun the same acquisition and rule;
4. ensure the target finding is resolved and no new blocking finding appears.

Automatic transformations can be added for narrow, deterministic cases later. A generative edit
must never grade itself without rerunning SightLint.

## Product success criteria

SightLint is succeeding when:

- a user receives useful basic checks without repeating every rule in a prompt;
- identical normalized inputs produce identical reports;
- every result is traceable to evidence, policy, rule version, and target;
- missing meaning reduces coverage rather than fabricating certainty;
- rules have low false-positive rates at stated coverage;
- mutations demonstrate detection of claimed defects;
- adapters can be replaced without rewriting the kernel;
- web, mobile, slide, document, and image observations reuse shared concepts without web hacks;
- coding agents can fix a finding and verify it independently;
- real users find the reports actionable, not merely technically correct.

## Non-goals

SightLint does not aim to:

- prove beauty, persuasion, brand quality, or universal usability;
- replace user research;
- mandate one visual style or exact layout;
- make a VLM the final authority;
- turn every best practice into a blocking rule;
- give one opaque quality score;
- require a hosted service or artifact upload;
- finish every adapter or codec before releasing useful value;
- use green synthetic tests as a claim of real-world accuracy.

## Open decisions

The following remain intentionally unresolved or evidence-gated:

- project/profile override syntax beyond the accepted base/recommended alpha profiles;
- rule-maturity thresholds for blocking eligibility;
- severity model;
- arbitrary-project and cross-platform browser product compatibility beyond the bounded policy;
- representative PPTX expansion and the next non-Web medium after the bounded ADR 0043
  source-geometry slice;
- perception models/runtimes and calibration requirements;
- exact formats and decoder strategy if new product evidence re-admits unsupported PNG coverage;
- interaction trace schema;
- later registry/binary distribution, signing, and attestation policy beyond the source alpha;
- telemetry/hosted-service policy if ever proposed.

Resolve them through current issues and new ADRs from the latest `main`, not by reviving stale
experimental branches.
