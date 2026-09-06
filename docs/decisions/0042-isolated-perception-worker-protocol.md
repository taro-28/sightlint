# ADR 0042 — Isolate perception workers behind a bounded observation protocol

- Status: Accepted
- Date: 2026-09-06
- Issue: #28
- Builds on: ADRs 0003, 0024 (product evaluation), 0030–0035, 0039, and 0041

## Context

SightLint needs richer observations for screenshot-only images, scanned documents, slides, and
other artifacts whose native structure is incomplete. OCR, deterministic CV, learned detectors,
and VLMs can propose text, regions, roles, hierarchy, and peer relations, but they are untrusted
sensors. They can crash, time out, omit objects, hallucinate semantics, vary across runtimes, and
expose artifact content if a remote service is selected.

The current medium-neutral Artifact IR already distinguishes `visionMeasured` from
`visionInferred`, records provenance and uncertainty, and requires calibrated confidence before
`visionInferred` evidence may enter the core. Issue #28 also requires that workers without a
calibrated probability do not invent a number. Therefore a process protocol needs to preserve
unquantified candidates without silently promoting them into core semantic facts.

The existing image-segmentation benchmark provides a useful reference input: it contains bounded
pixel components under an explicitly unconfirmed background hypothesis, while the Playwright Web
corpus provides stronger native/render evidence for differential evaluation. Neither is semantic
ground truth for OCR, roles, hierarchy, or peer membership.

## Decision

Define perception process protocol `0.1.0` and a local public wrapper outside the deterministic
Rust kernel. The wrapper invokes a caller-selected executable directly without a shell, exchanges
one bounded JSON request/response over standard streams, validates identity, versions, digests,
statuses, units, geometry, confidence availability, alternatives, and resource counts, and emits
canonical output. It kills the child on timeout or output overflow and emits no partial Artifact
IR after an operational failure.

Protocol v0 accepts a JSON-valued, SHA-256-linked input representation. Binary raster transport,
tiling, remote execution, model hosting, and device-specific acceleration require later compatible
families or protocol versions. Requests declare preprocessing, crop/scale/tile/seed status,
backend, timeout, byte/node budgets, local/remote behavior, retention, and redaction. Version `0.1`
accepts only local execution with no remote transmission.

Worker responses expose independent family statuses for regions, text, roles, hierarchy, and peer
groups. `observed`, `partial`, `unsupported`, `ambiguous`, and `untested` are acquisition coverage
states, not rule outcomes. The public run report is always nonblocking and records semantic rule
outcome as `untested`.

## Reference worker and reachable slice

Ship one dependency-free Node reference worker that consumes the existing
`benchmark-image-segmentation` report and exposes the qualified 95%-corner row-run regions as
`visionMeasured` pixel-component observations. It verifies the request/input digest and retains
the source policy's unconfirmed-background and `cantTell` semantic labels.

The reference worker deliberately returns:

- region family: `observed` when the qualified policy produced regions, otherwise its explicit
  unavailable status/reason;
- text family: `unsupported` because it performs no OCR;
- role, hierarchy, and peer-group families: `untested` because exact-color components do not
  establish those semantics;
- model identity and random seed: `notApplicable`;
- calibrated confidence: `notApplicable` for deterministic measurements.

The wrapper maps only observed region measurements into a new image Artifact IR. Each becomes an
`other` node with a device-pixel `renderBox` and its own `visionMeasured` evidence selector. The
complete worker response remains in `org.sightlint.perception@0.1.0`. No core role, name, parent,
or relation is created. The mapped candidate is passed through the public Rust `normalize`
command before it is written, so the Rust kernel validates the resulting IR.

This reference worker proves the process, resource, provenance, abstention, canonicalization, and
mapping boundary. It is not an OCR, component-role, hierarchy, or semantic peer solution.

## Confidence and promotion

Protocol observations use an explicit confidence state:

- `calibratedProbability` contains a finite probability and names the calibration contract;
- `notProvided` records that a worker has no calibrated probability;
- `notApplicable` is allowed only for deterministic measurements or explicit unavailable states.

Uncalibrated semantic candidates remain in the perception extension. They do not become core
`visionInferred` evidence, roles, names, hierarchy, or relations. A later mapper may promote an
inferred observation only when the core contract's calibrated-confidence requirement and an
accepted family-specific mapping are both satisfied. Repeated-run agreement is a separate field
and never substitutes for model confidence.

