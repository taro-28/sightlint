# Artifact IR

## Purpose

Artifact IR is the versioned, medium-neutral boundary between acquisition and deterministic
policy/rules. It represents what an adapter knows, how it knows it, in which units and coordinate
space, and with what uncertainty. It does not encode a final quality verdict and it does not make
one medium's native object model universal.

The design borrows useful concepts from:

- accessibility trees for role, name, state, action, and hierarchy;
- scene graphs and design-tool nodes for canvases, nodes, geometry, transforms, clipping, styles,
  groups, and z-order;
- Web Annotation-like selectors for linking observations to source regions/objects;
- image annotation formats for regions and categories inside adapters;
- ACT-style input aspects, atomic/composite rules, and outcome distinctions;
- platform UI-automation hierarchies for semantic and hit geometry;
- slide/document/PDF object and tag trees for pages, shapes, text, and reading order;
- versioned JSON schemas for language-neutral compatibility.

No existing structure covers all targeted artifact types, distinct source/render/hit geometry,
provenance, uncertainty, source conflicts, policy, and future interaction extensions. SightLint
therefore owns an independent schema while keeping familiar concepts.

## Implemented core boundary

Current `sightlint-ir` implements and validates the foundations needed by M1–M3:

- artifact descriptor and artifact kind;
- one or more canvases/coordinate spaces with explicit size, unit, and direction;
- nodes with stable identifiers, hierarchy references, node kind, optional role/name, and
  geometry;
- distinct layout, rendered/ink, and hit rectangles where supplied;
- relations between nodes;
- observations linked to evidence;
- evidence source, class, adapter/version, selector, confidence, and uncertainty;
- versioned namespaced extensions;
- semantic validation, compatibility handling, and deterministic canonicalization.

The exact Rust types and emitted JSON schema on current `main` are the implementation authority.
This document describes intent and evolution constraints; it must be updated when public schema
facts change.

## Planned observation families

The architecture anticipates richer data, but these are not all implemented as stable core fields:

- style and typography observations;
- text content/style runs and reading order;
- transforms, clipping, z-order, and occlusion evidence;
- visual and semantic peer groups;
- color/compositing/color-space observations;
- accessibility states/actions and platform semantics;
- source/render reconciliation and conflict records;
- policy/baseline references;
- actions, effects, states, traces, safeguards, and recovery;
- perception alternatives and calibration metadata.

Add such data through versioned extensions first unless it is truly shared, stable, and needed by
multiple media. A future promotion into core requires an ADR, migration plan, fixtures, and
compatibility tests.

## Conceptual observation envelope

A probabilistic observation may conceptually look like:

```json
{
  "value": "heading",
  "evidenceId": "evidence-42",
  "confidence": 0.91,
  "uncertainty": null,
  "alternatives": [
    { "value": "paragraph", "confidence": 0.07 }
  ]
}
```

This is illustrative, not a promise that every field is present in the current JSON schema.

Exact native facts normally do not need probabilistic alternatives, but they still require
provenance. Do not attach a fake confidence of `1.0` to make an exact fact look certain; evidence
class and confidence are different concepts. Likewise, do not omit version/runtime/capture
information merely because a value came from a deterministic adapter.

## Evidence model

Evidence should be expressive enough to distinguish sources such as:

- exact source/native extraction;
- exact deterministic transformation;
- platform semantics or UI-automation observation;
- declared project/design-system/platform contract;
- deterministic rendered/pixel measurement under stated assumptions;
- empirical heuristic with measured quality;
- OCR, detector, classifier, VLM, or other inferred perception;
- controlled interaction trace;
- conflicting, partial, unavailable, or unclassified evidence.

The current enum names are defined by the schema/source. Do not add a new label solely for prose
convenience; update the versioned contract and tests when evidence semantics change.

Every evidence record should make applicable details inspectable:

- adapter/worker/model/runtime and versions;
- local versus external processing;
- input identity/digest or source reference;
- selector into DOM/native structure, pixels, text, page, trace, or file;
- coordinate/preprocessing/capture assumptions;
- confidence/calibration/uncertainty/alternatives where real;
- conflicts and unavailable coverage.

