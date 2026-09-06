# ADR 0052 — External holdout manifests and sanitized run attestations

- Status: Accepted
- Date: 2026-09-06
- Issue: #75
- Parent: #71
- Blocks: #74
- Roadmap: M2/M4 protected-evaluation foundation
- Owners: @taro-28

## Context

ADR 0051 adds a multi-family public Web evaluation registry and a strict admission record for a
future protected holdout. The admission record deliberately remains `notOperational`: the public
repository has no independently administered bundle, evaluator, exposure log, or run. This is the
correct current claim, but it leaves external operators without a stable byte contract for
freezing a bundle, binding an invocation and environment, recording private results, or publishing
safe evidence.

Putting protected fixture source, membership, labels, screenshots, selectors, or per-case results
in this repository would expose them to implementation tuning. Running the protected data in
ordinary GitHub Actions would expose it to repository workflows and credentials. Conversely, a
free-form statement that an evaluator ran “the same version” would not bind the evaluated source,
binary, adapter, bundle, oracle, command, or environment closely enough to support a defensible
claim.

Issue #75 needs a code-only foundation before issue #74 can perform a real independent review and
protected run. The foundation must exercise its protocol with public fictional conformance data
without relabeling that data as holdout evidence.

## Decision

Define five independently versioned JSON surfaces at version `1.0.0`:

1. a **protected bundle manifest** inventories opaque families, cases, raw artifact digests,
   source/ownership/license provenance, and privacy/retention review;
2. a **protected oracle manifest** records separate acquisition and rule annotation document
   digests and reviewed outcome-category counts;
3. an **invocation manifest** binds the repository commit and tree, build-input archive, public
   binary, adapter lock, command templates, declared runtime environment, and resource limits;
4. a **private result manifest** binds per-case execution output digests and aggregate source
   counts; and
5. a **sanitized public attestation** binds the other four manifests, evaluator and second-verifier
   declarations, admission/exposure state, lifecycle, and only disclosure-safe aggregate metrics.

The schemas are public. A fictional chain under `evaluation/web/conformance/holdout/` is public
conformance data and is permanently ineligible as product evidence. Real instances of the first
four surfaces remain with the external storage authority. Only the sanitized attestation may be
committed after issue #74 establishes an operational admission and a valid independent run.

The canonical current public record is `evaluation/web/holdout-run.json`. It remains `notRun`,
contains no protected binding or result, and must agree with
`evaluation/web/holdout-admission.json` being `notOperational`.

## Canonical bytes and digest boundaries

Digest-bound manifests use a deliberately smaller JSON domain rather than attempting to implement
all of RFC 8785:

- UTF-8 without a byte-order mark;
- JSON objects, arrays, printable ASCII strings, booleans, null, and base-10 integers only;
- no floating-point values;
- object keys serialized in ascending Unicode-code-point order;
- array order retained and separately required to follow each schema's stable ID/order field;
- no insignificant whitespace and no trailing newline;
- JSON string escaping produced by the standard JSON grammar; because values are printable ASCII,
  Unicode normalization does not enter the boundary.

Every manifest has a top-level `manifestDigest`. Its value is SHA-256 over the canonical bytes of
the same object with only that top-level field removed. Raw source, screenshot, request, oracle,
binary, lock, environment, and result artifacts are independently SHA-256-bound as raw bytes.
Paths in private manifests are normalized relative POSIX paths with no empty, dot, parent, absolute,
drive, URL, or backslash forms. Public attestations contain no artifact paths or URLs.

The first version bounds each input file to 1 MiB, families to 128, cases to 4,096, files per case
to 64, aggregate metric cells to 512, command arguments and rule bindings to 128, and individual
printable strings to 512 bytes unless a smaller schema limit applies. Environment fields use a
closed schema rather than an arbitrary entry map. The verifier
rejects the exact one-over boundary before retaining partial output.

## Authority and separation

