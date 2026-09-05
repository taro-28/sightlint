# Contributing to SightLint

SightLint is pre-alpha and architecture-first. Contributions are welcome, but implementation must
not outrun the evidence, contracts, and sequencing recorded in this repository.

## Before opening code

1. Read [`CODEX.md`](CODEX.md), [`AGENTS.md`](AGENTS.md), and the documents in their required
   order.
2. Start from the latest green `main`; verify its exact CI run.
3. Search current issues, accepted ADRs, and active PRs.
4. Select one primary issue and the earliest appropriate roadmap slice.
5. Do not use or revive closed Draft PRs #12–#17 or their legacy branches. Their remaining ideas
   were transferred to issues #22–#27.
6. For a new adapter, rule family, schema/protocol, trust boundary, compatibility policy,
   external service, or durable resource model, open/accept an ADR before implementation.
7. Define the user-visible claim, evidence, applicability, policy, uncertainty, fixtures,
   evaluation, privacy/security/resource model, and explicit non-goals.

Issue #34 is the canonical near-term execution epic. Realistic evaluation (#22), Playwright
acquisition (#23), the first advisory recommended Web pack (#24), and the bounded local agent
fix/rerun loop (#42) are complete. The alpha release gate (#33) is next.

## Development workflow

1. Fetch/prune and update `main` with `--ff-only`.
2. Create one focused branch for one coherent issue slice.
3. Write or update the contract, fixture, evaluation plan, or ADR before implementation when
   applicable.
4. Implement the smallest complete vertical path through the real public command/process.
5. Add independent conformance, acquisition, rule, mutation, hard-negative, and determinism
   evidence as applicable.
6. Run the complete gate in `AGENTS.md` and `docs/development.md`.
7. Update `docs/handoff.md` and `docs/roadmap.md` when facts or priorities change.
8. Open a pull request using the repository template and make its claims commit-specific.
9. Verify all required CI jobs on the exact final head.
10. After merge, verify the actual `main` tree and its own CI, update issues, and confirm that
    GitHub deleted the merged head branch automatically.

Do not create placeholder/final/review/ready/bootstrap/repair/`v2` branch chains. Do not use
self-writing GitHub Actions to assemble, format, repair, commit, or push feature code. Do not leave
unconnected implementations or duplicate “next” modules.

## Complete baseline commands

```bash
python3 tools/generate_e2e_fixtures.py --check
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_inspection_corpus.py --check
python3 tools/check_web_evaluation.py
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked -p sightlint-cli --test e2e
cargo test --locked -p sightlint-cli --test png_filter_e2e
cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
cargo test --locked -p sightlint-cli --test evaluation_corpus
cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
cargo +1.85.0 check --workspace --all-targets --all-features --locked
```

New public generators, adapters, or evaluation targets must be added to CI, `AGENTS.md`, the PR
template, development guide, and handoff in the same change.

## Commit and pull-request style

Use clear, imperative commit messages. Conventional prefixes are encouraged:

- `feat:` user-visible functionality;
- `fix:` defect correction;
- `chore:` tooling or maintenance;
- `docs:` documentation only;
- `refactor:` behavior-preserving restructuring;
- `test:` test/evaluation-only change.

A pull request must explain:

- issue, milestone, exact base, and exact final head;
- reachable user-visible behavior;
- exact versus inferred evidence and trust boundary;
- semantic applicability, policy source, units, tolerance, and alternatives;
- `cantTell`, inapplicable, `untested`, and failure behavior;
- fixtures, mutations, hard negatives, acquisition/rule evaluation, and non-claims;
- privacy, security, resource, dependency, and compatibility effects;
- exact final-head CI and post-merge verification.

Do not describe a capability from unconnected code, a historical branch, or an old CI run. Do not
claim general UI/UX accuracy from synthetic data.

## Code and data quality

- Rust uses the 2024 edition and the workspace lint policy.
- Unsafe Rust is forbidden by default.
- Public APIs require documentation and stable error/version contracts.
- Medium-specific acquisition stays outside the deterministic engine.
- Exact facts, inferred observations, policy, outcome, severity, confidence, and maturity remain
  separate.
- New dependencies require an ownership, license, security, determinism, isolation, platform,
  MSRV, and maintenance rationale.
- Generated data identifies its generator and fails CI on drift.
- Reviewed oracles are not snapshots; changes require a semantic reason.
- Real fixtures require explicit provenance, redistribution rights, and privacy review.
- The model/implementation under evaluation must not generate its own ground truth.

## Repository administration

Administrative issues #19 and #32 are complete. The active `Protect main` ruleset requires an
up-to-date pull request, the five documented CI contexts, linear history, and resolved review
conversations; it blocks force pushes and deletion without routine bypass. Squash is the only
merge method, and merged head branches are deleted automatically. These hosting safeguards do not
relax local, exact-head, or post-merge CI discipline.

## Licensing

No contribution/source license has been selected. Until proposed ADR 0007 and issue #33 are
resolved and a repository license is added, external code contributions should remain discussion,
review, or explicitly authorized work. Public visibility is not permission to use or redistribute
the code or fixtures.
