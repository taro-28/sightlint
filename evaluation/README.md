# Product evaluation corpus

SightLint has two separate verification responsibilities:

1. **Conformance:** does the implementation obey its declared schemas, rule contracts, safety
   limits, reports, and exit codes?
2. **Product quality:** does the observable tool behavior match the outcomes a reviewed oracle
   says users should receive?

`fixtures/e2e/` answers the first question. This directory answers the second. Passing one does
not imply passing the other.

## Current scope

The version `0.1.0` corpus is a required synthetic smoke evaluation. It reuses reviewed Artifact
IR fixtures to establish the evaluation harness before real screenshots and native artifacts are
introduced. It proves that:

- clean cases retain their intended passing outcomes;
- ambiguous evidence produces `cantTell` rather than a guessed verdict;
- inapplicable rules remain explicitly inapplicable;
- each targeted mutation changes one intended quality property and is killed by its named rule;
- the public `sightlint` binary produces byte-identical reports across repeated runs;
- Web, mobile, slide, document, PDF, image, and other artifact kinds share the same evaluation
  contract.

The initial required inventory contains:

- 20 smoke cases;
- 8 clean-baseline-to-mutant relations;
- all 7 currently modeled static artifact kinds;
- 2 independent public-binary executions per case in each workspace test run.

These counts are descriptive rather than a quality score. Removing coverage requires an explicit
manifest and documentation review; adding many near-duplicate cases must not be used to inflate
apparent precision or recall.

`evaluation/web/` is a separate, independently versioned foundation for issue #22. It adds a
repository-owned dashboard fixture, separate acquisition and rule annotations, an explicit
holdout policy, one targeted mutation, one intentional-grouping hard negative, and three runnable
public-binary declared-IR smoke cases. The separate 23-case browser companion now exercises the
issue #23 adapter and issue #24 advisory recommended Web rules through the actual Node process and
built Rust binary. The original six projections retain their explicit acquisition abstentions;
browser output is not copied into those oracles. Neither corpus establishes representative
adapter accuracy or real-world UI/UX precision.

This bootstrap corpus does **not** prove real-world precision or UX value. Those claims require
human-reviewed native artifacts and rule-specific validation.

## Files

- `corpus.json` is the versioned manifest consumed by the public-binary evaluation test.
- `corpus.schema.json` is the language-neutral schema for the manifest.
- `crates/sightlint-cli/tests/evaluation_corpus.rs` validates the manifest, runs the real CLI,
  checks declared outcomes, and verifies mutation pairs.

The `0.1.0` manifest accepts only repository-relative Artifact IR inputs. Native PNG, browser,
slide, PDF, and mobile inputs will be added through a future schema version after their oracle
and licensing requirements are defined.

## Splits

Cases declare one of three splits:

- **smoke:** small, deterministic, required on every pull request;
- **development:** reviewed cases available for rule development and per-rule diagnostics;
- **holdout:** frozen cases that must not be used to tune a rule before a release evaluation.

The current manifest contains only `smoke` cases. Adding a development or holdout split requires
documenting how leakage is prevented and how its results are reported.

## Oracles and provenance

Every case references a declared source containing its origin, license status, and review status.
Synthetic mutation cases must identify both:

- the clean baseline case;
- the single target rule expected to change from `passed` to `failed`.

Future human-reviewed sources must define annotation guidance, reviewer qualifications, conflict
resolution, and inter-reviewer agreement. External screenshots or documents must not be committed
without permission compatible with repository distribution. Private data, credentials, personal
information, and customer content must be removed before inclusion.

## Gates and metrics

The required smoke gate currently demands:

- all declared expectations match;
- all targeted mutations are killed;
- every case is deterministic for the configured number of runs;
- no undeclared `failed`, `cantTell`, or `untested` outcomes occur when the case forbids them.

As real reviewed cases are added, metrics will be reported per rule and split: precision, recall,
false-positive rate, abstention rate, measured coverage, mutation kill rate, and reviewer
agreement. SightLint must not collapse these into a universal UX or design score.

## Adding a case

1. Add or generate the valid input in the appropriate fixture or evaluation-data directory.
2. Record source, license, review status, medium, split, and expected rule outcomes.
3. For a synthetic defect, add a clean baseline and one targeted mutation relation.
4. Keep case IDs unique and lexicographically sorted.
5. Run:

   ```bash
   cargo test --locked -p sightlint-cli --test evaluation_corpus
   ```

6. Inspect the semantic change rather than accepting an oracle update merely because behavior
   changed.

A rule or adapter change that affects a claimed product capability must update this corpus when
an applicable case exists. Contract E2E alone is not evidence that the product still behaves as
intended.