The bundle manifest is artifact inventory, not an oracle. The oracle manifest is expected
acquisition/rule truth, not implementation output. The invocation manifest is execution intent,
not a result. The private result manifest describes observations produced by an execution, not
reviewed truth. The public attestation binds and summarizes those authorities without copying
case membership, labels, selectors, paths, screenshots, or per-case results.

Acquisition and rule oracle documents remain separate. The bundle and oracle manifests may share
opaque case IDs only inside the protected store. Implementation output is forbidden as an oracle
authoring basis. A correction creates a new oracle version and digest, records reviewer rationale
and impact, and either refreezes or supersedes affected evidence; it never silently replaces the
old bytes.

Browser automation and protected-data parsing remain untrusted external processes. The
deterministic Rust rule kernel receives only normalized Artifact IR and configuration. Neither the
kernel nor ordinary public CI reads holdout credentials or private manifests.

Every bundle records a source authority, ownership basis, license identifier, redistribution
decision, privacy review, personal/customer/credential status, external-processing status, and
retention-policy version. Protected data may be controlled and non-redistributable; those fields
do not authorize publication. Public conformance data must be repository-authored fictional data,
redistributable under `MIT OR Apache-2.0`, processed locally, and contain no personal data,
customer data, or credentials.

## Invocation and environment binding

The invocation manifest binds:

- exact 40-hex source commit and tree;
- SHA-256 of a reproducible source/build-input archive;
- SHA-256 of the built `sightlint` binary;
- SHA-256 of the Playwright package lock and compiled adapter entry point;
- explicit argument arrays using a closed set of placeholders instead of shell text;
- rule/profile/configuration versions and the expected exit-code contract;
- operating system, architecture, Rust, Node.js, Playwright, Chromium, locale, timezone, theme,
  reduced-motion, viewport, device-scale, and text-scale declarations;
- environment-manifest digest and bounded resource policy.

No shell interpolation is part of the contract. Secrets and private absolute paths are supplied by
the external runner and must not appear in a public attestation. A change to any bound input,
command, version, or material environment value requires a new manifest digest and new run.

## Lifecycle

The public run lifecycle keeps these states distinct:

- `notRun`: no eligible protected evidence exists; blockers are explicit;
- `valid`: a successful run is bound to an operational admission and all required external
  declarations and digests;
- `invalidated`: previously produced evidence is no longer eligible because of exposure, drift,
  conflict, correction, or verification failure;
- `superseded`: an immutable prior record points to a newer admitted record and is not current
  evidence.

The `recordPurpose` is separately `currentStatus`, `holdoutEvidence`, or `conformanceExample`.
Public conformance examples may exercise a shape equivalent to `valid`, but
`evidenceEligible` is always false and the verifier labels them `conformance-only`. A current
`valid` evidence record requires ADR 0051 admission status `operational`; status text alone cannot
make it valid.

Exposure after freeze, access by a tuning role, command/environment/binary drift, digest mismatch,
missing second verification, failed execution, or an unversioned oracle correction invalidates the
run. Invalidation and supersession records are append-only external history; the public current
record names the applicable immutable record digest without erasing prior facts.

## Sanitized metrics and disclosure

The private result manifest derives integer source counts by fixture family, split, rule, and
evidence class. The public attestation may contain only these named measures:

- executed case coverage;
- acquisition expectation coverage;
- rule-result coverage;
- failure precision;
- false-positive failures;
- reviewed abstention agreement;
- targeted mutation kill rate;
- execution errors; and
- retained native/pixel conflicts.

Every reported metric stores an integer numerator and denominator, not a percentage. The
numerator must not exceed the denominator. A zero denominator is a distinct `zeroDenominator`
cell with explicit zero counts. A nonzero cell below the declared publication threshold is
`suppressed` and contains neither count. The initial threshold is at least five denominator units;
an external disclosure authority may choose a higher value. Suppressed cells are still bound by
the private result digest.

Scopes use opaque cohort IDs and declared metric/rule/evidence categories, never private family or
case names. There is no universal score, weighted rollup, rank, or inferred percentage. Published
counts remain sample-specific evidence and do not establish representative accuracy, WCAG
conformance, or blocking maturity.

