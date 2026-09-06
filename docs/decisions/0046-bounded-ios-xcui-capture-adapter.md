# ADR 0046 — Add a bounded iOS source/XCUI capture process adapter

- Status: Accepted
- Date: 2026-09-06
- Issue: #60
- Parent: #29
- Builds on: ADRs 0003, 0006, 0013–0019, 0024 (product evaluation), 0033, 0034,
  and 0042–0045

## Context

Issue #29 sequences iOS after the bounded PPTX, PDF, and Android slices. iOS exposes several
different views of one screen, but none can safely stand in for all the others:

- UIKit layout APIs can report a fixture View's allocation in window points, but not its visible
  ink or full physical activation region;
- XCUITest can report accessibility element identifiers, types, labels, values, frames, and
  selected/enabled/hittable state, but an accessibility frame is not necessarily the UIKit source
  frame, activation point, or rendered bounds;
- a simulator screenshot proves encoded pixels at one instant, but not which pixels belong to an
  accessibility or UIKit object;
- SwiftUI, custom accessibility containers, multiple scenes/windows, system overlays, keyboard,
  Dynamic Type, and localization can change these relationships.

Using XCUITest alone would lose exact source allocation needed by the existing shared containment
rule. Using only in-app UIKit introspection would omit the user-facing accessibility projection.
Promoting either projection to hit or ink geometry would violate ADR 0015. A screenshot-only path
would discard the native structure that motivates an iOS adapter.

The first slice therefore needs paired authentic source and XCUI observations from a
repository-owned fixture, while remaining a deterministic file conversion on all supported host
systems. It must not claim general application capture, device orchestration, or representative
iOS/UI accuracy.

## Decision

Add `adapters/ios/` as a local Python 3.9+ process adapter with protocol and
`org.sightlint.ios` extension version `0.1.0`. It uses only the Python standard library. It reads a
caller-selected canonical capture manifest and paired PNG beneath an explicit repository root. It
does not invoke `xcodebuild`, `simctl`, boot a simulator, install/launch an app, execute an XCUI
action, or parse an `.xcresult` bundle.

Add a repository-owned, programmatic UIKit Atlas settings fixture and an XCUITest capture target
under `fixtures/ios/atlas-app/`. The explicit maintainer capture path:

1. builds and launches one pinned simulator/test configuration;
2. makes the app write source-view facts after layout stabilization;
3. has XCUITest attach a screenshot, then query named accessibility elements and attach its
   independent observation;
4. exports the attachments and source observation on the host;
5. canonicalizes one combined capture manifest and paired PNG per scenario;
6. verifies source and capture digests before committed artifacts are reviewed.

Build products, DerivedData, result bundles, simulator images, signing material, and runtime
containers are not committed. Cross-platform CI verifies the committed source/capture relation and
adapter behavior without booting a simulator. Simulator recapture is an explicit macOS maintainer
operation because GitHub runner images and Xcode runtimes are not a stable evidence source.

The public conversion path is:

```text
repository-owned iOS fixture source
  -> explicit capture on pinned Xcode/iOS simulator profile
  -> canonical source + XCUI facts and paired screenshot with digests
  -> bounded local iOS file adapter
  -> exact source layout facts + separate platform accessibility facts
  -> public sightlint adapt-image for screenshot validation
  -> Artifact IR 0.1.0 + org.sightlint.ios 0.1.0
  -> public sightlint normalize
  -> caller may run public sightlint check --profile base
```

The adapter exits `0` for successful explicitly partial acquisition and `2` for usage, path,
schema, digest, resource, compatibility, output, or Rust-validation failures. It never exits `1`;
rule and process-gate behavior remains owned by the Rust binary.

## Selected acquisition model

Protocol v0 selects paired app-instrumented UIKit and XCUITest observations.

For each admitted UIKit View, the fixture records:

- a unique nonempty `accessibilityIdentifier` selected by fixture source, used as stable identity;
- its UIKit class name and admitted parent identifier;
- `bounds` converted to the key window's top-left screen-point coordinate space after layout;
- `frame`, `bounds`, center, transform identity, hidden, alpha, user-interaction, enabled, selected,
  and window-attachment state where the concrete class exposes them;
- window intersection and safe-area intersection as separate source facts;
- UTF-8 byte counts and SHA-256 digests for admitted labels/values without writing plaintext.

For the same named identifiers, XCUITest independently records:

- element type, identifier, existence, enabled, selected, hittable, and focus state when the public
  API supports it;
- accessibility frame in screen points;
- UTF-8 byte counts and SHA-256 digests for label, value, title, and placeholder where nonempty;
- the query selector and capture status.

