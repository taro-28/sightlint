# Artifact IR

## Purpose

Artifact IR is the versioned boundary between observation and policy. It represents what an
adapter knows, how it knows it, and how certain that knowledge is. It does not encode a final
quality verdict.

The design borrows proven concepts rather than inventing every field:

- accessibility trees for role, name, state, and hierarchy
- scene-graph and design-tool nodes for geometry, transforms, styles, and clipping
- COCO-like annotations inside vision adapters for regions and categories
- Web Annotation-style selectors for linking evidence to source regions
- ACT-inspired input aspects and outcomes for rule execution

SightLint's serialized schema remains independent because no existing structure covers all
artifact types, rendered geometry, provenance, uncertainty, and interaction extensions.

## Core entities

The planned core contains:

- `Artifact`: one analyzed source and its metadata
- `Canvas`: a page, slide, viewport, screen, frame, or other coordinate space
- `Node`: a visual or semantic entity in a hierarchy
- `Observed<T>`: a value plus provenance and uncertainty
- `Evidence`: the observation supporting a value or rule result
- `Selector`: a stable reference into source structure, text, time, or pixels
- `Geometry`: layout, render/ink, and hit geometry
- `Style`: visual properties with explicit units and color spaces
- `TextContent`: text and optional style runs
- `Extension`: versioned medium-specific data

## Observation envelope

Conceptually, values follow this shape:

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

Exact native facts may omit probabilistic alternatives, but they still preserve provenance.
Confidence must not be used where the source makes an exact declaration; evidence class and
confidence are different concepts.

## Evidence classes

The first schema should support at least:

- `exactSource`: DOM, accessibility, PPTX, DOCX, PDF tags, platform API, or user contract
- `exactRender`: deterministic measurements from rendered output
- `platformSemantics`: accessibility or test hierarchy supplied by a platform
- `visionMeasured`: deterministic or bounded pixel measurement
- `visionInferred`: OCR, classifier, detector, VLM, or other probabilistic inference
- `interactionTrace`: observed state, focus, event, or network transition
- `declaredContract`: project, design-system, API, or effect metadata
- `unknown`: evidence unavailable or unclassified

## Geometry

The IR distinguishes:

- `layoutBox`: the space allocated by the source layout system
- `renderBox` or `inkBox`: pixels actually visible, including or excluding effects as defined
- `hitBox`: the interactive target area
- transforms, clipping, and coordinate-space references

Every geometry value has a unit and coordinate-space identifier. Normalized 0–1 coordinates
may be derived, but they do not replace native units.

## Hierarchy and relations

Parent-child hierarchy is stored as a fact when supplied or reconstructed. Many relations are
derived by deterministic queries rather than redundantly serialized:

- contains
- overlaps or occludes
- aligned edges or baselines
- gap and nearest neighbor
- same row or column
- repeated geometry or typography pattern

Semantic peer groups and visual grouping may require inference; their evidence and confidence
must remain visible.

## Interaction extension

Interaction data is not mandatory for static artifacts. A versioned extension may add:

- controls and actions
- user tasks
- effects such as destructive, financial, multi-user, or non-idempotent
- states and state transitions
- focus movement
- network or platform events
- safeguards and recovery paths

The static core must remain usable without this extension.

## Schema invariants

The first implementation must enforce:

- globally unique identifiers inside an artifact
- valid parent-child references and acyclic hierarchy
- valid canvas and coordinate-space references
- finite numeric geometry
- explicit units
- confidence values in a closed 0–1 interval
- uncertainty bounds that contain the value when applicable
- evidence references that resolve
- deterministic collection ordering during serialization
- extension namespaces and schema versions

## Source versus rendered reality

SightLint intentionally keeps declared and rendered observations side by side. A mismatch can
be the target of a rule. Do not overwrite one with the other during normalization.
