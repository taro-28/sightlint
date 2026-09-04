# ADR 0008 — Development and release gates

- Status: Accepted
- Date: 2026-09-04
- Owners: @taro-28

## Context

SightLint will be developed substantially by coding agents while the maintainer may review
from a mobile device. Architectural drift is likely unless the repository itself encodes the
required reading, milestone boundaries, validation commands, and evidence expectations.

The project is also a future quality gate, so its own changes need reproducible remote checks
rather than relying on an agent's claim that code works.

## Decision

Use specification-first, pull-request-based development:

1. `AGENTS.md`, accepted ADRs, and the current roadmap milestone are normative.
2. Architectural changes require an ADR before implementation.
3. Functional work uses focused branches and pull requests; `main` is not the working branch.
4. GitHub Actions is the initial canonical remote execution environment.
5. Stable Rust quality checks, Rust 1.85.0 MSRV checking, and Linux/macOS/Windows tests are
   required for the foundation.
6. Each milestone has explicit exit criteria. Feature work for the next milestone does not
   begin until the current foundation is accepted or a deferral is documented.
7. Release publication remains disabled until licensing and versioning decisions are accepted.

## Consequences

- The maintainer can review design, evidence, and CI results remotely.
- Coding agents have explicit constraints instead of relying on conversation memory.
- Foundation changes are somewhat documentation-heavy, but later implementation has a stable
  contract.
- CI runtime is higher because MSRV and cross-platform behavior are tested separately.
- Pull requests may contain several implementation commits but should normally be squash
  merged to keep `main` history focused.

## Alternatives considered

- Direct-to-main agent development: faster initially but weak review and drift control.
- Conversation-only instructions: not available to every agent or future contributor.
- Local-only validation: incompatible with mobile review and not independently observable.
- Implement-first documentation: risks crystallizing accidental technical choices.

## Verification

The repository contains normative agent guidance, ADRs, pull-request and issue templates,
M0 acceptance criteria, and a remote CI workflow. Pull requests state the milestone advanced
and link evidence proving the behavior.