The XCUITest list is not forced to match the UIKit list. Missing, merged, split, duplicated, or
conflicting projections remain explicit extension coverage/conflict facts. Traversal index never
becomes stable identity.

The screenshot is taken after the source hierarchy is published but before individual XCUI element
queries. Querying an accessibility element outside a scroll viewport can change that viewport as a
platform side effect. Preserving that side effect in the screenshot would make the acquisition
fixture depend on observer behavior rather than the ready source state. The capture remains
non-atomic, and later XCUI facts may describe platform state after such a side effect; the extension
therefore preserves capture order and never treats source, screenshot, and XCUI observations as one
simultaneous exact fact.

Only a UIKit source View with a unique identifier, finite nonnegative size, identity transform,
window attachment, non-hidden state, positive alpha, and nonempty window intersection may become a
core node. The full source allocation, not the clipped intersection, becomes `layoutBox`, enabling
the existing canvas-containment rule to detect a partly offscreen fixture mutation. Offscreen or
unsupported source Views remain extension-only. XCUITest frames never repair or replace source
geometry.

## Coordinate spaces and reconciliation

Each capture has two independent canvases:

- `ios:screen:points` uses core `point` units, top-left origin, right/down axes, and exact source
  screen bounds;
- `ios:screen:pixels` uses core `devicePixel` units and the paired PNG extent.

The capture records display scale as an exact finite positive ratio from the pinned simulator. The
adapter accepts only integral screenshot dimensions equal to point extent multiplied by scale. It
records this as `extentAndScaleAgree`; it does not transform node geometry into pixels or infer
node-to-pixel identity. Orientation and safe-area insets remain explicit point-valued extension
facts. A later rule may consume them only after a separate policy/evaluation decision.

UIKit source geometry uses `exactSource` evidence. XCUITest data uses `platformSemantics` evidence.
The screenshot canvas uses `exactRender` evidence from public `adapt-image`. A source/XCUI frame
disagreement is retained as `conflict`, not averaged or normalized away.

## Core and extension mapping

Admitted UIKit source Views map conservatively:

- enabled `UIButton`/`UISwitch` instances become core `control` nodes;
- noninteractive `UILabel` instances become `text` nodes;
- admitted `UIStackView`, `UIScrollView`, and ordinary container Views become `container` nodes;
- other admitted Views become `other` nodes;
- every mapped node carries only the exact-source `layoutBox`;
- no `hitBox`, `renderBox`, `inkBox`, core name, inferred role, reading order, or relation is
  emitted.

The versioned iOS extension preserves fixture/capture/tool provenance, source hierarchy and raw
frames, XCUI observations, label/value digests, safe area, scale reconciliation, mapping status,
unsupported counts/reasons, and per-fact evidence identifiers. The medium-neutral core gains no
iOS-only field.

The first shared rule claim is limited to `visual.bounds.within-canvas` over admitted source
`layoutBox` facts. A clean case passes, one targeted source-layout mutation fails for exactly its
named control, and an offscreen-scroll or source/XCUI conflict hard negative remains excluded or
abstaining as annotated. Accessibility `hittable` is a boolean platform observation, not an exact
hit region.

## Protocol, validation, and resources

Request, response, capture, and extension schemas are strict, separately versioned `0.1.0`
contracts. The request names capture and screenshot references plus SHA-256 digests, exact fixture
and tool profile, and budgets. The response reports versions, outcome, coverage, source/screenshot
digests, output reference/digest, normalization command, and limitations without embedding the IR.

The adapter:

- rejects duplicate JSON keys, excessive JSON nesting, unknown fields, non-UTF-8 input, nonfinite
  or malformed rectangles, duplicate identifiers/selectors, parent cycles, and unsupported
  transforms;
- resolves lexical and canonical paths under the repository root and rejects symlink escape;
- validates all input digests before parsing referenced content;
- enforces capture, screenshot, node, depth, attribute, string, and output-byte limits;
- requires the pinned Xcode/runtime/device/capture profile and exact point/pixel reconciliation;
- refuses an existing output path and removes no caller-owned file;
- uses argument arrays, not constructed shell strings, for public SightLint commands;
- emits stable LF diagnostics and leaves no partial output on failure.

## Evaluation and data governance

Add a three-case public regression corpus:

- `smoke`: clean Atlas settings state;
- `development`: one source-only off-canvas control mutation;
- `challenge`: valid scroll/safe-area or source/XCUI conflict hard negative.

Acquisition annotations and rule annotations are separate reviewed files. Neither adapter output nor
CheckReport output is oracle data. Annotation aspects explicitly distinguish exact, `cantTell`, and
`untested` coverage for source hierarchy/classes/states/geometry, accessibility projection,
labels/values, hit/activation geometry, screenshot identity, clipping/occlusion, safe areas,
SwiftUI, focus navigation, and dynamic behavior.

