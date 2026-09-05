# ADR 0037 — Source-first alpha release and compatibility contract

- Status: Accepted
- Date: 2026-09-06
- Owners: @taro-28

## Context

The useful bounded Web path is implemented and evaluated, repository protection is active, and
ADR 0007 resolves the project license. SightLint still has several independently versioned
schemas, rules, adapters, reports, and commands, while the one-command Web path requires both the
Rust workspace and the private Node companion. Treating package version alone as compatibility or
shipping a Rust binary as the whole product would misrepresent that state.

The first alpha needs a repeatable public artifact and cross-platform evidence without adding a
package registry, container, installer, signing service, or self-modifying workflow before demand
justifies those surfaces.

## Decision

Release `v0.1.0-alpha.1` as a GitHub prerelease with one deterministic source archive and a SHA-256
checksum. Do not publish prebuilt binaries, Cargo crates, an npm package, Homebrew formula, or
container in this release.

Source-first distribution is deliberate: the current user-visible Web workflow needs the Rust
CLI, the Node adapter, schemas, evaluation requests, and repository-owned fixture together. It
also avoids prematurely selecting libc/deployment targets, redistributing third-party binary
notices, or implying that the browser companion is portable beyond its evaluated environment.

The release workflow:

1. runs only for a version tag and never for pull-request code;
2. requires the tag version to equal the workspace package version and the tag commit to equal the
   current remote `main` head;
3. creates or updates a draft prerelease before uploading assets;
4. creates the archive from tracked tag contents with fixed archive metadata and no build output,
   dependency cache, secret, or user file;
5. verifies the checksum, archive safety, locked Rust workspace, and private Node package on
   Ubuntu x64, macOS arm64, and Windows x64;
6. reruns the complete browser/product path from the extracted archive on Linux;
7. publishes the draft only after every verification job succeeds.

Only release-upload/finalization jobs receive ephemeral `contents: write`; ordinary CI and
verification jobs remain read-only. The workflow may write GitHub Release metadata and assets but
must never modify, format, commit, or push repository source.

The alpha compatibility policy is surface-specific:

- package versions use SemVer prerelease identifiers but make no 1.0 stability promise;
- Artifact IR, CheckReport, workflow reports, adapter protocols, official extensions,
  configuration profiles, evaluation manifests, and perception protocols retain independent
  versions;
- rule identifiers are stable and each rule's semantic version changes when applicability or the
  deterministic obligation materially changes;
- a breaking alpha surface change requires a version change for that surface, release notes, and
  migration guidance; it must not be hidden only in the package version;
- the documented CLI exit meanings 0/1/2 remain stable throughout the `0.1` alpha line unless a
  later accepted ADR and migration note explicitly replace them;
- strict protocols reject unsupported fields/versions, while Artifact IR preserves unknown
  namespaced extensions according to its existing contract.

SHA-256 checksums detect corruption but do not authenticate the publisher. For this alpha,
authenticity is limited to the protected Git tag, the GitHub-hosted workflow record, and HTTPS
release transport. Signing and GitHub artifact attestations are explicitly deferred; adding them
requires a focused supply-chain decision and verified key/identity lifecycle before a beta claim.

## Consequences

- Users build locally with locked Cargo/npm dependencies and can inspect the exact source used.
- The Rust CLI is verified on the three supported hosted operating-system/architecture pairs.
- The Playwright product path remains supported only in its documented Node 20–24, pinned
  Chromium, Linux E2E environment; building the adapter on macOS/Windows is compatibility evidence,
  not cross-platform screenshot identity or product support.
- Release failures leave a draft rather than a partially published release. A rerun may replace
  draft assets but cannot overwrite a published release.
- Registry publication and native binary archives remain possible later without changing the
  deterministic kernel or product contracts.

## Alternatives considered

- Prebuilt Rust binaries: convenient, but incomplete for the current Web workflow and premature
  without a binary target/support and third-party notice policy.
- Cargo plus npm publication: creates two registry compatibility promises before package
  boundaries and demand are established.
- Container image: reproducible in one environment but weakens the local-first, ordinary-agent
  workflow and adds an unnecessary distribution/security surface.
- Manual release upload: harder to audit and repeat, and does not verify extracted artifacts on
  each supported host.
- Artifact signing/attestation now: desirable, but needs additional permissions/actions and a
  durable identity policy. Checksums plus an explicit non-claim are sufficient for this alpha.
- No release workflow: fails issue #33's repeatability, checksum, and supported-platform gates.

## Verification

- Unit tests create the source archive twice from identical entries, compare bytes, reject unsafe
  extraction members, validate tags/versions, and verify checksum corruption detection.
- A dependency-license check evaluates the complete locked Cargo graph and npm lockfile against
  the reviewed permissive-license set.
- Pull-request CI validates release tooling without write permission.
- The tag workflow builds/tests the extracted artifact on Ubuntu x64, macOS arm64, and Windows x64
  and runs the Linux browser/product E2E before publishing.
- Release notes list the exact independent contract versions, supported environments,
  installation/removal steps, checksums, known limitations, and non-claims.
- Completion requires exact-head PR CI, post-merge `main` CI, the published prerelease and assets,
  and a successful tag-workflow run tied to the released commit.
