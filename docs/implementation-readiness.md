# Implementation readiness

Functional implementation begins only after milestone M0 is accepted.

## Required before M1 starts

1. The foundation pull request is reviewed and merged.
2. Remote CI is green on stable Rust, Rust 1.85.0, Linux, macOS, and Windows.
3. The maintainer accepts the decisions in `docs/foundation-review.md`, or records explicit
   changes in the review.
4. Licensing was permitted to remain unresolved for the original local pre-alpha M1 start. It was
   later resolved as `MIT OR Apache-2.0`; package publication remains disabled for the source-only
   alpha.
5. M1 work is opened as a separate pull request and does not add image, browser, ML, mobile,
   document, cloud, MCP, or GUI dependencies.

## M1 first vertical slice

The first implementation pull request should provide one complete, deterministic path:

```text
versioned JSON Artifact IR
        -> validation
        -> exact geometry query
        -> atomic rule execution
        -> evidence-linked human and JSON reports
        -> stable exit code
```

Initial implementation is successful only when repeated execution produces byte-identical
canonical JSON and mutation fixtures prove that each rule detects its intended defect.

## Deferred intentionally

The following are important but not part of the first implementation:

- screenshot interpretation and OCR
- Playwright and runtime UI state exploration
- PPTX, PDF, Android, and iOS adapters
- probabilistic semantic grouping
- project baseline learning
- MCP and editor integrations
- hosted processing or telemetry

Deferral protects the core contracts from being designed around one adapter or model.
