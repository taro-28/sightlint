# Development

This guide describes the local development workflow. Coding agents must also follow `AGENTS.md`,
`docs/handoff.md`, and the selected issue. Those files are normative when this guide is less
specific.

## Toolchain

SightLint uses Rust 2024. The declared minimum supported Rust version is 1.85.0, the first stable
toolchain supporting the 2024 edition. Local development follows the `stable` channel; CI also
checks the minimum version.

Install Rust through `rustup`, clone the repository, and allow the checked-in
`rust-toolchain.toml` to install `rustfmt` and Clippy. The Playwright adapter uses pinned
TypeScript/Node dependencies outside the deterministic Rust kernel; future process adapters may
likewise use the language that best matches their platform while declaring a pinned toolchain.

## Authoritative starting point

New work always starts from the latest green `main`.

```bash
git fetch --all --prune
git switch main
git pull --ff-only
git status --short --branch
git log -1 --format='commit=%H tree=%T subject=%s'
```

Before editing:

1. confirm the corresponding `main` CI completed successfully;
2. read `AGENTS.md`, `docs/handoff.md`, `docs/product-rationale.md`,
   `docs/decision-history.md`, and `docs/roadmap.md`;
3. read the selected issue and accepted ADRs;
4. search for an existing active branch or PR for that issue;
5. inspect current source and tests rather than an old implementation branch.

At handoff time, Draft PRs #12–#17 were closed as superseded. Their branches are reference history
only. Issue #32 tracks branch deletion. Do not revive, merge, or use one as a base because its name
appears to describe a desired future feature.

## Current task sequence

Issue #34 is the near-term execution epic. Unless a repair is required, prefer:

1. #22 — realistic evaluation corpus and annotation process (complete);
2. #23 — Playwright web adapter and native/pixel evidence matrix (complete);
3. #24 — first evaluated advisory recommended Web pack (complete);
4. the local agent edit/check/fix/rerun demo in #34 (next);
5. #33 — license, packaging, compatibility, and alpha release.

Other work remains available in #25–#32, but a pre-existing stale branch does not make it higher
priority. Explain any deviation in the issue and PR.

## Planning before implementation

Write a short implementation plan that states:

- primary issue and roadmap milestone;
- user-visible claim that will become reachable;
- exact, declared, empirical, or inferred evidence used;
- trust/process boundary;
- applicability, policy source, units, and tolerance;
- possible outcomes and ambiguity behavior;
- pass, fail/mutation, `cantTell`, inapplicable, `untested`, malformed, boundary, resource,
  determinism, differential, and product-evaluation cases that apply;
- privacy, security, compatibility, and release impact;
- explicit non-goals;
- whether an ADR must be accepted first.

If the claim and evidence cannot be stated precisely, work on the specification, benchmark, or
corpus before production implementation.

## Branch and PR workflow

Create one focused branch from `main`:

```bash
git switch -c feat/<focused-name>
```

Allowed conventional prefixes include `feat/`, `fix/`, `test/`, `docs/`, `refactor/`, and
`chore/`.

Rules:

- one primary issue, one branch, and one PR per coherent vertical slice;
- no placeholder/final/review/ready/bootstrap/repair/`v2` branch chains for one task;
- do not push feature work directly to `main`;
- do not use GitHub Actions to assemble, format, repair, commit, or push feature code;
- do not leave parallel implementations waiting for “later wiring”;
- architecture, schema, trust-boundary, protocol, compatibility, or policy changes begin with an
  ADR;
- implement through the public command/process path, not only an internal module;
- update `docs/handoff.md` and `docs/roadmap.md` whenever current behavior or priority changes;
- future ideas belong in issues rather than long-lived Draft implementation PRs;
- close a superseded PR with an explicit replacement link and preserve unique rationale before
  abandoning it.

A PR may begin as Draft while implementation is active, but it should not remain an unofficial
backlog item after work stops. Convert to ready only after the final head passes the complete local
and remote gates.

## Local commands

Common Cargo aliases are available:

```bash
cargo check-all   # workspace, all targets and features
cargo lint        # Clippy with warnings denied
cargo test-all    # workspace tests
cargo docs        # workspace documentation
```

The complete current gate is:

```bash
python3 tools/generate_e2e_fixtures.py --check
python3 tools/generate_raster_corpus.py --check
python3 tools/generate_inspection_corpus.py --check
python3 tools/check_web_evaluation.py
npm --prefix adapters/playwright ci --ignore-scripts
npm --prefix adapters/playwright run install:browser
npm --prefix adapters/playwright run check
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:e2e
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

The full workspace test already includes integration tests; the explicit targets make important
public product contracts visible in CI and local reviews. A new corpus or process adapter adds its
own check to the generator, normal CI, `AGENTS.md`, the PR template, and handoff.

Do not delete, skip, ignore, snapshot-bless, or loosen an oracle simply to make a branch green.
Correct the implementation or explain and review a genuine semantic contract change.

## End-to-end implementation shape

Prefer the smallest complete vertical path:

```text
committed native input
  -> adapter or acquisition process
  -> validation and explicit observations
  -> reconciliation and Artifact IR / advisory contract
  -> deterministic rule or clearly labeled observation report
  -> public command
  -> stdout, stderr, exit code, evidence, and oracle assertions
