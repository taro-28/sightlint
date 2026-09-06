# ADR 0044 — Add a bounded PDF link-annotation geometry process adapter

- Status: Accepted
- Date: 2026-09-06
- Issue: #54, focused child of #29
- Builds on: ADRs 0003, 0006, 0013–0019, 0024 (product evaluation), 0033, 0034,
  0041, and 0043

## Context

Issue #29 requires one structured medium at a time. ADR 0043 proved the process-adapter pattern
with PPTX source geometry. PDF/document is next, but PDF does not reliably provide a semantic
document tree: a file may contain tagged structure, paint operators, annotations, scanned pages,
or mixtures of them. Reading order, text layout, and visible ink cannot be inferred from page
boxes alone.

The kernel already supports `pdf` artifacts, `pdfPoint` canvases, distinct layout/render/ink/hit
geometry, exact-source and exact-render evidence, and the shared
`visual.bounds.within-canvas@0.1.0` rule. No PDF-specific mandatory core field is needed. The
smallest useful exact source fact beyond a page is the activation rectangle of a rectangular Link
annotation. PDF 1.6 specifies that `QuadPoints`, when present and valid, defines the activation
region; otherwise `Rect` is used. PDF 2.0 can also express non-rectangular link paths. Treating the
bounding `Rect` as the exact hit area in those cases would overstate the evidence.

PDF parsing is a large, security-sensitive problem. A custom subset parser would either reject
common cross-reference/object-stream forms or recreate format machinery unrelated to SightLint's
product advantage. A system Poppler/QPDF dependency would couple acquisition to platform package
installation and make the parser binary identity harder to lock across the supported CI matrix.

`pypdf` 6.17.0 is a pure-Python, OS-independent, BSD-3-Clause reader requiring Python 3.9 or
later. Its current release contains security/robustness limits added across recent releases, and
its strict mode makes correctable malformed input fatal. It still parses attacker-controlled
object graphs in the adapter process and is not a sandbox or memory ceiling.

## Decision

Add `adapters/pdf/` as a local Python 3.9+ process adapter with request, response, and
`org.sightlint.pdf` extension version `0.1.0`. Pin the universal `pypdf==6.17.0` wheel by SHA-256,
record its license and Python compatibility, and install it with `pip --require-hashes` in CI.
The adapter verifies the imported version at runtime and records the Python and parser versions in
provenance.

The public process is:

```text
strict digest-pinned local request
  -> bounded source file identity and PDF reader initialization
  -> strict unencrypted page/annotation inspection
  -> exact explicit unrotated CropBox and rectangular internal-Link activation geometry
  -> optional repository-owned rendered PNG validation through sightlint adapt-image
  -> Artifact IR 0.1.0 plus org.sightlint.pdf 0.1.0
  -> sightlint normalize
  -> caller may run sightlint check
```

The adapter writes candidate Artifact IR to an exclusive output path only after public
`sightlint normalize` accepts it. It exits `0` for successful explicitly partial acquisition and
`2` for usage, dependency, path, digest, parser, encryption, resource, schema, output, or Rust
validation errors. It never exits `1`; deterministic rule and blocking policy remain in Rust.

## Exact source boundary

Protocol `0.1.0` maps only these source facts:

- the PDF header version;
- page order and indirect page object reference;
- a directly declared, integral, finite `MediaBox` and `CropBox` on an unrotated page;
- whether the catalog declares a structure tree, without walking or interpreting that tree;
- indirect annotation object references, declared subtype, `Rect` availability, `QuadPoints` or
  `Path` presence, flags, and action/destination class;
- for `/Subtype /Link` with an internal `/Dest`, no `QuadPoints` or `Path`, flags absent or zero,
  and an integral finite nonempty `Rect`, the exact rectangular activation region.

Each supported page CropBox becomes a zero-origin top-left `pdfPoint` canvas. The extension
retains the original lower-left and upper-right box coordinates. The exact transform for an
annotation rectangle `[left, bottom, right, top]` is:

```text
x      = left - cropLeft
y      = cropTop - top
width  = right - left
height = top - bottom
```

