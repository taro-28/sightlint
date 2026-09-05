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

### PNG raster acquisition corpus

`fixtures/png-raster/` contains 38 committed PNG byte cases:

- 31 supported exact raster cases;
- five explicit unavailable interpretations;
- two malformed inputs;
- filter and Adam7 variants;
- exact expected pixel bytes/checksums;
- alpha extremes and hidden RGB;
- a clean/mutated card pair whose semantic spacing verdict remains `untested`.

It proves the current source-pixel path, not screenshot UI understanding.

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

## Next evaluation gate: issue #22

Before promoting broad image inference, Playwright-derived rules, or recommended zero-setup
profiles, build a realistic corpus with repository-owned local web fixtures.

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
