# ADR 0051 — Multi-family Web evaluation and protected-holdout admission

- Status: Accepted
- Date: 2026-09-06
- Issue: #72
- Parent: #71
- Roadmap: M2/M4 evaluation diversity and governance
- Owners: @taro-28

## Context

ADRs 0032–0035 established a public, repository-owned Atlas fixture family, separate browser
acquisition and rule oracles, an isolated Playwright process, and three advisory recommended Web
rules. The public browser evaluation now covers many states, but every state belongs to the same
fictional dashboard/settings family, every label is visible to implementers, and review is by a
repository maintainer. Those results are valuable regression evidence, but they cannot establish
cross-family precision, independent-review agreement, or holdout generalization.

The original `evaluation/web/corpus.json` compatibility surface describes the first declared-IR
foundation and fixes holdout status to `notEstablished`. Reinterpreting that historical `0.1.0`
document as a multi-family or protected evaluation would silently change its meaning. Conversely,
placing private artifacts or labels in the public repository would expose them to tuning and make
the claimed protection false.

Issue #72 needs an additive evaluation contract before adding a second realistic fixture family.
It must make fixture-family diversity, review independence, exposure, leakage, and holdout
admission inspectable without changing the adapter, rules, or deterministic kernel.

## Decision

Add `evaluation/web/evaluation-v1.json` as an additive Web evaluation registry at schema version
`1.0.0`. It indexes fixture families and their independently versioned acquisition and rule
datasets. It does not replace or reinterpret the existing declared-IR corpus, browser oracle, or
managed-loopback oracle.

Each family record declares:

- a stable family and source identity;
- product context and reviewed user tasks;
- repository source root and revision basis;
- ownership, license, redistribution, privacy, external-asset, network, and processing status;
- public/controlled exposure and whether tuning may use the data;
- oracle-authoring independence from implementation output;
- reviewer roles, qualification categories, independence status, agreement, and adjudication;
- explicit sampling limitations.

Each dataset record joins one family to separate acquisition and rule oracle documents and a
formal command surface. Its sorted case inventory records split and classification without
copying acquisition observations or rule outcomes into the registry. Validators require the
registry, acquisition oracle, and rule oracle to agree on case identity, request, split, and
classification while preserving their different authorities.

The first expansion family is a repository-owned support-inbox application. It is functionally and
visually distinct from Atlas: a queue, conversation detail, reply composer, and related controls
replace the dashboard metric-card context. Its first cases cover a clean control, one targeted
programmatic-name mutation, a visually similar valid hard negative, and an evidence-insufficient
abstention. They use the existing Playwright capture protocol and recommended Rust rule; this ADR
does not add a rule or adapter capability.

## Review status

Review metadata distinguishes annotation authors from independent reviewers and adjudicators.
`maintainerOnly` means the author and reviewer authority are not independent. It cannot be
relabeled `independentlyReviewed` without at least one separately identified reviewer, an
independence statement, an agreement assessment, and any required adjudication record.

The initial Atlas and support-inbox records remain `maintainerOnly`. An automated implementation
agent does not count as an independent human reviewer of labels it helped author. The registry
therefore improves review transparency but makes no independent-review claim in this slice.

## Protected holdout admission

`evaluation/web/holdout-admission.json` is public admission metadata, not a holdout corpus. Its
schema has two states:

- `notOperational` records blockers and forbids holdout-result or maturity claims;
- `operational` requires an exact freeze commit, opaque external bundle identifier and digest,
  separate access-control authority, authorized access roles, an independent evaluator, an
  exposure log, tuning exclusion, pinned evaluation command and environment, oracle-correction
  procedure, and split-specific reporting plan.

Raw protected artifacts and labels remain outside the public repository and ordinary public CI.
The committed digest binds an admitted external bundle without revealing its content. Access to
artifacts and labels must be separated from implementation tuning where feasible; every exposure
is recorded. A correction cannot silently replace a frozen oracle: it requires a new bundle
version/digest, rationale, reviewer, and impact analysis.

The initial status is `notOperational` because no separately controlled bundle or independent
evaluator exists. Public smoke, development, and challenge cases remain tuning-visible and are
never counted as protected holdout data.

## Metrics and claims

Evaluation reports group results by fixture family, split, rule, and evidence class. They report
integer numerators and denominators for:

- executed case coverage;
- failure precision and false-positive failures;
- reviewed abstention agreement;
- targeted mutation kill rate;
- acquisition expectation coverage;
- native/pixel agreement and conflict categories where annotated.

A zero denominator is reported as counts without inventing a percentage. No aggregate UX score is
introduced. Cross-family public counts remain regression evidence and do not estimate population
accuracy. Holdout, independent-review, WCAG-conformance, blocking-maturity, and arbitrary-site
claims remain unavailable until their own evidence gates are satisfied.

## Compatibility and trust boundary

Evaluation registry and holdout-admission schema `1.0.0` are new compatibility surfaces. Existing
Web corpus, browser acquisition/rule schemas, capture/request protocols, Web extension, Artifact
IR, CheckReport, profiles, rule versions, CLI behavior, and exit codes remain unchanged.

Browser acquisition remains an untrusted local process. The Rust kernel remains deterministic and
does not read holdout credentials, start an evaluator, or infer review status. Fixture source,
annotations, and evaluation output remain distinct artifacts. Captured IR, screenshots, and
reports must never be copied into oracle fields as expected truth.

## Alternatives considered

### Extend the original Web corpus in place

Rejected because its `0.1.0` semantics deliberately describe pre-Playwright declared-IR cases and
a non-established holdout. In-place reinterpretation would weaken compatibility and reviewability.

### Commit a hidden or encrypted holdout bundle to the public repository

Rejected. Repository history exposes keys, content, membership, or tuning signals eventually, and
ordinary contributors need not receive private data. Only digest-bound admission metadata belongs
in the public contract.

### Call the second public family a holdout

Rejected because its source, requests, and labels are visible to implementation authors.

### Add a new rule with the second fixture family

Rejected for this focused slice. Evaluation diversity and governance are prerequisites for later
rule admission; combining them would make rule behavior its own evidence gate.

## Non-goals

- actual protected-holdout results or an independent-review claim;
- representative real-world Web precision, recall, accessibility, or usability claims;
- a new or promoted rule, blocking enforcement, or universal score;
- Playwright protocol, browser-support, arbitrary-repository, iframe, shadow-DOM, or SPA changes;
- OCR, CV, VLM, hosted processing, customer data, or third-party fixture content.

## Verification

- Strict schemas reject mixed authority, unknown fields, invalid family/dataset references,
  undeclared exposure, inconsistent review states, and an operational holdout missing any required
  freeze/access/evaluator/leakage/execution/correction/reporting record.
- Public evaluation checks source digests, one-to-one acquisition/rule joins, family and split
  inventories, implementation-output exclusion, clean/mutation/hard-negative/abstention coverage,
  and metric denominators.
- The existing Playwright process and built `sightlint` binary execute every new runnable case at
  least twice with byte-stable response, IR, screenshot, report, diagnostics, and exit code in the
  declared compatibility environment.
- Existing declared-IR, browser, managed-loopback, interaction, image, medium-adapter, release,
  MSRV, Linux, macOS, and Windows gates remain green.
