# Evaluation strategy

SightLint is a quality gate, so implementation tests cannot be the only judge of whether the
product works. This document separates the kinds of evidence required for parser correctness,
observation acquisition, rule behavior, and actual user benefit. It complements
`docs/testing-strategy.md`, which defines the general test layers and executable E2E requirements.

## Four questions, four evidence layers

### 1. Conformance

**Question:** Does the implementation obey its declared contract?

Conformance covers:

- schema validation and compatibility;
- parser/decoder algorithms;
- units, coordinate transforms, tolerances, and canonical ordering;
- rule semantics and outcome composition;
- evidence linkage and report serialization;
- malformed inputs and stable diagnostics;
- time, memory, allocation, node, page, frame, and output limits;
- public command behavior, stdin/file paths, formats, and exit codes;
- deterministic output on repeated runs and supported platforms.

Synthetic generated fixtures are excellent for exact conformance boundaries. A green conformance
suite proves that the code matches the written contract; it does not prove that the contract
captures useful UI/UX quality.

### 2. Acquisition evaluation

**Question:** Did the adapter or perception worker recover the observations that exist in the
artifact?

Examples:

- pixel decoder returned the correct RGBA values;
- browser adapter captured the correct node, role, font, rectangle, clipping ancestor, and hit
  target;
- image analysis found the intended regions and did not fragment/merge hard negatives;
- OCR recovered text and bounds;
- native and pixel observations were correctly reconciled or marked conflicting;
- unavailable/ambiguous cases abstained rather than fabricating structure.

Acquisition metrics include:

- object/text/region precision and recall;
- bounds/coordinate error;
- hierarchy, reading-order, and peer-relation accuracy;
- fragmentation and false-merge rates;
- coverage and correct abstention;
- run-to-run agreement;
- native-versus-pixel agreement/conflict classification;
- latency and resource consumption.

Acquisition success does not establish that a design rule applies. Correctly measuring gaps
`[1, 2]` is not proof that the elements are semantic peers or that the layout is wrong.

The ADR 0043 PPTX corpus applies this separation directly: acquisition annotations specify native
IDs, group hierarchy, exact source EMU rectangles, digest-only text metadata, and slide/render
extent reconciliation. A different rule annotation then evaluates only
`visual.bounds.within-canvas`. The asymmetric challenge case declares no peer-spacing relation,
and shape-to-pixel identity remains `cantTell`. Its in-memory CI metrics are public-regression
measurements, not a protected-holdout or real-world presentation-accuracy estimate.

ADR 0044 applies the same separation to PDF. Its acquisition oracle specifies explicit page boxes,
indirect Link annotation references, source-to-top-left hit-box transforms, action/type facts,
render extent, and a required `QuadPoints` abstention. A different rule oracle covers only the
exact rectangular hit boxes consumed by `visual.bounds.within-canvas`. The source-only off-page
mutation leaves rendered pixels unchanged, so source and render disagreement is preserved rather
than using one as truth for the other.

ADR 0045 applies the same separation to Android. Its acquisition oracle labels platform display,
View allocation/state, accessibility geometry, screenshot extent, mapping exclusions, and
abstentions independently from adapter output. Its rule oracle covers only the admitted exact
View `layoutBox` facts consumed by `visual.bounds.within-canvas`. Clipped or invalid accessibility
bounds remain separate evidence and cannot repair source geometry or create touch/render facts.

### 3. Rule/product evaluation

**Question:** Given sufficient observations, applicability, and policy, did SightLint return the
outcome a qualified reviewer or exact contract expects?

Rule evaluation needs:

- valid targets and peer/role relationships;
- policy source and version;
- tolerated alternatives and scoped exceptions;
- clean and targeted mutation pairs;
- hard negatives where the same visual pattern is intentional;
- expected `cantTell`, `inapplicable`, and `untested` results;
- evidence strength and maturity requirements.

Metrics include, per rule, medium, evidence class, profile, and split:

- precision and false-positive rate;
- recall at measured coverage;
- correct abstention;
- mutation kill rate;
- outcome stability;
- reviewer agreement and adjudication rate;
- effect of native/pixel conflicts;
- error breakdown by acquisition, applicability, policy, and judgment.

Do not report one universal design/UX score. Aggregate dashboards may summarize independent
metrics but cannot replace rule-level evidence.

### 4. User-outcome evaluation

**Question:** Does the checker help people and coding agents produce better artifacts or avoid
costly recurring defects?

Later evidence may include:

