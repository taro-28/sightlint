# PDF annotation guide 0.1.0

Annotate acquisition from `tools/generate_pdf_fixtures.py` and the authored PDF object plan before
running the adapter. Use original default-user-space rectangles for source facts and independently
calculate top-left CropBox-relative hit rectangles with the transform in ADR 0044. Record
`QuadPoints` and `Path` as activation-geometry abstentions; never substitute their bounding
`Rect` as a core hit area.

Annotate rule verdicts in a separate file. The clean rectangular links pass the existing canvas
bounds rule. The targeted mutation fails only the moved link `hitBox`. The hard negative must not
gain a core node for its `QuadPoints` link and must have no unexpected failure or inferred peer
relation.

Do not copy pypdf values, adapter output, normalized IR, CheckReport, `pdfinfo`, or `pdftoppm`
measurements into ground truth. A legitimate oracle correction requires an explanation against
the generator specification, review of the baseline/mutation/hard-negative relation, and explicit
holdout-leakage consideration. There is currently one maintainer and no independent review or
protected holdout.
