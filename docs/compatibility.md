# Alpha compatibility policy

SightLint versions each compatibility surface independently. The package version identifies one
tested bundle; it does not replace schema, protocol, extension, rule, or evaluation versions.

`v0.1.0-alpha.1` is an immutable, unpublished workflow attempt and has no supported compatibility
surface. ADR 0038 makes alpha.2 the first published bundle rather than moving the old tag.

## `v0.1.0-alpha.2` surface inventory

| Surface | Current version or contract | Alpha compatibility rule |
|---|---|---|
| Rust workspace / `sightlint` binary | `0.1.0-alpha.2` | SemVer prerelease; breaking changes require release notes and a package-version change. |
| Artifact IR | `0.1.0` | Exact recognized core version; unknown namespaced extensions are preserved, while malformed recognized data is rejected. |
| CheckReport | `0.3.0` | Consumers must inspect `reportSchemaVersion`; incompatible changes require a new report version. |
| Visual extension | `org.sightlint.visual@0.1.0` | Recognized unsupported versions or malformed payloads are errors. |
| Web extension | `org.sightlint.web@0.3.0` | Recognized unsupported versions or malformed payloads are errors; retained `0.1.0`/`0.2.0` schemas are compatibility fixtures only. |
| Playwright capture protocol | `0.1.0` | Strict request/response fields; unknown fields and versions are rejected. |
| Playwright adapter implementation | `0.3.0` | Implementation provenance, not a substitute for protocol/extension versions. |
| Private Node package | `0.4.0` | Not published to npm; version covers the source package and command bundle only. |
| Web workflow report | `0.1.0` | Strict canonical envelope around capture and CheckReport; incompatible fields require a new version. |
| Base/recommended profiles | `sightlint:base@0.1.0`, `sightlint:recommended@0.1.0` | Profile identity and policy source are report data. Rule admission or precedence changes require release notes and an appropriate profile/rule decision. |
| Executable rules | stable ID plus per-rule version, currently `0.1.0` | Material applicability or deterministic-obligation changes require a rule-version change. |
| Synthetic evaluation corpus | `0.1.0` | Human-authored oracle changes require semantic rationale; generator output cannot rewrite the oracle. |
| Web evaluation corpus | `0.1.0` | Acquisition, rule, and agent-workflow annotations remain separate and independently versioned. |
| Browser acquisition oracle | `0.3.0` | Strict current schema; prior `0.1.0`/`0.2.0` schemas are retained for rejection/compatibility tests. |
| Browser rule oracle | `0.2.0` | Strict current schema; `0.1.0` remains a compatibility fixture. |
| Agent workflow oracle | `0.1.0` | Public smoke regression only, not a holdout or generalization estimate. |
| Perception-worker protocol | not part of alpha.2 | Added after the published tag; see the unreleased surface table. |

## Unreleased current-`main` additions after alpha.2

The source tree after `v0.1.0-alpha.2` adds evaluation-only image segmentation, exact PNG
source-alpha geometry, the PNG format-demand decision, the local perception protocol foundation,
and the first bounded PPTX, PDF, and Android source/capture-adapter slices. They are not part of the
published alpha.2 archive and therefore do not retroactively change that release.

