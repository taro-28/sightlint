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

“Sandbox” here is a trust zone and design objective, not a claim that every adapter runs in an OS
sandbox. Each adapter section below states its actual enforcement boundary.

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

## Current managed Web boundary

ADR 0048 treats the target page and caller-authorized development server as untrusted. Protocol
`0.2.0` accepts only a direct argv array, verifies that `{port}` appears once, canonicalizes the
target-repository working directory, checks the requested 1024–65535 port before spawn, and
requires the explicit `--allow-server-command` flag before any process is started. The adapter
drains but does not serialize bounded server stdout/stderr. It owns shutdown on success, failure,
SIGINT, and SIGTERM: POSIX targets the child process group with TERM then KILL, and Windows uses a
validated positive PID with `taskkill.exe /T /F`.

The browser accepts only same-origin HTTP at literal `127.0.0.1:<port>`, aborts external HTTP(S),
and blocks/counts WebSocket and service-worker attempts. Request bodies, each response, aggregate
response bytes, response count, argv size, startup duration, and server output are bounded. The
serialized source identity is derived from method, target/request-body digests, status, byte count,
and buffered response bytes; raw query strings, request/response bodies, variable headers, server
logs, environment values, and PIDs are omitted. Runtime selectors are retained, but source-file
and source-line attribution is explicitly unavailable.

These controls are not an OS sandbox. The server command inherits all caller environment variables
and host privileges, and its own outbound network is not intercepted. The authorization flag is
therefore a consent boundary, not proof that the command is safe. A hostile page still executes in
Chromium, and digest-only fields can reveal equality or permit guessing of low-entropy values.
Treat the report and screenshot as sensitive source-derived artifacts.

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
Before broader browser support, define credential/storage isolation for authenticated production
sessions, iframe and worker policy, arbitrary host/origin handling, and interaction-trace
redaction. Managed protocol `0.2.0` deliberately covers only one newly started loopback origin and
does not admit an existing server, remote URL, browser profile, or stored session.
