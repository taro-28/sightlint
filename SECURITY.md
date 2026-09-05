# Security policy

SightLint parses untrusted artifacts and may eventually execute browser or platform
adapters. Treat every input and adapter as hostile.

## Supported versions

Security fixes are best-effort for the current `0.1.0-alpha.1` source release and the default
branch. Alpha compatibility is documented in `docs/compatibility.md`; no long-term support window
or stable-API guarantee exists yet.

## Reporting a vulnerability

Use GitHub's [private vulnerability reporting
form](https://github.com/taro-28/sightlint/security/advisories/new) for this repository. Do not
publish exploit details in a public issue.

## Security expectations

- The deterministic kernel must not access the network.
- Artifact parsers must enforce resource limits and reject malformed or oversized input.
- External perception workers must be opt-in and clearly disclose data transmission.
- Browser and document adapters must run with least privilege.
- Results from untrusted adapters require provenance and validation before entering the
  trusted rule engine.

See `docs/threat-model.md` for the working threat model.
