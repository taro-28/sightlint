# ADR 0033 — Playwright web adapter process and capture protocol

- Status: Accepted
- Date: 2026-09-05
- Issue: #23

## Context

ADR 0032 established a repository-owned Web evaluation fixture, separate acquisition and rule
oracles, and public-binary rule projections. Browser-derived DOM, accessibility, computed geometry,
screenshot, and reconciliation evidence deliberately remained `untested`. Issue #22 cannot measure
acquisition correctness, and issue #24 cannot admit useful zero-setup Web rules, until a real browser
adapter supplies those observations without moving browser code into the trusted Rust kernel.

Browser automation executes application JavaScript and consumes complex native state. Its output
depends on the browser build, host platform, fonts, viewport, capture timing, and page behavior. It
is therefore an untrusted sensor with a declared compatibility environment, not part of the
deterministic rule kernel.

## Decision

Add `adapters/playwright/` as a process-isolated TypeScript/Node adapter. Version `0.1.0` uses an
exactly locked Playwright dependency and its matching Chromium build. Node 20 through 24 is the
declared runtime range for this protocol version. The adapter communicates through versioned JSON
requests and responses and emits ordinary Artifact IR `0.1.0` plus an adapter-private
`org.sightlint.web` extension at version `0.1.0`.

The Rust crates do not link Playwright, Node, Chromium, or the Web extension implementation. The
existing `sightlint` binary remains the validator and deterministic rule executor for emitted IR.
The first public path is:

```text
versioned local capture request
  -> sightlint-web Node process
  -> controlled Chromium page and synchronized screenshot
  -> canonical Artifact IR plus org.sightlint.web extension
  -> sightlint check --format json
```

The Node executable writes the screenshot and canonical Artifact IR to explicitly supplied output
paths. Its canonical response is written to standard output. Adapter and binary E2E execute the
real process twice and compare response, IR, screenshot, report, stderr, and exit codes.

## Initial input boundary

Version `0.1.0` accepts only a repository-relative HTML entrypoint below a caller-supplied,
canonical repository root. Absolute paths, traversal, symlinks escaping that root, non-files,
unsupported query fields, and all non-`file:` top-level destinations are rejected.

The browser context blocks service workers, starts offline, and aborts every `http:`, `https:`,
`ws:`, and `wss:` request. The response records attempted external requests without their bodies.
One page containing exactly one main frame is allowed. Child frames are counted and rejected in
this version; supporting even repository-local frames requires a later protocol that assigns frame
and document identity to every observation. Arbitrary URLs and general hostile-site navigation
require a later threat-model and protocol version.

No artifact content leaves the machine. `externalProcessing` is always false. The adapter never
uploads screenshots, DOM content, accessibility data, or reports.

## Deterministic capture environment

Every request declares:

- viewport width and height in CSS pixels;
- device-pixel ratio;
- locale, timezone, color scheme, reduced-motion preference, and text scale;
- fixture state and a readiness selector;
- a logical screenshot reference that is stable across temporary output directories.

The adapter fixes headless Chromium, one browser context and page, no permissions, blocked service
workers, offline mode, default scroll position, reduced motion, animation-disabled screenshot
capture, hidden caret, CSS-pixel screenshot scale, and PNG output. It waits for DOM load, the
fixture readiness selector, and `document.fonts.ready`; it does not use an arbitrary sleep.

The response records Node, platform, adapter, Playwright, and browser versions; launch/capture
options; the input source digest; and the exact request digest. Reproducibility is asserted only for
identical input and declared compatibility environment. Cross-platform byte identity is not
claimed because host font and raster stacks can differ. The required adapter E2E is Linux; macOS
and Windows remain documented development targets while the existing Rust suites continue on all
three systems.

## Resource and failure model

Version `0.1.0` enforces these hard limits before returning a successful capture:

- request JSON: 1 MiB;
- one page and exactly one main frame;
- at most 200 captured nodes;
- viewport dimensions from 1 through 4096 CSS pixels per axis;
- device-pixel ratio from 1 through 2;
- at most 16,777,216 screenshot pixels and 16 MiB of PNG bytes;
- at most 16 MiB of canonical Artifact IR and response output;
- 20 second navigation/readiness/capture timeout.

Unsupported input, browser launch failure, timeout, external navigation, duplicate stable locator,
budget overflow, and serialization failure exit with code 2, empty stdout, and one stable
`sightlint-web: <code>: <message>` diagnostic on stderr. Successful capture exits 0 with empty
stderr. The adapter does not produce a rule failure exit code; the Rust binary owns rule outcomes.

## Stable identity and privacy

The adapter automatically discovers elements with `data-testid`, a unique HTML `id`, or a
supported semantic element/role. Users do not enumerate elements in the request. Locator priority
is unique `data-testid`, unique `id`, then a structural CSS path. Node identifiers are derived from
the locator value, not traversal order or randomized hashes. Duplicate preferred locators are
rejected rather than disambiguated by array position.

The adapter does not serialize full DOM HTML or arbitrary text content. It records tag, selected
attributes, stable locator, hierarchy among captured nodes, computed fields needed by the first
evaluation, and an accessibility role/name/state summary for the selected node. Descendant
accessibility content is reduced to a digest plus the selected root line; this preserves evidence
without copying an entire private subtree. Fixture data remains fictional and repository-owned.

## Geometry and coordinate semantics

The adapter emits separate `document` and `viewport` canvases in CSS pixels, each with positive x
to the right and positive y down. Core node geometry uses document coordinates. The viewport canvas,
scroll offsets, and document-to-screenshot translation remain explicit in the Web extension.

