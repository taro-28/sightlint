# PDF source-adapter evaluation

This directory evaluates the bounded PDF process defined by ADR 0044. It is public development
data for one fictional repository-owned report family, not a representative PDF sample or a
protected holdout.

The data layers are deliberately separate:

- `annotations/acquisition.json` is authored from the fixture generator specification and states
  page, annotation, coordinate-transform, action-class, render-extent, and abstention truth;
- `annotations/rules.json` states the existing bounds-rule outcomes independently;
- adapter response, Artifact IR, CheckReport, and renderer output are implementation results and
  are never stored as an oracle;
- `metric-contract.json` defines the measurements computed in memory by public E2E.

`tools/generate_pdf_fixtures.py --check` verifies deterministic source fixtures. Committed render
PNGs are reviewed differential evidence generated separately with Poppler 26.05.0 at 72 DPI using
the CropBox; CI checks their digests and public PNG path but does not regenerate them.

All inputs and labels are fictional SightLint project data under `MIT OR Apache-2.0`. They contain
no personal, customer, credential, or third-party document data. The adapter makes no network
request and does not follow actions or destinations. Output still exposes source-derived path,
digest, page/object, type, and geometry metadata and should be handled as sensitive.