Metrics include acquisition fact coverage, evaluated-case coverage, verdict precision,
false-positive rate, abstention retention, and mutation kill rate where applicable. All source,
captures, labels, and splits are visible to implementers. The corpus is maintainer-authored and has
no protected holdout or independent reviewer. Perfect regression metrics must not be reported as
representative iOS, accessibility, or UI/UX accuracy.

Fixture source, screenshots, capture manifests, and annotations are repository-owned and released
under the repository's `MIT OR Apache-2.0` terms. Fixture data is fictional and contains no customer
or personal data. Full UI strings are not serialized into the adapter IR/response; identifiers and
screenshots are still classified as potentially sensitive. The adapter has no network path and
never transmits artifact content.

## Compatibility

Protocol v0 pins the exact maintainer capture environment recorded in the corpus, initially Xcode
26.3 build 17C529 with iOS Simulator 26.3.1 build 23D8133. The concrete device type, architecture,
screen extent/scale, app/test bundle identifiers, capture runner version, Swift version, locale,
content-size category, and orientation are fixed by the generated manifests after authentic
capture. A changed Xcode/runtime/device/profile requires a new compatibility row, reviewed
recapture, or protocol version; the adapter does not silently accept nearby versions.

Artifact IR and report versions remain unchanged. The iOS extension is optional, so existing
normalization preserves it and existing consumers may ignore it. No existing profile, severity,
enforcement, or exit-code semantics change.

## Alternatives considered

### XCUITest-only capture

Rejected for the first shared-rule slice. It supplies the platform accessibility projection but
does not prove source layout allocation, rendered ink, or exact activation geometry. Treating its
frame as core layout/hit geometry would overclaim evidence.

### UIKit-only in-app instrumentation

Rejected as the complete acquisition source. It can establish fixture source allocation but does
not demonstrate the accessibility projection that users and automation clients encounter.

### Generic simulator accessibility dump or private APIs

Rejected. There is no stable documented `simctl` hierarchy dump equivalent suitable for a public
contract, and private Accessibility frameworks would create compatibility, review, and signing
risk.

### Live orchestration inside the adapter

Rejected. Xcode, simulator lifecycle, app execution, and XCUITest remain untrusted acquisition and
explicit maintainer tooling. The deterministic cross-platform adapter consumes only bounded files.

### Screenshot-only OCR/CV/VLM

Rejected. Pixels do not establish UIKit/XCUI semantics or exact node identity. Optional perception
remains behind ADR 0042 and cannot promote model output into blocking facts.

## Consequences

Positive:

- #29 gains a fourth structured-medium pattern without changing the Rust kernel;
- source layout, accessibility semantics, and rendered pixels remain independently inspectable;
- the existing containment rule can consume justified iOS source geometry;
- committed captures provide deterministic Linux/macOS/Windows regression without simulator CI;
- privacy, compatibility, and unsupported coverage are explicit.

Costs and limitations:

- authentic recapture requires the pinned macOS/Xcode/simulator environment;
- the corpus covers one UIKit fixture, device/runtime, locale, content size, and orientation;
- XCUITest enumeration and simulator rendering may change across Apple toolchains;
- SwiftUI, custom containers, activation geometry, pixel identity, occlusion, dynamic interaction,
  focus navigation, and production application acquisition remain untested or unsupported;
- there is no protected holdout or representative accuracy estimate.

## Verification

Completion requires:

- deterministic fixture/capture generation and drift checks, including source and tool-profile
  digests;
- strict-schema compilation and malformed/unknown/duplicate/version/path/digest/resource tests;
- clean, targeted mutation, hard negative, and separate acquisition/rule oracle checks;
- public-process native-input-to-IR, `adapt-image`, `normalize`, `check`, output-collision,
  evidence-selector, conflict, privacy, and byte-determinism E2E;
- all existing CLI, PNG, image, Web, perception, PPTX, PDF, Android, release, license, MSRV,
  rustdoc, and Linux/macOS/Windows gates;
- exact final-head and post-merge `main` CI.

## Non-goals

- arbitrary application or production-device orchestration;
- equating accessibility frames with layout, hit, activation, render, or ink bounds;
- OCR/VLM semantics, pixel-only blocking, dynamic interaction traces, focus contracts, automated
  remediation, iOS-specific kernel logic, a universal score, representative iOS accuracy, or a
  protected holdout;
- interaction-contract work from #30 or ecosystem packaging from #31.

## Supersession

This is the first iOS capture contract. Any broader runtime/device/app support, SwiftUI semantics,
live orchestration, or geometry promotion needs new product evidence and a superseding ADR or
explicit compatible extension version. It does not supersede Android/PPTX/PDF/Web decisions.
