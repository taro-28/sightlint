# ADR 0003 — Medium-neutral Artifact IR

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Web DOM, accessibility trees, design-tool scene graphs, slide formats, PDFs, mobile semantic
trees, and images expose different structures. No single existing standard represents all of
the required geometry, semantics, provenance, uncertainty, and interaction information.

## Decision

Create a small, versioned SightLint Artifact IR. Borrow concepts from established structures:

- accessibility trees for roles, names, states, and hierarchy
- scene graphs for nodes, transforms, clipping, and visual style
- COCO-like detections inside vision adapters
- Web Annotation-style source selectors
- ACT-inspired rule input aspects and outcomes

Mandatory core entities remain medium-neutral. Platform-specific data uses namespaced,
versioned extensions.

## Consequences

- Adapters can be polyglot and independently replaceable.
- Rules can operate across media when they require the same input aspects.
- The project owns schema compatibility and validation.
- Native fidelity may require extensions rather than flattening all data into the core.

## Alternatives considered

- Accessibility tree as the complete IR: lacks rich visual and source geometry.
- Figma or DOM node model as the complete IR: too medium-specific.
- COCO as the complete IR: insufficient hierarchy, semantics, style, and provenance.

## Verification

Schema reviews require explicit units, evidence references, and no mandatory web-only fields.
Fixtures eventually cover at least two structurally different media using the same core types.
