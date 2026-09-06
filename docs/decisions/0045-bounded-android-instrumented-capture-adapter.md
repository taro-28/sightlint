# ADR 0045 — Add a bounded Android instrumented-capture process adapter

- Status: Accepted
- Date: 2026-09-06
- Issue: #56
- Parent: #29
- Builds on: ADRs 0003, 0006, 0013–0019, 0024 (product evaluation), 0033, 0034,
  and 0042–0044

## Context

Issue #29 requires one structured adapter at a time and names Android after the bounded PPTX and
PDF slices. Android can expose a native View hierarchy, accessibility semantics, screen geometry,
display density, insets, orientation, and rendered pixels. Those sources are complementary but not
interchangeable:

- a View's measured screen allocation is not necessarily its visible ink;
- `AccessibilityNodeInfo` bounds are platform-semantic focus geometry, not proof of the physical
  touch target;
- a screenshot proves encoded pixels for one captured state but not node identity;
- generic UIAutomator XML omits View layout allocation, display/inset metadata, complete action
  semantics, Compose semantics ownership, and capture synchronization guarantees;
- app instrumentation can expose exact View facts, but only for a debuggable/test-controlled app
  and only for the APIs that the capture code actually records.

Promoting every accessibility rectangle to `hitBox`, or every screenshot crop to a native node,
would violate ADR 0015 and create blocking failures from insufficient evidence. A screenshot-only
mobile adapter would also discard the native structure that makes Android more useful than a bare
image.

The first Android slice therefore needs an authentic platform capture without claiming production
device orchestration or general application compatibility. It must be reproducible from a
repository-owned realistic fixture application, keep private strings out of normalized output,
and run the existing deterministic kernel through public commands.

## Decision

Add `adapters/android/` as a local Python 3.9+ process adapter with protocol and
`org.sightlint.android` extension version `0.1.0`. The adapter uses only the Python standard
library. It consumes a caller-selected Android capture manifest and paired PNG below an explicit
repository root; it does not invoke `adb`, boot an emulator, install or execute an APK, or perform
an accessibility action.

Add a repository-owned Atlas settings fixture application and a dependency-free Android
instrumentation runner under `fixtures/android/atlas-app/`. The runner uses Android platform APIs
to capture three named, static states on a pinned emulator profile. It records View hierarchy facts
and `AccessibilityNodeInfo` facts separately and writes one canonical capture manifest plus one
full-screen PNG per state. APKs, Gradle caches, emulator images, signing keys, and runtime output
directories are not committed.

The public process is:

```text
repository-owned Android app source
  -> explicit maintainer capture on pinned emulator/API profile
  -> versioned capture manifest + paired PNG + digests
  -> bounded local Android file adapter
  -> exact View layout facts + separate platform accessibility facts
  -> public sightlint adapt-image for the PNG
  -> Artifact IR 0.1.0 + org.sightlint.android 0.1.0
  -> public sightlint normalize
  -> caller may run public sightlint check --profile base
```

The adapter exits `0` for a successful explicitly partial acquisition and `2` for usage, path,
schema, digest, resource, compatibility, output, or Rust-validation failures. It never exits `1`;
rule and process-gate behavior remains owned by the Rust binary.

## Selected capture source

Protocol v0 selects app instrumentation over a generic UIAutomator dump for the initial exact
geometry path. The instrumentation runner records, for each admitted View:

- a stable Android resource ID and its package-qualified resource name;
- parent resource ID where the parent is also admitted;
- Java class name;
- platform-reported visibility, enabled, clickable, focusable, focused, selected, checked,
  checkable, scrollable, and long-clickable states;
- top-left screen location plus measured width and height after layout;
- `getGlobalVisibleRect` presence and rectangle;
- separately initialized `AccessibilityNodeInfo` class/package/view-ID, bounds in screen, actions,
  and boolean states;
- UTF-8 byte counts and SHA-256 digests for nonempty View text and content descriptions, without
  writing their full values to the committed capture or adapter output.

Only Views with a unique nonempty resource ID, finite nonnegative integer size, identity matrix,
and a supported screen-space transform are admitted to core nodes. Resource identity, not capture
traversal order, determines stable node IDs. Unidentified nodes remain counted as unsupported and
are not assigned an iteration-derived core identity.

The runner is fixture acquisition tooling, not trusted policy. Its manifest is still validated as
untrusted adapter input. The first corpus captures classic Android Views only. Compose semantics,
WebView descendants, SurfaceView pixels, multiple application windows, dialogs, IME content,
accessibility-service overlays, magnification, display cutouts, foldable postures, and live
animations remain untested or unsupported.