- time to identify and fix a defect;
- percentage of findings accepted versus suppressed/rejected;
- recurrence reduction for named defect classes;
- review effort and CI noise;
- whether coding agents can navigate evidence, make a correct edit, and verify it;
- whether rules improve task success, accessibility, trust, or recovery in real use;
- qualitative feedback about explanations and policy control.

A technically precise rule can still be unhelpful or annoying. User-outcome evidence is required
before claiming broad workflow or usability benefit.

## Current committed evaluation assets

### Artifact IR conformance corpus

`fixtures/e2e/` contains deterministic generated inputs covering clean artifacts, targeted
mutations, ambiguity, inapplicability, malformed inputs, compatibility, ordering, and supported
artifact kinds. The public binary executes these fixtures.

### Rule smoke product corpus

`evaluation/corpus.json` is a versioned smoke oracle over reviewed synthetic Artifact IR. It checks
required outcomes, forbids undeclared failures/abstentions where configured, repeats outputs, and
requires clean/mutation pairs to change the named rule.

This is useful regression evidence but not real-world accuracy.

### Realistic Web evaluation foundation

`evaluation/web/` implements ADR 0032 as the first issue #22 slice. It contains one repository-owned
fictional dashboard, six environment/state records, separate acquisition and rule annotations,
review metadata, source/license/privacy declarations, an explicit non-holdout policy, one targeted
peer-spacing mutation, and one intentional-grouping hard negative.

Three smoke cases execute independently authored Artifact IR projections through the built
`sightlint` binary and require byte-stable reports. Three development cases preserve ambiguous
peer intent, a narrow viewport, and increased text scale as deferred abstentions. A separate
23-case ADR 0033–0035 companion now exercises controlled local Playwright capture, selected DOM and
accessibility observations, computed geometry, overflow, clipping, center-hit samples, writing
direction, synchronized screenshot extent, and the built Rust binary against independent
acquisition and rule oracles. It reports 76 acquisition expectations, 45 acquisition abstentions,
11/11 observed acquisition mutations, 6/6 rule-eligible mutation kills, 6/6 matched emitted
failures, and zero unexpected or hard-negative failures. For each of the three recommended rules,
the browser E2E records 5/5 contracted outcome-category entries, 1/1 matched failure, 2/2 reviewed
abstentions, 1/1 killed targeted mutation, and zero hard-negative failures. Pixel-content identity,
complete hit regions, and semantic peer inference remain `cantTell`/`untested`. These counts are
regression evidence for one public fictional application, not real-world acquisition accuracy or
representative rule precision.

ADR 0036 adds a separate public agent-workflow oracle and E2E. The test runs one combined local
capture/check command, joins the reviewed finding to a native selector and source bundle, applies
one human-authored edit only in a temporary fixture copy, and requires the named finding to
disappear without a new failure. It also preserves one ambiguous control and one intentional
overlay as `cantTell`. Repeated JSON and human bytes are checked within the declared environment.
This is a deterministic product-path regression, not representative agent user-outcome evidence:
the task, locator, edit, and labels are public and visible to the implementation.

### Interaction trace evaluation

`evaluation/interaction/` implements ADR 0047 over the repository-owned Atlas settings app. Its
eight public cases record 35 manually reviewed acquisition facts separately from two-rule verdict
truth, kill missing-pending and missing-recovery mutations, accept save-draft as a valid hard
negative, and retain `cantTell`, inapplicable, and `untested` outcomes. DOM, accessibility,
screenshots, and app-declared effects remain separate evidence sources.

All cases and labels are public maintainer-authored development data. There is no protected
holdout or independent reviewer, so perfect regression results do not establish representative
interaction, accessibility, or UI/UX accuracy.

### Managed loopback Web evaluation

ADR 0048 adds a managed-loopback evaluation with three public cases: clean, one named unnamed-
control mutation, and an intentional-overlay hard negative. ADR 0049 separates its current exact
acquisition expectations and acquisition abstentions from its rule-verdict oracle. The adapter
must start the repository-owned server, traverse a redirect and same-origin API call, run the built
kernel, and release its process tree and port. A separate lifecycle matrix evaluates failures and
resource boundaries. The product E2E derives coverage, failure precision, abstention, unexpected-
failure, mutation-kill, and hard-negative counts from the reviewed documents rather than from a
combined test constant. This establishes the opt-in startup/capture/check/cleanup path and its
redaction contract; because it reuses the same Atlas family and public labels, it does not add a
holdout, representative sampling, or evidence for changing rule maturity or enforcement.

### PDF source-adapter evaluation

