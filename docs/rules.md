# Rule model

SightLint rules are deterministic obligations over normalized, evidence-backed observations. They
are not free-form review prompts and they do not own artifact acquisition.

Read `docs/product-rationale.md` for the full problem model and issue #24 for the recommended
zero-setup rule-pack epic.

## Separation of concerns

A trustworthy rule result is assembled from separate inputs:

```text
observed facts
  + semantic applicability and target relations
  + selected policy/expectation
  + units, tolerance, and valid alternatives
  + evidence sufficiency
  -> deterministic obligation
  -> outcome and explanation
```

Do not collapse these stages.

Examples:

- “font size is 11 px” is an observation;
- “this is body text on platform/profile X” is applicability evidence;
- “profile X requires at least Y for that role” is policy;
- comparing 11 with Y is the deterministic obligation;
- whether the result blocks is rule maturity and CI policy.

A gap array `[8, 8, 16]` is not automatically a spacing failure. The rule needs evidence that the
elements are equivalent peers and that one uniform interval is expected rather than intentional
grouping.

## Atomic rule contract

Every atomic rule declares:

- stable rule identifier and semantic version;
- title and user-visible defect class;
- required input aspects;
- target selection and applicability;
- accepted evidence classes and sufficiency threshold;
- policy source and precedence;
- exact expectation or accepted alternatives;
- units, coordinate spaces, tolerance, and rounding;
- possible outcomes and diagnostics;
- severity inputs;
- maturity and default blocking policy;
- false-positive/false-negative risks;
- explanation and remediation contract;
- compatibility and migration behavior;
- conformance, acquisition, product, hard-negative, and mutation evidence.

Rule metadata is part of the public compatibility surface. A stable ID must not silently change
meaning. Use a new semantic version or rule ID when an incompatible change cannot be migrated.

## Required input aspects

A rule lists exactly what it needs rather than assuming a complete object:

- source/layout, render/ink, or hit geometry;
- hierarchy or peer relation;
- role, name, state, or action;
- typography/style/color observation;
- canvas/page/screen properties;
- viewport, scale, direction, locale, or platform;
- project/design-system/platform policy;
- pixel or native reconciliation status;
- interaction action/effect/state/trace;
- evidence source, confidence, uncertainty, and conflicts.

Missing a required aspect produces `cantTell` or `untested` according to the contract, not a
guessed pass/failure.

## Applicability

Applicability answers whether the obligation is meaningful for the targets.

Examples:

- three cards are true peers for spacing consistency;
- an intentionally separated call-to-action is not part of the card peer group;
- body-text minimum policy does not apply to a decorative watermark;
- hit-target policy applies only to an interactive target under a named platform profile;
- contrast applies only when foreground, background, compositing, and color assumptions are known;
- an async-feedback rule applies only to an action with an observable latent effect;
- a destructive-safeguard rule accepts confirmation, undo, soft deletion, history, or another
  explicit approved alternative.

Applicability can be exact/declared, empirically reconciled, or inferred. Its evidence grade must be
visible. Inferred applicability with insufficient confidence or source conflict remains advisory or
`cantTell`.

## Policy resolution

SightLint should eventually provide useful defaults without requiring every project to configure
every value. Expectations are resolved in this order:

1. explicit project contract or scoped exception;
2. exact design-system or platform contract;
3. statistically inferred project norm with visible confidence and support;
4. platform convention;
5. conservative built-in baseline.

A result records which source won, its version, scope, and any overridden alternatives. Project
exceptions must be narrow and reviewable; a broad ignore switch should not hide unrelated results.

ADR 0035 defines `sightlint:recommended` as additive over `sightlint:base` and exposes
`--profile base` as the initial opt-out. Future web, mobile, slides, documents, platform versions,
and organization/project overlays require another ADR before stabilization.

## Inferred project norms

A future policy source may infer spacing scales, typography roles, radii, density, or repeated
component patterns. It remains evidence, not exact truth.

Requirements:

- record sample count, clusters/modes, support, confidence, and exceptions;
- tolerate multi-modal systems rather than forcing one value;
- use robust baselines so existing defects do not become the standard;
- let explicit project/design-system contracts override inference;
- evaluate on a frozen holdout;
- version baseline changes;
- return `cantTell` when the project does not supply enough representative data.

