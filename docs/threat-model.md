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

## Future work

Before accepting real artifact files, add parser-specific resource budgets, archive expansion
limits, decompression-bomb protection, path-traversal tests, fuzzing, and sandbox guidance.
Before browser support, define origin isolation, credential handling, request interception,
and trace redaction.