`evaluation/pdf/` implements ADR 0044 with three deterministic repository-owned report pages and
separate acquisition/rule annotations. The clean case has three rectangular internal links, the
targeted mutation moves only one source annotation rectangle beyond the CropBox while keeping the
render bytes identical, and the asymmetric hard negative gives one link disjoint `QuadPoints`
that must not become an exact core hit box.

The public process and built binary recover eight reviewed exact hit boxes, retain the one required
non-rectangular abstention, kill the one declared mutation, and emit no clean/hard-negative
failure. Source and render digests, Poppler render provenance, fictional ownership, dual license,
privacy, public smoke/development/challenge exposure, and missing holdout are explicit. These are
three maintainer-authored regression cases, not representative PDF, accessibility, interaction,
or document-quality accuracy. Text, tags, paint, viewer hit testing, and node-to-pixel identity
remain `untested` or `cantTell`.

### Android capture-adapter evaluation

`evaluation/android/` implements ADR 0045 with three API-35 captures produced from the
repository-owned Atlas account/settings application. The clean case supplies a realistic static
screen, the targeted mutation moves only the Save View allocation beyond the display, and the
asymmetric hard negative adds ordinary offscreen scroll content whose platform accessibility
bounds are invalid after clipping.

The public adapter and built binary match 114 reviewed acquisition facts, emit the one expected
source-bounds failure, retain the hard-negative exclusion, kill the one declared mutation, and
emit no clean/hard-negative failure. View, accessibility, and screenshot evidence retain separate
classes and coordinate meaning. Capture/request/PNG digests, tool/device/build provenance,
fictional ownership, dual license, privacy, public smoke/development/challenge exposure, and
missing holdout are explicit.

These are three public maintainer-authored regression cases, not representative Android,
accessibility, device, or UI/UX accuracy. Compose, arbitrary applications, live capture, touch
regions, dynamic behavior, occlusion/ink, and rendered node identity remain unimplemented,
`untested`, or `cantTell`.

### iOS capture-adapter evaluation

`evaluation/ios/` implements ADR 0046 with three captures produced from the repository-owned
UIKit Atlas account/settings application on one pinned iPhone 17 Pro simulator profile. The clean
case supplies a realistic static screen, the targeted mutation moves only the Save button's UIKit
source allocation beyond the point canvas, and the asymmetric hard negative adds ordinary
offscreen scroll content whose UIKit allocation and XCUITest projection must not manufacture a
containment defect.

The public adapter and built binary match 122 reviewed acquisition facts, emit the one expected
source-bounds failure, retain four reviewed hard-negative exclusions, kill the one declared
mutation, and emit no clean/hard-negative failure. UIKit, XCUITest, and screenshot evidence retain
separate classes and coordinate meaning; a source/XCUI frame disagreement remains conflict
evidence. Capture/request/PNG/source digests, capture order, Xcode/runtime/device provenance,
fictional ownership, dual license, privacy, public smoke/development/challenge exposure, and
missing holdout are explicit.

These are three public maintainer-authored regression cases, not representative iOS,
accessibility, device, or UI/UX accuracy. SwiftUI, arbitrary applications, live capture,
activation geometry, dynamic behavior, occlusion/ink, focus navigation, and rendered node
identity remain unimplemented, `untested`, or `cantTell`.

### PNG raster acquisition corpus

`fixtures/png-raster/` contains 43 committed PNG byte cases:

- 36 supported exact raster cases;
- five explicit unavailable interpretations;
- two malformed inputs;
- filter and Adam7 variants;
- exact expected pixel bytes/checksums;
- alpha extremes and hidden RGB;
- a clean/mutated card pair whose semantic spacing verdict remains `untested`.

It proves the current source-pixel path, not screenshot UI understanding.

### PNG format-demand assessment

`evaluation/png-format-demand/` is a versioned scope-admission assessment, not a decoder
conformance corpus or prevalence study. It inventories every repository PNG and links the nine
ephemeral pinned-browser screenshots that product evaluation passes through the public image
command. The assessment keeps five unsupported synthetic raster controls explicitly separate from
product-demand evidence.

All reviewed product inputs use the current eight-bit RGB/RGBA subset. That is evidence for
retaining the present boundary under ADR 0041, not evidence that users never have other PNG
formats. The labels are public development data, no customer telemetry or artifact content is
collected, no protected holdout or representative sample exists, and broader decoding remains
`untested`. A drift checker requires review when a repository PNG, capture contract, unsupported
control, or decoder dependency changes.

### Source-alpha acquisition evaluation

`evaluation/image-alpha/` contains five repository-owned Northstar transparent UI assets with
separate human-authored acquisition and rule annotations. The acquisition oracle covers visible
and opaque half-open bounds, alpha-class counts, transparent insets, edge occupancy, and expected
`inkBox`; the rule oracle keeps every semantic padding question `untested` with `cantTell` or
`inapplicable` applicability.

