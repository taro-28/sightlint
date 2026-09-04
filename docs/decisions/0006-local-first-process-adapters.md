# ADR 0006 — Local-first process adapters

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

Artifacts may contain private application data, documents, credentials, or customer
information. Adapters also require incompatible runtimes and process untrusted files or pages.
An in-process plugin architecture would enlarge the trusted computing base.

## Decision

Core checks are local-first. Early adapters communicate with the kernel through versioned
files or process streams rather than a native dynamic-plugin ABI. Networked or hosted
perception is opt-in and visible.

GitHub Actions provides the initial remote execution environment using the same CLI and Cargo
commands as local development.

## Consequences

- Users can analyze private inputs without uploading them.
- Adapters can be written in platform-native languages.
- Crashes and dependencies are easier to isolate.
- Streaming and process startup performance may require later optimization.

## Alternatives considered

- cloud-first API: simpler centralized upgrades but unacceptable as a mandatory privacy model.
- native shared-library plugins: fast but difficult to stabilize safely across languages.
- embed all adapters in the Rust binary: creates dependency and platform pressure too early.

## Verification

The kernel performs no network I/O. External transmission requires an explicit adapter and is
reported in provenance. CI and local commands use the same deterministic engine.
