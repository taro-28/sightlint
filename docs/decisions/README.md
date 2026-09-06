# Architecture decision records

Architecture decision records capture choices that constrain future implementation. The files
listed under **Accepted decisions** are normative because they are present and indexed on current
`main`. A file on an unmerged or closed branch is not accepted repository policy merely because
its own header says `Status: Accepted`.

Read `docs/handoff.md` for the current operational state and `docs/decision-history.md` for the
background, alternatives, and disposition of historical experiments.

## Status meanings

- **Proposed:** under discussion and not normative.
- **Accepted:** normative until superseded; listed in this index on `main`.
- **Superseded:** replaced by a later accepted ADR.
- **Rejected:** considered and not adopted.
- **Historical branch decision:** design reference from an unmerged/superseded branch; never
  normative on current `main`.

## Accepted decisions

- [0001 — Product scope and name](0001-product-scope-and-name.md)
- [0002 — Rust deterministic kernel](0002-rust-deterministic-kernel.md)
- [0003 — Medium-neutral Artifact IR](0003-medium-neutral-artifact-ir.md)
- [0004 — Isolate probabilistic perception](0004-isolate-probabilistic-perception.md)
- [0005 — ACT-inspired rule outcomes](0005-act-inspired-rule-outcomes.md)
- [0006 — Local-first process adapters](0006-local-first-process-adapters.md)
- [0007 — Dual MIT or Apache-2.0 licensing](0007-licensing.md)
- [0008 — Development and release gates](0008-development-and-release-gates.md)
- [0009 — Initial workspace boundaries](0009-initial-workspace-boundaries.md)
- [0010 — Schema and rule compatibility](0010-schema-and-rule-compatibility.md)
- [0011 — Evidence grading and CI policy](0011-evidence-grading-and-ci-policy.md)
- [0012 — Rule authoring lifecycle](0012-rule-authoring-lifecycle.md)
- [0013 — Observation provenance](0013-observation-provenance.md)
- [0014 — Units and coordinate spaces](0014-units-and-coordinate-spaces.md)
- [0015 — Layout, render, and hit bounds](0015-layout-render-and-hit-bounds.md)
- [0016 — Pixels as common observation layer](0016-pixels-as-common-observation-layer.md)
- [0017 — Derived relations, not redundant facts](0017-derived-relations-not-redundant-facts.md)
- [0018 — Fixture-driven binary E2E](0018-fixture-driven-binary-e2e.md)
- [0019 — Version official IR extensions](0019-version-official-ir-extensions.md)
- [0020 — Explicit visual consistency contracts](0020-explicit-visual-consistency-contracts.md)
- [0021 — Deterministic PNG adapter boundary](0021-deterministic-png-adapter.md)
- [0022 — Bounded full-stream PNG chunk validation](0022-bounded-png-chunk-validation.md)
- [0023 — Bounded PNG IDAT inflation](0023-bounded-png-idat-inflate.md)
- [0024 — Separate conformance tests from product evaluation](0024-product-evaluation-corpus.md)
- [0024 — Deterministic PNG filter reconstruction](0024-deterministic-png-filter-reconstruction.md)
- [0030 — Verified staged raster and byte corpus](0030-verified-staged-raster-and-corpus.md)
- [0031 — Advisory image-region inspection](0031-advisory-image-region-inspection.md)
- [0032 — Realistic web evaluation foundation](0032-realistic-web-evaluation-foundation.md)
- [0033 — Playwright web adapter process and capture protocol](0033-playwright-web-adapter-process.md)
- [0034 — Web evidence matrix and extension evolution](0034-web-evidence-matrix-and-extension-evolution.md)
- [0035 — Recommended Web profile and advisory enforcement](0035-recommended-web-profile-and-advisory-enforcement.md)
- [0036 — Local Web check orchestration and agent report](0036-local-web-check-orchestration.md)
- [0037 — Source-first alpha release and compatibility contract](0037-first-alpha-release-contract.md)
- [0038 — Workflow-artifact verification and immutable release-tag recovery](0038-release-artifact-transport-and-tag-recovery.md)
- [0039 — Evaluate broader background hypotheses without changing the strict default](0039-background-segmentation-benchmark.md)

Two different files received number 0024 during the remote-development phase. Their full paths are
stable and both are accepted. Do not silently rename or renumber historical links. New decisions
continue at **0039** or later.

## Historical branch decisions 0025–0029

Draft PRs #13–#17 created ADRs on branches that were never merged into the recovered/current
`main`. Those files are not present in this index and are not normative. Their useful reasoning
has been transferred as follows:

| Historical ADR | Subject | Current disposition |
|---|---|---|
| 0025 | broad PNG-encoded RGBA8 normalization, palette, sub-byte, 16-bit, `tRNS` | optional strategy and requirements in issue #27 |
| 0026 | exact alpha-visible bounds, insets, edge counts, `inkBox` | clean current-main implementation contract in issue #26 |
| 0027 | ranked exact corner/edge background candidates | benchmark candidate in issue #25 |
| 0028 | layered image bytes/current assertions/future ground truth corpus | current raster/inspection corpora plus real evaluation gate #22 |
| 0029 | 95%-qualified border and row-run/union-find components | benchmark candidate in issue #25 |

The corresponding PRs were closed as superseded. Do not copy their `Status: Accepted` header into
current policy, merge their branches wholesale, or reuse their obsolete self-writing workflows.
Start from current green `main`, review the linked issue, and write a new ADR when the evidence is
sufficient to choose an approach.

## Decision-authoring protocol

Before implementing an architectural, schema, protocol, trust-boundary, compatibility,
policy-precedence, resource-model, or public-report change:

1. confirm the current issue and roadmap gate;
2. read `docs/handoff.md`, `docs/product-rationale.md`, and `docs/decision-history.md`;
3. compare relevant alternatives, including doing nothing and using an existing library/process;
4. state evidence, assumptions, uncertainty, privacy, security, resource, compatibility, and
   evaluation implications;
5. define non-goals and a migration/supersession plan;
6. add the ADR to this index in the same PR;
7. implement only after the decision is reviewable;
8. update the handoff and roadmap when the accepted decision changes current work.

Use [the template](template.md) and the next unused number at or after 0040.