## Core mapping and evidence classes

Each capture has one `devicePixel` screen coordinate space with top-left origin and right/down
axes. The canvas size comes from the platform display metrics recorded by the runner and must agree
with the paired PNG extent before any core node is emitted.

For an admitted View:

- the platform screen location and measured extent become `layoutBox` only when the capture says
  the View is shown, has a nonempty global visible rectangle, uses the identity transform, and the
  resource ID is unique;
- the box retains the full allocated View extent, not the clipped global-visible rectangle, so a
  partially clipped static control can be evaluated by the existing canvas-containment rule;
- its evidence class is `exactSource`, because the fixture capture reads deterministic View layout
  state from the source platform API;
- a clickable View becomes core `control`; a nonclickable ViewGroup becomes `container`; a
  nonclickable TextView becomes `text`; other supported Views become `other`;
- class, resource ID, raw layout/global-visible/accessibility rectangles, boolean states, string
  digests, capture selector, and unsupported reasons remain in the Android extension;
- no core `name` is emitted from a digest and no role is inferred from a Java class;
- no `hitBox`, `renderBox`, or `inkBox` is emitted.

Accessibility facts use separate `platformSemantics` evidence. In particular,
`AccessibilityNodeInfo.boundsInScreen` remains `accessibilityBoundsDevicePixels` in the Android
extension. It does not become `hitBox` because an accessibility action can be routed without a
physical pointer inside that rectangle, touch delegates can enlarge a target, and clipping or
platform services can alter the reported bounds. It does not become `renderBox` because it is not
a pixel measurement.

The existing `visual.bounds.within-canvas@0.1.0` rule may consume the exact View `layoutBox`
without an Android-specific kernel branch. That rule proves only the named mechanical source-layout
obligation. It does not prove visibility, tappability, accessibility, or UX quality.

Views that are fully outside the global visible region are retained in extension coverage as
`notMappedNotGloballyVisible`; they do not receive a core box and therefore do not manufacture a
failure for ordinary offscreen scroll content. Missing or unsupported evidence remains absent.

## Screenshot and native reconciliation

The instrumentation runner calls `UiAutomation.takeScreenshot()` after the UI thread is idle and
the fixture state disables animations. Capture order, rather than atomic synchronization, is
recorded explicitly. The capture manifest and PNG each carry a SHA-256 digest, and the request pins
both.

The adapter invokes public `sightlint adapt-image` for the paired PNG and retains the image as a
second exact-render evidence record and `devicePixel` canvas. Version 0.1.0 reconciles only:

- manifest display width/height versus PNG width/height; and
- declared orientation/rotation versus the captured canvas orientation.

Extent disagreement is a stable adapter error because screen coordinates could not be mapped
safely. Native node-to-pixel identity, clipping shape, occlusion, z-order, color, text rendering,
and ink bounds remain `cantTell`. The adapter never changes native facts to make them agree with
pixels.

## Fixture and evaluation contract

The public Atlas corpus contains one source family captured from a realistic settings/account
screen:

1. `clean`: all admitted static View allocations are within the display;
2. `off-canvas-control-mutant`: one identity-transform static action View is positioned by its
   source layout parameters so its allocated right edge exceeds the display;
3. `scroll-offscreen-hard-negative`: a valid scroll container has a child outside the global
   visible region; the child remains extension evidence but is not mapped to a core layout box.

The fixture source, scenario declaration, capture manifest, screenshot, and request are
digest-pinned. The capture provenance records Android API level, build fingerprint, emulator/device
profile, display size/density, orientation, font scale, locale/direction, fixture application ID
and version, capture-runner version, commands, capture order, tool versions, and known limitations.

Acquisition and rule annotations are separately versioned and authored from the fixture source and
capture review rather than adapter output. The acquisition oracle records exact View and
accessibility facts, mapping status, screenshot extent, conflicts, and abstentions. The rule oracle
records only existing-rule applicability and expected outcomes. Implementation reports are
temporary and never stored as ground truth.

All three cases are public to implementers: smoke, development, and challenge. They are fictional,
maintainer-authored, and not independently reviewed. There is no protected holdout. Perfect
regression precision, coverage, abstention retention, or mutation kill rate does not establish
representative Android, accessibility, device, or general UI/UX accuracy.

## Resource and failure contract

The request selects limits no larger than:

- capture manifest: 8 MiB;
- paired PNG: 64 MiB;
- nodes: 10,000;
- hierarchy depth: 64;
- attributes per node: 64;
- one string/digest label: 1,024 UTF-8 bytes;
- canonical response and Artifact IR: 16 MiB each.

