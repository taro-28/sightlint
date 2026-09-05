# Playwright Web adapter

`sightlint-web` is SightLint's process-isolated, untrusted browser acquisition adapter. It loads a
repository-owned local HTML fixture in the Playwright-pinned Chromium build, captures selected DOM
and accessibility observations, computed layout/render geometry, bounded clipping/overflow and
center-hit samples, and a synchronized viewport screenshot, then writes Artifact IR for the
deterministic Rust `sightlint` binary.

The adapter and local orchestration command are governed by ADRs 0033–0036. They are not part of
the Rust kernel and do not decide whether a UI is good or bad.

## Compatibility

- Node.js: 20 through 24; CI verifies that the hosted runtime is in range and records its exact
  version in every capture.
- Playwright: exactly `1.63.0` with its matching Chromium build.
- Schema validation: AJV exactly `8.20.0` in development/E2E.
- private adapter package: `0.4.0`.
- capture request/response: `0.1.0`.
- adapter implementation: `0.3.0`.
- `org.sightlint.web` extension: `0.3.0`; strict `0.1.0` and `0.2.0` schemas remain available for
  version dispatch and historical validation.
- Artifact IR: `0.1.0`.
- local workflow report: `0.1.0`; it embeds CheckReport `0.3.0` without changing its verdicts.

Install the locked dependencies and browser once:

```bash
npm --prefix adapters/playwright ci --ignore-scripts
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run build
cargo build --locked -p sightlint-cli
```

## One-command local check

After the preparation above, the bounded agent path captures the repository-owned fixture and
runs the public Rust binary with one command. Human output is the default:

```bash
node adapters/playwright/dist/src/check-cli.js \
  --request evaluation/web/requests/dashboard-browser-unnamed-control.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint
```

Use `--format json` for canonical machine output. The versioned envelope preserves the complete
capture provenance and CheckReport and joins node results to captured native selectors and the
source-bundle file list. A selector is a navigation hint, not an exact source-code line or proof
of cause.

Capture IR and screenshot bytes are held in private temporary storage and removed in all normal
success/error paths. Temporary paths never appear in the report. Exit 0/1 comes from the Rust
check's blocking policy; capture, spawn, usage, unsupported-report, and contract errors exit 2
with empty stdout and one stable diagnostic. Advisory Web findings remain visible with exit 0.

The reviewed E2E uses the same command on an isolated copy of the Atlas unnamed-control mutation,
applies only the human-authored edit from `evaluation/web/annotations/agent-workflow.json`, and
reruns it. It verifies that the named finding disappears and no new failure appears. This scripted
public smoke case is not evidence of autonomous fix selection or representative agent accuracy.

## Public process path

Output files must not already exist. The screenshot reference inside the request is a logical,
repository-relative evidence reference; it does not need to equal the temporary output path.

```bash
capture_dir="$(mktemp -d)"
node adapters/playwright/dist/src/cli.js \
  --request evaluation/web/requests/dashboard-browser-clean.json \
  --repository-root "$PWD" \
  --artifact-ir-out "$capture_dir/artifact-ir.json" \
  --screenshot-out "$capture_dir/screenshot.png" \
  > "$capture_dir/response.json"

target/debug/sightlint check "$capture_dir/artifact-ir.json" --format json
# Optional override: --profile base omits the additive recommended Web rules.
```

Success writes canonical response JSON to stdout, nothing to stderr, and exits 0. Invalid input,
unsupported browser state, resource-limit failure, or capture failure writes one stable diagnostic
to stderr, nothing to stdout, and exits 2. Rule verdicts belong to the subsequent Rust command.
The default recommended Web rules are advisory, while pre-existing base-rule failures retain their
blocking exit behavior.

## Trust, network, and privacy boundary

Protocol `0.1.0` accepts only a repository-contained `file:` HTML entrypoint and exactly one main
frame/page. It rejects path escape, duplicate preferred locators, child frames, more than 200
selected nodes, and output/resource budgets. The context is offline, blocks service workers,
allows no permissions, aborts external requests, and never selects a hosted processor.

The extension does not serialize full HTML or arbitrary text. It keeps a locator-scoped
accessibility root summary and a digest of the complete scoped snapshot; descendants are redacted.
Fixture data is fictional and repository-owned under `MIT OR Apache-2.0`. Do not use
private/customer pages or screenshots as committed test data. Third-party or future real fixtures
require an independent license, redistribution, provenance, and privacy record before commit.

The controlled Atlas fixture has no time-, randomness-, storage-, history-, or network-dependent
output. Protocol `0.1.0` does not virtualize time or random sources in arbitrary applications.
Scrollbar presence is not normalized across operating systems; viewport/document sizes and scroll
offsets are recorded instead.

## Evidence and non-claims

Core geometry is in document CSS pixels. Viewport dimensions, scroll translation, screenshot
extent, layout/render disagreement, and partial screenshot coverage stay explicit in the Web
extension. Native structure never overwrites pixel evidence. Pixel-content identity is
`cantTell` because this version performs no segmentation, OCR, CV, or visual identity matching.

Extension `0.3.0` keeps the `0.2.0` client and scroll sizes, computed
white-space/text-overflow values,
rectangular overflow-ancestor intersections, and the selected element at a render-box-center hit
sample. It additionally names the exact DOM, render, and optional accessibility evidence record
for every selected node so the trusted engine can validate the evidence class, adapter/version,
source digest, local-processing flag, and native locator before executing a rule.
These are measurements, not UX verdicts. A center sample is not a complete hit rectangle; core
`hitBox` remains absent and `hitRegion` is explicitly `cantTell`.

The adapter does not infer semantic peer groups. Consequently, the existing peer-spacing rule is
`inapplicable` for raw adapter output until a later evaluated relation source exists. The
recommended profile consumes only three narrow evidence patterns admitted by ADR 0035; raw
overflow, transforms, repeated dimensions, and screenshots do not become automatic failures. The
reviewed browser acquisition and rule oracles are separate files; captured output is temporary
and is not used to generate either oracle. Passing this synthetic fixture does not establish
real-world Web UI/UX accuracy, accessibility conformance, or blocking rule maturity.

## Validation

```bash
npm --prefix adapters/playwright run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:e2e
```

The E2E executes the actual Node processes and built Rust binary, validates current and historical
strict schemas, checks 23 reviewed clean/mutation/hard-negative/ambiguous/responsive/text-scale and
evidence-matrix cases, exercises recommended/default/base profile behavior and stable
malformed/resource errors, and compares repeated response, IR, screenshot, and rule-report bytes
within one declared compatibility environment. It also checks the workflow report schema,
source-target join, reviewed temporary fix/rerun, human format, operational errors, and retained
abstention/hard-negative behavior. It reports per-rule contract coverage, failure
precision, reviewed abstention, mutation kill rate, and hard-negative failures with explicit
denominators. Linux is the required browser E2E platform; cross-platform screenshot byte identity
is not claimed.