```

Avoid implementing a broad library surface before one public path proves it. Tests for an API that
is not exported or not called by the actual command are not product completion.

## Remote CI

GitHub Actions is the canonical cross-platform verification environment. The normal workflow is
read-only and checks:

- committed fixture/corpus reproducibility;
- rustfmt and Clippy with warnings denied;
- full tests and explicit public-binary evaluation targets;
- rustdoc with warnings denied;
- Rust 1.85.0 compilation;
- Linux, macOS, and Windows tests.

Verification is commit-specific:

1. run local gates;
2. push the final head;
3. verify all required jobs on that exact head;
4. review the complete changed-file list and trust/evidence boundaries;
5. merge with the expected head SHA;
6. verify the resulting `main` tree and its own CI.

`mergeable: true`, a successful earlier workflow, or a green branch that moved afterwards is not
sufficient evidence.

Branch protection is not active yet and is explicitly deferred in #19. That means GitHub may not
prevent a bad merge; it does not relax the workflow above. Repository/branch cleanup remains #32.

## Pull request content

The PR description must include:

- linked issue and milestone;
- exact base and final head;
- reachable user-visible behavior;
- exact/inferred evidence, assumptions, uncertainty, and policy source;
- conformance, acquisition, rule, and product oracle changes;
- hard negatives and valid alternatives;
- privacy, security, resource, and compatibility effects;
- explicit non-claims;
- exact final-head CI run and all required job outcomes.

The PR cannot claim general UI/UX accuracy from synthetic fixtures. An advisory acquisition report
is not a trusted rule result. A model confidence value is not proof of applicability.

## Dependency policy

Add dependencies only when the current issue requires them and the selected architecture benefits
from them. Explain:

- ownership and maintenance status;
- license and supply-chain implications;
- security history and untrusted-input surface;
- deterministic/version behavior;
- platform/MSRV impact;
- resource use and isolation;
- why a dependency is safer or more maintainable than custom code.

The lesson from the PNG phase is not “never use dependencies.” It is to avoid spending SightLint's
product effort on custom codec breadth without evidence. Issue #27 requires an explicit
library-versus-custom decision before broad PNG support.

No hosted service, database, model runtime, GUI, MCP server, or plugin framework should be added
before its roadmap issue requires it.

## Generated artifacts and evaluated data

Generated schemas, reports, fixtures, or bindings must state:

- source of truth and generator command/version;
- whether output is canonical or derived;
- whether CI checks it for drift;
- provenance/license/privacy status;
- which assertions are conformance, acquisition ground truth, or semantic rule ground truth.

Do not hand-edit generated files.

Reviewed product-evaluation oracles are not ordinary snapshots. Changing one requires a reason,
review of the underlying semantics, and consideration of holdout leakage. Never derive expected
regions or outcomes by running the implementation being evaluated.

## Schema, protocol, and rule changes

A serialized change must define compatibility separately for:

- Artifact IR;
- report schema;
- adapter/perception process protocol;
- namespaced extension;
- rule ID and semantic version;
- configuration/profile schema;
- CLI/output/exit behavior;
- evaluation manifest.

A rule semantics change must state whether it is a bug fix, a new rule version, or a policy change.
Stable IDs must never silently change meaning.

New ADR numbers continue at 0036 or later. Branch-only ADRs 0025–0029 are historical references,
not accepted decisions.

## Privacy and untrusted inputs

Core operation is local-first. A new adapter or worker must specify:

- whether it reads arbitrary files, URLs, pages, processes, or devices;
- allowed schemes/paths and sandbox/process boundaries;
- time, memory, node, page, frame, output, and artifact limits;
- whether anything leaves the machine;
- exact transmitted fields, endpoint/model, retention assumptions, and opt-in mechanism for any
  remote mode;
- how secrets, personal data, customer data, and licensed artifacts are excluded from fixtures.

A reduced/redacted input is evidence about that transformed artifact; do not pretend it is
identical to the original.

## After merge

- verify the expected commit and tree on `main`;
- verify `main` CI on that exact commit;
- update/close the issue and any superseded design records;
- update handoff/roadmap if facts changed;
- delete the branch when repository settings permit;
- check that no temporary workflow, duplicate module, generated drift, or open stale Draft remains.

## Repository administration and release

- #19: branch protection and required checks;
- #32: legacy branch pruning and automatic deletion/settings;
- #33: license, compatibility, packaging, security checks, and alpha distribution.

Do not report any of these as complete before direct repository/release evidence confirms them.
