# ADR 0038 — Workflow-artifact verification and immutable release-tag recovery

- Status: Accepted
- Date: 2026-09-06
- Owners: @taro-28
- Amends: ADR 0037

## Context

The first run of ADR 0037's release workflow created a valid draft for the public annotated tag
`v0.1.0-alpha.1`, then every verification job failed before extraction. GitHub's release API does
not expose draft releases to the jobs' normal `contents: read` token, so `gh release download`
reported `release not found`. Giving the cross-platform test matrix `contents: write` would make
the download work but would violate the least-privilege boundary that the release contract was
designed to preserve.

The tag was already pushed publicly. Moving or deleting and recreating it would make a released
name resolve to different source and weaken the provenance model even though the draft itself was
never published.

## Decision

Keep `v0.1.0-alpha.1` as an immutable, unpublished failed release-candidate tag. The first
publishable release becomes `v0.1.0-alpha.2`, with the workspace version and release documentation
updated to match. Remove the stale alpha.1 draft only after its failed workflow, asset digests, and
reason are recorded; retain the Git tag.

Use one short-retention GitHub Actions workflow artifact as the prepublication transport:

1. the package job creates the source archive and checksum once in `release-dist/`;
2. it uploads those exact files both as a one-day, no-extra-compression workflow artifact and as
   draft release assets;
3. read-only Linux x64, macOS arm64, and Windows x64 verification jobs download the workflow
   artifact, verify/extract it, and test the extracted source;
4. the read-only Linux product job runs the complete extracted Playwright/SightLint path;
5. only after those jobs succeed, the final `contents: write` job downloads both the workflow
   artifact and the draft release assets, compares the two files byte-for-byte, verifies the
   checksum record, and publishes the draft.

The workflow uses the official `actions/upload-artifact` and `actions/download-artifact` actions
pinned to immutable commit SHAs. Add only those two action families to the repository's selected
Actions allowlist. Normal CI remains read-only and the release workflow still cannot edit, commit,
or push repository source.

The workflow artifact is transport evidence, not a second release channel. It expires quickly;
the GitHub prerelease assets, tag, release workflow, and checksum remain the durable public record.

## Consequences

- Verification jobs retain `contents: read` and cannot mutate repository or release state.
- Every host tests the exact archive/checksum bytes later compared with the draft assets.
- The final publish job already needs release write permission, so draft access and final byte
  comparison do not widen the authority of another job.
- Alpha.1 remains visible as a tag whose commit contains the original failed workflow. It is not a
  supported or published version; release notes and changelog must state this without presenting
  it as a user-facing release.
- A failed rerun still leaves an inspectable draft and workflow artifact; publication remains
  fail-closed.

## Alternatives considered

- Give all verification jobs `contents: write`: operationally simple but unnecessarily grants
  release/repository mutation authority to the largest job set.
- Publish first and verify the public assets afterward: exposes an unverified release and makes
  failure rollback destructive.
- Rebuild the archive independently in every job: tests equivalent source, not necessarily the
  exact bytes uploaded to the release, and cross-platform compressors need not be byte-identical.
- Move or recreate the alpha.1 tag after repairing `main`: makes one public version name resolve to
  different commits over time and damages provenance.
- Use an external object store or long-lived credential: adds a service, secret, retention, and
  privacy boundary that the alpha does not need.

## Verification

- Pull-request CI and `actionlint` validate the pinned workflow structure without write access.
- Release unit tests continue to prove deterministic archive bytes, checksum rejection, and safe
  extraction.
- The alpha.2 run must show package, three-host source, Linux product, byte-comparison, and publish
  jobs all successful on the tag's exact protected `main` commit.
- The published release asset digest and downloaded checksum are compared independently after the
  workflow completes.
- The alpha.1 draft is removed only after the failure evidence above is recorded, while both tags
  remain immutable.
