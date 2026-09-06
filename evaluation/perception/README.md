# Perception protocol evaluation

This corpus evaluates the ADR 0042 worker boundary against three existing repository-owned
Northstar Web states. It does not claim OCR, semantic role, hierarchy, peer-group, downstream-rule,
or real-world model accuracy.

The browser companion captures native Artifact IR and a synchronized PNG. The public
`benchmark-image-segmentation` command produces bounded pixel candidates. Because the Atlas shell
has dark and light edge surfaces, the qualified and strict policies abstain; the protocol
evaluation explicitly selects the ranked policy only to exercise bounded mapping. Its regions stay
unconfirmed `visionMeasured` hypotheses. Native and pixel outputs are retained separately during
evaluation; neither rewrites the other or the annotations.

`annotations/acquisition.json` defines expected protocol/family coverage and the native/pixel
facts that must remain visible. `annotations/rules.json` independently records that no semantic
rule is implemented, every outcome remains `untested`, and blocking is forbidden. Implementation
output is never stored as an oracle.

All sources and labels are public development data owned by this repository under
`MIT OR Apache-2.0`. They contain no customer or personal data and are not a protected holdout.

The E2E reports protocol case coverage, region-family coverage, family abstentions, repeated-byte
stability, retained native conflicts, acquisition-mutation observation, and semantic hard-negative
failures. Region/object precision and recall are `untested` because no region-object matching
oracle is asserted for the unsafe ranked baseline. Rule mutation kill rate and OCR/role/hierarchy/
peer/rule accuracy are also `untested`, not zero and not passed.