## Outcomes

### `passed`

Applicable, sufficient evidence was available and at least one accepted obligation/alternative was
satisfied.

### `failed`

Applicable, sufficient evidence was available and no accepted obligation/alternative was
satisfied. A failed outcome is not automatically blocking; maturity and CI policy decide that.

### `inapplicable`

The artifact/targets are understood, but the rule does not apply. This is different from missing
evidence.

### `cantTell`

The rule was considered, but required observations, semantic applicability, policy, or conflict
resolution were insufficient for a trustworthy pass/failure.

### `untested`

The relevant acquisition or execution did not run—for example an unavailable adapter, unsupported
format, timeout, or missing trace. `untested` is never a pass.

## Evidence and confidence

Evidence strength describes the source and verification path. Confidence describes uncertainty in
an inferred observation or relation. Neither is the outcome or severity.

Illustrative evidence categories include:

- exact source/native extraction;
- exact deterministic transform;
- project/platform declaration;
- reconciled native and rendered observation;
- empirical heuristic with measured precision;
- model inference with version/calibration;
- conflicting or incomplete evidence.

A high-confidence cosmetic observation can have low severity. A low-confidence potentially severe
problem usually needs investigation/`cantTell`, not an inflated severity or automatic failure.

Do not invent numeric confidence when a source does not provide a calibrated probability. Record
alternatives and repeated-run agreement separately.

## Severity

Severity should be derived from explicit inputs rather than one intuition label:

- user harm and task criticality;
- affected scope and reversibility;
- frequency/likelihood under the evaluated environment;
- recoverability and available workaround;
- accessibility, safety, trust, data-loss, or financial implications;
- whether the issue is visual polish, interaction feedback, or an incorrect/hidden effect.

Severity is independent of evidence confidence. A future severity/CI maturity ADR must define how
these inputs map to labels and organizational policy before broad stable release.

## Maturity and CI policy

Rules progress through documented maturity rather than becoming blocking when first implemented:

- **experimental:** semantics and acquisition still changing; visible only when requested or in
  development profiles;
- **advisory:** useful evaluated signal, but not eligible to fail the default build;
- **blocking-eligible:** stable rule/evidence/policy contract with adequate real-case precision,
  coverage, abstention, hard negatives, compatibility, and remediation;
- **blocking:** explicitly enabled by the selected project/organization profile.

A rule can be severe but not blocking-eligible if acquisition is unreliable. A rule can be highly
precise but remain advisory because its policy is subjective.

Model-only or heuristic-only semantic inference cannot silently block. Project policy may choose a
stricter gate only when the report exposes the evidence and expected consequences.

## Composite rules

Composite rules combine atomic results into user-level obligations. They must preserve child
outcomes and evidence rather than hiding them behind one score.

Examples:

- a destructive action is safe if confirmation, undo, soft deletion, version history, or another
  accepted safeguard is proven;
- an interactive target satisfies both visual/hit geometry and accessible role/name obligations;
- a responsive component preserves essential content across declared viewports;
- a form failure provides actionable error identification, retained input, and a recovery path.

Composition logic is versioned and deterministic. It defines how `cantTell`, `inapplicable`, and
`untested` children propagate.

## Current implemented rule areas

Current structured-IR rules/contracts include:

- bounds within canvas;
- declared non-overlap;
- explicit peer spacing consistency;
- parent containment;
- logical alignment;
- peer width/height consistency;
- peer typography consistency;
- project-supplied minimum font size;
- direction, coordinate-space, unit, tolerance, evidence, and ambiguity handling.

The additive zero-setup `sightlint:recommended` profile also includes three rules for strictly
validated `org.sightlint.web@0.3.0` and managed-loopback `org.sightlint.web@0.4.0` inputs:

- `web.accessibility.interactive-name@0.1.0` for visible DOM-interactive nodes with an observed
  platform role in a conservative UI-control set;
- `web.interaction.center-hit@0.1.0` for one exact render-box-center sample on a conservative
  native-control subset;
- `web.interaction.ancestor-clip@0.1.0` for rectangular clipping of native controls by
  non-scrollable ancestors.

