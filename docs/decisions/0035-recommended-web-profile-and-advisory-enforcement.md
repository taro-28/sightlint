# ADR 0035 — Recommended Web profile and advisory enforcement

- Status: Accepted
- Date: 2026-09-06
- Issue: #24
- Owners: @taro-28

## Context

ADRs 0032–0034 provide a repository-owned Web fixture, independently reviewed acquisition and
rule oracles, and a process-isolated Playwright adapter. The adapter can now prove selected native
semantics, document/render geometry, rectangular ancestor clipping, and one render-box-center hit
sample. It still cannot prove semantic peer membership, complete hit regions, pixel identity, or
whether arbitrary overflow and overlays are defects.

SightLint needs useful checks without per-project IR authoring. Adding every measured anomaly as a
failure would create false positives, while making early findings affect the process exit code
would silently promote public development data into blocking evidence. The profile name, Web
extension compatibility boundary, policy provenance, and enforcement behavior therefore need a
decision before implementation.

## Decision

### Profiles and execution

The public `check` and `check-image` commands select `sightlint:recommended` by default. This
profile is additive: it runs the existing base/explicit rules and, when a validated Web extension
is present, the admitted recommended-Web rules. `--profile base` is the initial explicit override
and runs only the pre-existing base/explicit rules. Profile selection is recorded in the report.

The Rust API keeps `check(document)` as the recommended default and adds an options-bearing entry
point for explicit profile selection. Profiles select rules; they do not change acquired facts or
rewrite Artifact IR.

### First admitted Web rules

The first pack contains three atomic, medium-specific rules:

1. `web.accessibility.interactive-name@0.1.0` checks a visible DOM-interactive node only when the
   platform accessibility snapshot exposes a role in a conservative UI-control applicability set.
   Its expectation source is WCAG 2.2 Success Criterion 4.1.2, “Name, Role, Value.” A missing
   programmatically determined name fails; missing or unadmitted platform semantics becomes
   `cantTell`; non-interactive or non-rendered nodes are `inapplicable`.
2. `web.interaction.center-hit@0.1.0` checks a visible, enabled, untransformed native control whose
   center lies in the captured viewport and whose exact browser sample used
   `elementFromPoint`. A hit on the control or its descendant passes; an unrelated, non-dialog
   blocker fails. A dialog or other unresolved overlay, clipping, off-viewport position, zero
   area, pointer-event override, or incomplete sample becomes `cantTell` or `inapplicable` rather
   than a guessed failure. The result describes only the sampled point, never the complete hit
   region.
3. `web.interaction.ancestor-clip@0.1.0` checks a visible, enabled, untransformed native control
   against the adapter's rectangular ancestor reconciliation. Partial or full clipping by
   `hidden`/`clip` ancestors fails. Scrollable `auto`/`scroll` ancestors, non-rectangular effects,
   and unavailable reconciliation become `cantTell`; an unclipped control passes.

The exact rule titles, applicability, evidence, alternatives, messages, and fixture targets are
part of the rule/evaluation contract. Measurements such as overflow, transformed text, repeated
peer dimensions, and responsive differences remain observations, not automatic verdicts.

### Maturity and enforcement

All three rules start at `advisory` maturity because the public corpus contains one fictional
application family, maintainer review only, and no frozen private holdout. Their failed outcomes
are reported but set `enforcement: advisory`, so they do not cause exit code 1. Existing rule exit
behavior remains unchanged in this compatibility slice. Future promotion requires a separate
reviewed decision and representative rule-specific evidence; confidence, severity, maturity,
outcome, and enforcement stay separate.

Every recommended result records:

- selected profile;
- policy source kind, stable identifier, version, and reference;
- enforcement (`advisory` or `blocking`);
- exact evidence identifiers and classes;
- the observed value or method in measurements/message where applicable.

This evolves CheckReport to `0.3.0`. The previous report version remains documented; no consumer
may silently interpret new policy/enforcement fields as `0.2.0`.

### Web extension and trusted boundary

`org.sightlint.web` evolves to `0.3.0`. The adapter adds explicit DOM, render, and optional
accessibility evidence identifiers to every selected node. The previously adapter-private payload
becomes an official optional extension consumed by the trusted Rust engine only after strict
versioned decoding and semantic validation. The `0.1.0` and `0.2.0` schemas remain available.

Validation proves unique node/reconciliation membership, core node and evidence references,
matching node sets, finite values, valid evidence classes, matching adapter/version/source-digest/
local-processing/native-selector provenance, and consistent hit-sample method/outcome combinations.
Browser acquisition remains outside Rust. The kernel consumes normalized records deterministically
and never launches Playwright or reads screenshots.

### Evaluation contract

The Web rule oracle evolves independently from acquisition annotations. For every admitted rule it
records policy provenance, maturity, enforcement, applicability, expected outcome, false-positive
and false-negative risks, qualitative severity inputs without deriving a severity label, and a
clean/mutation/hard-negative or abstention relation. Implementation output is never copied into
the oracle.

The public browser E2E reports per-rule decision coverage, precision with explicit denominators,
false-positive failures, correct abstention, and mutation kill rate. Public smoke/development/
challenge data remains non-holdout. No broad Web, WCAG-conformance, real-world precision, or
universal UX-quality claim is made.

## Consequences

- A default capture-then-check workflow reports three narrow Web findings without per-element rule
  authoring.
- Advisory findings can guide an agent without silently becoming a CI gate.
- Consumers can identify the selected profile, policy authority, and enforcement directly from
  canonical JSON.
- The trusted kernel gains a Web-specific optional-extension module while the mandatory core IR
  remains medium-neutral.
- The adapter/schema/report compatibility surface expands and requires old/current schema tests.
- A combined one-command capture/check/fix/rerun workflow remains follow-up work in issue #34.

## Alternatives considered

### Fail every measured overflow, transform, or peer outlier

Rejected because measurement does not prove applicability or harmful UX. Existing hard negatives
show intentional grouping, ellipsis, and overlays.

### Emit findings only from the Node adapter

Rejected because rule policy and verdicts must run in the deterministic trusted kernel. The
adapter remains an untrusted sensor.

### Copy Web observations into a second rule-specific extension

Rejected because it would duplicate facts and weaken reconciliation. The engine validates and
consumes the versioned Web extension directly.

### Make new failures blocking by default

Rejected because the current public data is development evidence, not representative validation
or a private holdout. Advisory results preserve usefulness without overstating maturity.

### Require a project configuration file for the first profile

Deferred. A default profile plus one CLI override proves zero-setup behavior with a small stable
surface. Scoped per-rule/project overlays require a later schema once real usage demonstrates the
needed precedence and exception model.

## Verification

- Keep strict Web extension 0.1/0.2 schemas and validate current 0.3 adapter output.
- Reject unsupported/malformed Web payloads before a dependent rule executes.
- Add clean, targeted mutation, `cantTell`, `inapplicable`, and hard-negative cases for each rule
  where meaningful.
- Exercise the real Node process and built `sightlint` binary under both `recommended` and `base`.
- Require advisory failures to remain exit code 0 and existing base-rule exits to remain stable.
- Report per-rule precision, coverage, abstention, false-positive, and mutation-kill counts.
- Compare response, IR, screenshot, report, diagnostics, and exit codes byte for byte on repeated
  same-environment runs.
- Preserve generator drift, all Rust/CLI/PNG/image/product/Web corpora, rustdoc, Rust 1.85.0, and
  Linux/macOS/Windows gates.