## Geometry

SightLint distinguishes geometry because the same object can have different rectangles:

- **source/layout box:** space allocated or declared by the native layout system;
- **render/ink box:** visible output according to a documented predicate;
- **hit box:** interactive activation area;
- **canvas/viewport/page/screen bounds:** containing coordinate space;
- future transform, clipping, occlusion, and safe-area observations.

Do not use one box as a substitute for another. Examples:

- transparent image padding makes source bounds larger than visible ink;
- CSS transforms can change rendered size without changing layout allocation;
- a control can have a small visual icon but a larger valid hit target;
- an accessibility node may exist while rendered output is clipped or occluded.

Every numeric geometry value has an explicit unit and coordinate-space identifier. Native units
such as CSS px, device pixels, points, EMUs, PDF user units, Android dp, or iOS points are not
interchangeable. Normalized coordinates may be derived for comparison but do not replace source
units or conversion evidence.

Rectangle edge/inclusion conventions, rounding, transforms, and tolerance must be versioned and
tested. Non-finite or negative-invalid geometry is rejected.

## Hierarchy and relations

Parent/child hierarchy is stored when supplied by a source or explicitly reconstructed. Its
evidence must show which.

Many geometric relations should be derived by deterministic queries rather than redundantly
serialized:

- containment;
- intersection and overlap extent;
- alignment and centers/baselines;
- gaps and nearest neighbors;
- same row/column under a defined tolerance;
- canvas/safe-area inclusion.

Semantic relations are different:

- equivalent peers;
- label/control association;
- reading order;
- group membership;
- action/effect ownership;
- repeated component role.

Equal size or color does not prove semantic peer membership. Image-only grouping remains an
inferred observation with assumptions and cannot be promoted into exact relations silently.

## Native and rendered reconciliation

Do not overwrite source/native observations with pixel observations or vice versa. The IR and
extensions must be able to preserve:

- agreement within a declared tolerance;
- expected loss from one source;
- native-only observation;
- pixel-only observation;
- transform or scale mismatch;
- clipping/occlusion conflict;
- semantic role conflict;
- capture timing/state mismatch;
- unresolved conflict requiring `cantTell`.

ADRs 0033 and 0034 implement the first Playwright-based reconciliation slice. ADR 0035 evolves it
to the official optional `org.sightlint.web@0.3.0` extension, with explicit DOM, render, and
optional platform-accessibility evidence references per selected node. The deterministic engine
strictly validates that extension and consumes it only for Web-profile rules; acquisition remains
in the untrusted Node process. A later generalized conflict model may require a core/schema ADR.

ADR 0048 admits `org.sightlint.web@0.4.0` for managed loopback capture without changing core IR
`0.1.0`. Its extension-only capture record distinguishes `loopbackResponses` from repository-file
source, retains a route path without query, target/source digests, response count and byte totals,
and blocked WebSocket/service-worker counts. Core evidence continues to reference a source digest,
but runtime DOM locators do not establish a source file or line; workflow attribution is therefore
explicitly unavailable rather than guessed.

## Policy is not an observation

Artifact IR records facts and evidence. Rule policy may be supplied by:

1. explicit project contract/exception;
2. design-system/platform contract;
3. inferred project norm;
4. platform convention;
5. built-in recommended baseline.

Do not store a selected expected value as though it were an observed artifact property. Reports
must preserve policy identity/version/scope separately from observations and targets.

## Medium-specific extensions

Extensions are required for rich native data that does not yet belong in core, for example:

- web DOM/accessibility/computed style/frame/capture metadata;
- PNG raster availability/checksum and advisory inspection;
- PPTX shapes/theme/placeholder/z-order data;
- PDF tagged/paint/text information;
- Android/iOS semantics and device environment;
- perception-worker alternatives/calibration;
- action/effect/state/trace contracts;
- adapter reconciliation details.

