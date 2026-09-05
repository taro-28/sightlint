# Historical integration recovery — 2026-09-05

> **Historical record, not the current starting point.** Continue development from the latest green
> `main` and read `docs/handoff.md`. This document explains an earlier failure and the process
> safeguards derived from it.

## Incident starting point

Recovery began from main commit `0b508a1497b47e8d100c8bfd9ec04759f98ee184`.
At that point, PNG adaptation stopped after bounded inflation. Filter integration tests referenced
APIs that `lib.rs` did not expose; two unconnected filter implementations and a broken,
write-enabled bootstrap workflow were present.

The repository API also reported `main` as unprotected. This document has never been evidence that
branch protection is active. Repository administration remains tracked separately in issue #19.

## Repair performed by PR #18

- connected the specified filter stage and errors to the public API;
- retained the one-based pass/row diagnostic contract used by committed tests;
- consolidated the filter implementation and removed the unconnected `_next` copy;
- removed the temporary write-enabled bootstrap workflow;
- kept existing conformance, product-smoke, adapter, and public-binary tests;
- corrected an E2E oracle that referenced the nonexistent `findings` field instead of the actual
  `results` contract;
- changed ordinary CI to a read-only verification workflow;
- verified the exact repair head and then the merged `main` commit across quality, MSRV, Linux,
  macOS, and Windows.

Later PRs #20 and #21 added the current staged raster corpus and advisory region/gap inspection on
top of the recovered line. See `docs/handoff.md` for the current capability.

## What caused the failure

The remote/mobile implementation phase accumulated process risks:

- multiple branch names for one logical change;
- placeholder and stale Draft PRs that appeared more advanced than verified `main`;
- self-writing GitHub Actions used to assemble, format, repair, or commit feature code;
- duplicate modules waiting for later wiring;
- tests for APIs that the public library did not expose;
- success reports based on old or partial workflow runs;
- confusion between a PR's `mergeable` flag and successful required checks;
- branch-only ADRs whose `Accepted` header could be mistaken for repository policy;
- continued feature work before restoring one trustworthy integration baseline.

The technical image/IR architecture was not invalidated. The incident was a source-of-truth,
integration, and reporting failure.

## Permanent safeguards

These rules are now encoded in `AGENTS.md`, `docs/handoff.md`, `docs/development.md`, the PR
template, and issue #32:

1. the latest green `main` is the only base for new work;
2. one focused issue, branch, and PR per coherent slice;
3. no placeholder/final/review/ready/bootstrap/repair/`v2` branch chains;
4. future intent lives in issues/roadmap rather than abandoned Draft implementation PRs;
5. GitHub Actions verify but do not write feature code;
6. no temporary workflow with write permissions survives a feature branch or merge;
7. no unconnected source implementation or duplicate “next” file;
8. public behavior is tested through the actual built command/process;
9. final-head CI is commit-specific and cannot be replaced by an earlier run;
10. the merged `main` tree and its own CI are checked before reporting completion;
11. branch names, PR descriptions, and agent prose are never implementation evidence;
12. conformance, acquisition, semantic rule quality, and real-world product validity are reported
    separately.

## Superseded Draft PR cleanup

Draft PRs #12–#17 were created before or around the recovery and later diverged from current
`main`. Their last recorded CI runs failed. During the local-Codex handoff they were closed with
explicit replacement links:

- #12: duplicate filter work already integrated through #10/#18;
- #13: optional broader PNG normalization preserved in issue #27;
- #14: alpha-visible geometry preserved in issue #26;
- #15 and #17: background/segmentation alternatives preserved in issue #25;
- #16: evaluation intent superseded by current corpora and realistic-data gate #22.

Do not reopen or merge those branches. Branch deletion is tracked by issue #32.

## Repository protection remains separate

The desired `main` policy is:

- PR-only updates;
- up-to-date required checks;
- no force pushes or branch deletion;
- linear history;
- resolved review conversations;
- required checks for administrators, with zero reviewer approvals acceptable for the
  single-maintainer workflow.

Required check names are recorded in issue #19. A source-controlled file or successful CI run
cannot enable hosting protection. Until a GitHub API/UI read confirms it, report protection as
unresolved.

## Reporting discipline

Always distinguish:

1. code written;
2. code connected to reachable public behavior;
3. tests run on an exact commit;
4. code merged into `main`;
5. `main` CI on the merged commit;
6. hosting settings enforced;
7. product accuracy demonstrated on appropriate evaluation data.

For example, successful PNG decoding or synthetic gap measurement is not evidence of general
screenshot UI/UX accuracy. A heuristic observation is not a blocking rule result.

## Current continuation

The incident is closed as an implementation recovery. The next product sequence is issue #34:
realistic evaluation (#22), Playwright native/pixel acquisition (#23), evaluated recommended rules
(#24), a local Codex fix-and-rerun workflow, and then the release gate (#33).
