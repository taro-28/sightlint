# Security policy

SightLint parses untrusted artifacts and may eventually execute browser or platform
adapters. Treat every input and adapter as hostile.

## Supported versions

No released version is supported yet. Security fixes apply to the default branch during the
pre-alpha phase.

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