Rules may consume an official extension only after its schema, version, validation, and
compatibility behavior are documented. Unknown extensions must be preserved according to current
compatibility rules rather than discarded or interpreted heuristically.

## Current PNG use

The PNG adapter emits source and raster evidence while keeping raw pixels internal. Current IR
contains bounded availability, dimensions/counts, checksum, and provenance. It does not serialize
all pixels or automatically create semantic component nodes.

`inspect-image` returns a separately versioned advisory acquisition report instead of altering the
trusted Artifact IR/CheckReport meaning. Its region/gap candidates are conditional hypotheses, not
accepted semantic relations. ADRs 0030 and 0031 define these boundaries.

## Current PPTX use

ADR 0043 leaves Artifact IR at `0.1.0` and maps a supported presentation slide to an `emu` canvas,
direct source shapes/groups to ordinary `shape`/`container` nodes, and supported source transforms
to evidence-linked `layoutBox` values. PPTX native IDs, local z-order, placeholder fields,
digest-only source-text metadata, acquisition coverage, unsupported features, and render
reconciliation live in `org.sightlint.pptx@0.1.0` rather than mandatory core fields.

The synchronized PNG is a second `devicePixel` canvas with separate exact-render evidence. Source
and render dimensions are both retained when their declared scale conflicts. No `renderBox` or
`inkBox` is inferred for a source node, and shape-to-pixel identity remains `cantTell`. The shared
canvas-containment rule may consume exact source `layoutBox` facts without learning PPTX concepts.

## Current PDF use

ADR 0044 also leaves core Artifact IR at `0.1.0`. Each supported explicit CropBox becomes a
top-left `pdfPoint` canvas. A rectangular internal Link annotation with sufficient exact source
evidence becomes a `control`/`link` node carrying only `hitBox`; source PDF coordinates, object
references, page order, action class, coverage, tag-presence status, unsupported features, and
render reconciliation stay in `org.sightlint.pdf@0.1.0`.

An optional page PNG is a separate `devicePixel` canvas with exact-render evidence. Extent
agreement or conflict is recorded without changing either coordinate space. `QuadPoints`, paths,
rotation, unsupported actions, and uncertain page geometry do not create approximate core hit
boxes. Rendered annotation identity and viewer behavior remain `cantTell`, while text, tags, paint,
and reading order remain `untested`.

## Current Android use

ADR 0045 leaves core Artifact IR at `0.1.0`. One platform display becomes an exact-source
`devicePixel` canvas. A supported shown and globally visible classic View becomes an ordinary
`control`, `container`, `text`, or `other` node carrying only an exact-source `layoutBox`.
Resource IDs, Java classes, hierarchy depth/parentage, View state, text/content-description
digests, accessibility actions and rectangles, device/build/capture provenance, coverage, and
unsupported features stay in `org.sightlint.android@0.1.0`.

The paired PNG is a second `devicePixel` canvas with exact-render evidence. Display extent
agreement is recorded without manufacturing node-to-pixel identity. Accessibility rectangles use
separate `platformSemantics` evidence and do not become touch or render geometry. A clipped
accessibility rectangle cannot repair a source allocation, and invalid or offscreen platform
geometry remains conflict/abstention evidence rather than an exact fact. The shared
canvas-containment rule may consume admitted View `layoutBox` facts without learning Android
concepts.

## Current iOS use

ADR 0046 leaves core Artifact IR at `0.1.0`. One UIKit screen becomes an exact-source `point`
canvas. A supported attached, visible, identity-transform UIKit View becomes an ordinary
`control`, `container`, `text`, or `other` node carrying only an exact-source `layoutBox`.
Accessibility identifiers, Objective-C class names, source hierarchy/state, safe-area
intersections, digest-only labels/values, XCUITest observations, capture order, simulator/tool
provenance, coverage, and unsupported features stay in `org.sightlint.ios@0.1.0`.

