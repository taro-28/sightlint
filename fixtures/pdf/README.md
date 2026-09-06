# PDF fixtures

The three PDF 1.7 files are deterministic outputs of
`python3 tools/generate_pdf_fixtures.py`. Do not hand-edit them.

They describe one fictional Atlas operating-review report:

- `atlas-clean.pdf` has three rectangular internal Link annotations inside an explicit Letter
  CropBox;
- `atlas-off-page-mutant.pdf` changes only object `9 0 R` so its Link rectangle extends past the
  right page edge; visible content is unchanged;
- `atlas-quadpoints-hard-negative.pdf` uses an asymmetric page composition and `QuadPoints` for
  object `9 0 R`, so its bounding Rect must not become an exact core hit box.

The inputs are project-owned test data under `MIT OR Apache-2.0` and contain no real customer,
credential, personal, or third-party document content. They intentionally include fictional text
sentinels so E2E can prove that source text and metadata do not leak into adapter output.