| Surface | Version or contract | Compatibility rule |
|---|---|---|
| Segmentation benchmark report | `0.1.0` | Strict canonical JSON; incompatible fields or semantics require a new report version. It is not a CheckReport. |
| Segmentation evaluation corpus | `0.1.0` | Human-authored acquisition and rule oracles remain separate; implementation output cannot rewrite either oracle. |
| `benchmark-image-segmentation` | evaluation-only command | Exit `0` returns a complete report, including explicit unavailability; usage, I/O, validation, or decoding errors exit `2`; it never exits `1`. |
| PNG extension | `org.sightlint.adapter.png@0.2.0` | Adds `alphaGeometry`; consumers must inspect the enclosing version. The published alpha.2 emitted `0.1.0`. |
| PNG alpha geometry | `0.1.0` | Exact encoded source-alpha predicates and device-pixel bounds; incompatible fields or semantics require a new nested version. |
| Source-alpha evaluation corpus | `0.1.0` | Acquisition and rule annotations remain separate; all labels are public development data and no holdout is claimed. |
| PNG format-demand assessment | `0.1.0` | Scope-admission evidence only. It adds no decoder, command, extension, report, or exit-code behavior; incompatible assessment fields require a new schema version. |
| Perception request/response protocol | `0.1.0` | Strict local process fields and typed region/text/role/hierarchy/peer records; incompatible fields or semantics require a new version. |
| Perception run report | `0.1.0` | Canonical nonblocking acquisition envelope; rule outcome is always `untested`, and the command never exits `1`. |
| Perception extension | `org.sightlint.perception@0.1.0` | Digest/identity/status/mapping summary; inferred semantics are not promoted into core IR. |
| Private perception Node package | `0.1.0` | Not published to npm; requires Node `>=20 <25` and has no package dependencies. |
| Perception evaluation corpus | `0.1.0` | Public three-state development data with separate acquisition/rule annotations and no protected holdout. |
| PPTX request/response protocol | `0.1.0` | Strict local process fields, digest-pinned source/renders, resource bounds, and partial coverage; incompatible fields or semantics require a new version. |
| PPTX extension | `org.sightlint.pptx@0.1.0` | Native IDs, hierarchy, z-order, digest-only text metadata, exact source geometry status, unsupported features, and render extent reconciliation remain versioned outside core IR. |
| PPTX adapter implementation | `0.1.0` | Unreleased Python 3.9+ source-tree process using transitional OOXML; runtime version is provenance, not a protocol version. |
| PPTX evaluation corpus and annotations | `0.1.0` | Public repository-owned regression data with separate acquisition/rule truth and no protected holdout; implementation output cannot rewrite an oracle. |
| PPTX metric contract | `0.1.0` | Defines public-corpus coverage/precision/abstention/false-positive/mutation metrics without storing observed implementation output as truth. |
| PDF request/response protocol | `0.1.0` | Strict local digest-pinned source/render fields, resource bounds, and explicitly partial coverage; incompatible fields or semantics require a new version. |
| PDF extension | `org.sightlint.pdf@0.1.0` | Page/object identity, original PDF rectangles, exact geometry status, action class, tag-presence status, unsupported features, and extent reconciliation remain outside core IR. |
| PDF adapter implementation | `0.1.0` | Unreleased Python 3.9+ source-tree process requiring exactly pypdf 6.17.0; parser/runtime identity is provenance, not a protocol version. |
| PDF evaluation corpus and annotations | `0.1.0` | Public repository-owned regression data with separate acquisition/rule truth and no protected holdout; implementation output cannot rewrite an oracle. |
| PDF metric contract | `0.1.0` | Defines separate public-corpus coverage/precision/abstention/false-positive/mutation expectations without storing observed implementation output as truth. |
| Android request/response and capture protocol | `0.1.0` | Strict local digest-pinned capture/PNG fields, bounded resources, and explicitly partial coverage; incompatible fields or semantics require a new version. |
| Android extension | `org.sightlint.android@0.1.0` | View hierarchy/allocation, platform accessibility facts, capture/device/build provenance, coverage, privacy, unsupported features, and screenshot extent reconciliation remain versioned outside core IR. |
| Android adapter implementation | `0.1.0` | Unreleased dependency-free Python 3.9+ source-tree file process; runtime version is provenance, and device orchestration is not part of this surface. |
| Android evaluation corpus and annotations | `0.1.0` | Public repository-owned regression data with separate acquisition/rule truth, explicit public splits, and no protected holdout; implementation output cannot rewrite an oracle. |
| Android metric contract | `0.1.0` | Defines public-corpus acquisition coverage, verdict precision, abstention, false-positive, and mutation expectations without storing observed implementation output as truth. |

The three candidate policies are versioned inside the report. None is a supported semantic UI
segmentation guarantee or a replacement for `inspect-image`.

ADR 0041 retains the current PNG subset and reason codes. Caller-selected conversion is evidence
about the converted bytes only; no conversion protocol or compatibility guarantee is introduced.

## CLI and process behavior

The `0.1` alpha line reserves these exit meanings:

- `0`: no blocking failure under the selected policy;
- `1`: a blocking failure or explicitly denied `cantTell`;
- `2`: usage, I/O, decoding, validation, adapter, or execution error.

Human text is intended for people and may improve between alpha releases. Canonical JSON is the
machine surface: consumers must check its own schema/version fields and must not infer semantics
from package version alone. Ordering and repeated output are byte-stable only within the declared
normalized inputs and compatibility environment documented by the applicable adapter/report.

`sightlint-web`, `sightlint-web-check`, and `sightlint-perception` require Node `>=20 <25`.
`sightlint-pptx` and `sightlint-android` require Python 3.9+. `sightlint-pdf` requires Python 3.9+
and exactly pypdf 6.17.0. All report the exact runtime patch version; repeated canonical bytes are
guaranteed for the same declared input and compatibility environment, not across different
Python/runtime versions. The evaluated browser path uses
the lockfile's Playwright/Chromium build on Linux. The private Node package can build on the hosted
macOS arm64 and Windows x64 runners, but cross-platform screenshot byte identity and browser E2E
support are not claimed.

## Change and migration policy

Before changing a public surface:

1. classify it as compatible, behavioral, or breaking for that surface;
2. update the surface's own version when required;
3. retain old valid/malformed fixtures when they are needed to prove migration behavior;
4. update release notes with the old and new versions plus required user action;
5. run public binary/process E2E on the exact release candidate and merged `main`;
6. never alter an oracle merely to make a new implementation pass.

No pre-1.0 surface is promised stable indefinitely. The explicit versions and migration notes are
the promise: incompatible behavior will be visible instead of silently hidden in the package
version.
