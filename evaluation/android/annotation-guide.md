# Android annotation guide 0.1.0

Annotations are authored from the repository fixture source, Android platform capture, and human
visual review before comparing adapter output. Never copy normalized IR or a CheckReport into an
oracle merely to make a test pass.

For acquisition annotations:

- record exact View allocation separately from global-visible and accessibility rectangles;
- preserve an inverted platform rectangle as `invalidPlatformBounds`, never repair it silently;
- use `mappedExactLayout` only for a shown, globally visible, identity-transform View with a
  nonempty unique resource ID and nonempty allocation;
- use `notMappedNotGloballyVisible`, `notMappedNotShown`, `notMappedEmptyLayout`, or
  `notMappedUnsupportedTransform` for excluded nodes;
- never label accessibility bounds as a touch hit region or rendered ink;
- record missing node-to-pixel identity, touch geometry, Compose, and dynamic behavior as
  `cantTell` or `untested`.

For rule annotations, label only the existing `visual.bounds.within-canvas@0.1.0` obligation over
admitted exact View `layoutBox` facts. A failure says the source allocation crosses the declared
screen canvas; it does not prove user harm, accessibility failure, or general UX quality.

Every case records source ownership, license, privacy, public split, false-positive risk,
abstentions, and absence of protected holdout. Any stronger maturity or blocking claim requires a
new decision and independent evidence.
