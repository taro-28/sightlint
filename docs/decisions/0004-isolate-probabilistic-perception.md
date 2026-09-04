# ADR 0004 — Isolate probabilistic perception

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Images and incomplete native structures require OCR, detection, or semantic inference. These
systems are valuable but may vary by model, version, hardware, preprocessing, and run.
Treating their output as exact would undermine reproducible lint results.

## Decision

Keep probabilistic perception outside the trusted kernel. Perception emits observations with
provenance, model identity, confidence, uncertainty, and source references. The deterministic
kernel evaluates rules against those observations and may return `cantTell` when evidence is
insufficient.

A free-form model judgment cannot be a default blocking verdict.

## Consequences

- Image-only mode may have lower coverage while remaining honest.
- Model upgrades change observation inputs, not hidden rule semantics.
- Local and remote workers can be swapped behind the same adapter protocol.
- CI policy can distinguish proven and inferred evidence.

## Alternatives considered

- End-to-end VLM audit: broad but opaque, variable, and hard to validate.
- No perception support: deterministic but excludes important artifact classes.

## Verification

The kernel has no model dependencies or network access. Serialized inferred values identify
their evidence class and confidence. Tests reject inferred values that omit required
provenance.