This distinction is required for ordinary scrollable pages. Treating the initial viewport as the
core bounds canvas makes legitimate below-the-fold content fail `visual.bounds.within-canvas`.
Conversely, silently expanding a viewport to the document extent makes viewport evidence false.
The document canvas uses the maximum document/body scroll and client extents after readiness;
viewport capture never substitutes for it.

- `layoutBox` is the pre-local-transform border box accumulated through an untransformed
  `offsetParent` chain and translated into document coordinates. If an ancestor transform or an
  unsupported layout condition prevents that mapping, the box is omitted and the extension
  records why.
- `renderBox` is `getBoundingClientRect()` after transforms, translated into document CSS pixels.
  It is a browser geometry measurement, not a visible-ink or unoccluded-pixel box.
- `hitBox` remains absent because one center-point hit test cannot establish the full interactive
  region. The exact center hit-test outcome is retained only in the Web extension; it is not a
  substitute rectangle.
- `inkBox` remains absent because this slice does not segment screenshot pixels.

Computed font size, line height, weight, visibility, display, opacity, overflow, writing direction,
transform, pointer events, clipping ancestors, and center hit-test status live in the Web
extension. Units and the measurement method are explicit. Missing values remain missing or carry
an unavailable reason; one rectangle is never silently copied into another geometry kind.

## Accessibility evidence

Playwright accessibility/ARIA snapshots are platform observations, not source declarations and
not proof of visible design. For every selected element, the adapter requests a locator-scoped ARIA
snapshot, retains a digest of the complete snapshot, and serializes only the selected root summary.
If that summary cannot be parsed conservatively, the extension records `cantTell`; it does not
invent a role or name. Core role/name fields are emitted only when supported by the retained
platform summary or an exact explicit source declaration with distinct evidence.

## Screenshot and reconciliation

The screenshot belongs to the same page state after readiness and immediately following native
collection. Its SHA-256 digest, PNG byte count, pixel dimensions, logical reference, capture
options, and input relationship are recorded. Raw pixels remain in the PNG file, not Artifact IR.

Version `0.1.0` performs bounded reconciliation that it can prove:

- screenshot dimensions versus declared viewport and screenshot scale;
- each document-space render rectangle translated by the captured scroll offset and intersected
  with viewport/screenshot coordinates;
- native visibility versus zero-area, clipping, off-viewport, and center hit-test observations;
- layout/render differences caused by transforms.

It does not claim that pixels inside a rectangle visually match the DOM node. Pixel-content
matching is `cantTell` in this version. Agreements and conflicts are separate extension records;
neither native nor pixel evidence overwrites the other. Only ordinary core geometry with exact
evidence reaches existing trusted rules.

## Compatibility

The request protocol, response protocol, and `org.sightlint.web` extension each start at `0.1.0`.
Unknown request or response fields are rejected. A consumer must reject unsupported protocol or
recognized extension versions rather than partially interpreting them. Artifact IR remains
`0.1.0`; CheckReport remains `0.2.0`; existing rule versions and CLI exit codes do not change.

The npm lockfile is part of the compatibility and supply-chain record. Playwright and TypeScript
are Apache-2.0; Node type declarations and the AJV schema validator are MIT. The project itself
remains unlicensed until issue #33, so adding dependencies does not imply a repository license
grant.

## Evaluation

The adapter extends the repository-owned issue #22 corpus rather than generating its own oracle.
Reviewed acquisition annotations state expected selected nodes, hierarchy, roles, relative
geometry, mutation effects, and deliberate unknowns. Implementation capture files are temporary
test artifacts, not copied into oracle data.

The first evaluated rule path uses existing `core.bounds.within-canvas@0.1.0` behavior on a clean
dashboard and one targeted out-of-viewport mutation. The rule needs no inferred semantic peer
relation. Existing declared peer-spacing projections remain separate and unchanged. An
intentional-grouping state remains a hard negative, and incomplete pixel-content reconciliation
remains `cantTell` rather than a blocking result.

CI validates schemas, unit/malformed/resource behavior, repeated adapter bytes, reviewed
acquisition expectations, screenshot/native differential records, and the built `sightlint`
binary on Linux. Existing generator, IR, PNG, image-inspection, rule, MSRV, macOS, and Windows gates
remain required.

## Alternatives considered

### Embed a browser crate in the Rust kernel

Rejected because it expands the trusted computing base, couples browser lifecycle to rule logic,
and conflicts with accepted process isolation.

### Use screenshots without native structure

Rejected because it discards higher-quality DOM/accessibility evidence and would repeat the
semantic ambiguity that issue #23 exists to address.

### Serialize the entire DOM and accessibility tree

Rejected as the default because it increases privacy exposure, output size, instability, and
irrelevant data. The selected-node contract can be broadened in a later reviewed version.

### Treat browser output as reviewed ground truth

Rejected because the implementation under evaluation cannot generate its own oracle.

### Require browser E2E on all operating systems immediately

Deferred. The first exit criterion requires Linux native-input-to-IR E2E and a portability
strategy. Platform raster/font differences must be characterized before cross-platform golden
screenshots are made blocking.

## Non-goals

- arbitrary network URLs or every SPA/framework;
- full DOM, accessibility, iframe, shadow-DOM, or interaction tracing;
- OCR, CV, VLM, image segmentation, or pixel-content identity;
- automatic semantic peer inference;
- new recommended or blocking rule policy;
- universal design scores or broad aesthetic critique;
- packaging the Node adapter into a public release before issue #33.
