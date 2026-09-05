# Web evaluation foundation

This directory contains the first repository-owned realistic Web UI evaluation foundation for
SightLint. It is governed by ADR 0032 and is deliberately separate from:

- `fixtures/e2e/`, which proves schema, rule, CLI, and error conformance;
- `evaluation/corpus.json`, which is the existing synthetic Artifact IR product smoke corpus;
- `fixtures/image-inspection/`, which evaluates a narrow advisory pixel-acquisition hypothesis.

## Current scope

Version `0.1.0` contains one fictional dashboard application and six reviewed case records. Three
smoke cases exercise the existing `visual.spacing.peer-consistency@0.1.0` rule through the built
`sightlint` binary:

- a clean repeated-card sequence;
- one targeted gap mutation;
- an intentional grouping hard negative that must not fail the spacing rule.

Three development records preserve the next acquisition questions: ambiguous peer intent, a narrow
viewport, and increased text scale. They are explicitly `untested` until issue #23 provides the
Playwright adapter and synchronized native/pixel capture.

This corpus is realistic in structure, not representative in sampling. It contains one application
family, one language, one theme, and one evaluated rule. It does not establish real-world UI/UX
accuracy or recommended/blocking maturity.

## Files and authority

- `corpus.schema.json`: versioned artifact, provenance, split, environment, and execution contract.
- `annotation.schema.json`: versioned envelope for separate acquisition and rule annotations.
- `corpus.json`: reviewed case inventory and governance metadata.
- `annotations/acquisition.json`: what a future adapter should acquire, including untested aspects.
- `annotations/rules.json`: applicability, policy, expected rule outcomes, and false-positive risks.
- `fixture-app/`: repository-owned HTML, CSS, and JavaScript with no external assets or requests.
- `inputs/`: independently authored Artifact IR projections for currently runnable rule cases.

The annotations are reviewed oracles, not generated snapshots. Do not regenerate them from
SightLint output. A change to an expected observation or outcome requires a semantic explanation,
review-version decision, and review of related baseline, mutation, hard-negative, and split data.

## Acquisition status

Browser acquisition is not implemented in this slice. The corpus intentionally records DOM/
accessibility capture, computed render and hit geometry, screenshot pixels, and native/pixel
reconciliation as `untested`. Source-reviewed selectors, hierarchy, and intended peer membership do
not become exact rendered facts.

Issue #23 will add the isolated Playwright adapter and compare its output with the acquisition
annotations. Missing or conflicting observations must remain `cantTell`/`untested` or conflict
evidence rather than being copied from the rule oracle.

## Data governance

- All visible names and values are fictional.
- The fixture makes no network requests and contains no third-party images, fonts, brands, or code.
- No customer data, credentials, personal data, or private screenshots are allowed.
- `externalProcessing` remains false.
- The repository's license is unresolved; the source record therefore makes no independent OSS
  grant for the fixture.
- Public smoke, development, and challenge cases are visible to implementers and are not holdout.

Before holdout data is used, document its freeze commit, access policy, evaluator, leakage controls,
and oracle-correction process. Public CI must not require private raw artifacts.

## Evaluation command

The new integration target validates the linked documents and invokes the actual binary twice for
every runnable smoke case:

```bash
cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture
```

It reports explicit counts for labeled cases, applicability, covered pass/fail decisions, false
positives, correct abstentions, and mutation kills. Those counts are not a quality score.

