# Exact source-alpha geometry evaluation

This ADR 0040 corpus evaluates transparent PNG assets through the existing public `adapt-image`
and `check-image` commands. It uses a realistic fictional Northstar asset family rather than only
small rectangle conformance inputs.

The PNG bytes are produced by an independent deterministic fixture generator from reviewed drawing
primitives. `annotations/acquisition.json` and `annotations/rules.json` are maintained separately
and are never generated from SightLint output. Acquisition truth covers alpha predicates, bounds,
counts, insets, edge occupancy, evidence, and `inkBox`; rule truth remains `untested` and
nonblocking.

All cases and labels are public development data. The assets are repository-owned, fictional,
contain no personal/customer data or external resources, and use the repository's
`MIT OR Apache-2.0` license. There is no independent review or protected holdout.

Exact source alpha does not prove composited visibility, semantic whitespace, alignment quality,
or a UI/UX defect. This corpus must not be reported as general image or real-world UI/UX accuracy.
