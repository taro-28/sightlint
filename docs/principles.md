# Project principles

These principles constrain implementation choices and rule design.

## 1. Deterministic after normalization

Parsing and perception may be uncertain. Once observations are normalized into a versioned
IR, rule execution must be deterministic for a fixed engine, configuration, and rule set.

## 2. Evidence before verdict

A result is useful only when it identifies the observed facts, expected relationship, target,
and provenance. Explanation is not evidence by itself.

## 3. Uncertainty is data

Confidence, error bounds, conflicting observations, and missing information must survive the
pipeline. Ambiguity becomes `cantTell`; it must not be rounded into certainty.

## 4. Native structure first, pixels always available

Native DOM, accessibility, slide, document, and platform structures are normally more exact.
Rendered pixels expose the visual reality and are the common denominator. The strongest
analysis reconciles both.

## 5. Separate layout, render, and interaction geometry

A layout box, visible ink/render bounds, and a clickable or tappable hit box answer different
questions. They must never be collapsed into one generic rectangle.

## 6. Policy, not taste

Executable rules should verify narrow obligations. Broad aesthetic critique can exist as an
advisory extension, but it is not part of the trusted blocking kernel.

## 7. Atomic rules, composable obligations

Rules should be small enough to test and explain. Composite rules may accept multiple valid
ways to satisfy an obligation, such as confirmation, undo, recoverable trash, or version
history.

## 8. Semantics over implementation details

Rules should target roles, relationships, effects, and user-visible outcomes rather than a
specific framework, CSS class, or component library.

## 9. Adapters are untrusted sensors

An adapter may misparse, omit, or infer information. The kernel validates its output and
tracks provenance instead of granting it authority.

## 10. Blocking requires strong evidence

Deterministic source facts and reproducible traces may block CI. A model-only semantic guess
is advisory unless explicitly confirmed by a project contract.

## 11. Local-first and private by default

Core analysis runs locally. Network transmission, hosted models, and telemetry are explicit
choices with visible data boundaries.

## 12. Project policy precedes inferred norms

When selecting expectations, use this order:

1. explicitly declared project contract
2. exact design-system or platform contract
3. statistically inferred project norm
4. platform convention
5. conservative universal baseline

The report must state which level supplied the expectation.

## 13. Reproducibility over breadth

A small number of measured, well-tested rules is preferable to a long checklist of unstable
AI opinions.

## 14. No universal quality score in the trusted core

Outcome, severity, confidence, coverage, and evidence strength are independent. Aggregation
may be a presentation layer, never a substitute for individual results.

## 15. User evidence remains authoritative

Formal checks catch recurring defects. Real users determine whether the underlying product
model, language, trust, and workflow are appropriate.
