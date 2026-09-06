# SightLint

**Deterministic, evidence-backed visual linting for interfaces and artifacts.**

SightLint is an architecture-first project for finding visual and interaction-quality problems in
web interfaces, mobile applications, slides, documents, PDFs, and images. It is designed for both
humans and coding agents.

> **Status: `v0.1.0-alpha.2`.** Do not depend on the current API. General screenshot-only UI/UX defect
> detection is not implemented. Current image inspection supplies a narrow, advisory-only region
> and gap observation under explicit assumptions; it is not a semantic UX pass/fail verdict. Green
> synthetic tests are not evidence of real-world design-review accuracy.

## Continue in local Codex

The repository is the complete handoff from the earlier remote/mobile development session.

Start with [`CODEX.md`](CODEX.md), then follow [`AGENTS.md`](AGENTS.md) and
[`docs/handoff.md`](docs/handoff.md). The handoff records the exact current capability, historical
PR disposition, branch and hosting limitations, complete validation commands, open decisions, and
the canonical issue sequence. Product reasoning and rejected alternatives are preserved in:

- [`docs/product-rationale.md`](docs/product-rationale.md)
- [`docs/decision-history.md`](docs/decision-history.md)
- [`docs/evaluation-strategy.md`](docs/evaluation-strategy.md)
- [`docs/roadmap.md`](docs/roadmap.md)

Always start from the latest green `main`. Closed Draft PRs #12–#17 and their branches are
historical reference only and must not be reopened, merged, or used as a base. Their remaining
value is preserved in current issues.

