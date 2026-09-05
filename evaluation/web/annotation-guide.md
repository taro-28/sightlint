# Web evaluation annotation guide

- Guide version: `0.2.0`
- Governing decisions: ADR 0032; browser companion and recommended-profile evolution in ADRs
  0033–0035
- Initial reviewer qualification: repository maintainer product review

## Purpose

Annotate what an acquisition system should observe separately from what a rule should conclude.
The same reviewer may author both documents in this first slice, but each assertion must appear in
the correct layer and carry its own rationale.

## Acquisition annotations

Acquisition annotations may describe source-reviewed structure:

- stable node identity and source selector;
- broad node kind and semantic role;
- parent/child hierarchy;
- candidate or confirmed peer membership and sequence axis;
- explicitly unknown, disputed, unavailable, or untested aspects.

They must not contain `passed` or `failed`. A source selector or CSS declaration is not proof of a
rendered rectangle. Until a browser protocol captures the page, the following stay `untested`:

- computed layout/render rectangles and full hit regions;
- Accessibility Tree output;
- screenshot and pixel dimensions;
- clipping, occlusion, transforms, and visible ink;
- native/pixel agreement or conflict.

The ADR 0033/0034 companion oracle may label those browser observations only when adapter E2E
captures them in one synchronized run. It uses tolerances for authored CSS relationships rather
than snapshot-blessing exact platform coordinates. It must keep semantic peer membership and
pixel-content identity as `untested` or `cantTell` when the current protocol does not observe them.
Browser acquisition oracle `0.3.0` may separately assert client/scroll overflow, rectangular
ancestor clipping, computed writing mode, and render-box-center hit samples. It must keep complete
hit regions `cantTell`; a center sample is never annotated as an exact hit rectangle. Version
`0.3.0` also records the accessibility-name mutation as an acquisition expectation without
placing a WCAG or product verdict in the acquisition document.

Browser acquisition annotations and browser rule annotations are separate documents. A measured
layout/render transform is acquisition truth. Whether an existing deterministic rule can act on
that measurement is rule truth. The implementation's emitted IR, screenshot, or CheckReport must
not be copied into either document as the expected answer.

A disputed relation lists plausible alternatives. Do not force one peer group only to make an
existing rule executable.

## Rule annotations

Rule annotations state:

- stable rule identifier and version;
- target relation or a null target when the rule is inapplicable/ambiguous;
- applicability and rationale;
- policy identity, source, expected value, unit, and tolerance where applicable;
- expected outcome from the five SightLint outcomes;
- minimum evidence required for a trusted result;
- valid alternatives;
- structured severity inputs;
- maturity and blocking status;
- likely false-positive risk;
- reviewer status and rationale.

Browser rule oracle `0.2.0` additionally defines each admitted recommended rule once, including
its profile, policy source, required evidence, applicability, valid alternatives, false-positive
and false-negative risks, qualitative severity inputs, maturity, enforcement, and named
pass/fail/abstention/hard-negative cases. Expected results repeat that policy identity so the E2E
can prove the report did not silently select a different authority. A `failed` advisory result
remains distinct from a blocking failure and does not by itself require exit code 1. No severity
label is inferred from those inputs in this slice.

An unequal measured gap is a failure only when the compared nodes are confirmed equivalent peers
and the explicit fixture policy requires a uniform gap. Intentional grouping and disputed peer
membership are not failed spacing obligations.

## Mutation policy

A targeted mutation must:

1. reference a clean baseline;
2. change one named property;
3. record properties intended to remain stable;
4. identify the acquisition evidence expected to change;
5. identify the exact target rule when the mutation is rule-eligible;
6. preserve a valid inverse/fix;
7. avoid deriving its expected outcome from SightLint output.

The browser companion's targeted mutations each reference a clean browser request and name one
source change. Their `evidenceExpectations` identify layout/render conflict, offset, clipping,
overflow, center hit, peer-dimension, or accessibility-name evidence that must expose the change.
Acquisition-only mutations may remain rule-inapplicable; a test must not invent semantic relations
simply to raise a mutation-kill count.

## Hard negatives

A hard negative intentionally resembles a defect but is valid. The initial hard negative places a
promotion beside two repeated metrics. It is not a third metric peer, so visual asymmetry must not
become a spacing failure.

Current browser hard negatives cover intentional metric grouping, a source-declared dialog
overlay, and a partially visible control inside an explicitly scrollable region. Future additions
should cover mixed variants, masonry, editorial asymmetry, badges, sticky elements, charts,
photographs, skeletons, and loading/error/empty states.

## Review and oracle changes

- Initial annotations are `maintainerReviewed`; this is not independent dual review.
- Record unresolved ambiguity rather than inventing agreement.
- Never update expected data only because code output changed.
- A correction explains whether the fixture, annotation, policy, or implementation was wrong.
- Review related baseline, mutation, hard-negative, and future holdout effects together.
- Increasing the case count does not by itself strengthen representativeness.

## Holdout

No holdout case exists. Public repository cases are development-visible. Before
using holdout results for a maturity or accuracy claim, freeze data before tuning, restrict access
to labels or use an independent evaluator, record any exposure, and publish split-specific counts
and limitations.
