# Architecture

## Overview

SightLint uses ports and adapters around a deterministic Rust kernel. Language and process
boundaries are intentional trust boundaries.

```text
                         artifact sources
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
       pixels            native structure      interaction traces
         │                     │                     │
         └─────────────────────┼─────────────────────┘
                               ▼
                     untrusted adapters/workers
           exact extraction / browser / parser / OCR / CV / VLM
                               │
                               ▼
                 observations + provenance + uncertainty
                               │
                               ▼
               validation, normalization, reconciliation
                               │
                               ▼
                         Artifact IR
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
             geometry       queries       policy/baselines
                └──────────────┼──────────────┘
                               ▼
                     deterministic rules
                               │
                               ▼
                   evidence-linked reports
                               │
        ┌──────────────────────┼────────────────────────┐
        ▼                      ▼                        ▼
       CLI                 CI / GitHub            agent / local UI
```

Acquisition, semantic applicability, policy selection, and deterministic judgment are separate
stages. An adapter may provide an exact rectangle and an inferred role with different evidence
grades; the kernel must not collapse them.

## Trusted kernel

The trusted kernel is Rust code responsible for:

- validating and versioning Artifact IR;
- preserving evidence, selectors, confidence, uncertainty, and conflicts;
- explicit units and coordinate spaces;
- deterministic spatial, structural, and future temporal queries;
- policy/baseline resolution from declared inputs;
- atomic and composite rule execution;
- `passed`, `failed`, `inapplicable`, `cantTell`, and `untested` outcomes;
- result, evidence, coverage, compatibility, and exit-policy models;
- canonical ordering, serialization, and report generation.

The kernel must not:

- fetch network resources;
- run a browser, mobile device, office renderer, OCR, CV, VLM, or LLM;
- interpret arbitrary artifact formats directly;
- infer domain semantics without recording them as inferred evidence;
- rely on wall-clock time, random seeds, locale defaults, unstable iteration order, or an
  undeclared runtime environment;
- convert model confidence into a trusted outcome or severity;
- use one universal quality score as a release gate.

## Current Rust workspace

The current workspace contains four crates:

- `sightlint-ir`
  - versioned medium-neutral contracts;
  - validation and canonicalization;
  - evidence, selectors, units, geometry, relations, and official extensions.
- `sightlint-engine`
  - deterministic geometry/query behavior;
  - atomic visual rules;
  - report construction and outcome policy.
- `sightlint-adapter-png`
  - bounded untrusted PNG source parsing;
  - complete chunk/CRC validation, inflation, filter reconstruction, supported raster expansion;
  - advisory image-region inspection outside the trusted rule verdict path.
- `sightlint-cli`
  - local command entry point;
  - input limits, human/canonical JSON output, and stable exit semantics;
  - composition of adapters and engine without moving adapter authority into the kernel.

The dependency direction remains intentionally narrow. The adapter may depend on IR to emit
observations, and the CLI composes adapter and engine. The engine must not depend on a medium-
specific adapter.

Future crates require actual ownership/code pressure. Likely boundaries include adapter protocol,
process runner, rule packs, or reporting, but do not create empty architectural crates in advance.

## Adapters

Adapters acquire observations and emit validated IR fragments or a separately versioned advisory
observation contract.

The best language may differ by platform:

- Rust for bounded native file parsing and deterministic image primitives;
- TypeScript/Node for Playwright/browser automation;
- Kotlin for Android semantics and UI automation;
- Swift for iOS accessibility and UI automation;
- Python for OCR, CV, and model experiments.

Early development prefers versioned process protocols to a shared in-process plugin ABI. Process
isolation limits crashes, memory, dependency conflicts, runtime choice, and untrusted content.

Every adapter declares:

- adapter/protocol/runtime/version;
- input identity/digest and source selector;
- exact versus inferred observations;
- units, coordinate spaces, transforms, scale, and rounding;
- stable IDs independent of traversal order;
- unsupported/partial/ambiguous states;
- privacy, network, and external-processing status;
- time, memory, object, frame/page, and output limits;
- compatibility environment and deterministic capture/preprocessing parameters;
- native-input-to-IR or advisory-report E2E.

Medium-specific fields live in versioned namespaced extensions. A field does not enter core IR
only because one platform exposes it.

## Current PNG adapter boundary

The current PNG path is a deliberately narrow, verified acquisition slice:

```text
source bytes
  -> signature/IHDR and bounded complete chunk validation
  -> bounded IDAT zlib/DEFLATE inflation
  -> standard scanline-filter reconstruction
  -> non-interlaced/Adam7 pass mapping
  -> row-major PNG-encoded RGBA8 for supported eight-bit formats
```

The resulting bytes are source code values, not color-managed display values. Raw pixels stay
inside the adapter API. The IR receives source/raster availability, bounded metadata, checksum,
and exact-source evidence.

`inspect-image` consumes those pixels through a separate advisory contract. It can hypothesize one
uniform opaque perimeter color and measure simple regions/gaps, but it does not create trusted
semantic nodes or blocking rule results. See ADRs 0030 and 0031.

Broader format/segmentation work is evidence-gated by issues #22, #25, #26, and #27. Do not merge
historical branch implementations.

## Perception workers

Perception is optional and untrusted. A worker may detect text, classify a region as a heading,
button, caption, group, or other role, or propose hierarchy and peer relations.

Its output must include, where applicable:

- protocol, worker, model, runtime, and backend versions;
- exact input reference/digest;
- crop/scale/tile and deterministic preprocessing parameters;
- local/remote execution and transmitted-data declaration;
- confidence or calibrated probability when actually available;
- alternatives, uncertainty, and repeated-run agreement;
- geometry and source-region evidence;
- partial, unsupported, timeout, and resource-limit statuses.

Do not fabricate a numeric confidence for a model that does not provide one. Canonicalize output
before it reaches the kernel. A worker supplies observations; the deterministic engine supplies
outcomes. Issue #28 defines the future protocol work.

## Native and pixel reconciliation

Native structure and rendered pixels observe different truths.

Examples:

- CSS declares 14 px text, while a transform renders it near 11.2 px;
- accessibility exposes a button whose visual pixels are clipped or occluded;
- a shape has aligned source bounds while transparent image padding shifts its visible ink;
- OCR finds text without a matching native node;
- a native node exists but no visual region is captured;
- browser geometry and screenshot disagree because of zoom, frame transforms, animation, or
  capture timing.

Do not merge by choosing one source globally. Represent:

- agreement within a declared tolerance;
- expected loss from one source;
- native-only and pixel-only observations;
- transform/capture-state conflicts;
- unresolved conflicts that force `cantTell`.

Issue #23 uses Playwright as the first structured adapter and reconciliation proving ground. ADRs
0033 and 0034 implement its local-fixture process boundary and versioned acquisition evidence
matrix without adding browser dependencies to the Rust kernel. ADR 0035 makes the resulting
`org.sightlint.web@0.3.0` payload an official optional extension: Rust strictly validates its
normalized records and evidence references but never launches Playwright or reads the screenshot.

## Artifact IR boundaries

Core IR represents shared concepts:

- artifact and canvas/page/screen/frame;
- node/object and hierarchy;
- roles, names, states, and relations where evidence exists;
- source/layout, rendered/ink, and hit geometry;
- style, typography, and color observations;
- evidence, selectors, confidence, uncertainty, and conflicts;
- versioned extensions for medium-specific and future interaction data.

It does not make DOM, CSS, DrawingML, PDF operators, Android classes, or XCUI traits mandatory.

Derived geometry such as gap, overlap, center, or containment normally belongs in a query result,
not as duplicated source facts. Semantic peer membership is an observation with provenance, not a
consequence of equal rectangles alone.

## Policy and rule boundary

An observation is not an expectation. The engine resolves policy in this order:

1. explicit project contract/exception;
2. exact design-system or platform contract;
3. inferred project norm with visible confidence;
4. platform convention;
5. conservative built-in baseline.

A rule executes only after its targets, required aspects, applicability, policy, units, and
tolerance are known. Missing meaning produces `cantTell`, `inapplicable`, or `untested` according
to the rule contract.

ADR 0035 implements the first issue #24 profile slice. `sightlint:recommended` is the additive
default, while `--profile base` runs only pre-existing explicit/base rules. Three Web-specific
atomic rules consume validated native structure and browser reconciliation; their policy
provenance and advisory enforcement are serialized independently from outcome and maturity.
Broad aesthetic critique is not a trusted blocking obligation.

## Interaction architecture

Future dynamic analysis adds versioned action/effect/state/trace extensions rather than pretending
a static screenshot proves behavior.

Potential sources include browser/mobile automation, application-declared effects, controlled
network/failure harnesses, and screenshots at named states. The kernel normalizes ordering,
durations, causal IDs, scope, and alternatives before evaluating obligations such as pending
feedback, idempotency, recovery, destructive safeguards, and focus behavior.

Browsers/devices and controlled clocks remain adapter/test-harness responsibilities. Issue #30
contains the interaction roadmap.

## Evaluation architecture

Testing is split into:

1. implementation conformance;
2. acquisition correctness;
3. semantic rule/product evaluation;
4. eventual user-outcome evidence.

Synthetic fixtures establish algorithms, boundaries, mutations, and determinism. They cannot
establish real-world UI/UX accuracy. Realistic reviewed artifacts, hard negatives, metrics, and a
holdout process are required by issue #22 and `docs/evaluation-strategy.md`.

The model under evaluation must never generate its own ground truth. Oracle changes require a
semantic reason, not snapshot blessing.

## Process and release boundaries

- The latest green `main` is the only development base.
- Historical Draft PRs and branch-only ADRs are non-authoritative.
- Normal CI is read-only; self-writing feature workflows are prohibited.
- Public behavior needs real binary/process E2E and exact final-head plus post-merge CI.
- Branch protection and branch cleanup are administrative issues #19 and #32.
- License, compatibility surfaces, packaging, and release are issue #33.
- MCP, GitHub Checks, editor/browser UI, and other surfaces wrap the same kernel and are issue #31;
  they must not duplicate rule semantics.

## Compatibility

Track compatibility independently for:

- Artifact IR schema;
- report schema;
- adapter/perception process protocol;
- official namespaced extensions;
- rule identifiers and semantic versions;
- configuration and recommended profiles;
- CLI commands, stdout/stderr, and exit codes;
- evaluation manifest schemas;
- package/binary releases.

One package version must not conceal incompatible changes across these surfaces. The first release
policy remains unresolved in issue #33.

## Determinism contract

Determinism includes:

- stable node/evidence/result ordering;
- canonical serialization;
- finite numbers, explicit units, comparison tolerance, and rounding;
- locale-independent formatting;
- stable IDs and map/set behavior;
- fixed rule evaluation ordering;
- declared adapter/model/browser/runtime versions;
- no hidden network, time, randomness, or environment dependence;
- repeated-byte tests where canonical output is claimed.

These details are correctness, not implementation trivia.
