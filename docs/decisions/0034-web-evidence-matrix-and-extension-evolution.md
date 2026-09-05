# ADR 0034 — Web evidence matrix and extension evolution

- Status: Accepted
- Date: 2026-09-05
- Issue: #23

## Context

ADR 0033 introduced the process-isolated Playwright adapter and the first synchronized native and
pixel capture. Its `org.sightlint.web` extension `0.1.0` deliberately records only a center-point
hit-test sample and viewport intersection. That slice cannot express several distinctions required
by issue #23 without overclaiming:

- a box may overflow its client area without proving whether the user-facing content is truncated;
- an ancestor may clip a rendered box, while screenshot extent alone still says that the box lies
  inside the viewport;
- one sampled point may be occluded, but it is not a measured hit rectangle;
- computed font size may remain unchanged while a transform changes rendered scale;
- hidden, disabled, offscreen, RTL, and vertical-writing states require explicit reviewed
  expectations rather than incidental test assertions.

The repository-owned Atlas dashboard also needs a broader, reviewable fixture matrix before a
recommended rule pack can consume browser evidence. Captured output must not become its own oracle,
and acquisition correctness must remain separate from rule verdict correctness.

## Decision

Evolve the adapter-private `org.sightlint.web` extension to `0.2.0` and the reviewed browser
acquisition oracle to `0.2.0`. Artifact IR stays at `0.1.0`; the capture request and response
protocol stay at `0.1.0`; existing Rust rules and reports do not change. The adapter package version
becomes `0.2.0` so reports identify the implementation that emitted the new extension.

The previous Web extension schema remains available as
`adapters/playwright/schemas/web-extension-0.1.schema.json`. The unversioned
`web-extension.schema.json` path describes the current emitted version. Consumers must dispatch on
`extensionVersion`; they must not validate a `0.1.0` payload as `0.2.0` or silently supply new
fields. The browser acquisition corpus is deliberately migrated as reviewed evaluation data; its
`0.1.0` schema remains available as `evaluation/web/browser-acquisition-0.1.schema.json` for
historical validation.

The unchanged response protocol treats `adapter.version` as semantic-versioned provenance rather
than a protocol discriminator. Its schema therefore accepts semantic adapter versions while still
pinning the response shape, protocol version, adapter name, Playwright version, and browser kind.
An adapter implementation bump alone does not invalidate an otherwise compatible response.

## Evidence additions

For each selected node, extension `0.2.0` adds:

- `clientSize` and `scrollSize`, measured in CSS pixels;
- computed `whiteSpace` and `textOverflow` values;
- an `overflowMeasurement` that reports horizontal and vertical client/scroll overflow as exact
  browser layout measurements, but does not claim that overflow is a UX defect;
- an `ancestorClip` reconciliation record derived from intersections with recorded clipping
  ancestors;
- a `centerHitSample` with document-space point, outcome, and the stable selected-node locator hit
  at that point when available;
- a `hitRegion` outcome fixed to `cantTell` until a later protocol measures the full activation
  region.

The center sample is not a hit rectangle. Core `hitBox` remains absent. An element's render box,
client/scroll measurements, clipping evidence, and center hit sample remain separate observations.
This prevents a convenient browser rectangle from being promoted into unsupported exact hit-area
ground truth.

`ancestorClip` has `notClipped`, `partiallyClipped`, `fullyClipped`, or `cantTell`. It compares the
node render rectangle with the rectangular padding boxes of recorded overflow-clipping ancestors.
It does not model non-rectangular clips, masks, opacity compositing, or pixel visibility. Those
limitations remain explicit, and pixel-content identity remains `cantTell`.

## Reviewed fixture matrix

Extend the repository-owned Atlas dashboard with realistic states covering:

- ancestor clipping and content overflow;
- center-point occlusion and an intentional overlay hard negative;
- inconsistent peer dimensions;
- transformed text whose computed font size and rendered scale disagree;
- visual-control and interactive-control extent differences;
- hidden, disabled, and offscreen controls;
- RTL and vertical writing;
- a desktop/mobile responsive mutation pair.

The fixture remains synthetic, fictional, repository-owned, free of external assets and personal
data, and subject to the repository's unresolved license. Public smoke, development, and challenge
cases are not a secret holdout. They provide regression evidence only and must not be described as
real-world accuracy evidence.

The acquisition oracle may assert exact source-authored state, browser-observed styles, bounded
geometry with tolerances, and conservative reconciliation outcomes. The rule oracle remains a
separate document. New acquisition-only mutants may legitimately produce only current-rule
`inapplicable` or `cantTell` results; they do not become failures just because their names describe
possible defects.

## Deterministic fixture policy

Protocol `0.1.0` does not virtualize time or replace randomness in arbitrary pages. Instead, its
local fixture contract admits only reviewed fixtures whose capture state has no clock-, random-,
storage-, network-, or user-history-dependent output. The Atlas fixture uses fixed fictional data
and no timers or random sources. Supporting arbitrary applications requires a later request field,
threat model, and compatibility decision rather than an undocumented preload script.

Scrollbar presence is not normalized across operating systems. The versioned request fixes the
viewport and the adapter records document and viewport sizes plus scroll offsets. Fixture states
used for byte-stability avoid scrollbar-dependent horizontal geometry; cross-platform screenshot
byte identity remains a non-claim under ADR 0033.

## Evaluation and compatibility

The browser E2E executes the actual Node process and built `sightlint` binary for every reviewed
case. It validates both current schemas, checks exact/toleranced acquisition expectations, retains
representative screenshots as CI artifacts where configured, and reports counts for cases,
expectations, abstentions, targeted acquisition mutations, current-rule mutation kills, and hard
negative failures. It never copies implementation output into the oracle.

Same-input response, Artifact IR, screenshot, report, diagnostics, and exit-code stability remain
required within one declared compatibility environment. Linux runs the browser gate. macOS and
Windows continue to run the Rust gates, while their browser/font/raster portability remains a
documented non-claim until characterized.

## Alternatives considered

### Populate core `hitBox` from `getBoundingClientRect()`

Rejected. A render rectangle does not prove the complete activation region after clipping,
occlusion, pointer-events, descendants, transforms, and non-rectangular shapes.

### Treat scroll overflow as proven truncation or a rule failure

Rejected. Scroll/client dimensions prove overflow under the captured layout. Whether it is
intentional, reachable, visibly truncated, or harmful needs additional evidence and policy.

### Add broad rule behavior in the same change

Rejected. This change completes the acquisition evidence matrix needed by issue #23. Issue #24
owns admission, thresholds, applicability, false-positive controls, and blocking maturity for the
zero-setup recommended rule pack.

### Preserve only the latest schema file

Rejected. Keeping the prior strict schema makes version dispatch testable and prevents historical
`0.1.0` payloads from being silently reinterpreted under `0.2.0` semantics.

## Non-goals

- exact pixel-to-node identity, visible-ink segmentation, or a full hit-region measurement;
- arbitrary URLs, hostile-page support, frames, shadow DOM, or interaction tracing;
- changing deterministic Rust rule semantics or introducing a blocking Web rule;
- claiming real-world precision, recall, accessibility conformance, or universal UX quality;
- defining private holdout governance or resolving the project license.
