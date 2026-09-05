# Web evaluation annotation guide

- Guide version: `0.1.0`
- Governing decision: ADR 0032
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
rendered rectangle. Until issue #23 captures the page, the following stay `untested`:

- computed layout/render/hit rectangles;
- Accessibility Tree output;
- screenshot and pixel dimensions;
- clipping, occlusion, transforms, and visible ink;
- native/pixel agreement or conflict.

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

An unequal measured gap is a failure only when the compared nodes are confirmed equivalent peers
and the explicit fixture policy requires a uniform gap. Intentional grouping and disputed peer
membership are not failed spacing obligations.

## Mutation policy

A targeted mutation must:

1. reference a clean baseline;
2. change one named property;
3. record properties intended to remain stable;
4. identify the exact target rule;
5. preserve a valid inverse/fix;
6. avoid deriving its expected outcome from SightLint output.

The initial mutation changes only the third card's rendered offset in the reviewed Artifact IR
projection. Content, peer identity, viewport, expected gap, tolerance, and other card geometry stay
fixed.

## Hard negatives

A hard negative intentionally resembles a defect but is valid. The initial hard negative places a
promotion beside two repeated metrics. It is not a third metric peer, so visual asymmetry must not
become a spacing failure.

Future additions should cover mixed variants, masonry, editorial asymmetry, badges, overlays,
sticky elements, charts, photographs, skeletons, and loading/error/empty states.

## Review and oracle changes

- Initial annotations are `maintainerReviewed`; this is not independent dual review.
- Record unresolved ambiguity rather than inventing agreement.
- Never update expected data only because code output changed.
- A correction explains whether the fixture, annotation, policy, or implementation was wrong.
- Review related baseline, mutation, hard-negative, and future holdout effects together.
- Increasing the case count does not by itself strengthen representativeness.

## Holdout

No holdout case exists in version `0.1.0`. Public repository cases are development-visible. Before
using holdout results for a maturity or accuracy claim, freeze data before tuning, restrict access
to labels or use an independent evaluator, record any exposure, and publish split-specific counts
and limitations.
