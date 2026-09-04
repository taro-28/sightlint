# ADR 0012 — Rule authoring lifecycle

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

SightLint rules may become release gates. A rule that is plausible but poorly calibrated can
create false positives, teach coding agents to game the checker, or make teams disable the
whole system. Rules therefore need a lifecycle rather than becoming blocking when first
implemented.

## Decision

Rules progress through explicit maturity levels:

1. **experimental** — semantics and fixtures are under development; never blocking
2. **advisory** — stable enough for reports, with documented limitations; non-blocking
3. **candidate** — measured on representative fixtures and mutation tests; optionally blocking
   only through explicit project opt-in
4. **recommended** — validated precision, coverage, determinism, and false-positive handling;
   eligible for recommended policy packs
5. **deprecated** — retained for compatibility while migration guidance is available

Promotion requires rule-specific evidence, not only code coverage. At minimum, a mature rule
has passing, failing, non-applicable or unknown, boundary, and mutation fixtures. Rule version
and maturity appear in machine-readable metadata.

## Consequences

- The default ruleset can remain trustworthy as the ecosystem grows.
- New research and model-assisted rules can ship without pretending to be release-grade.
- Promotion adds calibration and fixture work.
- A rule may be useful before it is safe to block CI.

## Alternatives considered

- Boolean enabled/disabled status: does not express quality or adoption readiness.
- All rules advisory forever: safe but prevents reliable automation.
- Immediate blocking after implementation: fast but too risky for a quality gate.

## Verification

M1 rule metadata reserves a maturity field. Policy resolution rejects default blocking for
experimental and advisory rules. Promotion changes include documented measurements and
fixtures in the same pull request.