The adapter rejects absolute or escaping references, symlink escape, wrong digests, duplicate JSON
keys, unknown request/capture fields, incompatible protocol/extension/capture versions, duplicate
resource IDs, cycles, dangling parents, invalid integers/booleans/rectangles, unsupported display
rotation, display/PNG extent conflict, resource overflow, malformed PNG through `adapt-image`, and
preexisting output paths. It writes outputs exclusively after public `normalize` accepts the
candidate IR.

JSON nesting is bounded before semantic traversal. The Python process boundary and input limits do
not constitute an OS sandbox or generic memory ceiling. Python and Android SDK/emulator tooling
remain outside the trusted Rust kernel.

## Privacy, network, and licensing

- The adapter performs no network access and reports `externalProcessing: false`, retention
  `none`, and no transmitted fields.
- Capture and screenshot paths must resolve below the caller-selected repository root.
- Full text, content descriptions, account data, hierarchy JSON, and screenshot pixels are not
  copied into Artifact IR. Version 0.1.0 admits only byte counts and unsalted SHA-256 digests for
  strings.
- Unsalted digests, resource names, class names, package names, geometry, and screenshots remain
  sensitive source-derived data. Low-entropy labels can be guessed offline; users must protect
  adapter outputs and captures like source artifacts.
- Repository fixture source, manifests, annotations, and screenshots contain fictional data and
  use the repository's `MIT OR Apache-2.0` license. No customer, credential, analytics, or personal
  data is admitted.
- The Python adapter adds no package dependency. Android platform/SDK/emulator and Gradle/Android
  Gradle Plugin are capture-time tools with their own licenses and are not bundled in release
  archives or runtime dependencies. The dependency checker records any committed wrapper or
  package only if a later change adds one.

## Compatibility

Android request, response, extension, capture, corpus, acquisition-annotation, rule-annotation,
and metric surfaces start independently at `0.1.0`. Artifact IR remains `0.1.0`; CheckReport,
existing extensions, rules, profiles, commands, and exit meanings do not change. Incompatible
Android semantics require a new version and coexistence/migration tests rather than silent
reinterpretation.

This is a source-tree capability after `v0.1.0-alpha.2`. It does not change that release or claim a
bundled Python/Java/Android runtime, live-device command, APK distribution, registry package,
cross-device screenshot identity, or stable production Android support.

## Alternatives considered

### Generic UIAutomator XML plus screenshot

Deferred as the initial exact geometry source. It is broadly obtainable but does not expose the
View allocation needed to justify `layoutBox`, and its accessibility bounds do not justify
`hitBox` or `renderBox`. It also omits essential capture/device metadata and can leave a stale dump
file after a failed command. A later UIAutomator-only mode may preserve extension facts while core
rules abstain.

### App-instrumented View and accessibility capture

Selected for the first slice. It produces authentic, separately typed platform facts and an exact
View allocation path for a repository-owned application. Its debug/test-only and classic-View
scope is made explicit rather than generalized to arbitrary apps or Compose.

### Hand-authored capture JSON without runnable Android source

Rejected. It would test a JSON converter but not establish that the facts correspond to Android
platform APIs or a rendered app.

### Run `adb` inside the adapter

Deferred. Live orchestration adds executable discovery, device selection, authorization, shell
quoting, timeouts, changing state, platform support, and privacy risks. Capture and conversion are
separate in protocol v0.

### Treat accessibility bounds as touch targets

Rejected. It would erase the exact distinction required by ADR 0015 and could create blocking
findings from false evidence.

### Add Android concepts to mandatory Artifact IR

Rejected. Resource IDs, Java classes, API levels, density, insets, accessibility actions, and
capture metadata remain in a versioned namespaced extension. Only established shared concepts map
to core.

## Consequences

- SightLint gains an authentic mobile-native acquisition path without device logic in the kernel.
- Native View geometry, platform accessibility geometry, and rendered pixels remain independently
  inspectable and can conflict without one replacing another.
- Existing canvas containment can evaluate one exact Android layout mutation without an
  Android-specific rule.
- The slice is deliberately limited to one repository-owned classic-View app, one emulator/API
  profile, static states, and public regression data.
- A later Android slice may add generic UIAutomator/Compose/live capture, exact touch-delegate
  acquisition, richer accessibility semantics, multiple windows/insets, and broader evaluation
  only after evidence supports those claims.
- iOS remains the next separate child under issue #29 after this Android slice is merged and its
  post-merge CI is green.
