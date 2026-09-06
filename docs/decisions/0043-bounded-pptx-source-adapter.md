# ADR 0043 — Add a bounded PPTX source-geometry process adapter

- Status: Accepted
- Date: 2026-09-06
- Issue: #29
- Builds on: ADRs 0003, 0006, 0013–0018, 0024 (product evaluation), 0033, 0034,
  and 0042

## Context

Issue #29 is the next roadmap gate after the Web and perception protocol slices. It is an
umbrella for several media and explicitly requires one adapter at a time. PPTX is the first
candidate because OOXML exposes slide size, native object identifiers, hierarchy, z-order,
placeholder metadata, text runs, and geometry while a rendered slide can independently expose the
pixels a viewer receives.

The current kernel already understands a `slide` artifact, multiple canvases, `emu` units,
containers, shapes, text, charts, tables, hierarchy, exact-source evidence, and ordinary geometry
rules. It does not need a slide-specific mandatory core field. The missing boundary is a bounded
native package reader and a versioned PPTX extension that distinguishes source geometry from
rendered evidence and unsupported DrawingML features.

PPTX files are ZIP archives containing XML and optional binary parts. They are untrusted and can
contain path tricks, duplicate members, decompression bombs, external relationships, macros,
malformed XML, deep group trees, unsupported transforms, and private text. Running an office suite
inside the adapter would add document scripting, font, renderer, and platform variability to the
initial trust boundary.

## Decision

Add `adapters/pptx/` as a local Python 3.9+ process adapter with protocol and
`org.sightlint.pptx` extension version `0.1.0`. The implementation uses only the Python standard
library. It parses a caller-selected repository-contained `.pptx` without executing Office,
macros, embedded objects, hyperlinks, or external relationships. It writes candidate Artifact IR
to an exclusive output path only after the existing public `sightlint normalize` command accepts
it.

The public process is:

```text
versioned local request
  -> bounded ZIP member inventory
  -> bounded presentation/relationship/slide XML parsing
  -> exact unrotated source geometry and native metadata
  -> optional repository-owned rendered PNG validation through sightlint adapt-image
  -> Artifact IR 0.1.0 plus org.sightlint.pptx 0.1.0
  -> sightlint normalize
  -> caller may run sightlint check
```

The process exits `0` for a successful complete or explicitly partial acquisition and `2` for
usage, path, archive, XML, resource, protocol, output, or Rust-validation errors. It never exits
`1`; rule and blocking policy remain owned by the Rust binary.

## Exact source boundary

Protocol v0 maps these source facts when present and supported:

- the presentation slide size from `p:sldSz`, in integer EMUs;
- slide order through `p:sldIdLst` and the presentation relationship table;
- `p:sp` and `p:grpSp` native object identifiers and source z-order;
- parent/child hierarchy for nested groups;
- axis-aligned `a:xfrm` offsets and extents, including group child-coordinate translation and
  scaling when all required terms are finite positive integers;
- placeholder type/index as PPTX-extension metadata;
- whether a shape contains text, plus UTF-8 byte count and SHA-256 of the normalized source text.

Core nodes receive `layoutBox`, never `renderBox` or `inkBox`, from `ExactSource` evidence. Text
content and shape names are not serialized. A text-bearing shape remains a shape; the adapter does
not claim that source text is an accessible name, visible ink, or a standalone semantic text node.
The adapter records native type, z-order, placeholder data, text digest/count, and geometry status
only in the versioned PPTX extension.

Object IDs derive from slide part plus the native `p:cNvPr/@id`; they do not depend on ZIP or XML
iteration order. Duplicate native IDs are rejected. Canonical output uses sorted object keys and
stable identifier ordering rather than locale-sensitive comparison.

Protocol v0 does not map rotation, flips, skew, three-dimensional transforms, connectors, pictures,
tables, charts, diagrams, media, animations, notes, masters, theme-resolved style, font metrics, or
text layout into core geometry. Encountered unsupported native object types and transforms are
listed explicitly. Unsupported geometry remains absent; it is not approximated with an
axis-aligned source box.

## Rendered evidence and reconciliation

A request may name one repository-contained PNG render per slide, with an exact SHA-256 digest and
an integer `emuPerPixel` mapping. The adapter asks the public `sightlint adapt-image` command to
validate and describe the render. It records the PNG evidence separately as `ExactRender` and
checks only whether:

```text
pixel width  * emuPerPixel == slide width in EMUs
pixel height * emuPerPixel == slide height in EMUs
```

The result is `agreement` or `conflict` and both source and render dimensions remain visible. No
rescaling changes source observations. Pixel-to-node identity, shape ink bounds, effects, font
substitution, text wrapping, clipping, and z-order appearance are `cantTell` in version `0.1.0`.
A render conflict is evidence, not an automatic rule result.

Repository evaluation renders are reviewed artifacts produced from the matching PPTX with a named
LibreOffice build and conversion command. CI verifies their bytes and the adapter path; it does not
regenerate them under an unpinned host renderer or treat adapter output as ground truth.

## Resource and failure contract

Before decompression, the adapter validates every central-directory member and enforces request
limits no larger than:

