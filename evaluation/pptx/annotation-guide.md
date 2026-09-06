# PPTX annotation guide 0.1.0

The acquisition oracle is transcribed from `tools/generate_pptx_fixtures.py` and the DrawingML
fixture specification before the adapter is executed. Reviewers calculate group coordinates from
the declared `off`, `ext`, `chOff`, and `chExt` terms and independently calculate source-text
SHA-256 digests. They do not paste adapter output into `annotations/acquisition.json`.

The rule oracle is a separate review of those acquisition facts against the published
`visual.bounds.within-canvas@0.1.0` contract. A source rectangle is `failed` only when its exact
`layoutBox` exceeds the exact EMU slide canvas. Source containment does not establish rendered
visibility or general slide quality.

The clean baseline and targeted mutation differ only in Card 6's source x offset. The challenge
case deliberately changes card widths, labels, and gaps without declaring a peer relation. It is
a hard negative against inventing spacing intent from visual proximity.

Each render is independently reviewed for the intended fixture variant and retained as a fixed
byte artifact. It was produced by LibreOfficeDev `26.8.0.0.alpha0` at commit
`2c87e51eeaa2b413ff4ae097b2705eea1995d8e5`, using headless PPTX-to-PDF conversion followed by
`pdftoppm -png -scale-to-x 960 -scale-to-y 540 -singlefile`. The renderer is not run in CI and its
pixels are not a shape-level oracle.

Annotations are maintainer-authored public development data. Changes require a stated factual or
contract reason; a failing implementation is not a reason. There is no independent reviewer or
protected holdout in version 0.1.0.
