# Development

## Toolchain

SightLint uses Rust 2024. The declared minimum supported Rust version is 1.85.0, the first
stable toolchain supporting the 2024 edition. Local development follows the `stable` channel;
CI also checks the minimum version.

Install Rust through `rustup`, then clone the repository. The checked-in
`rust-toolchain.toml` installs `rustfmt` and Clippy.

## Commands

```bash
cargo check-all   # workspace, all targets and features
cargo lint        # Clippy with warnings denied
cargo test-all    # workspace tests
cargo docs        # workspace documentation
```

The canonical CI commands are also listed in `AGENTS.md`.

## Branch and PR workflow

- `main` should remain reviewable and releasable for the current project phase.
- Use `chore/`, `feat/`, `fix/`, `docs/`, or `refactor/` prefixes for branches.
- Architectural changes start with an ADR.
- Feature implementation follows the current roadmap milestone.
- Pull requests must include evidence, tests, compatibility notes, and uncertainty behavior.

## Remote development

GitHub Actions is the canonical remote execution environment during early development. This
allows work and review from a mobile device while keeping build commands visible and
repeatable. The same Cargo commands run locally and remotely.

The CI workflow checks:

- formatting, Clippy, tests, and documentation on stable Rust
- compilation on Rust 1.85.0
- tests on Linux, macOS, and Windows

## Dependency policy

The foundation crates intentionally use no third-party Rust dependencies. Add dependencies
only when a milestone requires them, explain the choice in the PR, and prefer narrowly scoped,
well-maintained crates. Dependency licensing and advisory policy will be enabled after the
repository license decision.

## Generated artifacts

Generated schemas, reports, fixtures, or bindings must state:

- source of truth
- generator command and version
- whether the output is canonical or derived
- whether CI verifies it is up to date

Do not hand-edit generated files.

## Schema and rule changes

A serialized schema change must include compatibility fixtures. A rule semantics change must
explain whether it is a fix, a new major rule version, or a policy/configuration change.
Stable rule IDs should never silently change meaning.