- source archive: 64 MiB;
- archive members: 2,048;
- declared total uncompressed bytes: 128 MiB;
- one selected XML part: 8 MiB;
- compression ratio: 100:1 for a nonempty compressed member;
- slides: 100;
- mapped native nodes: 10,000;
- group depth: 32;
- canonical response and Artifact IR: 16 MiB each.

Absolute names, backslashes, `..`, NULs, duplicate normalized members, encryption, unsupported ZIP
compression, missing required parts, mismatched content relationships, DTD/entity declarations,
malformed XML, dangling slide relationships, unsupported external slide relationships, invalid
integers, duplicate object IDs, and resource overflow are stable errors. The adapter reads only
the required presentation, relationship, and slide XML members plus caller-selected local render
files. It never extracts the archive to disk.

These checks bound the process input and output but are not an operating-system sandbox or memory
ceiling. A malicious Python runtime or library is outside this contract. The standard-library ZIP
and XML parsers remain untrusted sensor implementation, not trusted kernel code.

## Privacy, network, and licensing

- Input and render paths must resolve below the caller-supplied repository root without symlink
  escape.
- No network API is used and external relationships are not followed.
- `externalProcessing` is always false and retention is `none`.
- Full source text, shape names, XML, embedded binaries, and rendered pixels are not serialized in
  the IR or response; text is digest/count only.
- Repository fixtures, annotations, and renders are fictional project-owned data under
  `MIT OR Apache-2.0`, with no customer, credential, or personal data.
- The adapter adds no package dependency. Python and any separately used LibreOffice renderer keep
  their own upstream licenses and are not bundled in the source release.

## Evaluation contract

Add a public PPTX corpus with separate acquisition and rule annotation documents. The first family
contains:

- a clean single-slide baseline with shapes, source text, and a nested group;
- a targeted mutation that moves one declared content shape outside the slide canvas;
- an asymmetric but valid composition hard negative that declares no peer-spacing relation.

The acquisition oracle is authored from the fixture specification before running the adapter and
records exact native IDs, hierarchy, EMU boxes, text digests, source/render extent agreement, and
all deliberate abstentions. The rule oracle separately records expected existing-rule outcomes,
policy, applicability, targets, and nonblocking/non-claim boundaries. Implementation output and
rendered reports are not stored as oracle data.

The E2E invokes the real Python process, public `adapt-image`, public `normalize`, and public
`check` commands. It checks malformed/archive/resource failures, stable diagnostics, output
collision behavior, same-environment byte determinism, mutation detection, hard-negative behavior,
and retained `cantTell` render-node reconciliation. Acquisition and rule metrics remain separate.

The public cases are smoke/development/challenge data visible to implementers. They are one
fictional presentation family, maintainer-authored, independently specified but not independently
reviewed, and have no protected holdout. They establish regression behavior only.

## Compatibility

PPTX request, response, extension, evaluation corpus, acquisition annotation, and rule annotation
start at `0.1.0` as independent surfaces. Artifact IR remains `0.1.0`; CheckReport, existing
extensions, rules, profiles, Rust commands, and exit meanings do not change. An incompatible PPTX
surface change requires its own version and migration/coexistence tests.

This adapter is an unreleased source-tree addition after `v0.1.0-alpha.2`. It does not change that
published archive or claim a registry, prebuilt binary, bundled Python runtime, office renderer,
or cross-renderer pixel identity.

## Alternatives considered

### Parse OOXML inside the trusted Rust kernel

Rejected. Office ZIP/XML parsing is an acquisition concern and would enlarge the kernel and its
dependency surface. Shared rules need normalized facts, not DrawingML code.

### Add Rust ZIP/XML libraries in a new adapter crate

Viable later, especially for a packaged single-binary path, but deferred for the first process
slice. A dependency-free Python standard-library reader provides a smaller reviewable boundary and
keeps untrusted parsing out of the kernel while demand and the exact PPTX subset are evaluated.

### Run LibreOffice as the adapter

Rejected for source acquisition. It would improve fidelity for rendering but introduces a much
larger executable/runtime attack surface, macro and embedded-object concerns, font/platform drift,
and weaker source selectors. Reviewed renders remain differential evidence only.

### Treat source geometry as rendered geometry

Rejected. Office renderers, fonts, effects, text layout, and transparent assets can disagree with
DrawingML bounds. Protocol v0 emits only source `layoutBox` and preserves rendered uncertainty.

### Support every OOXML object in version 0.1.0

Rejected. The first slice proves one exact source path and shared-rule reuse. Unsupported objects
and transforms stay visible rather than being approximated.

## Consequences

- SightLint gains its first non-Web structured adapter without adding medium-specific core fields.
- Existing canvas containment can execute on exact PPTX source geometry with no slide-specific
  kernel branch.
- Native and rendered slide evidence coexist, while node-level visual reconciliation remains
  honestly incomplete.
- A later PPTX slice may add pictures, tables, charts, richer text, rendering orchestration, and
  slide-specific evaluated rules under new evidence rather than silently broadening this contract.
- PDF/document and mobile adapters remain later focused work under issue #29.