## Stable identity and canonicalization

- `requestId` is caller supplied and stable for the evaluated case.
- An input digest covers the canonical JSON value actually sent to the worker.
- The wrapper records the selected executable's SHA-256 independently from the worker's claimed
  name/version/model metadata.
- Reference region identifiers derive from the complete measured tuple rather than collection
  iteration order.
- Observations, alternatives, family statuses, evidence, nodes, and extension objects are sorted
  before canonical serialization.
- Runtime/backend/model/preprocessing versions are part of the declared compatibility
  environment. Byte stability is required for repeated identical runs in that environment, not
  across different model/runtime/platform versions.
- Wall-clock duration is evaluation telemetry, not canonical evidence, and is not stored in the
  worker response or Artifact IR.

## Failure and resource contract

The v0 wrapper enforces reviewed hard limits for request bytes, embedded input bytes, response
bytes, stderr bytes, observations, geometry, text/alternative lengths, and timeout. It rejects:

- malformed JSON, unknown fields, duplicate IDs, invalid versions/digests/units/statuses;
- worker identity, input digest, backend, or model declarations that differ from the request;
- non-finite/out-of-canvas geometry and counts that exceed the request budget;
- nonzero exit, signal, timeout, stderr overflow, stdout overflow, or protocol-invalid output;
- an Artifact IR candidate rejected by the public Rust normalizer.

Success, partial coverage, explicit unsupported families, and ambiguity exit `0`. Usage,
execution, protocol, resource, or mapping failures exit `2`. This command never exits `1` and
never produces a trusted rule failure.

The process boundary isolates crashes and standard streams; it is not an operating-system sandbox.
A locally selected worker may still access the caller's account, filesystem, devices, or network
unless separately sandboxed. The reference worker reads only standard input and writes only
standard output.

## Reconciliation and evaluation

Use three repository-owned Northstar Web states from the #22/#23 corpus: clean, a targeted render
offset mutation, and an intentional-grouping hard negative. Capture native Artifact IR and pixels
synchronously, run the existing public segmentation command, invoke the reference worker through
the new public wrapper, and validate its mapped IR through the Rust binary.

Acquisition annotations and rule annotations remain separate. Evaluation records region coverage,
native/pixel agreement/conflict categories, family coverage and abstention, repeated-run bytes,
resource/error handling, mutation observation, and hard-negative semantic claims. It must retain
the Playwright native layout/render conflict alongside the independent pixel result; neither
source overwrites the other. OCR, role, hierarchy, peer-group, model calibration, and downstream
rule accuracy remain `untested`.

The three public cases are smoke/development/challenge data visible to implementers. They are one
fictional application family with maintainer-authored labels, no independent reviewer, and no
protected holdout. Metrics are regression evidence only.

## Security, privacy, licensing, and deployment

- Core behavior remains local-first; v0 rejects remote execution and nonempty remote-transmission
  declarations.
- Requests and responses declare retention/redaction status without claiming that transformed
  input equals the original.
- The reference worker has no model weights, native modules, package dependencies, telemetry, or
  network calls.
- Repository code, schemas, fixtures, and labels are owned by the project and distributed under
  `MIT OR Apache-2.0`; no customer artifacts, secrets, personal data, or external screenshots are
  committed.
- A future remote worker needs a separate privacy/security decision covering endpoint, fields,
  retention, consent, credentials, and failure behavior.

## Compatibility

Protocol request, worker response, run report, and perception extension are independent `0.1.0`
surfaces. Incompatible fields or semantics require a new version. The current Artifact IR,
CheckReport, Web/PNG extensions, existing commands, rules, profiles, and exit meanings do not
change. Unknown perception extensions continue to be preserved by the core.

## Consequences

- SightLint gains a reviewable language-neutral worker boundary and one real local process path.
- Untrusted worker behavior is bounded and canonicalized before Rust validation.
- Measured pixel components can coexist with stronger native evidence without semantic promotion.
- The reference implementation is intentionally low-coverage and cannot justify a blocking rule.
- Adding a real OCR/CV/VLM backend later still requires model/runtime evaluation, calibration,
  privacy, licensing, and family-specific mapping decisions.
- After this protocol slice is complete, the next roadmap gate is issue #29.
