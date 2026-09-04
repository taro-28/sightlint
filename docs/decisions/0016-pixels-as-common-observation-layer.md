# ADR 0016 — Pixels are the common observation layer, not the canonical truth

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Rendered images are available for nearly every target medium and expose the visual result that
users actually see. They allow shared checks across web interfaces, mobile screens, slides,
documents, PDFs, and exported images. However, pixels alone cannot reveal hit targets,
accessible names, focus order, interaction effects, exact source units, or hidden states.

## Decision

Treat rendered pixels as the universal minimum observation layer. Prefer and reconcile richer
native structure whenever available. Do not make image-derived structure the canonical truth
when exact source or platform semantics exist.

Image-only analysis may run a subset of visual rules and must report unavailable coverage.
Probabilistic semantic reconstruction remains optional perception with provenance and
uncertainty.

## Consequences

- Shared visual rules can operate across artifact classes.
- Web or mobile support is not required for slide and image analysis.
- Image-only mode returns fewer conclusions instead of invented structure.
- Reconciliation between native declarations and rendered reality becomes central.

## Alternatives considered

- Native structure only: accurate but excludes images and fragments the rule engine by medium.
- Pixels as sole truth: broad but unable to verify semantics and interaction.
- Convert all media to DOM: web-centric and lossy.

## Verification

Adapters can emit pixel canvases without semantic nodes. Rules declare whether pixels are
sufficient. Tests verify that missing native semantics reduce coverage and produce non-binary
outcomes rather than fabricated roles or actions.
