# Interaction annotation guide 0.1.0

Annotate fixture intent before comparing implementation output.

For acquisition truth, record whether the trace ran, the ordered user-visible states, declared
effect resolutions, offered/activated recovery alternatives, evidence-source families, retained
conflicts, and explicit abstentions. DOM/native state, accessibility semantics, screenshot extent,
and app-declared effect events are distinct facts. Do not infer invisible effects from pixels.

For rule truth, determine applicability from the declared action contract, then record exactly one
of `passed`, `failed`, `cantTell`, `inapplicable`, or `untested` for each evaluated rule. A public
targeted mutation changes one named behavior. A hard negative changes a valid implementation
alternative and must not create a false failure.

Do not copy adapter output into annotations. Do not change an expected outcome solely to make a
test pass. Explain any oracle revision from fixture intent or a versioned rule-contract change.