All three are `advisory` maturity and enforcement. They preserve `cantTell` for incomplete
accessibility data, intentional/source-observed dialog overlays, transformed controls, and
scrollable clipping. They do not convert generic overflow, screenshot heuristics, or repeated
geometry into verdicts. `--profile base` is the explicit opt-out; recognized Web extensions are
still validated before base rules run.

ADR 0048 changes acquisition provenance and source attribution only. It does not change any rule
identifier, version, applicability, policy, maturity, outcome, or enforcement behavior.

ADR 0051/#72 exercise `web.accessibility.interactive-name@0.1.0` against a second public fixture
family, including a valid `aria-labelledby` hard negative and an ambiguous focusable surface. This
adds cross-family regression evidence but no independent review or operational holdout, so it does
not change the rule's version, policy, applicability, advisory maturity, or enforcement.

These rules consume declared/evidence-backed inputs. Current `inspect-image` observations do not
become trusted semantic peers or blocking spacing failures.

ADR 0047 adds two medium-neutral interaction rules to `sightlint:base`:

- `interaction.async-feedback@0.1.0` applies only to an action whose declared effect latency is
  observable and requires pending or optimistic native-state evidence after activation and before
  controlled resolution;
- `interaction.failure-recovery@0.1.0` applies to an exercised declared failure path and accepts
  any declared retry or save-draft alternative that is offered, activated, resolves successfully,
  and reaches visible success.

Both are advisory maturity and enforcement. Missing trace execution is `untested`, immediate
completion or an unexercised failure path is `inapplicable`, and retained native/instrumentation
conflict is `cantTell`. App-declared resolution does not become proof of an invisible real-world
effect, and screenshot pixels alone cannot satisfy either rule.

## Further recommended candidates

The first admitted slice is intentionally narrow. Further candidates remain evidence-gated:

- out-of-viewport, clipping, and overflow;
- overlap/occlusion with native/render evidence;
- repeated peers with spacing, alignment, or extent outliers;
- text overflow/truncation;
- exact text-size and hit-target policies under a named platform/profile;
- responsive loss across declared viewports;
- missing/hidden focus indication when a controlled trace proves focus.

Contrast requires foreground/background/compositing/color-management evidence. Broad hierarchy,
density, aesthetics, or “AI-looking” judgments remain advisory until narrow obligations and real
evaluation exist.

## Rule admission checklist

A rule cannot enter the recommended profile until it has:

- stable versioned semantics;
- exact applicability and required aspects;
- named policy source and override model;
- explicit units/tolerance/rounding;
- valid alternatives and scoped exceptions;
- pass, targeted fail/mutation, `cantTell`, inapplicable, and `untested` fixtures where meaningful;
- malformed/boundary/resource/determinism coverage;
- hard negatives;
- public-binary/process E2E;
- realistic acquisition and rule evaluation under `docs/evaluation-strategy.md`;
- per-rule precision, coverage, abstention, and mutation evidence;
- explanation, target navigation, and remediation contract;
- a declared maturity decision and compatibility policy.

A new rule may ship experimental/advisory earlier, but documentation must not imply default
blocking maturity.

## Explanation and remediation

Every result should expose:

- rule ID/version and maturity;
- target IDs/selectors and evidence links;
- observed values, units, and source;
- applicability reason;
- selected policy, precedence, tolerance, and alternatives;
- conflicting/missing evidence;
- outcome and blocking policy;
- severity inputs;
- narrow remediation choices and known tradeoffs.

Auto-fix is separate from diagnosis. A coding agent may propose a change, but the same rule and
relevant regression suite must be rerun. Do not let a model edit an artifact and declare its own
success.

## Testing

Every rule receives the applicable test matrix:

- passing and targeted mutation/failure;
- `cantTell`, inapplicable, and `untested`;
- malformed, boundary, resource, direction, scale, and ordering;
- valid alternative solutions and hard negatives;
- evidence/conflict permutations;
- canonical repeated-byte output;
- public-binary/process E2E;
- separate acquisition and semantic rule evaluation;
- frozen holdout before strong maturity/accuracy claims.

Oracle changes require a semantic reason. Never regenerate expected rule outcomes from the engine
being evaluated.
