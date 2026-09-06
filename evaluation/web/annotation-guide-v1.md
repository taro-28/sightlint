# Web evaluation annotation guide v1

- Guide version: `1.0.0`
- Governing decision: ADR 0051
- Scope: multi-family public Web evaluation and protected-holdout admission

## Authority boundaries

Fixture source defines the state to review. Acquisition annotations describe what the browser or
another sensor should observe. Rule annotations describe applicability, policy, and expected
verdict. The registry joins those authorities but does not copy their facts.

Do not generate an oracle from captured Artifact IR, screenshots, reports, diagnostics, or another
implementation output. A tool may validate structure, references, and digests; it may not decide
the expected observation or verdict. An oracle changes only after a semantic review identifies
whether source, acquisition truth, rule applicability, policy, or implementation was wrong.

## Fixture families

A family represents one coherent product context, visual system, and set of user tasks. A CSS
recolor or route within the same application is not a new family. Record the source root, revision
basis, ownership, license, redistribution, privacy review, external assets and network behavior,
external processing, exposure, tuning visibility, and sampling limitations.

Repository-owned fixtures use fictional content and no customer/personal data, secrets, copied
brands, third-party assets, or undeclared code. Public families are development-visible even when
their case split is named `challenge`; they are not protected holdout.

## Acquisition annotations

Record only observations that the selected acquisition protocol can meaningfully compare:

- stable native identity, hierarchy, role/name/state, and selected source fields;
- layout and render geometry with explicit units, coordinate spaces, and tolerances;
- viewport, device scale, locale, direction, theme, and screenshot extent from the same state;
- clipping, overflow, center-hit samples, and native/pixel reconciliation by their actual method;
- explicit `cantTell` or `untested` for complete hit regions, pixel identity, semantic relations,
  or any unavailable evidence.

Native and pixel disagreement remains conflict evidence. Do not repair one source by copying the
other. Acquisition annotations never contain `passed` or `failed` rule verdicts.

## Rule annotations

Record the stable rule/version, target, applicability, policy source/version/reference, required
evidence, expected outcome, maturity, enforcement, valid alternatives, false-positive and
false-negative risks, and qualitative severity inputs. A measured anomaly is not a defect until
applicability and policy are supported. Keep `passed`, `failed`, `cantTell`, `inapplicable`, and
`untested` distinct.

The first support-inbox slice may evaluate only the existing advisory programmatic-name rule. It
does not establish WCAG conformance, complete accessibility, blocking maturity, or another rule.

## Cases and splits

- `smoke`: public deterministic cases required in ordinary CI;
- `development`: public cases available for design and tuning;
- `challenge`: public hard negatives or unusual valid alternatives;
- protected holdout: external, frozen, access-controlled data admitted only by
  `holdout-admission.schema.json`.

A targeted mutation starts from a valid baseline, changes one named property, lists preserved
properties, names the expected acquisition signal and rule when eligible, and retains a valid
inverse. A hard negative resembles a defect while remaining valid. An ambiguous case names the
missing/conflicting evidence and expects abstention rather than a guessed pass or failure.

## Review record

Record each participant with a stable project identifier, role, qualification category, and
independence from the annotation author. `maintainerOnly` is the honest status when no separate
reviewer exists. `independentlyReviewed` requires an independent reviewer and an agreement result.
`adjudicated` additionally requires an independent adjudicator and the rationale resolving or
preserving disagreement.

An implementation agent that authored fixture or oracle changes is not an independent human
reviewer for those labels. Unresolved disagreements stay explicit and reduce claimable coverage.

## Holdout and leakage

The public repository contains admission metadata only. Raw protected artifacts and labels must be
held by a separately administered authority. Freeze the exact bundle before tuning, bind it by
digest, limit access, name an independent evaluator, log every exposure, pin the evaluation
environment, and version legitimate oracle corrections. If membership, artifacts, labels, or
detailed case results are exposed to tuning, reclassify or replace affected cases rather than
continuing to call them protected.

## Reporting

Report integer numerators and denominators by family, split, rule, and evidence class for case
coverage, acquisition expectation coverage, failure precision, false positives, reviewed
abstention agreement, mutation kill rate, and native/pixel conflicts where available. Do not hide
zero denominators behind a percentage and do not combine independent metrics into a universal UX
score. State sampling, review, exposure, and unsupported-capability limitations beside results.
