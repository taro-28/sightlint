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

## Future work

Before broadening PPTX or accepting other real artifact formats, add format-specific fuzzing,
parser/runtime compatibility characterization, and stronger sandbox guidance; every new parser
still needs its own resource and archive policy.
Before browser support, define origin isolation, credential handling, request interception,
and trace redaction.
