# Contributing to SightLint

SightLint is pre-alpha and architecture-first. Contributions are welcome, but broad feature
work should not outrun the contracts documented in this repository.

## Before opening code

- Read `AGENTS.md` and the documents it references.
- Search existing issues and architecture decisions.
- For a new adapter, rule family, external service, or schema change, open an issue or ADR
  first.
- Keep pull requests focused and independently reviewable.

## Development workflow

1. Create a branch from `main`.
2. Write or update the contract, fixture, or ADR before implementation when applicable.
3. Implement the smallest vertical slice that satisfies the contract.
4. Run the repository checks.
5. Open a pull request using the provided template.

Common commands:

```bash
cargo check-all
cargo lint
cargo test-all
cargo docs
```

## Commit and pull-request style

Use clear, imperative commit messages. Conventional prefixes are encouraged:

- `feat:` user-visible functionality
- `fix:` defect correction
- `chore:` tooling or maintenance
- `docs:` documentation only
- `refactor:` behavior-preserving restructuring
- `test:` test-only change

A pull request must explain:

- which contract or milestone it advances
- which evidence proves the behavior
- how uncertainty and failure modes are represented
- whether the serialized schema or compatibility surface changes

## Code quality

- Rust code uses the 2024 edition and the workspace lint policy.
- Unsafe Rust is forbidden by default.
- Public APIs require documentation.
- New dependencies require a reason in the pull request and must pass dependency policy
  checks once those checks are enabled.
- Generated files must identify their generator and source of truth.

## Licensing

No contribution license has been selected yet. Until ADR 0007 is accepted and a repository
license is added, external code contributions should be limited to discussion and review.
