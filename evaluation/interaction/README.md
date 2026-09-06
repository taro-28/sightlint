# Interaction evaluation corpus

This directory evaluates the bounded ADR 0047 interaction slice against the repository-owned
Atlas Web fixture. It is product-evaluation regression evidence, not representative Web UI/UX
accuracy.

Acquisition truth and rule-verdict truth are separate. `annotations/acquisition.json` records what
the controlled adapter must observe. `annotations/rules.json` records independently reviewed rule
outcomes. Neither file is generated from adapter output or a CheckReport.

All source, requests, scripts, and annotations are public maintainer-authored development data.
There is no protected holdout or independent review. Fixture content is fictional and released
under `MIT OR Apache-2.0`; the adapter denies external network access and external processing.

