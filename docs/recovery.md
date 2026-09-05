# Integration recovery — 2026-09-05

## Starting point

Recovery starts from main commit `0b508a1497b47e8d100c8bfd9ec04759f98ee184`.
The source tree, not a PR title, commit message, or earlier agent report, defines what is
implemented. At this starting point, PNG adaptation stopped after bounded inflation.
Filter integration tests referenced APIs that lib.rs did not expose; two unconnected filter
implementations and a broken, write-enabled bootstrap workflow were present.

The repository API also reported `main` as unprotected. This document is not evidence that
branch protection is active. Repository administration must be verified separately from CI.

## Scope of this repair

- Connect the already specified ADR 0024 filter stage and its errors to the public API.
- Keep the original one-based pass/row diagnostic contract used by the committed tests.
- Use one filter implementation and remove the unconnected `_next` copy.
- Remove the temporary bootstrap workflow rather than fixing or adding self-writing CI.
- Keep all existing conformance, product-smoke, adapter, and public-binary tests.
- Fix implementation defects and integration drift; do not weaken tests to obtain green CI.
- Verify the exact repair head through the normal read-only CI workflow.

No RGBA, alpha geometry, background inference, component extraction, OCR, semantics, or new
lint rule is part of this repair. Draft PRs for those stages are not completed capabilities
and must not be merged merely because their titles describe desired behavior.

## Acceptance evidence

Record the final PR head, base, resulting tree, workflow run, five successful required jobs,
and public-command tests before merging. Then check the resulting main source and main CI.
Do not reuse a successful workflow belonging to an older commit or another branch. A
`mergeable` flag is not a successful quality check.

Do not remove tests or ignore failures, force-update main, create placeholder PRs, reopen an
already merged PR, or mutate historical PR metadata to imply completion. Superseded work
should be closed with an explanation, preserving branch history until it has been audited.

## Repository protection remains a separate requirement

The desired main policy is PR-only updates, no force pushes or branch deletion, linear
history, resolved review conversations, and these required CI checks with an up-to-date base:

- Format, lint, test, and docs
- Minimum Rust 1.85.0
- Test on ubuntu-latest
- Test on macos-latest
- Test on windows-latest

The checks must be required for administrators too; the single-maintainer workflow can use
zero required reviewer approvals. Setting this policy requires a repository administration
write capability. A source-controlled policy file or a successful CI run cannot enable it.
Until the GitHub API confirms enforcement, report it as an unresolved administrative task.

## Reporting discipline

Separate (1) code written, (2) reachable public behavior, (3) verified tests, (4) merged code,
and (5) enforced hosting settings. Report limitations plainly. The image-only UX quality
hypothesis is still unproven; successful PNG decoding is not evidence of UX defect detection.
