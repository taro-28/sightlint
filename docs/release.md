# Release and installation guide

The first supported distribution channel is a GitHub prerelease source archive. Crates.io, npm,
Homebrew, containers, and prebuilt binaries are intentionally not release channels yet.

## Verify the release

Download both assets from the `v0.1.0-alpha.2` GitHub release:

- `sightlint-v0.1.0-alpha.2-source.tar.gz`;
- `sightlint-v0.1.0-alpha.2-source.tar.gz.sha256`.

First compare the archive with the downloaded checksum record. On Linux use
`sha256sum -c sightlint-v0.1.0-alpha.2-source.tar.gz.sha256`; on macOS use
`shasum -a 256 -c sightlint-v0.1.0-alpha.2-source.tar.gz.sha256`. On Windows PowerShell:

```powershell
$expected = (Get-Content sightlint-v0.1.0-alpha.2-source.tar.gz.sha256).Split()[0]
$actual = (Get-FileHash sightlint-v0.1.0-alpha.2-source.tar.gz -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw "SightLint source archive checksum mismatch" }
```

With a source checkout of the same tag, verify and extract them without trusting tar path
handling:

```bash
python3 tools/release.py verify-archive \
  --tag v0.1.0-alpha.2 \
  --archive sightlint-v0.1.0-alpha.2-source.tar.gz \
  --checksum sightlint-v0.1.0-alpha.2-source.tar.gz.sha256 \
  --extract-dir unpacked
```

The SHA-256 file detects corruption. It does not independently authenticate the publisher; this
alpha relies on the protected Git tag, linked GitHub Actions run, and GitHub HTTPS transport.
Signing and artifact attestation remain explicitly deferred by ADR 0037.

## Prerequisites

- Rust 1.85.0 or newer for the Rust workspace;
- Node 20 through 24 for the private Playwright companion;
- npm and network access to install locked dependencies and the Playwright-pinned Chromium build;
- Python 3.9 or newer for release/corpus verification scripts.

Supported release verification environments are Ubuntu x64, macOS arm64, and Windows x64. The
browser acquisition/product E2E claim is Linux-only. See `docs/compatibility.md` before depending
on a schema or process surface.

## Build the Rust CLI

From the extracted directory:

```bash
cargo build --locked --release -p sightlint-cli --bin sightlint
./target/release/sightlint version
```

On Windows, the executable is `target\release\sightlint.exe`. You may copy that one file to a
directory on your `PATH`, but the Web workflow still needs the source checkout and Node companion.

## Prepare the local Web workflow

```bash
npm ci --ignore-scripts --prefix adapters/playwright
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run build

node adapters/playwright/dist/src/check-cli.js \
  --request evaluation/web/requests/dashboard-browser-unnamed-control.json \
  --repository-root . \
  --sightlint-binary target/release/sightlint \
  --format json
```

The command accepts only the bounded local fixture request contract described in the adapter
README. It is not yet an arbitrary-project installer or general browser audit command.

## Remove the alpha

- delete the extracted source directory;
- delete any manually copied `sightlint`/`sightlint.exe` binary;
- remove `adapters/playwright/node_modules` and `adapters/playwright/dist` if you retained the
  checkout;
- use Playwright's own uninstall command if you want to remove its shared browser cache, after
  checking that no other local Playwright project uses it.

SightLint creates no database, daemon, login item, hosted account, telemetry record, or core
network state. The browser adapter uses temporary capture files and deletes them after the
one-command workflow.

## Maintainer release procedure

1. Merge the focused release PR only after exact-head CI and CodeQL succeed.
2. Verify the resulting `main` commit/tree and its six CI jobs.
3. Create the annotated version tag on that exact `main` commit and push only the tag.
4. The tag-only Release workflow validates the tag/version and current `origin/main`, creates a
   draft source archive/checksum, and transports the exact bytes through a one-day workflow
   artifact to all three supported runner pairs.
5. The workflow publishes the prerelease only after the Linux product E2E and cross-platform
   source tests succeed and the final job byte-compares the verified workflow artifact with the
   draft release assets. A failure leaves a draft for inspection.
6. Verify the release tag, commit, assets, digests, workflow, install instructions, and open
   security alerts through the GitHub API before closing the release issue.

The Release workflow has no pull-request trigger and never changes repository source. Only the
package and publish jobs receive ephemeral `contents: write` permission for release metadata and
assets; verification jobs and ordinary CI remain read-only.

`v0.1.0-alpha.1` is an immutable, unpublished failed release-candidate tag. Its workflow could not
read draft assets from read-only verification jobs. ADR 0038 records the failure and why the tag
was not moved; it is not a supported release.
