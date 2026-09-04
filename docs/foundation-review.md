# Foundation review checklist

This checklist is the acceptance gate for milestone M0. Functional SightLint implementation
must not begin until these items are accepted or explicitly deferred.

## Product and scope

- [ ] SightLint is defined as evidence-backed linting for interfaces and visual artifacts.
- [ ] Cross-artifact scope is an architectural constraint, not an instruction to build every
  adapter immediately.
- [ ] The project does not promise universal beauty, usability, or a trusted aggregate score.

## Trust and determinism

- [ ] The Rust kernel is the trusted deterministic boundary.
- [ ] Identical normalized input, engine, rules, and configuration produce identical output.
- [ ] Probabilistic perception is isolated and marked with provenance and uncertainty.
- [ ] `cantTell`, `inapplicable`, and `untested` are first-class outcomes.
- [ ] Model-only opinions are non-blocking by default.

## Data model

- [ ] The Artifact IR is medium-neutral and language-neutral at process boundaries.
- [ ] Layout, render/ink, and hit geometry remain distinct.
- [ ] Every measurable value has an explicit unit and coordinate space.
- [ ] Native structure and rendered reality may coexist and conflict without being overwritten.
- [ ] Medium-specific details use versioned extensions.

## Rule model

- [ ] Executable rules are atomic or explicitly composite.
- [ ] Rules declare input aspects, applicability, expectation, tolerances, and evidence.
- [ ] Severity, confidence, evidence strength, and outcome are independent.
- [ ] Project policy takes precedence over inferred norms and universal fallbacks.

## Testing and quality

- [ ] CI checks format, Clippy, tests, docs, MSRV, Linux, macOS, and Windows.
- [ ] The strategy includes contract, property, golden, mutation, metamorphic, differential,
  determinism, and end-to-end tests.
- [ ] Rules must earn blocking status through per-rule evidence.

## Privacy and security

- [ ] Core analysis is local-first and performs no network I/O.
- [ ] Adapters are untrusted sensors with process and schema boundaries.
- [ ] Remote perception is explicit opt-in with visible data transmission.
- [ ] Real customer or private artifacts are prohibited as repository fixtures.

## Development discipline

- [ ] `AGENTS.md` is normative for coding agents.
- [ ] Architectural changes require ADRs before implementation.
- [ ] Roadmap milestone exit criteria control scope.
- [ ] Licensing remains an explicit unresolved decision and crates are not publishable yet.

## M0 exit

M0 is accepted when this checklist reflects the maintainer's intent and the remote CI is green.
The next pull request may then implement M1: the deterministic JSON IR vertical slice.