Core vertical direction is therefore `down`, while original PDF default-user-space coordinates
remain inspectable in the extension. A supported link becomes a core `control` node with exact
source role `link` and `hitBox`; it does not receive `layoutBox`, `renderBox`, `inkBox`, accessible
name, visible text, or reading-order relation. Page and annotation stable IDs derive from indirect
object number and generation, not page or annotation traversal order. Duplicate indirect object
references are rejected.

The adapter does not follow a destination or action. External URI, remote-go-to, launch,
JavaScript, submit/import, named, and other actions are recorded only as an unsupported action
class and do not create a core interactive node. Direct annotation dictionaries, widgets,
non-Link annotations, nonzero flags, missing destinations, `QuadPoints`, `Path`, non-integral
geometry, rotated pages, inherited/missing page boxes, and malformed rectangles remain explicit
unsupported or `cantTell` coverage. They are not approximated.

All successful protocol-v0 responses are `partial`. Paint streams, text, fonts, images, tags,
reading order, forms, attachments, signatures, optional content, JavaScript, metadata, page
labels, scanned-page/OCR state, and content geometry are not read into IR.

## Rendered evidence and reconciliation

A request may name one repository-contained PNG render per page, with an exact digest and positive
integer rational `pdfPointsPerPixel`. The adapter asks public `sightlint adapt-image` to validate
the render and records it as a separate device-pixel canvas with `ExactRender` evidence. It checks
only exact page extent:

```text
pixelWidth  * numerator == cropWidthPoints  * denominator
pixelHeight * numerator == cropHeightPoints * denominator
```

The result is `agreement` or `conflict`; neither source nor render dimensions are rewritten.
Annotation-to-pixel identity, visible link styling, text association, clipping, occlusion, and
viewer hit testing remain `cantTell`. A source hit-box failure or extent conflict is evidence, not
proof of a user-facing PDF defect.

Repository renders are reviewed artifacts produced from the matching fixture with a named Poppler
`pdftoppm` version and fixed 72-DPI crop-box command. CI verifies their bytes and public image
path but does not regenerate them with an unpinned renderer. The renders must remain inside the
currently accepted PNG subset; their addition updates the repository-wide ADR 0041 inventory
rather than expanding codec scope.

## Dependency, resource, and failure contract

The only Python package is the universal `pypdf==6.17.0` wheel, pinned by SHA-256. No crypto,
image, or other extras are installed. Dependency metadata records its BSD-3-Clause license,
Python `>=3.9`, source URL, wheel filename, size, digest, and reviewed release. The existing
dependency-license checker validates that record.

Before parser construction, the adapter resolves and size-checks the repository-contained source,
streams its SHA-256, and rejects files above the request limit. Request maxima are no larger than:

- source PDF: 32 MiB;
- each rendered PNG: 64 MiB;
- cross-reference objects inventoried by the reader: 50,000;
- pages: 100;
- annotations: 10,000 total and 1,000 per page;
- canonical response and Artifact IR: 16 MiB each.

The adapter uses `PdfReader(..., strict=True, root_object_recovery_limit=1)` and walks the raw
page tree iteratively with cycle detection instead of using pypdf's inherited-page-property
expansion. It does not access content streams, images, attachments, outlines, XMP, forms, or text
extraction. Encrypted input is rejected rather than decrypted. Missing/invalid catalog or page tree, indirect-reference
collisions, malformed boxes/annotations, source/render digest mismatch, path escape, dependency
version mismatch, over-budget input/object/page/annotation/output, Rust rejection, and exclusive
output collision have stable error categories and produce no partial artifact file.

These controls bound the selected operation but do not impose an OS-level CPU, memory, syscall,
or recursion sandbox. `pypdf` and Python remain an untrusted sensor. Hostile PDF fuzzing and a
stronger external sandbox remain future work before broad arbitrary-document claims.

## Privacy and network

- Source and render paths must resolve beneath the caller-supplied repository root without
  symlink escape.
- The adapter makes no network request and never follows destinations or actions.
- `externalProcessing` is false and retention is `none`.
- Text, URI strings, destinations, metadata, attachment names/content, JavaScript, annotations
  contents, raw PDF objects, content streams, and pixels are not serialized.
