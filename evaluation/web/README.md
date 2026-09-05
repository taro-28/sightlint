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

ADRs 0033–0035 add a separate browser-acquisition/rule companion with 23 reviewed requests. It runs
the repository-owned fixture through the isolated `sightlint-web` process and covers clean,
out-of-document, clipping, overflow, occlusion, peer-dimension, transformed-text, control-state,
responsive, RTL/vertical-writing, unnamed/ambiguous controls, scrollable versus non-scrollable
control clipping, and intentional-asymmetry/overlay cases.
`annotations/browser-acquisition.json` records acquisition truth, while
`annotations/browser-rules.json` independently records expected results from the built Rust
binary. Captured Artifact IR and screenshots remain temporary review/test artifacts and are never
copied into either oracle.

This corpus is realistic in structure, not representative in sampling. It contains one application
family, one language, one theme, and a small set of explicitly reviewed rule paths. It does not
establish real-world UI/UX accuracy, WCAG conformance, or blocking maturity.

## Files and authority

- `corpus.schema.json`: versioned artifact, provenance, split, environment, and execution contract.
- `annotation.schema.json`: versioned envelope for separate acquisition and rule annotations.
- `corpus.json`: reviewed case inventory and governance metadata.
- `annotations/acquisition.json`: what a future adapter should acquire, including untested aspects.
- `annotations/rules.json`: applicability, policy, expected rule outcomes, and false-positive risks.
- `browser-acquisition.schema.json` and `annotations/browser-acquisition.json`: reviewed browser
  structure, geometry, reconciliation, mutation, hard-negative, and abstention expectations.
- `browser-acquisition-0.1.schema.json` and `browser-acquisition-0.2.schema.json`: retained strict
  previous schemas for compatibility checks; the current browser acquisition oracle is `0.3.0`.
- `browser-rule.schema.json` and `annotations/browser-rules.json`: independent public-binary
  verdict expectations, rule admission contracts, policy/enforcement provenance, metrics, and
  explicit non-claims. The current rule oracle is `0.2.0`; strict `0.1.0` is retained.
- `agent-workflow.schema.json` and `annotations/agent-workflow.json`: the independent reviewed
  source-navigation, fix/rerun, abstention, hard-negative, governance, and non-claim contract for
  the one-command issue #42 path. It is public smoke data, not generated output or a holdout.
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

Browser protocol `0.1.0` and `org.sightlint.web@0.3.0` measure selected DOM/accessibility
observations, computed layout/render geometry, client/scroll overflow, rectangular ancestor
clipping, center hit samples, viewport screenshot extent, and bounded native/screenshot
reconciliation for the 23 companion cases. Version `0.3.0` adds explicit DOM, render, and optional
accessibility evidence identifiers for trusted-kernel provenance validation. Screenshot
pixel-content identity and complete hit regions remain `cantTell`, and semantic peer membership
remains `untested`/`cantTell`; source-reviewed intent does not become an inferred fact.

The default `sightlint:recommended` profile consumes this extension for three advisory rules:
programmatic names, one exact center-hit sample, and rectangular non-scrollable ancestor clipping.
The oracle preserves pass, targeted failure, `cantTell`, `inapplicable`, and hard-negative
relations for each rule. `--profile base` omits those rules but still validates the recognized
extension. Raw measurements are not copied into verdict ground truth.

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

Together they currently report 23/23 case coverage, 76 reviewed acquisition expectations, 45
reviewed acquisition abstentions, 11/11 acquisition mutations observed, 6/6 rule-eligible
mutations killed, 6/6 matched emitted failures, zero unexpected failures, and zero hard-negative
failures. Each recommended rule records 5/5 contracted outcome-category entries, 1/1 matched
failure, 2/2 reviewed abstentions, 1/1 killed mutation, and zero hard-negative failures. Those
small public-fixture counts are not a quality score or a real-world accuracy estimate.

The agent-workflow E2E additionally reports 1/1 initial named finding, 1/1 source-target join, 1/1
reviewed fix verified, zero new failures, 2/2 repeated JSON and human byte checks, and 2/2 reviewed
`cantTell` controls with zero false-positive failures. The task, edit, and labels are public, so
these are workflow regression counts rather than agent-generalization metrics.