The public-binary E2E records 5/5 acquisition matches, 1/1 targeted padding mutation observed, and
2/2 hard negatives without blocking. A hidden-RGB metamorphic pair preserves alpha geometry while
changing the encoded-raster checksum. All assets and labels are public to implementers, derived
from one fictional family, and have no protected holdout or independent reviewer; these counts do
not estimate real-world UI/UX accuracy or justify an alpha-padding rule.

### Image-inspection acquisition corpus

`fixtures/image-inspection/` contains 30 committed cases:

- 19 observed region/gap cases;
- nine explicit unavailable/abstention cases;
- two malformed inputs;
- independent bounds and gap oracles;
- horizontal/vertical, translation, scale, recoloring, multiple groups, blocker, differing
  size/color, holes, mixed regions, touching/diagonal, uniform, border variation, and alpha
  controls;
- an intentional-grouping hard negative with identical pixels to the unequal-gap case.

It proves the narrow unanimous-perimeter acquisition policy and nonblocking uncertainty. It is not
a benchmark for rounded, shadowed, textual, photographic, or complex application screenshots.

### Image-segmentation policy benchmark

`evaluation/image-segmentation/` contains nine temporary-browser-capture cases from one fictional,
repository-owned Northstar application. Its `0.1.0` corpus and schemas keep source-authored visible
surface bounds separate from the rule oracle. The latter records no executable rule,
`applicabilityGroundTruth` as `cantTell` or `inapplicable`, `expectedOutcome: untested`, and no
blocking authority.

The built public binary compares strict perimeter flood fill, ranked exact-border flood fill, and
95%-qualified corner row-run/union-find. Reviewed smoke/development/challenge cases cover clean,
targeted edge contamination, recoloring, translation, device scale, modal, split-pane and gradient
hard negatives, and checkerboard resource stress. Reports and screenshots are temporary rather
than expected outputs.

The initial reviewed metrics show that strict/ranked/qualified region recall is respectively
`1/21`, `2/27`, and `2/27`, with 4, 5, and 5 false groups. Qualified selection correctly abstains
on both semantic-background hard negatives; ranked selection observes both unsafely. These are
small public-corpus regression counts, not representative precision or a private holdout. They do
not justify changing `inspect-image` or creating a downstream rule.

## Next evaluation expansion gates

The first #22 evaluation foundation, #24 advisory rule slice, #42 local-agent path, and #25
segmentation comparison are complete. Before promoting
broader image inference, more Playwright-derived rules, or any rule to blocking maturity, expand
the repository-owned Web applications, independent review, and protected-holdout process. The
following records remain the required shape for those additions.

### Artifact records

Each case should record:

- stable ID and version;
- artifact source revision and deterministic generation/capture command;
- license, ownership, privacy review, and redistribution status;
- medium, viewport, device-pixel ratio, text scale, locale/direction, theme, and platform/browser;
- exact native snapshot and screenshot captured from the same declared state;
- transformation/mutation relation to a baseline;
- known sampling limitations.

### Acquisition annotations

Annotate or derive independently:

- canvases/frames/pages;
- source, layout, rendered/ink, and hit geometry;
- text boxes and relevant text metadata;
- roles, states, actions, hierarchy, reading order, and peer groups;
- clipping, occlusion, overflow, and visibility;
- source selectors and reconciliation links;
- unknown/disputed observations and reviewer alternatives.

Do not force consensus where the artifact is genuinely ambiguous.

### Rule annotations

Record:

- intended rule family and stable target relation;
- exact policy source and accepted alternatives;
- expected outcome and evidence threshold;
- severity inputs rather than a free-form severity label;
- rationale and likely false positives;
- whether the case is clean, targeted mutation, or hard negative;
- whether the result may block or is advisory.

### Split policy

- **smoke:** small, deterministic, required on every PR;
- **development:** visible to implementers for design/tuning;
- **holdout:** frozen and not consulted while tuning the evaluated rule/worker;
- **challenge/hard-negative:** intentionally difficult valid alternatives and unusual layouts.

Before using a holdout, document its freeze commit, access policy, leakage controls, evaluation
command, and process for legitimate oracle corrections.

## Annotation quality

Human-reviewed sources should record:

- reviewer qualification category;
- annotation guide version;
- independent annotations before adjudication where feasible;
- agreement and disagreement;
- final adjudication rationale;
- unresolved ambiguity;
- known cultural, language, platform, and sampling bias.

