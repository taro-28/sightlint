# ADR 0032 — Realistic web evaluation foundation

- Status: Accepted
- Date: 2026-09-05
- Issue: #22

## Context

SightLint has deterministic Artifact IR, rule, PNG-raster, and advisory image-inspection
conformance coverage. Its product corpus still consists of synthetic Artifact IR, however, and
cannot tell a future web adapter whether it acquired the intended DOM objects, accessibility
semantics, peer relations, or rendered geometry. It also cannot distinguish an acquisition error
from a rule-applicability or policy error.

Issue #22 needs a small repository-owned realistic fixture before the Playwright adapter in issue
#23 and the recommended rule pack in issue #24. The first slice must establish reviewable ground
truth without claiming that browser acquisition already exists.

## Decision

Add an independently versioned web-evaluation corpus under `evaluation/web/`. It supplements,
but does not replace or change, the existing Artifact IR product corpus in `evaluation/corpus.json`.

The corpus has three separately reviewed layers:

1. **Artifact case records** identify a repository-owned fixture state, environment, provenance,
   split, mutation or hard-negative relation, and execution availability.
2. **Acquisition annotations** state the native structure and relations a sensor should recover,
   together with explicit unknown, disputed, and untested aspects. They never contain a rule
   verdict.
3. **Rule annotations** state applicability, policy, expected outcome, valid alternatives,
   evidence threshold, false-positive risk, and nonblocking maturity. They do not become native
   observations.

The initial fixture is a realistic but synthetic dashboard authored in this repository. It uses
fictional content and no external assets, requests, customer data, credentials, or personal data.
Because the repository license is unresolved, its source record says that redistribution remains
subject to the repository license decision rather than inventing an open-source grant.

The first smoke cases evaluate the existing `visual.spacing.peer-consistency@0.1.0` rule through
the built `sightlint check --format json` command using independently authored, reviewed Artifact
IR projections. A clean case, one targeted mutation, and an intentional-grouping hard negative are
required. Development cases reserve responsive, text-scale, and ambiguous-applicability states.

The projections and expected outcomes are annotation data. They must not be generated from
SightLint output or rewritten merely because implementation output changes. Oracle changes require
a semantic rationale and annotation-version review.

## Acquisition boundary

This slice does **not** implement browser capture. Screenshot, DOM/accessibility snapshot,
computed geometry, and native/pixel reconciliation fields are recorded as `untested`, with issue
#23 as the reason. Source-reviewed structure is not relabeled as exact rendered geometry.

Issue #23 may add generated capture artifacts and compare its adapter output with these acquisition
annotations, but it must define a separate accepted ADR for its process protocol, browser/runtime
pinning, coordinate transforms, synchronization, resource limits, network policy, privacy, and
compatibility. It must preserve disagreements as conflict evidence.

## Splits and holdout

- `smoke` is small, public, deterministic, and required on every pull request.
- `development` is public and may be used to design acquisition and rules.
- `holdout` is reserved but contains no initial cases.
- `challenge` contains public hard negatives and is never described as secret holdout data.

Public repository data cannot provide a secret holdout. Before a holdout is used for a maturity or
accuracy claim, a later decision must record its freeze commit, access policy, evaluator,
authorized oracle-correction process, and leakage controls. Private raw holdout data must not be
required for ordinary public CI.

## Metrics and gates

The first E2E reports counts with explicit denominators for labeled cases, applicable cases,
covered pass/fail decisions, false positives, correct abstentions, and mutation kills. It does not
combine them into a quality score. The smoke gate requires:

- byte-identical stdout, stderr, and exit code across repeated public-binary runs;
- the reviewed expected rule outcome for each runnable case;
- the clean baseline to pass and the targeted mutation to fail the named rule;
- zero blocking failure for the intentional-grouping hard negative;
- explicit `untested` status for acquisition that has not run.

These small repository-owned cases are development evidence and regression protection. They are
not evidence of real-world UI/UX precision, recall, or generalization.

## Compatibility

The web corpus and annotation documents start at schema version `0.1.0`. They are a distinct
evaluation compatibility surface. This decision does not change Artifact IR `0.1.0`, the visual
extension `0.1.0`, CheckReport `0.2.0`, rule semantics, or CLI exit codes.

Future incompatible annotation changes require a new schema version and migration or coexistence
plan. Existing corpus documents remain reviewable instead of being silently reinterpreted.

## Alternatives considered

### Implement the Playwright adapter in the same pull request

Rejected for this slice. It would combine ground-truth design with the acquisition implementation
being evaluated and make issue #22 too large. Issue #23 consumes this foundation.

### Derive expected annotations from adapter or rule output

Rejected because it makes the implementation its own oracle.

### Put acquisition and rule ground truth in one undifferentiated snapshot

Rejected because a correct measurement does not prove semantic applicability or a defect.

### Treat public development examples as holdout data

Rejected because visible artifacts and labels are available for tuning.

## Verification

- JSON schemas and integration tests reject unsupported versions, unknown fields, duplicate IDs,
  dangling references, repository escapes, and inconsistent mutation or hard-negative records.
- The built public binary is run at least twice for every runnable smoke case.
- The clean, mutation, hard-negative, and explicit untested acquisition expectations are asserted.
- Existing conformance, PNG, image-inspection, and product-evaluation suites remain unchanged and
  green on Linux, macOS, Windows, and Rust 1.85.0.

