# Playwright Web adapter

`sightlint-web` is SightLint's process-isolated, untrusted browser acquisition adapter. It loads a
repository-owned local HTML fixture in the Playwright-pinned Chromium build, captures selected DOM
and accessibility observations, computed layout/render geometry and center hit tests, and a synchronized viewport
screenshot, then writes Artifact IR for the deterministic Rust `sightlint` binary.

The adapter is governed by ADR 0033. It is not part of the Rust kernel and does not decide whether
a UI is good or bad.

## Compatibility

- Node.js: 20 through 24; CI uses Node 24.
- Playwright: exactly `1.63.0` with its matching Chromium build.
- Schema validation: AJV exactly `8.20.0` in development/E2E.
- capture request/response: `0.1.0`.
- `org.sightlint.web` extension: `0.1.0`.
- Artifact IR: `0.1.0`.

Install the locked dependencies and browser once:

```bash
npm --prefix adapters/playwright ci --ignore-scripts
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run build
cargo build --locked -p sightlint-cli
```

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
```

Success writes canonical response JSON to stdout, nothing to stderr, and exits 0. Invalid input,
unsupported browser state, resource-limit failure, or capture failure writes one stable diagnostic
to stderr, nothing to stdout, and exits 2. Rule failures belong to the subsequent Rust command,
which retains its existing exit codes.

## Trust, network, and privacy boundary

Protocol `0.1.0` accepts only a repository-contained `file:` HTML entrypoint and exactly one main
frame/page. It rejects path escape, duplicate preferred locators, child frames, more than 200
selected nodes, and output/resource budgets. The context is offline, blocks service workers,
allows no permissions, aborts external requests, and never selects a hosted processor.

The extension does not serialize full HTML or arbitrary text. It keeps a locator-scoped
accessibility root summary and a digest of the complete scoped snapshot; descendants are redacted.
Fixture data is fictional and repository-owned. Do not use private/customer pages or screenshots
as committed test data. The project license remains unresolved, so the fixture and adapter carry no
independent redistribution grant.

## Evidence and non-claims

Core geometry is in document CSS pixels. Viewport dimensions, scroll translation, screenshot
extent, layout/render disagreement, and partial screenshot coverage stay explicit in the Web
extension. Native structure never overwrites pixel evidence. Pixel-content identity is
`cantTell` because this version performs no segmentation, OCR, CV, or visual identity matching.

The adapter does not infer semantic peer groups. Consequently, the existing peer-spacing rule is
`inapplicable` for raw adapter output until a later evaluated relation source exists. The reviewed
browser acquisition and rule oracles are separate files; captured output is temporary and is not
used to generate either oracle. Passing this synthetic fixture does not establish real-world Web
UI/UX accuracy, accessibility conformance, or rule-pack maturity.

## Validation

```bash
npm --prefix adapters/playwright run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:e2e
```

The E2E executes the actual Node process and built Rust binary, validates both versioned schemas,
checks reviewed clean/mutation/hard-negative/ambiguous/responsive/text-scale cases, exercises
stable malformed/resource errors, and compares repeated response, IR, screenshot, and rule-report
bytes within one declared compatibility environment. Linux is the required browser E2E platform;
cross-platform screenshot byte identity is not claimed.
