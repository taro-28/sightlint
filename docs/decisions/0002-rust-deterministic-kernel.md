# ADR 0002 — Rust deterministic kernel

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

The trusted part of SightLint performs parsing validation, geometry, graph queries, rule
execution, canonical serialization, and CLI reporting. It must be fast, reproducible,
cross-platform, memory-safe, suitable for a single native binary, and compilable to WebAssembly
where practical.

Adapters and perception have different ecosystem needs.

## Decision

Implement the trusted kernel and primary CLI in Rust 2024. Declare Rust 1.85.0 as the initial
MSRV. Forbid unsafe Rust in the kernel by default.

Use other languages at adapter boundaries when they are the natural platform choice:
TypeScript for browser tooling, Python for perception, Kotlin for Android, and Swift for iOS.

## Consequences

- Cross-language boundaries use a versioned serialized IR rather than shared in-process types.
- The kernel remains independent of Python, Node.js, JVM, or Apple runtimes.
- Performance-sensitive geometry can remain in the kernel without later migration pressure.
- Adapter experimentation does not compromise deterministic execution.

## Alternatives considered

- TypeScript core: strong web integration, weaker fit for a native cross-artifact kernel.
- Python core: strong ML ecosystem, weaker deterministic distribution and runtime isolation.
- Go core: viable native tooling, but Rust better fits low-level parsers, image primitives,
  WASM, and strong type modeling.

## Verification

CI runs stable and MSRV Rust across Linux, macOS, and Windows. Workspace lint policy forbids
unsafe code and denies warnings in CI.
