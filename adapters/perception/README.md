# Perception worker protocol

`sightlint-perception` is a local, process-isolated wrapper for untrusted OCR/CV/VLM-style
workers. Protocol `0.1.0` represents region, text, role, hierarchy, and peer-group observations
with explicit family coverage, source links, confidence availability, alternatives, uncertainty,
model/runtime identity, preprocessing, privacy declarations, and resource budgets.

The wrapper is outside the deterministic Rust kernel. It validates and canonicalizes one JSON
request/response exchange, hashes the selected worker source, and sends the mapped candidate
through the public Rust `sightlint normalize` command. A worker supplies observations; it never
supplies a trusted rule verdict.

## Current reference slice

The dependency-free Node reference worker consumes the canonical JSON output of the existing
`benchmark-image-segmentation` command. The request names exactly one benchmark policy; there is
no fallback. Observed exact-color components remain unconfirmed `visionMeasured` regions with
device-pixel half-open bounds and `cantTell` semantic applicability.

Only model-free measured regions become core Artifact IR `other` nodes. Inferred regions, OCR
text, roles, hierarchy, and peer groups remain in the separately written canonical worker
response and are listed as unmapped by `org.sightlint.perception@0.1.0`. They create no core role,
name, parent, relation, rule result, or blocking authority. The conformance worker under
`fixtures/perception/workers/` proves all family shapes, calibrated-versus-unavailable confidence,
alternatives, and this non-promotion boundary; it is not a model or product oracle.

This slice does not implement OCR, learned detection, semantic classification, automatic
reconciliation, or a downstream rule. A deterministic component boundary is not necessarily a UI
object.

## Public local process

Install the package metadata, build the Rust binary, and run a caller-authored protocol request:

```bash
npm --prefix adapters/perception ci --ignore-scripts
cargo build --locked -p sightlint-cli

node adapters/perception/src/cli.mjs \
  --request REQUEST.json \
  --worker-program "$(command -v node)" \
  --worker-argument adapters/perception/src/reference-worker.mjs \
  --worker-source adapters/perception/src/reference-worker.mjs \
  --sightlint-binary target/debug/sightlint \
  --response-out RESPONSE.json \
  --artifact-ir-out ARTIFACT-IR.json
```

The request embeds a canonical `benchmark-image-segmentation@0.1.0` report and its SHA-256,
declares its canvas and policy, and supplies logical output references. On success, the wrapper
writes canonical response and normalized Artifact IR files, writes a canonical nonblocking run
report to stdout, writes nothing to stderr, and exits `0`. Unsupported/partial/ambiguous
acquisition also exits `0`. Usage, process, resource, protocol, mapping, or normalizer errors write
one stable diagnostic to stderr, no stdout, no Artifact IR, and exit `2`. The wrapper never exits
`1`. Existing output files are never overwritten, and a failed pair write removes any new partial
output while preserving caller-owned data.

## Trust, privacy, and resource boundary

Protocol `0.1` accepts only local execution, `externalProcessing: false`, an empty transmitted
field list, no retention, and no redaction transform. Remote workers require a later explicit
privacy/security decision. The reference worker reads standard input, writes standard output, has
no telemetry or package/model dependencies, and makes no network calls.

The request and wrapper bound request/input/output/stderr bytes, observations, OCR text length,
hierarchy depth, canvas geometry, and process duration. Worker identity, source digest,
model/backend declarations, observation references, hierarchy cycles, and coordinate extents are
validated. Timeout, overflow, nonzero exit, malformed JSON, identity drift, and invalid references
are covered by process E2E.

This is process isolation, not an operating-system sandbox. A caller-selected worker can still
access the user's files, network, devices, or account, and generic worker memory is not constrained
by protocol v0. Use a separately reviewed sandbox before running untrusted third-party code.

## Evaluation and non-claims

`evaluation/perception/` uses the repository-owned fictional Atlas Web fixture. The Playwright
adapter captures native structure and a synchronized screenshot; the built Rust binary produces
the pixel benchmark; then this wrapper and the Rust normalizer run as public processes. Clean,
targeted render mutation, and intentional-grouping hard-negative cases verify repeated bytes,
retained native/pixel conflict, one observed acquisition mutation, no semantic promotion, no
blocking result, and no hard-negative failure.

Atlas has both dark and light top-level edge surfaces, so the qualified and strict benchmark
policies abstain. The evaluation explicitly selects the ranked policy only to exercise mapping and
retains its unsafe background as unconfirmed. These three public states are development regression
data, not a protected holdout or evidence of real-world OCR, role, hierarchy, peer, rule, or UI/UX
accuracy.

## Validation

```bash
python3 tools/check_perception_evaluation.py
npm --prefix adapters/perception run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/perception run test:e2e
npm --prefix adapters/playwright run test:e2e
```

The final command contains the realistic differential evaluation. Linux is the required browser
platform; the protocol/unit/process suite also runs on Linux, macOS, and Windows in CI.
