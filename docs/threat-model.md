# Threat model

## Assets

SightLint may process confidential interfaces, documents, customer information, source
structure, screenshots, credentials visible in rendered output, and interaction traces.
Primary assets are artifact confidentiality, report integrity, host safety, and deterministic
kernel correctness.

## Threat actors and inputs

Assume:

- artifact files are malformed or malicious
- web pages execute hostile script
- browser targets attempt network or filesystem access
- OCR and model workers return malformed or adversarial output
- third-party actions and dependencies may be compromised
- reports may expose source content through snippets or screenshots
- a coding agent may attempt to weaken a failing rule rather than fix the product

## Trust zones

### Trusted deterministic kernel

- no network access
- validated, bounded IR only
- deterministic computations
- no dynamic code loading
- unsafe Rust forbidden by default

### Adapter sandbox

- strict input and output schema
- time, memory, file-size, node-count, and recursion limits
- restricted filesystem and network access
- explicit coordinate and unit declarations
- content digests for evidence reconciliation

### Optional remote perception

- opt-in only
- disclose provider and transmitted fields
- redact when configured
- never receive secrets by default
- model output remains inferred evidence

## Initial mitigations

- Rust kernel and no unsafe code
- local-first execution
- process boundaries for untrusted adapters
- schema validation before rule execution
- explicit evidence provenance
- least-privilege GitHub Actions permissions
- dependency update automation
- no artifact uploads in the foundation milestone

## Current PPTX parser boundary

ADR 0043 treats OOXML ZIP/XML parsing as an untrusted local process. The adapter requires
repository-contained digest-pinned paths, inventories archive members before decompression,
rejects traversal/duplicate/encrypted/unsupported-compression entries and DTD/entities, caps
archive/render/XML/expanded/object/depth/output resources, follows only the required internal
presentation/slide relationships, never extracts the archive, and never launches Office or an
embedded object. Candidate IR still passes the trusted public normalizer before rules run.

These controls are not an OS sandbox or hard process-memory limit. The standard-library parsers
and Python runtime remain in the adapter trust zone. Digest-only text metadata also has a privacy
limit: unsalted hashes of low-entropy strings may be guessed offline, while caller-supplied title
and relative path metadata remains visible. Treat output as source-derived sensitive data.

## Current PDF parser boundary

ADR 0044 treats pypdf and PDF object traversal as another untrusted local process. The exact
universal `pypdf==6.17.0` wheel is SHA-256 locked, its version is checked at runtime, strict mode is
enabled, and the adapter rejects encryption. Repository-contained source/render paths and digests,
source/render bytes, cross-reference objects, page-tree traversal with cycle detection, pages,
annotations, and output are bounded. The adapter does not read content streams, extract text or
images, interpret tags, follow destinations/actions, or serialize URI/action/text/metadata values.
Candidate IR still passes public Rust normalization before it is written.

Those controls do not provide a CPU, memory, syscall, recursion, or filesystem sandbox. A hostile
PDF still reaches pypdf under the caller's process privileges, and parser version locking does not
prove safety. Object IDs, rectangles, relative paths, titles, and digests are source-derived
sensitive metadata even though document text and pixels are omitted from serialized output.

## Current Android capture boundary

ADR 0045 treats both the Android instrumentation output and its local Python converter as
untrusted sensors. The converter accepts only repository-contained digest-pinned capture/PNG
paths, rejects duplicate/unknown fields and hierarchy identity errors, bounds request/capture/
screenshot/node/depth/attribute/string/output resources, validates the PNG through public
`adapt-image`, requires display/PNG extent agreement, and passes candidate IR through public Rust
normalization before exclusive output creation. It does not invoke `adb`, boot or mutate a device,
install an APK, perform an accessibility action, or use the network.

These controls are not a CPU, memory, syscall, or filesystem sandbox. Capture acquisition runs
under explicit maintainer-controlled Android/Gradle tooling outside CI, and the committed
manifests remain untrusted input. Resource IDs, class/package names, device/build identifiers,
geometry, relative paths, screenshots, and unsalted text/content-description digests are
source-derived sensitive data; low-entropy values can be guessed offline. Treat captures and
adapter output like source artifacts even though plaintext View strings and pixels are not copied
into Artifact IR.

## Current iOS capture boundary

ADR 0046 treats UIKit instrumentation output, XCUITest observations, screenshots, and their local
Python converter as untrusted sensors. The converter accepts only repository-contained
digest-pinned capture/PNG paths, rejects duplicate/unknown fields and hierarchy identity errors,
bounds request/capture/screenshot/node/depth/attribute/string/output resources, validates the PNG
through public `adapt-image`, requires screen/PNG extent-and-scale agreement, and passes candidate
IR through public Rust normalization before exclusive output creation. It does not invoke Xcode or
`simctl`, boot or mutate a simulator, install/launch an app, execute an XCUI action, parse an
`.xcresult`, or use the network.

These controls are not a CPU, memory, syscall, or filesystem sandbox. Authentic capture runs
under explicit maintainer-controlled Xcode/simulator tooling outside CI, and the committed
manifests remain untrusted input. Accessibility identifiers, selectors, class/bundle names,
device/build identifiers, geometry, relative paths, screenshots, and unsalted label/value digests
are source-derived sensitive data; low-entropy values can be guessed offline. Treat captures and
adapter output like source artifacts even though plaintext labels/values and pixels are not copied
into Artifact IR.

## Future work

Before broadening PPTX/PDF/Android/iOS or accepting other real artifact formats, add format-specific
fuzzing, parser/runtime compatibility characterization, and stronger sandbox guidance; every new
parser still needs its own resource and archive policy.
Before browser support, define origin isolation, credential handling, request interception,
and trace redaction.
