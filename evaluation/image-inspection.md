# Image inspection: acquisition evaluation, not a semantic UX score

ADR 0031 introduces a separate advisory observation report. Its executable product oracle is
`fixtures/image-inspection/corpus.json`, run by the real-binary
`crates/sightlint-cli/tests/image_inspection_e2e.rs` test target. Normal CI runs both the corpus
reproducibility check and this E2E explicitly; the three-OS full suite includes it as well.

The existing `evaluation/corpus.json` is intentionally unchanged: it expects deterministic
rule results over Artifact IR. Adding an observation as a made-up rule there would conflate
acquisition with semantic applicability. This document records the parallel acquisition
oracle required by ADR 0031; neither suite replaces the other.

## Claimed capability

Under a uniform opaque perimeter/background hypothesis, acquire four-connected region bounds
and same-size, single-color solid-rectangle repetition candidates. Measure horizontal/vertical
gaps in device pixels. Report unequal gaps as an advisory, never as a proven UI/UX violation.

## Corpus and guardrails

Thirty committed cases comprise 19 observed results, nine explicit unavailable results, and
two malformed inputs. Expected bounds and groups come from independently declared source
shapes, not SightLint output. The PNG fixture encoder uses Python's zlib with fixed-Huffman
strategy; committed bytes and generator output must match. Compression is a transport detail,
not the region/gap oracle. Changes to either must be inspected rather than auto-blessed.

The main product mutation is the preserved PR #20 card pair: gaps [1,1] versus [1,2]. A third
case intentionally reuses the *same* unequal pixels with a possible intentional-grouping
interpretation. Its output must be identical: hidden test labels must not influence inference,
and a measured pattern is not proof of design intent. The original future semantic-spacing
status remains `untested` in the source raster corpus.

Negative controls include a foreign blocker, mismatched sizes/colors, one/two components,
hollow and mixed-color regions, touching and diagonal regions, a uniform image, border noise,
alpha values 0/1/254, and unsupported PNG interpretations. Metamorphic controls translate,
scale, or recolor shapes; multiple rows exercise grouping isolation. Exact pixel/component
budget limits and overflows also have direct raster unit tests.

Public E2E verifies actual bounds and gaps, summaries, evidence references, semantic uncertainty,
API/file/stdin equivalence, human/JSON output, repeated bytes, malformed error behavior, CLI
usage, and unchanged trusted check-image/adapt-image/check behavior. Coverage counts and
required controls cannot silently disappear.

## Not demonstrated

This corpus is synthetic and not human-annotated real-world accuracy evidence. It does not
establish typography, rounded-card detection, reading order, hierarchy, semantic peer groups,
contrast, click targets, or general spacing correctness. Before broad automatic findings,
collect realistic native/screenshot pairs and hard negative examples, reconcile inferred
meaning with source evidence, and evaluate precision and abstention separately. Do not turn
this prototype into a general hand-written vision model or equate green CI with product fit.