The bounded first-alpha execution epic,
[Issue #34](https://github.com/taro-28/sightlint/issues/34), is complete:

1. [#22](https://github.com/taro-28/sightlint/issues/22) — realistic human-reviewed UI evaluation
   foundation (complete);
2. [#23](https://github.com/taro-28/sightlint/issues/23) — Playwright native/pixel web adapter and
   acquisition evidence matrix (complete);
3. [#24](https://github.com/taro-28/sightlint/issues/24) — first evaluated zero-setup recommended
   Web pack (complete);
4. [#42](https://github.com/taro-28/sightlint/issues/42) — one-command Codex
   edit/check/fix/rerun path (complete);
5. [#33](https://github.com/taro-28/sightlint/issues/33) — license, compatibility, packaging, and
   first alpha release (complete).

The #25 benchmark is complete without changing the strict image-inspection default. Issue #26 adds
exact source-alpha geometry without introducing a padding rule. Issue #27 is complete through ADR
0041 without broadening PNG coverage because current product evidence did not establish a format
gap. The earliest remaining implementation gate is
[#28](https://github.com/taro-28/sightlint/issues/28).

## Why

AI can generate plausible artifacts quickly, but basic quality failures still slip through:
inconsistent spacing and typography, clipping, overlap, responsive breakage, missing states,
unsafe actions, inaccessible controls, and misleading feedback.

Static linters understand code but often not the rendered result. Pixel diffs detect change but
not whether it is good. Accessibility tools cover an important subset. Design-system checks do
not cover every relationship or state. Free-form AI critique can be useful but is difficult to
reproduce, audit, version, evaluate, and trust as a release gate.

SightLint separates the problem into explicit stages:

```text
native structure / pixels / interaction traces
                    │
                    ▼
              untrusted adapters
                    │
                    ▼
   observations + provenance + uncertainty
                    │
                    ▼
       validation and reconciliation
                    │
                    ▼
             Artifact IR
                    │
                    ▼
     applicability + policy resolution
                    │
                    ▼
       deterministic queries and rules
                    │
                    ▼
 passed / failed / inapplicable / cantTell / untested
```

The intended invariant is:

> Given the same normalized observations, rule versions, configuration, engine version, and
> declared compatibility environment, SightLint produces the same canonical results.

Probabilistic perception may help construct observations, but inferred meaning must not be
disguised as fact. Confidence, alternatives, uncertainty, and source conflicts remain data.

## Product requirement

The north star is not “measure a screenshot.” It is:

> A coding agent should apply ordinary UI/UX and artifact-quality fundamentals even when the user
> did not remember to enumerate spacing, typography, accessibility, layout, and interaction rules
> in every prompt.

That requires reusable policy packs, rich acquisition, explicit applicability, evaluation, and
explainable deterministic obligations. It does not require one universal visual style or opaque
quality score.

Policy precedence is:

1. explicit project contract or exception;
2. exact design-system or platform contract;
3. statistically inferred project norm with visible confidence;
4. platform convention;
5. conservative built-in baseline.

Every result should state which policy supplied the expectation.

## Current verified capabilities

### Artifact IR and deterministic rule engine

The Rust kernel supports:

- versioned, medium-neutral Artifact IR;
- semantic validation and canonical serialization;
- stable IDs, evidence selectors, confidence, and uncertainty;
- explicit units and coordinate spaces;
- separate layout, render/ink, and hit geometry;
- atomic rules and evidence-linked human/JSON reports;
- `passed`, `failed`, `inapplicable`, `cantTell`, and `untested`;
- stable CLI exit behavior and byte-level determinism tests.

When sufficient facts and explicit relations/policies exist, current visual contracts cover:

- bounds within a canvas;
- declared non-overlap;
- peer spacing consistency;
- parent containment;
- logical alignment;
- peer width/height consistency;
- peer typography consistency;
- project-supplied minimum font-size policy;
- direction, unit, tolerance, and ambiguity behavior.

This does not mean all required facts or peer relationships can be inferred from an arbitrary
screenshot.

### Zero-setup recommended Web rules

`sightlint check` now selects `sightlint:recommended` by default. For a validated
`org.sightlint.web@0.3.0` extension, the deterministic Rust kernel emits three narrow advisory
rules for programmatic control names, one render-box-center hit sample, and rectangular clipping
of native controls by non-scrollable ancestors. Each result records its profile, policy source,
maturity, enforcement, and exact DOM/render/accessibility evidence identifiers.

`--profile base` keeps the pre-existing explicit/base rules and omits the recommended Web rules;
it does not skip validation of a recognized Web extension. Recommended failures are advisory and
therefore do not fail the default process gate. This first pack is evaluated only on the public,
repository-owned fictional application, so it does not establish WCAG conformance, complete hit
regions, representative real-world precision, or blocking maturity.

### Deterministic PNG acquisition

The current M3 path performs:

```text
PNG signature/IHDR validation
  -> bounded complete chunk/order/CRC validation
  -> bounded IDAT zlib/DEFLATE inflation
  -> all five scanline-filter reconstructions
  -> non-interlaced or Adam7 handling
  -> row-major PNG-encoded RGBA8 for supported inputs
```

Supported raster inputs are eight-bit grayscale, RGB, grayscale-alpha, and RGBA without `tRNS`.
Palette/indexed, sub-byte, 16-bit, `tRNS`, animation, and over-budget cases are explicitly
unavailable instead of guessed.

ADR 0041 keeps that boundary after a versioned format-demand assessment found that all five
repository PNG assets and the nine pinned-browser product captures use the supported subset.
Unsupported formats remain conformance controls rather than product-demand evidence; no decoder
dependency, automatic conversion, compatibility change, prevalence claim, or protected holdout is
introduced. A future observed gap requires a new issue and ADR.

The bytes are unassociated PNG-encoded samples, not display-corrected sRGB or linear-light color.
No gamma/ICC/chromaticity transform or alpha compositing is applied, so these values alone cannot
support a trusted colorimetric or contrast verdict. Raw pixels remain inside the adapter API;
serialized IR contains bounded metadata, checksum, and provenance.

For supported rasters, `alphaGeometry@0.1.0` makes one deterministic source-sample pass and records
half-open bounds for `alpha > 0` and `alpha == 255`, exact alpha-class counts, transparent insets,
and visible edge occupancy. A dedicated exact-source evidence item links nonempty visible bounds
to the image node's device-pixel `inkBox`; entirely transparent images omit that box. These facts
do not establish composited display visibility, semantic whitespace, clipping, alignment intent,
or a UI/UX defect, and no alpha-based rule or blocking result exists.

### Advisory image-region inspection

`inspect-image` uses one deliberately strict acquisition hypothesis:

- all raster pixels are opaque;
- the complete perimeter has one exact RGBA value;
- that value is recorded as an unconfirmed background candidate;
- four-connected non-candidate regions are extracted within fixed budgets;
- groups require at least three same-size, same-color solid rectangles aligned in one row/column;
- foreign regions intersecting the intervening strip prevent grouping;
- exact device-pixel gaps are reported as `uniform` or `unequal` observations.

The committed clean/mutated pair yields `[1, 1]` and `[1, 2]`. Unequal gaps produce a nonblocking
advisory, while `uxVerdict` remains `cantTell`: identical pixels could express intentional grouping.

This prototype does not generally support text, rounded cards, shadows, gradients, photos,
antialiasing, hierarchy, semantic roles, or design intent.

### Evaluation-only segmentation benchmark

`benchmark-image-segmentation` compares the unchanged strict perimeter/flood-fill policy with a
ranked exact-border candidate and a 95%-qualified corner/row-run candidate. Its versioned canonical
report records candidates, exact device-pixel regions, evidence, resource counters, and explicit
abstention. It never emits a rule verdict, never blocks, and does not affect `inspect-image`.

The nine-case repository-owned Northstar fixture includes a targeted edge mutation, metamorphic
variants, split-pane and gradient hard negatives, and checkerboard resource stress. It shows that
the 95% policy narrowly recovers edge contamination, while realistic shadows still merge semantic
surfaces and the ranked policy selects both unsafe hard negatives. No policy is admitted as the
product default; see [`evaluation/image-segmentation/results.md`](evaluation/image-segmentation/results.md).

## Install and current commands

The first alpha is a source-only GitHub prerelease with a deterministic archive and SHA-256
checksum. Verify, build, and remove it using [`docs/release.md`](docs/release.md); read the
surface-specific guarantees in [`docs/compatibility.md`](docs/compatibility.md). Prebuilt binaries
and registry packages are not published.

From a verified source tree:

```bash
# Structured Artifact IR check.
cargo run --locked -p sightlint-cli -- \
  check fixtures/e2e/pass-web.json

# Canonical rule report.
cargo run --locked -p sightlint-cli -- \
  check fixtures/e2e/pass-web.json --format json

# Explicitly opt out of the additive recommended profile.
cargo run --locked -p sightlint-cli -- \
  check fixtures/e2e/pass-web.json --profile base --format json

# Validate/adapt supported PNG source facts.
cargo run --locked -p sightlint-cli -- \
  adapt-image screenshot.png

# Adapt a PNG and run the trusted rule engine.
cargo run --locked -p sightlint-cli -- \
  check-image screenshot.png --format json

# Obtain advisory region and gap observations.
cargo run --locked -p sightlint-cli -- \
  inspect-image screenshot.png --format json

# Compare three nonblocking segmentation hypotheses in canonical JSON.
cargo run --locked -p sightlint-cli -- \
  benchmark-image-segmentation screenshot.png

# Binary stdin is supported.
cat screenshot.png | cargo run --locked -p sightlint-cli -- adapt-image -

# Explicitly deny cantTell in a trusted check policy.
cargo run --locked -p sightlint-cli -- \
  check fixtures/e2e/cant-tell-missing-box.json --deny-cant-tell

# Canonicalize valid IR and expose the schema/version.
cargo run --locked -p sightlint-cli -- normalize fixtures/e2e/pass-web-shuffled.json
cargo run --locked -p sightlint-cli -- schema
cargo run --locked -p sightlint-cli -- version
```

For trusted checks, exit codes are:

| Code | Meaning |
|---:|---|
| `0` | no blocking failure; advisory failures and `cantTell` do not fail unless explicitly denied |
| `1` | a blocking rule failed, or strict policy denied `cantTell` |
| `2` | usage, I/O, decoding, adapter, or semantic-validation error |

`inspect-image` never exits 1 for a heuristic. Observed or explicitly unavailable coverage exits
0; malformed/usage/execution failure exits 2.

### Current local Web agent sequence

After installing the locked adapter/browser dependencies described in
[`adapters/playwright/README.md`](adapters/playwright/README.md), build both local processes once:

```bash
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run build

node adapters/playwright/dist/src/check-cli.js \
  --request evaluation/web/requests/dashboard-browser-unnamed-control.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --format json
```

Omit `--format json` for a stable color-free human report. The machine envelope preserves capture
and runtime provenance, the unchanged CheckReport, and exact node-ID joins to native selectors and
the loaded source-bundle files. The selector is a navigation hint rather than exact source-line
causality. Temporary IR and screenshot files are removed, and the Node layer never issues or
modifies a verdict.

The reviewed E2E runs this same command on the unnamed-control mutation, applies one independently
authored edit to an isolated fixture copy, and reruns it. It checks that the named finding is gone
and no new failure appears; it does not let the edit itself declare success.

## Executable verification

SightLint separates conformance, acquisition evaluation, semantic rule evaluation, and eventual
user-outcome evidence. See [`docs/evaluation-strategy.md`](docs/evaluation-strategy.md).

Current committed assets include:

- generated `fixtures/e2e/` Artifact IR conformance data;
- a versioned synthetic rule smoke oracle under `evaluation/`;
- **38 PNG raster cases** with exact independent pixel, unavailable, or malformed outcomes;
- **30 image-inspection cases** with independent region/gap, abstention, and malformed outcomes;
- a nine-case realistic image-segmentation benchmark with separate acquisition/rule oracles,
  hard negatives, abstention, targeted mutation, metamorphic relations, and bounded refusal;
- a repository-owned realistic Web fixture foundation with six separately annotated acquisition
  and rule cases, including one targeted mutation and one intentional-grouping hard negative;
- a 23-case Playwright companion that captures selected DOM/accessibility structure, computed
  geometry, client/scroll overflow, ancestor clipping, center-hit samples, writing direction, and
  a synchronized viewport screenshot through a separate Node process, then evaluates the public
  binary's base and zero-setup recommended profiles;
- a versioned one-command agent report and one reviewed temporary source-edit/fix/rerun case with
  byte-stable JSON/human output, native source navigation, and retained ambiguity/hard-negative
  controls;
- targeted mutations, hard negatives, budget boundaries, file/stdin/API comparisons, and repeated
  byte-identical results.

Normal read-only CI verifies corpus drift, rustfmt, Clippy with denied warnings, all tests, explicit
public E2E, rustdoc, Rust 1.85.0, and Linux/macOS/Windows. Public behavior is incomplete until the
exact final PR head and the merged `main` commit both pass.

Synthetic and repository-owned regression data does not establish real-world precision. The
#22–#24 slices define a reviewed Web evaluation contract, one controlled local browser path, and
three advisory rules, but representative sampling, independent review, semantic peer inference,
complete hit regions, pixel-content identity, and a protected holdout process remain future work.

## Architecture

```text
Rust kernel       deterministic IR, validation, queries, rules, reports
Adapters          browser, image, PPTX, PDF, Android, iOS, traces
Perception        optional isolated OCR/CV/VLM workers
Integrations      CLI, CI, Codex/MCP, GitHub, editor, future local UI
```

The kernel does not run a browser/model, fetch network resources, interpret every source format,
or use wall-clock/random/locale defaults. Adapter languages may match their platforms: TypeScript
for Playwright, Kotlin for Android, Swift for iOS, Python for perception experiments, and Rust for
bounded deterministic primitives.

Read:

- [`docs/handoff.md`](docs/handoff.md)
- [`docs/product-rationale.md`](docs/product-rationale.md)
- [`docs/decision-history.md`](docs/decision-history.md)
- [`docs/vision.md`](docs/vision.md)
- [`docs/principles.md`](docs/principles.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/artifact-ir.md`](docs/artifact-ir.md)
- [`docs/rules.md`](docs/rules.md)
- [`docs/testing-strategy.md`](docs/testing-strategy.md)
- [`docs/evaluation-strategy.md`](docs/evaluation-strategy.md)
- [`docs/roadmap.md`](docs/roadmap.md)
- [`docs/decisions/README.md`](docs/decisions/README.md)
- [`docs/development.md`](docs/development.md)
- [`docs/compatibility.md`](docs/compatibility.md)
- [`docs/dependency-policy.md`](docs/dependency-policy.md)
- [`docs/release.md`](docs/release.md)

## Preserved backlog

| Issue | Purpose |
|---|---|
| #22 | realistic human-reviewed evaluation gate |
| #23 | Playwright web adapter and reconciliation |
| #24 | zero-setup recommended rule packs |
| #25 | completed background/segmentation benchmark research; no production admission |
| #26 | completed exact source-alpha transparent-asset geometry; no rule admitted |
| #27 | completed PNG format-demand/decoder strategy decision; broader coverage not admitted |
| #28 | isolated OCR/CV/VLM worker protocol |
| #29 | PPTX, PDF/document, Android, and iOS adapter roadmap |
| #30 | interaction states, effects, traces, and recovery |
| #31 | Codex, MCP, GitHub Checks, editor/local UI ecosystem |
| #33 | completed license, compatibility, source packaging, and alpha release gate |
| #34 | completed first evidence-backed zero-setup web UI alpha epic |

Issue state alone does not prove implemented behavior. New architecture decisions continue at ADR
0042 or later. Historical branch-only ADRs 0025–0029 are reference material and are mapped to
current issues in the ADR index. Administrative issues #19 and #32 are complete: GitHub now
enforces the documented `main` ruleset and automatically removes merged head branches, and the
legacy branch set has been pruned.

## Development rules

- start from the latest green `main`;
- one focused issue, branch, and PR;
- ADR before architecture/schema/protocol/trust/policy changes;
- no self-writing feature workflows;
- no placeholder/final/review/ready/`v2` branch chains;
- no unconnected implementation;
- public-binary/process E2E and independent oracles;
- hard negatives and conservative abstention;
- exact final-head and post-merge CI;
- update handoff and roadmap when facts change;
- local-first and no artifact upload by default.

The complete local gate is in [`AGENTS.md`](AGENTS.md) and
[`docs/development.md`](docs/development.md).

GitHub's active `Protect main` ruleset requires a pull request, an up-to-date branch, the five
documented CI contexts, linear history, and resolved review conversations. It blocks force pushes
and deletion without routine bypass. Squash is the only merge method, and merged head branches are
deleted automatically. A green CI badge remains commit-specific evidence, not a substitute for
checking the exact protected head and post-merge `main` run.

## What SightLint is not

SightLint is not intended to:

- prove that an artifact is universally beautiful, persuasive, or usable;
- replace representative user research;
- assign one opaque universal UX score;
- let an LLM make the final blocking decision;
- force every medium into DOM/CSS concepts;
- require a hosted service or artifact upload;
- treat every visual difference as a defect;
- let a coding agent grade its own edit without rerunning the checker.

## License and release

SightLint is licensed under your choice of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE). Repository-authored fictional fixtures, generated corpus data, and
documentation use the same terms unless a more specific notice says otherwise. Third-party
dependencies and browser downloads retain their own licenses; see
[`docs/dependency-policy.md`](docs/dependency-policy.md).

`v0.1.0-alpha.2` is distributed as a source archive/checksum GitHub prerelease. Rust crates remain
`publish = false` and the Node package remains private; no prebuilt binary, crates.io/npm package,
container, signature, or attestation is claimed.

The immutable `v0.1.0-alpha.1` tag is an unpublished failed workflow attempt, not a supported
release. ADR 0038 records the read-only draft-asset failure and alpha.2 recovery without moving the
old tag.
