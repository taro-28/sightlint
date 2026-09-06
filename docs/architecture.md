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

ADR 0043 also uses Python 3.9+ standard-library ZIP/XML parsing for the first bounded PPTX slice.
It remains a separate local process: the Rust kernel receives only normalized medium-neutral facts
and never depends on OOXML parsing code.

ADR 0044 uses an exact hash-locked pypdf wheel in a second local Python process for a bounded PDF
page/Link-annotation slice. The parser and object graph remain untrusted; only normalized exact
source facts cross into the medium-neutral kernel.

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

## Current PPTX adapter boundary

The `sightlint-pptx` process validates a strict local request, streams source/render digests within
caller limits, inventories an OOXML archive before bounded XML decompression, maps directly
declared unrotated shapes/groups and group transforms to exact source EMU `layoutBox` geometry,
and passes candidate IR through public `sightlint normalize`. Native IDs, parentage, local z-order,
placeholder metadata, and digest-only source-text metadata live in
`org.sightlint.pptx@0.1.0`.

Optional repository-contained PNG renders are validated by public `sightlint adapt-image` and
remain a separate `devicePixel` canvas with `ExactRender` evidence. The adapter records extent
agreement/conflict but does not manufacture node-to-pixel identity. Master/layout objects,
theme-resolved styles, strict OOXML, rotated/flipped geometry, other DrawingML objects, rendered
ink, and text layout remain partial/unsupported/`cantTell`. The Python ZIP/XML implementation is
an untrusted sensor and is not an operating-system sandbox.

## Current PDF adapter boundary

The `sightlint-pdf` process validates a strict digest-pinned local request, checks the exact pypdf
version, rejects encryption, inventories cross-reference objects, and walks the raw page tree with
cycle and page limits. Only explicit integral unrotated MediaBox/CropBox values and indirect
rectangular internal Link annotations with zero flags and no `QuadPoints`/`Path` become exact
source `pdfPoint` canvases and core `hitBox` nodes. Candidate IR passes public
`sightlint normalize` before it is written.

Optional repository-contained PNG page renders pass public `adapt-image` and remain separate
`devicePixel` canvases. Extent agreement/conflict is retained, while annotation-to-pixel identity
and viewer hit testing remain `cantTell`. Text, tag interpretation, paint/ink, reading order,
forms, actions, attachments, and metadata are not mapped. The pypdf/Python process is an untrusted
sensor with request budgets, not an operating-system sandbox.

## Current PNG adapter boundary

The current PNG path is a deliberately narrow, verified acquisition slice:

```text
source bytes
  -> signature/IHDR and bounded complete chunk validation
  -> bounded IDAT zlib/DEFLATE inflation
  -> standard scanline-filter reconstruction
  -> non-interlaced/Adam7 pass mapping
  -> row-major PNG-encoded RGBA8 for supported eight-bit formats
  -> exact source-alpha geometry for supported rasters
```

The resulting bytes are source code values, not color-managed display values. Raw pixels stay
inside the adapter API. The IR receives source/raster availability, bounded metadata, checksum,
and exact-source evidence. ADR 0040 also records bounds for `alpha > 0` and `alpha == 255`, sample
counts, transparent insets, and edge occupancy. Only nonempty source-visible bounds become an
evidence-linked device-pixel `inkBox`; this is not composited visibility or semantic whitespace.

`inspect-image` consumes those pixels through a separate advisory contract. It can hypothesize one
uniform opaque perimeter color and measure simple regions/gaps, but it does not create trusted
semantic nodes or blocking rule results. See ADRs 0030 and 0031.

ADR 0039 adds a second, evaluation-only report that compares the unchanged strict hypothesis with
ranked exact-border flood fill and a 95%-qualified corner/row-run implementation. Candidate colors
and connected pixels remain untrusted acquisition hypotheses. The report is not Artifact IR or a
CheckReport, contains no executable rule result, and cannot block. Its realistic fixture shows
unsafe ranked selection and shadow-connected false grouping, so no broader policy is admitted.

ADR 0041 records that current repository and pinned-browser product evidence does not establish a
broader-format gap. Indexed, sub-byte, 16-bit, `tRNS`, and animated inputs therefore remain
explicitly unavailable; no decoder dependency or automatic conversion is admitted. A future
observed gap requires a new issue and ADR. Do not merge historical branch implementations.

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
outcomes.

ADR 0042 implements protocol `0.1.0` as a dependency-free local Node wrapper/reference worker.
The strict family records cover region, text, role, hierarchy, and peer candidates, with bounded
references and hierarchy depth. The reference implementation currently supplies only deterministic
exact-color regions from a named image-segmentation benchmark policy. Only model-free
`visionMeasured` regions map to core `other` nodes; inferred families stay in the canonical worker
response and perception extension summary. The public Rust normalizer validates the candidate IR,
and no perception record creates a rule result or blocking authority.

The v0 process limit is not an operating-system sandbox or memory ceiling. It rejects remote
execution and bounds standard-stream bytes, time, observations, text, hierarchy, geometry, and
input size. A third-party worker still requires an independently reviewed sandbox and deployment
policy.

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
- The active `Protect main` ruleset and automatic merged-branch cleanup implement administrative
  issues #19 and #32; exact-head and post-merge verification remain mandatory.
- Accepted ADR 0007, ADR 0037, and ADR 0038 define dual licensing, surface-specific alpha
  compatibility, source-only packaging, read-only prepublication verification, immutable tags,
  and the first release boundary from issues #33/#47.
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

One package version must not conceal incompatible changes across these surfaces. ADR 0037 and
`docs/compatibility.md` define the first alpha policy: each surface retains its own version and a
breaking alpha change requires a surface version change, release note, and migration guidance.

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