The project need not begin with a large corpus. A smaller, carefully reviewed dataset is more
valuable than hundreds of weak or self-generated labels.

## Mutation design

Targeted mutations are central because many UI/UX defects have no single ideal screenshot.

A good mutation:

- begins from a valid baseline;
- changes one named property or state;
- preserves unrelated content and environment;
- identifies the rule/acquisition capability it should affect;
- retains expected measurements before and after;
- has a valid inverse/fix where practical;
- does not derive its oracle from SightLint output.

Examples:

- change one peer gap;
- move one element outside a viewport;
- introduce clipping or overlap;
- reduce one text/hit target under an exact policy;
- hide a focus indicator;
- remove pending feedback;
- make retry duplicate an effect;
- remove one accepted recovery path.

## Hard negatives and valid alternatives

Every broad-looking principle needs cases where the same surface pattern is valid:

- intentional grouping and asymmetric editorial layout;
- mixed component variants;
- masonry and data-dependent grids;
- badges or popovers crossing parent bounds;
- sticky/fixed headers and overlays;
- charts, maps, photos, illustrations, code editors, and canvas/WebGL;
- loading, skeleton, empty, error, success, and permission states;
- decorative text and icon assets;
- multiple valid destructive-action safeguards;
- platform-specific patterns and localized/RTL layouts.

Precision is not demonstrated by detecting only artificial positives.

## Reconciliation evaluation

For artifacts with native structure and pixels, label these outcomes explicitly:

- agreement within declared tolerance;
- expected loss from one source;
- native-only observation;
- pixel-only observation;
- coordinate transform mismatch;
- occlusion/clipping conflict;
- semantics/role conflict;
- capture timing or state conflict;
- unresolved conflict requiring `cantTell`.

Do not compute one “source accuracy” number that hides conflict categories. Downstream rule
metrics should show which evidence combination was used.

## Model/perception evaluation

A perception worker must be evaluated by exact model/runtime/preprocessing version. Record:

- object/text/role/relation metrics;
- confidence calibration where probabilities exist;
- alternatives and abstention thresholds;
- repeated-run agreement across supported backends;
- hardware/runtime sensitivity;
- resource and latency distributions;
- downstream rule precision/coverage, not only detector metrics;
- local versus remote privacy and retention conditions.

A model update is a new evidence version even if its API name is unchanged.

ADR 0042's protocol-v0 evaluation deliberately stops before model-quality claims. Three public
Atlas states exercise a local deterministic region worker against synchronized native and pixel
evidence. They require byte stability, explicit unavailable/untested families, retained
layout/render conflict, one acquisition mutation observation, no semantic promotion, no blocking,
and no hard-negative failure. Atlas has two edge surfaces, so qualified and strict exact-color
policies abstain; the evaluation names the ranked policy only to exercise mapping and keeps its
background hypothesis unconfirmed. OCR/text/role/hierarchy/peer precision and recall, probability
calibration, latency distributions, backend sensitivity, and downstream rule metrics remain
`untested`. The public maintainer-authored cases are not a protected holdout.

## Rule maturity and blocking

A rule begins experimental or advisory. Eligibility to block should require documented thresholds
appropriate to its harm/false-positive cost, including:

- stable semantics and compatibility plan;
- sufficient real-case precision and hard negatives;
- useful measured coverage;
- conservative abstention;
- deterministic kernel behavior;
- reliable acquisition for the selected evidence class;
- reviewed policy/default source;
- mutation detection;
- actionable explanation and scoped suppression/exception behavior;
- no material unresolved privacy or platform compatibility risk.

There is no universal numeric threshold for every rule. The threshold and evidence must be stated
in a rule-specific maturity decision.

## Oracle changes

An oracle is reviewed data, not a snapshot of current output.

When an expected observation or outcome changes:

1. identify whether the old oracle, implementation, policy, or annotation was wrong;
2. explain the semantic reason;
3. update guide/version/provenance as required;
4. check related baseline/mutation/hard-negative and holdout implications;
5. review the diff independently;
6. never regenerate expected outcomes by executing the implementation under test.

## Reporting evaluation results

A credible evaluation report includes:

- code, rule, adapter/model, protocol, corpus, and environment versions;
- split and case counts;
- exclusions and unavailable coverage;
- per-capability precision, recall at coverage, abstention, mutation kill rate, and determinism;
- error examples and categories;
- reviewer agreement and sampling limitations;
- whether data was used during tuning;
- non-claims and known unsupported media/states.

Report uncertainty plainly. Green CI means current regressions conform; it does not by itself mean
SightLint is a generally accurate design reviewer.
