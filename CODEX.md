# Continue SightLint in local Codex

This repository, not the earlier ChatGPT conversation, is the complete handoff.

## Mandatory reading

Read in this order before editing:

1. [`AGENTS.md`](AGENTS.md)
2. [`docs/handoff.md`](docs/handoff.md)
3. [`docs/product-rationale.md`](docs/product-rationale.md)
4. [`docs/decision-history.md`](docs/decision-history.md)
5. [`docs/vision.md`](docs/vision.md)
6. [`docs/principles.md`](docs/principles.md)
7. [`docs/architecture.md`](docs/architecture.md)
8. [`docs/artifact-ir.md`](docs/artifact-ir.md)
9. [`docs/rules.md`](docs/rules.md)
10. [`docs/testing-strategy.md`](docs/testing-strategy.md)
11. [`docs/evaluation-strategy.md`](docs/evaluation-strategy.md)
12. [`docs/roadmap.md`](docs/roadmap.md)
13. the selected issue and its accepted ADRs
14. [`docs/development.md`](docs/development.md)

## Source of truth

Start from the latest green `main`. Closed PRs and legacy branches are historical reference only.
Do not reopen or merge superseded Draft PRs #12–#17. Do not recreate or use their deleted branches
as a base. Issue #32 records the completed branch cleanup.

```bash
git fetch --all --prune
git switch main
git pull --ff-only
git status --short --branch
git log -1 --format='commit=%H tree=%T subject=%s'
```

Verify the corresponding `main` CI before planning.

## Canonical next work

Issue #34's bounded execution sequence is complete:

1. #22 — realistic, human-reviewed UI evaluation and hard negatives (complete);
2. #23 — isolated Playwright web adapter with native/pixel reconciliation (complete);
3. #24 — evaluated zero-setup recommended rule packs (complete);
4. #42 — Codex edit/check/fix/rerun demonstration within #34 (complete);
5. #33 — license, compatibility, packaging, and first alpha release (complete).

Issue #25 is complete as an evaluation-only benchmark and did not admit a broader segmentation
policy. Issue #26 adds exact source-alpha geometry without admitting an alpha-padding rule. Issue
#27 is complete through ADR 0041 without broadening PNG decoding: current product evidence did not
establish a format gap, unsupported formats remain explicit, and no decoder dependency was added.
#28 is complete for protocol v0 through ADR 0042: the local bounded worker boundary and typed
perception records are implemented, while real OCR/model quality remains `untested`. Issue #29's
PPTX, PDF, and Android focused slices are implemented through ADRs 0043–0045. The Android slice
uses a bounded local file adapter over a repository-owned instrumented fixture, keeps View,
accessibility, and PNG evidence distinct, and makes no general mobile-quality claim. iOS is next
inside #29, followed by #30–#31. Existing stale code does not change priority.

## Non-negotiable workflow

- one focused issue, branch, and PR from current `main`;
- ADR before architecture/schema/protocol/trust/policy changes;
- no self-writing GitHub Actions;
- no placeholder/final/review/ready/`v2` branch chains;
- no unconnected modules or tests for unreachable APIs;
- public-binary/process E2E plus separate acquisition and rule evaluation;
- exact final-head CI and post-merge `main` CI;
- update handoff and roadmap whenever current facts or priority change;
- uncertainty becomes `cantTell`/`untested`, never fabricated certainty;
- model or heuristic semantics do not block by default;
- no universal UX score;
- local-first and no artifact transmission by default.

The complete local validation command list is in `AGENTS.md` and `docs/development.md`.
