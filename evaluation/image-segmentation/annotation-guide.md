# Image-segmentation benchmark annotation guide

This guide governs the public development corpus accepted by ADR 0039. It separates the question
“did an exact-color policy recover the annotated visible surface?” from “does that surface prove a
UI/UX defect?” The first belongs to `annotations/acquisition.json`; the second belongs to
`annotations/rules.json`.

## Acquisition annotation

Annotate a region only when the repository-owned HTML/CSS identifies a visually continuous
surface with reviewable source bounds. Record the source selector, integer device-pixel rectangle,
and an edge tolerance that accounts for the explicitly present border, rounded antialiasing, and
shadow. Do not copy a component rectangle from SightLint or a browser capture into the oracle.

`backgroundUsability` means only whether one exact encoded-RGB candidate is a defensible geometric
reference for this controlled image:

- `usable`: the source design intentionally provides one global exact canvas/backdrop;
- `unsafe`: the candidate policy returns a measurement, but the source has multiple/continuous
  layers for which its selected exact color is not a global background;
- `notSelected`: the policy must abstain before choosing a candidate.

`observed` and `unavailable` describe acquisition coverage. Neither means `passed` or `failed`.
Resource exhaustion must be annotated as unavailable with its exact stable reason and no partial
region target credit.

## Rule annotation

No executable rule consumes these benchmark components. Set `expectedOutcome` to `untested` and
`blockingAllowed` to false. `applicabilityGroundTruth` is `cantTell` when a human would need native
structure or policy to decide the semantic question; use `inapplicable` for intentional hard
negatives that explicitly reject the proposed semantic interpretation.

Never turn an unequal gap, a recovered rectangle, a dominant edge color, or a large component into
a rule failure in this corpus.

## Matching and metrics

Evaluation performs deterministic one-to-one matching. A predicted region matches a target only
when all four edges are within the target tolerance. Precision is matched predictions divided by
all predictions on cases whose background is annotated `usable`; recall is matched targets divided
by all targets on those cases. Unmatched predictions are fragmentation/false-region evidence.
A prediction intersecting more than one target is recorded as possible false grouping. Edge error
is reported as integer absolute edge deltas for matched pairs.

Unsafe-background observations are reported separately and never counted as useful coverage.
Correct hard-negative abstention means the policy returned unavailable where the acquisition oracle
requires `notSelected`. Metamorphic relations report region-count and source-bound changes without
assuming byte identity across distinct inputs.

Canonical benchmark reports contain no wall-clock time. Tests record deterministic pixels, edge
samples, runs, and components. Any local timing comparison must name hardware/runtime and is a
diagnostic only.

## Governance

The Northstar fixture is fictional, repository-owned, local-only, and uses no external assets.
Source, annotations, schemas, and fixture code are licensed `MIT OR Apache-2.0`. Do not add private,
customer, credential, or personal data, or an external screenshot without a reviewed redistribution
record. Captured PNGs and implementation reports stay temporary.

All labels are public and available during implementation. `smoke`, `development`, and `challenge`
are not holdout data. A holdout requires a recorded freeze, access controls or independent
evaluation, exposure tracking, disagreement handling, and split-specific reporting.