## Declarations and assurance boundary

The attestation binds separate evaluator and verification declaration digests, stable project
identifiers, qualifications, independence-from-tuning statements, conflict-of-interest review,
and the external authority that verified their detached signatures. Version `1.0.0` does not add a
cryptographic dependency or trust root to SightLint. The public verifier checks declared fields
and digest relationships; it cannot prove a person's identity, qualification, independence, store
policy, signature validity, or absence of undisclosed leakage.

The attestation therefore says `cryptographicVerificationPerformedBySightLint: false`. Issue #74
requires real external authorities and a second verifier. Later signature infrastructure, if
demanded, requires its own ADR and cannot retroactively upgrade these declarations.

## Public verifier

Add a standard-library, read-only Python process. Its default mode validates the canonical
`notRun` public record against current admission. A conformance mode validates the complete
fictional public chain, recomputes every manifest digest, checks cross-document bindings and
aggregate arithmetic, and emits an explicit `evidence_eligible=false` result.

The process does not execute a browser, build a binary, open network connections, read environment
secrets, mutate files, generate oracles, or write results. It emits one stable stdout line on
success, stable categorized diagnostics on stderr, and exit code 2 on invalid input before any
partial success output.

## Compatibility

These are additive evaluation surfaces. They do not modify Artifact IR, CheckReport, adapter or
perception protocols, rule IDs or semantics, profiles, enforcement, CLI behavior, or the
historical Web corpora. `evaluation-v1.json` and `holdout-admission.json` remain version `1.0.0`.
The run record references them rather than adding a required field to the existing registry
schema.

Schema evolution uses new versions and files. A later actual attestation must remain
distinguishable from the public conformance chain and from the `notRun` record.

## Alternatives considered

### Commit encrypted or access-controlled fixture bytes to GitHub

Rejected. Repository history, workflows, administrators, keys, and metadata would weaken the
claimed separation and create avoidable privacy/leakage risk.

### Run the private bundle in ordinary GitHub Actions

Rejected. It would expose data and credentials to a public-repository workflow and invite
self-writing result commits. Public CI validates only schemas, current non-operational state, and
fictional conformance data.

### Publish per-case digests and results without content

Rejected. Membership/digest correlation and small-cell counts can still reveal labels or enable
tuning. The sanitized record uses opaque cohorts, aggregate suppression, and a private result
digest.

### Let SightLint certify reviewer identity and access separation

Rejected. A local structural verifier has no independent identity or storage authority. It must
state that limitation rather than turn declarations into facts.

### Use floating-point percentages or one UX score

Rejected. Percentages hide zero denominators and rounding; a universal score collapses unrelated
coverage, precision, abstention, and maturity dimensions.

## Non-goals

- an actual independent review, protected bundle, evaluator, run, or operational claim;
- a new Web rule, rule promotion, or blocking-policy change;
- a browser-capture, interaction, perception, or medium-adapter change;
- cryptographic identity proof or a signing/PKI dependency;
- representative accuracy, WCAG conformance, or universal UI/UX scoring;
- private-data upload, remote service, database, credential store, or self-writing workflow.

## Verification

- JSON Schemas accept every current/conformance document and reject unknown or mixed-authority
  fields and invalid lifecycle combinations.
- The read-only verifier enforces file/count/string/argument/environment bounds, canonical digest
  projections, stable ordering, relative paths, cross-document bindings, current admission state,
  exposure/correction invariants, metric arithmetic, disclosure suppression, and prohibited public
  detail.
- Process E2E repeats valid current and conformance input byte-for-byte and checks stable stdout,
  stderr, and exit code for digest, binding, path, lifecycle, metric, disclosure, and one-over
  failures.
- Public conformance output always says it is ineligible product evidence; current output remains
  `notRun` and `notOperational`.
- Existing Web, image, interaction, perception, medium-adapter, release, MSRV, and cross-platform
  gates remain green.