- Caller-provided artifact title and repository-relative references, source/render digests,
  object numbers, page dimensions, annotation subtype/action class, and rectangles remain
  sensitive source-derived metadata.
- Repository fixtures and annotations are fictional project-owned data under
  `MIT OR Apache-2.0`, with no personal, credential, customer, or third-party document data.

## Evaluation contract

Add one public repository-owned report family:

- a clean page with rectangular internal navigation links inside its explicit CropBox;
- a targeted mutation that changes only one link annotation `Rect` so its exact hit region extends
  outside the page while rendered content remains unchanged;
- an asymmetric hard negative containing a `QuadPoints` link which must retain activation geometry
  abstention rather than promoting its bounding `Rect`.

The source generator is the fixture specification. Acquisition annotations are authored from
that specification before adapter execution and separately declare page/object IDs, source boxes,
normalized hit rectangles, action/geometry statuses, render extent, and deliberate abstentions.
Rule annotations independently declare expected existing-rule targets/outcomes and forbid
unexpected failures. Implementation response, IR, reports, and renderer output are never copied
into an oracle.

The public E2E invokes the Python process plus public `adapt-image`, `normalize`, and
`check --profile base`. It covers repeated canonical bytes, mutation kill, hard-negative
abstention, no unexpected failure, malformed/encrypted/resource/digest/dependency/output-collision
errors, and source-text/URI non-leakage. Acquisition fact coverage, evaluated-case coverage,
verdict precision, abstention retention, false-positive rate, and mutation kill rate remain
separate metrics.

The data is visible public smoke/development/challenge regression data, maintainer-authored and not
independently reviewed. No protected holdout exists. Perfect regression metrics do not estimate
representative PDF acquisition, accessibility, interaction, or document-quality accuracy.

## Compatibility

PDF request, response, extension, corpus, acquisition annotation, rule annotation, metric
contract, and dependency lock begin at `0.1.0` as independent unreleased surfaces. Artifact IR,
CheckReport, rules, profiles, Rust commands, and exit meanings do not change. An incompatible PDF
surface change requires a new version and coexistence/migration fixtures.

The adapter requires Python 3.9+ and exactly pypdf 6.17.0. Canonical bytes are guaranteed only for
the same request, input bytes, parser version, Python runtime, SightLint binary, and declared
environment. Linux, macOS, and Windows process behavior is tested; cross-renderer PNG byte identity
is not claimed. The addition is after `v0.1.0-alpha.2` and does not alter that immutable release.

## Alternatives considered

### Custom PDF parser

Rejected. Even a page/annotation slice needs xref tables and streams, object streams, indirect and
inherited values, tokenization, and cycle handling. Reimplementing those mechanisms would create a
new security surface and make PDF parsing—not evidence-backed linting—the product center.

### System Poppler or QPDF as source parser

Viable for later rendering or differential characterization, but rejected as the first source
contract. The supported CI systems do not expose one identical locked binary by default, and a
subprocess JSON/text bridge would still need version, locale, diagnostic, and package governance.

### In-process Rust PDF crate

Deferred. It would add PDF parsing and a larger dependency graph to a Rust process that also owns
trusted normalization/rules. A separate Python process keeps the parser outside the kernel and can
be replaced behind versioned output.

### Extract text, paint geometry, or tags in the first slice

Rejected. Text boxes require font matrices and layout decisions, paint bounds require graphics
state and clipping, and tags may be absent or misleading. The first slice proves exact page and
rectangular activation geometry while leaving these capabilities visibly untested.

## Consequences

- SightLint gains a second non-Web structured source adapter without adding PDF concepts to the
  mandatory core.
- The existing canvas-containment rule can inspect exact source link hit rectangles.
- Native source and rendered page evidence coexist without fabricating node-to-pixel identity.
- A reviewed dependency replaces custom format parsing, but remains an untrusted, bounded process
  with explicit version and license governance.
- Broader PDF text/tag/paint/OCR work requires new evidence and a focused issue/ADR.
- Android and iOS remain later children of issue #29.
