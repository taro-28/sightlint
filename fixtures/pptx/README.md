# Repository-owned PPTX fixtures

`tools/generate_pptx_fixtures.py` deterministically creates these fictional, project-owned PPTX
packages from explicit DrawingML. The files are licensed `MIT OR Apache-2.0`, are safe to
redistribute, and contain no personal, customer, credential, or third-party asset data.

- `atlas-clean.pptx` is the clean baseline.
- `atlas-off-slide-mutant.pptx` changes only Card 6's x offset so it crosses the slide edge.
- `atlas-asymmetric-hard-negative.pptx` keeps all source boxes in bounds while intentionally using
  unequal card widths and gaps with no declared peer relation.

These fixtures are public smoke/development/challenge cases, not a protected holdout and not
evidence of general presentation-quality accuracy. The corresponding acquisition and rule
oracles live separately under `evaluation/pptx/annotations/`.
