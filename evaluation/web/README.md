# Web evaluation foundation

This directory contains the first repository-owned realistic Web UI evaluation foundation for
SightLint. It is governed by ADR 0032 and is deliberately separate from:

- `fixtures/e2e/`, which proves schema, rule, CLI, and error conformance;
- `evaluation/corpus.json`, which is the existing synthetic Artifact IR product smoke corpus;
- `fixtures/image-inspection/`, which evaluates a narrow advisory pixel-acquisition hypothesis.

## Current scope

The original version `0.1.0` corpus contains one fictional dashboard application and six reviewed
rule-projection records. Three smoke cases exercise the existing
`visual.spacing.peer-consistency@0.1.0` rule through the built `sightlint` binary:

- a clean repeated-card sequence;
- one targeted gap mutation;
- an intentional grouping hard negative that must not fail the spacing rule.

Three development records preserve acquisition questions: ambiguous peer intent, a narrow
viewport, and increased text scale. Their original declared-IR projections remain explicitly
`untested`; they are not rewritten from browser output.

ADR 0033 adds a separate browser-acquisition companion slice with seven reviewed requests. It runs
the repository-owned fixture through the isolated `sightlint-web` process and covers clean,
out-of-document mutation, spacing mutation, intentional-grouping hard negative, ambiguous,
responsive, and 125% text-scale states. `annotations/browser-acquisition.json` records acquisition
truth, while `annotations/browser-rules.json` independently records expected results from the
built Rust binary. Captured Artifact IR and screenshots remain temporary test artifacts.

This corpus is realistic in structure, not representative in sampling. It contains one application
family, one language, one theme, and one evaluated rule. It does not establish real-world UI/UX
accuracy or recommended/blocking maturity.

## Files and authority

- `corpus.schema.json`: versioned artifact, provenance, split, environment, and execution contract.
- `annotation.schema.json`: versioned envelope for separate acquisition and rule annotations.
- `corpus.json`: reviewed case inventory and governance metadata.
- `annotations/acquisition.json`: what a future adapter should acquire, including untested aspects.
- `annotations/rules.json`: applicability, policy, expected rule outcomes, and false-positive risks.
- `browser-acquisition.schema.json` and `annotations/browser-acquisition.json`: reviewed browser
  structure, geometry, reconciliation, mutation, hard-negative, and abstention expectations.
- `browser-rule.schema.json` and `annotations/browser-rules.json`: independent public-binary
  verdict expectations and explicit non-claims.
- `requests/`: versioned deterministic capture requests for the Playwright adapter.
- `fixture-app/`: repository-owned HTML, CSS, and JavaScript with no external assets or requests.
- `inputs/`: independently authored Artifact IR projections for currently runnable rule cases.

Each case's `sourceDigest` is SHA-256 over its sorted `sourceFiles`, with every repository-relative
UTF-8 path followed by a zero byte and then the exact file bytes. The validation script recomputes
this bundle digest; it does not generate annotations or expected outcomes.

The annotations are reviewed oracles, not generated snapshots. Do not regenerate them from
SightLint output. A change to an expected observation or outcome requires a semantic explanation,
review-version decision, and review of related baseline, mutation, hard-negative, and split data.

## Acquisition status

Browser protocol `0.1.0` measures selected DOM/accessibility observations, computed
layout/render geometry, center hit tests, viewport screenshot extent, and bounded native/screenshot
reconciliation for the seven companion cases. Screenshot pixel-content identity remains
`cantTell`, and semantic peer membership remains `untested`/`cantTell`; source-reviewed intent does
not become an inferred fact.

The original six-case declared-IR corpus is retained unchanged except for source-digest drift. It
continues to test the peer-spacing rule independently of acquisition. Missing or conflicting
browser observations remain abstentions or conflict evidence rather than being copied from the
rule oracle.

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

The browser companion uses the actual Node adapter process and built Rust binary:

```bash
npm --prefix adapters/playwright run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:e2e
```

Together they expose explicit cases for pass/fail coverage, false-positive protection,
abstention, and mutation detection. Those small public-fixture counts are not a quality score or a
real-world accuracy estimate.