The paired PNG is a second `devicePixel` canvas with exact-render evidence. Extent-and-scale
agreement is recorded without manufacturing node-to-pixel identity. XCUITest frames use separate
`platformSemantics` evidence and do not become layout, activation, touch, or render geometry. A
source/XCUI frame disagreement remains an explicit conflict. Fully offscreen source Views and a
direct clipped scroll-content allocation remain extension-only, so the shared canvas-containment
rule consumes only admitted UIKit allocation facts and learns no iOS concept.

## Interaction extension

ADR 0047 adds optional `org.sightlint.interaction@0.1.0` without changing core Artifact IR. The
first version represents stable actions, declared categorical effect latency, accepted retry or
save-draft recovery alternatives, captured/`untested` traces, canonical one-based event order,
attempt and causal identifiers, visible pending/optimistic/success/failure states, effect
resolution, recovery events, and retained cross-source conflicts.

Every action contract references `declaredContract` evidence. Every captured event includes
`interactionTrace` evidence and may additionally retain exact-render, platform-semantics, or
declared evidence. The schema contains no wall-clock timestamps. Static artifacts remain valid
without the extension, and a static screenshot cannot prove an invisible effect or temporal
obligation. Empty, partial, destructive safeguards, undo, focus/navigation, and broader recovery
types require later evaluated extension versions.

## Schema invariants

The implemented schema and future extensions must preserve:

- globally unique stable identifiers inside an artifact;
- resolvable parent, canvas, relation, evidence, and selector references;
- acyclic hierarchy;
- finite numeric values and explicit units;
- valid coordinate-space and direction relationships;
- bounded confidence values only when meaningful;
- uncertainty compatible with the observed unit/value;
- deterministic map/collection ordering and canonical serialization;
- versioned extension namespaces;
- no silent upgrade from inferred/candidate to exact;
- no silent loss of unknown extension data where compatibility says preserve;
- stable validation diagnostics and migration behavior;
- bounded input/object/output sizes in adapters and parsers.

## Compatibility and evolution

Artifact IR, official extensions, adapter protocols, reports, rule semantics, and package versions
are separate compatibility surfaces. A package bump alone cannot silently redefine all of them.

The ADR 0039 image-segmentation benchmark report is intentionally outside Artifact IR and
CheckReport. Its exact-color candidates and connected regions remain evaluation-only adapter
observations with `cantTell` semantic applicability and `untested` rule outcome; they do not create
nodes, relations, findings, or blocking policy in the medium-neutral core.

ADR 0040 uses the existing medium-neutral `inkBox` for exact PNG source-alpha bounds. Its
dedicated evidence and `alphaGeometry@0.1.0` extension preserve the predicate, coordinate space,
unit, counts, and source/display boundary. It does not add a PNG-specific mandatory core field or
turn the observation into a rule verdict.

ADR 0041 changes no IR or extension surface. Its format-demand assessment retains explicit
unavailability for unsupported source encodings; caller-selected conversion is evidence about the
converted bytes and must not be represented as exact facts about the original encoding.

ADR 0042 adds optional `org.sightlint.perception@0.1.0`. The extension records the canonical
worker-response digest, worker/model/runtime identity, acquisition-family statuses, observation
IDs, mapped versus unmapped counts, and reconciliation status. The complete typed response is a
separate canonical output. Protocol v0 maps only model-free `visionMeasured` pixel-component
regions into core `other` nodes with device-pixel `renderBox` evidence. Inferred regions, text,
roles, hierarchy, and peer groups are not promoted into core fields or relations; their response
records preserve source links, confidence availability, alternatives, and uncertainty.

For an IR or official-extension change:

- write an ADR when architecture/semantics change;
- identify compatible additions versus breaking changes;
- define migration and unknown-field behavior;
- update JSON schema and Rust types together;
- add valid old/new and malformed compatibility fixtures;
- verify canonical/idempotent normalization;
- update public-binary E2E, handoff, roadmap, and docs;
- never change an existing stable field's meaning without a versioned transition.

New ADR numbers continue at 0051 or later. Historical branch-only ADRs 0025–0029 are references,
not accepted current schema decisions.
