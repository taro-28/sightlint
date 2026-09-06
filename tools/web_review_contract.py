#!/usr/bin/env python3
"""Shared deterministic contracts for public Web review operations."""

from __future__ import annotations

import hashlib
import json
import math
import re
from datetime import date
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "evaluation" / "web"
PACKET_PATH = WEB / "review-packet.json"
BLANK_SUBMISSION_PATH = WEB / "reviewer-submission.blank.json"
PACKET_SCHEMA = "./review-packet.schema.json"
SUBMISSION_SCHEMA = "./reviewer-submission.schema.json"
COMPARISON_SCHEMA = "./review-comparison.schema.json"
VERSION = "1.0.0"

MAX_INPUT_BYTES = 1_048_576
MAX_PACKET_BYTES = 8_388_608
MAX_OUTPUT_BYTES = 8_388_608
MAX_FILES = 33
MAX_CASES = 27
MAX_JUDGMENTS = 512
MAX_STRING_BYTES = 4_096
MAX_IDENTIFIER_BYTES = 128
MAX_PATH_BYTES = 256

SHA256 = re.compile(r"sha256:[0-9a-f]{64}\Z")
IDENTIFIER = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}\Z")
ASPECT = re.compile(r"[A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)*\Z")
DATE = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}\Z")
URL = re.compile(r"(?:https?|file)://", re.IGNORECASE)
ABSOLUTE_PATH = re.compile(r"(?:^|\s)(?:/[A-Za-z0-9_.-]+/|[A-Za-z]:[\\/])")
CREDENTIAL = re.compile(
    r"(?:ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|"
    r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|Bearer\s+[A-Za-z0-9._~+/-]{16,})",
    re.IGNORECASE,
)

SOURCE_FILES = (
    "evaluation/web/fixture-app/app.js",
    "evaluation/web/fixture-app/index.html",
    "evaluation/web/fixture-app/styles.css",
    "evaluation/web/support-inbox-app/app.js",
    "evaluation/web/support-inbox-app/index.html",
    "evaluation/web/support-inbox-app/styles.css",
)

ATLAS_CASES = (
    "dashboard-browser-ambiguous",
    "dashboard-browser-ambiguous-control",
    "dashboard-browser-clean",
    "dashboard-browser-clipping",
    "dashboard-browser-control-boundaries",
    "dashboard-browser-control-clip",
    "dashboard-browser-control-offscreen",
    "dashboard-browser-intentional-grouping",
    "dashboard-browser-intentional-overlay",
    "dashboard-browser-mobile",
    "dashboard-browser-notification-clean",
    "dashboard-browser-occlusion",
    "dashboard-browser-occlusion-clean",
    "dashboard-browser-out-of-viewport",
    "dashboard-browser-overflow",
    "dashboard-browser-peer-dimension",
    "dashboard-browser-responsive-mobile-mutant",
    "dashboard-browser-rtl-vertical",
    "dashboard-browser-scrollable-control",
    "dashboard-browser-spacing-mutant",
    "dashboard-browser-text-scale",
    "dashboard-browser-transformed-text",
    "dashboard-browser-unnamed-control",
)

HARBOR_CASES = (
    "support-inbox-ambiguous-control",
    "support-inbox-clean",
    "support-inbox-labelledby-hard-negative",
    "support-inbox-unnamed-control",
)

FAMILIES = (
    {
        "familyId": "atlas-dashboard-settings-v1",
        "name": "Atlas dashboard and settings",
        "sourcePaths": list(SOURCE_FILES[:3]),
        "caseIds": list(ATLAS_CASES),
    },
    {
        "familyId": "harbor-support-inbox-v1",
        "name": "Harbor support inbox",
        "sourcePaths": list(SOURCE_FILES[3:]),
        "caseIds": list(HARBOR_CASES),
    },
)

CASE_TO_FAMILY = {
    **{case_id: "atlas-dashboard-settings-v1" for case_id in ATLAS_CASES},
    **{case_id: "harbor-support-inbox-v1" for case_id in HARBOR_CASES},
}

REQUEST_FILES = tuple(
    f"evaluation/web/requests/{case_id}.json" for case_id in sorted(CASE_TO_FAMILY)
)
ALLOWED_FILES = tuple(sorted((*SOURCE_FILES, *REQUEST_FILES)))


class ContractError(Exception):
    """Stable categorized error for process E2E."""

    def __init__(self, category: str, message: str) -> None:
        super().__init__(message)
        self.category = category


def fail(category: str, message: str) -> NoReturn:
    """Raise a stable contract error."""
    raise ContractError(category, message)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Reject duplicate JSON keys before they can be overwritten."""
    value: dict[str, Any] = {}
    for key, child in pairs:
        if key in value:
            fail("json", "a document contains a duplicate object key")
        value[key] = child
    return value


def reject_constant(value: str) -> NoReturn:
    """Reject NaN and infinities accepted by Python's permissive decoder."""
    fail("scalar-domain", f"a document contains unsupported numeric value {value}")


def load_json(path: Path, label: str, maximum: int = MAX_INPUT_BYTES) -> dict[str, Any]:
    """Load one bounded strict UTF-8 JSON object."""
    try:
        size = path.stat().st_size
    except OSError:
        fail("input", f"{label} is unavailable")
    if size > maximum:
        fail("input-budget", f"{label} exceeds the {maximum}-byte limit")
    try:
        raw = path.read_bytes()
    except OSError:
        fail("input", f"{label} is unavailable")
    if raw.startswith(b"\xef\xbb\xbf"):
        fail("json", f"{label} must not contain a UTF-8 byte-order mark")
    try:
        value = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except UnicodeDecodeError:
        fail("json", f"{label} is not UTF-8")
    except json.JSONDecodeError:
        fail("json", f"{label} is not valid JSON")
    if not isinstance(value, dict):
        fail("shape", f"{label} must contain a JSON object")
    validate_scalar_domain(value, label)
    return value


def validate_scalar_domain(value: Any, label: str, key: str | None = None) -> None:
    """Bound strings and reject non-finite numbers throughout a document."""
    if isinstance(value, dict):
        for child_key, child in value.items():
            bounded_text(child_key, f"{label} object key")
            validate_scalar_domain(child, label, child_key)
    elif isinstance(value, list):
        for child in value:
            validate_scalar_domain(child, label, key)
    elif isinstance(value, str):
        maximum = MAX_INPUT_BYTES if key == "contentUtf8" else MAX_STRING_BYTES
        bounded_text(value, label, maximum)
    elif isinstance(value, float) and not math.isfinite(value):
        fail("scalar-domain", f"{label} contains a non-finite number")
    elif value is not None and not isinstance(value, (bool, int, float)):
        fail("scalar-domain", f"{label} contains an unsupported JSON value")


def canonical_bytes(value: dict[str, Any], omit: str | None = None) -> bytes:
    """Return the ADR 0053 canonical JSON projection."""
    projected = dict(value)
    if omit is not None:
        projected.pop(omit, None)
    try:
        return json.dumps(
            projected,
            ensure_ascii=True,
            allow_nan=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
    except (TypeError, ValueError):
        fail("scalar-domain", "a document cannot be represented as canonical JSON")


def pretty_bytes(value: dict[str, Any]) -> bytes:
    """Return stable review-friendly generated JSON bytes."""
    return (
        json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2, sort_keys=False) + "\n"
    ).encode("utf-8")


def digest_bytes(raw: bytes) -> str:
    """Return a tagged lowercase SHA-256 digest."""
    return f"sha256:{hashlib.sha256(raw).hexdigest()}"


def digest(value: dict[str, Any], omit: str | None = None) -> str:
    """Digest one canonical JSON projection."""
    return digest_bytes(canonical_bytes(value, omit))


def obj(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("shape", f"{label} must be an object")
    return value


def array(
    value: Any,
    label: str,
    minimum: int = 0,
    maximum: int | None = None,
) -> list[Any]:
    if not isinstance(value, list):
        fail("shape", f"{label} must be an array")
    if len(value) < minimum or (maximum is not None and len(value) > maximum):
        fail("limit", f"{label} has an invalid item count")
    return value


def exact(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    result = obj(value, label)
    unknown = sorted(set(result) - fields)
    missing = sorted(fields - set(result))
    if unknown:
        fail("shape", f"{label} contains unsupported fields: {', '.join(unknown)}")
    if missing:
        fail("shape", f"{label} is missing required fields: {', '.join(missing)}")
    return result


def text(value: Any, label: str, maximum: int = MAX_STRING_BYTES) -> str:
    if not isinstance(value, str) or not value:
        fail("shape", f"{label} must be a non-empty string")
    return bounded_text(value, label, maximum)


def bounded_text(value: str, label: str, maximum: int = MAX_STRING_BYTES) -> str:
    if len(value.encode("utf-8")) > maximum:
        fail("string-budget", f"{label} exceeds the {maximum}-byte string limit")
    return value


def identifier(value: Any, label: str) -> str:
    result = text(value, label, MAX_IDENTIFIER_BYTES)
    if IDENTIFIER.fullmatch(result) is None:
        fail("shape", f"{label} must be a stable lowercase identifier")
    return result


def enum(value: Any, allowed: set[str], label: str) -> str:
    result = text(value, label)
    if result not in allowed:
        fail("shape", f"{label} has unsupported value {result!r}")
    return result


def boolean(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        fail("shape", f"{label} must be a boolean")
    return value


def nullable_text(value: Any, label: str) -> str | None:
    if value is None:
        return None
    return text(value, label)


def sha256(value: Any, label: str) -> str:
    result = text(value, label)
    if SHA256.fullmatch(result) is None:
        fail("digest", f"{label} must be a lowercase SHA-256 digest")
    return result


def relative_path(value: Any, label: str) -> str:
    result = text(value, label, MAX_PATH_BYTES)
    path = PurePosixPath(result)
    if (
        path.is_absolute()
        or "\\" in result
        or ":" in result
        or "//" in result
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
        fail("path", f"{label} must be a contained relative POSIX path")
    return result


def sorted_unique_strings(
    value: Any,
    label: str,
    maximum: int,
    validator: Any = text,
) -> list[str]:
    values = array(value, label, 0, maximum)
    result = [validator(item, f"{label} entry") for item in values]
    if result != sorted(result) or len(result) != len(set(result)):
        fail("ordering", f"{label} must be unique and sorted")
    return result


def nested_value(value: Any, aspect: str) -> Any:
    """Resolve a dotted comparison aspect without array-index syntax."""
    current = value
    for part in aspect.split("."):
        if not isinstance(current, dict) or part not in current:
            raise KeyError(aspect)
        current = current[part]
    return current


def repository_file(path_text: str) -> Path:
    """Resolve an allowlisted regular non-symlink repository file."""
    relative_path(path_text, "packet file path")
    path = ROOT.joinpath(*PurePosixPath(path_text).parts)
    try:
        if path.is_symlink() or not path.is_file():
            fail("path", f"allowlisted input is not a regular file: {path_text}")
        resolved = path.resolve()
    except OSError:
        fail("path", f"allowlisted input is unavailable: {path_text}")
    if not resolved.is_relative_to(ROOT.resolve()):
        fail("path", f"allowlisted input escapes the repository: {path_text}")
    return path


def _embedded_file(path_text: str) -> dict[str, Any]:
    path = repository_file(path_text)
    raw = path.read_bytes()
    if len(raw) > MAX_INPUT_BYTES:
        fail("input-budget", f"allowlisted input exceeds the {MAX_INPUT_BYTES}-byte limit")
    try:
        content = raw.decode("utf-8")
    except UnicodeDecodeError:
        fail("input", f"allowlisted input is not UTF-8: {path_text}")
    return {
        "path": path_text,
        "kind": "captureRequest" if path_text in REQUEST_FILES else "fixtureSource",
        "byteLength": len(raw),
        "sha256": digest_bytes(raw),
        "contentUtf8": content,
    }


def _request_document(entry: dict[str, Any], label: str) -> dict[str, Any]:
    try:
        value = json.loads(
            text(entry.get("contentUtf8"), f"{label} content", MAX_INPUT_BYTES),
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except json.JSONDecodeError:
        fail("request", f"{label} is not valid capture-request JSON")
    request = exact(
        value,
        {
            "$schema",
            "protocolVersion",
            "artifact",
            "fixture",
            "environment",
            "privacy",
            "network",
            "screenshot",
        },
        label,
    )
    if (
        request["$schema"] != "../../../adapters/playwright/schemas/capture-request.schema.json"
        or request["protocolVersion"] != "0.1.0"
    ):
        fail("request", f"{label} uses an unsupported capture protocol")
    privacy = obj(request["privacy"], f"{label} privacy")
    network = obj(request["network"], f"{label} network")
    if privacy.get("externalProcessing") is not False or network != {"mode": "deny"}:
        fail("privacy", f"{label} must remain local and network-denied")
    fixture = obj(request["fixture"], f"{label} fixture")
    entrypoint = fixture.get("entrypoint")
    if entrypoint not in {
        "evaluation/web/fixture-app/index.html",
        "evaluation/web/support-inbox-app/index.html",
    }:
        fail("request", f"{label} references an unapproved fixture entrypoint")
    return request


def build_packet() -> dict[str, Any]:
    """Build the deterministic Atlas/Harbor source-only packet."""
    files = [_embedded_file(path) for path in ALLOWED_FILES]
    entries = {entry["path"]: entry for entry in files}
    cases: list[dict[str, Any]] = []
    for case_id in sorted(CASE_TO_FAMILY):
        path = f"evaluation/web/requests/{case_id}.json"
        request = _request_document(entries[path], f"request {case_id}")
        cases.append(
            {
                "caseId": case_id,
                "familyId": CASE_TO_FAMILY[case_id],
                "requestPath": path,
                "requestDigest": entries[path]["sha256"],
                "fixtureState": text(
                    obj(request["fixture"], f"request {case_id} fixture").get("state"),
                    f"request {case_id} state",
                ),
            }
        )
    packet: dict[str, Any] = {
        "$schema": PACKET_SCHEMA,
        "schemaVersion": VERSION,
        "documentType": "webReviewPacket",
        "packetId": "sightlint-atlas-harbor-source-review-v1",
        "packetDigest": None,
        "recordPurpose": "publicSourceOnlyReviewInput",
        "evidenceEligible": False,
        "protocol": {
            "sourceFirstNotBlind": True,
            "oracleExposure": "forbiddenUntilFinalizedSubmission",
            "allowedInputKinds": ["captureRequest", "fixtureSource"],
            "prohibitedInputKinds": [
                "capturedArtifactIr",
                "diagnostic",
                "existingAcquisitionOracle",
                "existingRuleOracle",
                "expectedVerdict",
                "generatedScreenshot",
                "implementationOutput",
                "sightlintReport",
            ],
            "acquisitionAndRuleAuthoritySeparated": True,
            "implementationOutputMaySupplyJudgment": False,
            "unavailableMeasurementMayBeGuessed": False,
        },
        "governance": {
            "ownership": "sightlintRepository",
            "license": "MIT OR Apache-2.0",
            "redistribution": "permittedUnderRepositoryLicense",
            "privacyReview": "syntheticNoPersonalData",
            "containsPersonalOrCustomerData": False,
            "containsCredentials": False,
            "externalAssets": False,
            "externalNetwork": False,
            "externalProcessing": False,
            "exposure": "publicTuningVisible",
        },
        "reviewFields": {
            "acquisition": [
                "availability and exact reviewed value",
                "stable native subject and aspect",
                "viewport, coordinate space, units, and tolerance basis",
                "native evidence and pixel evidence separately",
                "native/pixel agreement, conflict, or absence",
                "unavailable evidence, confidence, and rationale",
            ],
            "rule": [
                "rule ID, version, target, and applicability",
                "required-evidence sufficiency",
                "passed, failed, cantTell, inapplicable, or untested outcome",
                "policy basis and valid alternative or hard-negative rationale",
                "false-positive risk, false-negative risk, confidence, and rationale",
            ],
        },
        "families": [dict(family) for family in FAMILIES],
        "files": files,
        "cases": cases,
        "nonClaims": [
            "Fixture state names and source are visible, so this is source-first review rather than blind evaluation.",
            "This packet contains no review answer and is not independent-review evidence by itself.",
            "Public Atlas and Harbor data is tuning-visible and is not a protected holdout.",
            "The workflow does not establish reviewer identity, qualification, independence, WCAG conformance, representative accuracy, or blocking maturity.",
        ],
    }
    packet["packetDigest"] = digest(packet, "packetDigest")
    if len(pretty_bytes(packet)) > MAX_PACKET_BYTES:
        fail("output-budget", f"review packet exceeds the {MAX_PACKET_BYTES}-byte limit")
    return packet


def _validate_file_entry(entry: Any, expected_path: str) -> dict[str, Any]:
    label = f"packet file {expected_path}"
    value = exact(entry, {"path", "kind", "byteLength", "sha256", "contentUtf8"}, label)
    path_text = relative_path(value["path"], f"{label} path")
    if path_text != expected_path:
        fail("ordering", "packet files must match the exact sorted source-only allowlist")
    expected_kind = "captureRequest" if path_text in REQUEST_FILES else "fixtureSource"
    if value["kind"] != expected_kind:
        fail("leakage", f"{label} has an unsupported input kind")
    content = value["contentUtf8"]
    if not isinstance(content, str):
        fail("shape", f"{label} contentUtf8 must be a string")
    raw = content.encode("utf-8")
    if len(raw) > MAX_INPUT_BYTES:
        fail("input-budget", f"{label} exceeds the {MAX_INPUT_BYTES}-byte file limit")
    if value["byteLength"] != len(raw) or value["sha256"] != digest_bytes(raw):
        fail("digest", f"{label} content does not match its byteLength and sha256")
    expected = repository_file(path_text).read_bytes()
    if raw != expected:
        fail("leakage", f"{label} does not match the current repository-owned input")
    if expected_kind == "captureRequest":
        _request_document(value, label)
    return value


def validate_packet(packet: dict[str, Any]) -> None:
    """Validate exact source-only packet content against the current repository."""
    exact(
        packet,
        {
            "$schema",
            "schemaVersion",
            "documentType",
            "packetId",
            "packetDigest",
            "recordPurpose",
            "evidenceEligible",
            "protocol",
            "governance",
            "reviewFields",
            "families",
            "files",
            "cases",
            "nonClaims",
        },
        "review packet",
    )
    if (
        packet["$schema"] != PACKET_SCHEMA
        or packet["schemaVersion"] != VERSION
        or packet["documentType"] != "webReviewPacket"
        or packet["packetId"] != "sightlint-atlas-harbor-source-review-v1"
        or packet["recordPurpose"] != "publicSourceOnlyReviewInput"
        or packet["evidenceEligible"] is not False
    ):
        fail("version", "review packet uses an unsupported identity or evidence state")
    recorded = sha256(packet["packetDigest"], "review packet packetDigest")
    if recorded != digest(packet, "packetDigest"):
        fail("digest", "review packet packetDigest does not match its canonical projection")

    protocol = exact(
        packet["protocol"],
        {
            "sourceFirstNotBlind",
            "oracleExposure",
            "allowedInputKinds",
            "prohibitedInputKinds",
            "acquisitionAndRuleAuthoritySeparated",
            "implementationOutputMaySupplyJudgment",
            "unavailableMeasurementMayBeGuessed",
        },
        "review packet protocol",
    )
    if protocol != build_packet()["protocol"]:
        fail("leakage", "review packet weakens the source-only review protocol")
    if packet["governance"] != build_packet()["governance"]:
        fail("privacy", "review packet governance does not match public fictional fixtures")
    if packet["reviewFields"] != build_packet()["reviewFields"]:
        fail("authority", "review packet changes the separate review field contract")
    if packet["families"] != build_packet()["families"]:
        fail("inventory", "review packet family inventory does not match Atlas and Harbor")
    if packet["nonClaims"] != build_packet()["nonClaims"]:
        fail("claims", "review packet changes required non-claims")

    files = array(packet["files"], "review packet files", len(ALLOWED_FILES), MAX_FILES)
    if len(files) != len(ALLOWED_FILES):
        fail("leakage", "review packet must contain only the exact source-only allowlist")
    indexed: dict[str, dict[str, Any]] = {}
    for entry, expected_path in zip(files, ALLOWED_FILES, strict=True):
        value = _validate_file_entry(entry, expected_path)
        if expected_path in indexed:
            fail("ordering", "review packet repeats a file path")
        indexed[expected_path] = value

    cases = array(packet["cases"], "review packet cases", len(CASE_TO_FAMILY), MAX_CASES)
    case_ids: list[str] = []
    for case in cases:
        item = exact(
            case,
            {"caseId", "familyId", "requestPath", "requestDigest", "fixtureState"},
            "review packet case",
        )
        case_id = identifier(item["caseId"], "review packet caseId")
        case_ids.append(case_id)
        expected_path = f"evaluation/web/requests/{case_id}.json"
        if (
            case_id not in CASE_TO_FAMILY
            or item["familyId"] != CASE_TO_FAMILY[case_id]
            or item["requestPath"] != expected_path
            or item["requestDigest"] != indexed[expected_path]["sha256"]
        ):
            fail("inventory", f"review packet case {case_id!r} has an invalid binding")
        request = _request_document(indexed[expected_path], f"request {case_id}")
        if item["fixtureState"] != obj(request["fixture"], "request fixture").get("state"):
            fail("binding", f"review packet case {case_id!r} has the wrong fixture state")
    if case_ids != sorted(CASE_TO_FAMILY) or len(case_ids) != len(set(case_ids)):
        fail("ordering", "review packet cases must be unique and sorted")


def build_blank_submission(packet: dict[str, Any]) -> dict[str, Any]:
    """Build a no-answer draft template bound to one packet."""
    cases = [
        {
            "caseId": case["caseId"],
            "caseContext": {
                "reviewedAs": "other",
                "rationale": "Replace with a source-based classification rationale.",
            },
            "acquisitionJudgments": [],
            "ruleJudgments": [],
        }
        for case in packet["cases"]
    ]
    return {
        "$schema": SUBMISSION_SCHEMA,
        "schemaVersion": VERSION,
        "documentType": "webReviewerSubmission",
        "submissionId": "replace-with-stable-submission-id",
        "submissionDigest": None,
        "lifecycle": "draft",
        "recordPurpose": "humanReviewCandidate",
        "evidenceStatus": "requiresGovernanceReview",
        "packetBinding": {
            "packetId": packet["packetId"],
            "packetDigest": packet["packetDigest"],
        },
        "reviewScope": {
            "familyIds": sorted(family["familyId"] for family in FAMILIES),
            "caseIds": sorted(CASE_TO_FAMILY),
            "completeForDeclaredScope": False,
        },
        "reviewer": {
            "stableProjectId": "replace-with-stable-project-id",
            "qualification": {
                "category": "other",
                "rationale": "Replace with relevant Web UI, accessibility, or product-review qualification.",
            },
            "independentFromAnnotationAuthors": "undeclared",
            "independenceRationale": "Replace with the factual relationship to annotation authors.",
            "priorExpectedLabelExposure": {
                "status": "undeclared",
                "caseIds": [],
                "rationale": "Replace with known prior exposure; do not conceal seen labels.",
            },
            "conflictOfInterest": {
                "status": "undeclared",
                "rationale": "Replace with known conflicts or a factual none declaration.",
            },
            "reviewedOn": "0000-00-00",
        },
        "declarations": {
            "sourceFirstNotBlind": True,
            "sightlintOutputUsedBeforeFinalization": False,
            "existingOracleViewedBeforeFinalization": False,
            "generatedCaptureOrReportUsedAsAnswer": False,
            "implementationOutputUsedAsAnswer": False,
            "containsProtectedOrPrivateData": False,
            "containsCredentials": False,
            "externalProcessingUsed": False,
            "identityOrQualificationVerifiedBySightLint": False,
            "signatureVerifiedBySightLint": False,
        },
        "cases": cases,
        "submissionLimitations": [
            "This draft contains no human judgment until a reviewer replaces the placeholders and fills the declared scope.",
            "Tooling validates structure and declarations but does not prove identity, qualification, independence, conflicts, or signature validity.",
            "A humanReviewCandidate requires project governance review before it can support issue 77.",
            "This public review cannot establish protected-holdout performance or representative UI and UX accuracy.",
        ],
    }


def _validate_privacy_text(value: Any, label: str) -> None:
    if isinstance(value, dict):
        for child in value.values():
            _validate_privacy_text(child, label)
    elif isinstance(value, list):
        for child in value:
            _validate_privacy_text(child, label)
    elif isinstance(value, str):
        if URL.search(value):
            fail("privacy", f"{label} must not contain URLs")
        if ABSOLUTE_PATH.search(value):
            fail("privacy", f"{label} must not contain absolute private paths")
        if CREDENTIAL.search(value):
            fail("privacy", f"{label} must not contain credential-like material")


def _evidence(value: Any, label: str) -> None:
    evidence = exact(value, {"status", "rationale"}, label)
    enum(evidence["status"], {"available", "notApplicable", "unavailable", "untested"}, f"{label} status")
    text(evidence["rationale"], f"{label} rationale")


def _acquisition_judgment(value: Any, label: str) -> str:
    judgment = exact(
        value,
        {
            "judgmentId",
            "subject",
            "aspect",
            "status",
            "value",
            "unitOrCoordinateSpace",
            "confidence",
            "rationale",
            "nativeEvidence",
            "pixelEvidence",
            "nativePixelRelationship",
            "unavailableEvidence",
        },
        label,
    )
    judgment_id = identifier(judgment["judgmentId"], f"{label} judgmentId")
    subject = exact(judgment["subject"], {"kind", "id"}, f"{label} subject")
    kind = enum(subject["kind"], {"abstention", "case", "node"}, f"{label} subject kind")
    if kind == "case":
        if subject["id"] is not None:
            fail("shape", f"{label} case subject id must be null")
    elif kind == "node":
        identifier(subject["id"], f"{label} subject id")
    else:
        abstention_id = text(subject["id"], f"{label} subject id", MAX_IDENTIFIER_BYTES)
        if ASPECT.fullmatch(abstention_id) is None:
            fail("shape", f"{label} abstention subject id must be a stable aspect name")
    aspect = text(judgment["aspect"], f"{label} aspect", MAX_IDENTIFIER_BYTES)
    if ASPECT.fullmatch(aspect) is None:
        fail("shape", f"{label} aspect must be a dotted stable field name")
    status = enum(judgment["status"], {"cantTell", "observed", "untested"}, f"{label} status")
    if status != "observed" and judgment["value"] is not None:
        fail("authority", f"{label} unavailable observation must have null value")
    nullable_text(judgment["unitOrCoordinateSpace"], f"{label} unitOrCoordinateSpace")
    enum(judgment["confidence"], {"high", "low", "medium"}, f"{label} confidence")
    text(judgment["rationale"], f"{label} rationale")
    _evidence(judgment["nativeEvidence"], f"{label} nativeEvidence")
    _evidence(judgment["pixelEvidence"], f"{label} pixelEvidence")
    enum(
        judgment["nativePixelRelationship"],
        {"agreement", "conflict", "notCompared", "untested"},
        f"{label} nativePixelRelationship",
    )
    unavailable = array(judgment["unavailableEvidence"], f"{label} unavailableEvidence", 0, 32)
    for item in unavailable:
        text(item, f"{label} unavailableEvidence entry")
    return judgment_id


def _rule_judgment(value: Any, label: str) -> str:
    judgment = exact(
        value,
        {
            "judgmentId",
            "ruleId",
            "ruleVersion",
            "targetKind",
            "targetId",
            "targetAspect",
            "applicability",
            "requiredEvidence",
            "outcome",
            "policyBasis",
            "validAlternativeOrHardNegative",
            "falsePositiveRisk",
            "falseNegativeRisk",
            "confidence",
            "rationale",
        },
        label,
    )
    judgment_id = identifier(judgment["judgmentId"], f"{label} judgmentId")
    identifier(judgment["ruleId"], f"{label} ruleId")
    if re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", text(judgment["ruleVersion"], f"{label} ruleVersion")) is None:
        fail("shape", f"{label} ruleVersion must be semantic version")
    enum(judgment["targetKind"], {"artifact", "node", "relation"}, f"{label} targetKind")
    identifier(judgment["targetId"], f"{label} targetId")
    nullable_text(judgment["targetAspect"], f"{label} targetAspect")
    applicability = enum(
        judgment["applicability"],
        {"applicable", "cantTell", "inapplicable", "untested"},
        f"{label} applicability",
    )
    evidence = enum(
        judgment["requiredEvidence"],
        {"conflicting", "insufficient", "sufficient", "untested"},
        f"{label} requiredEvidence",
    )
    outcome = enum(
        judgment["outcome"],
        {"cantTell", "failed", "inapplicable", "passed", "untested"},
        f"{label} outcome",
    )
    if outcome in {"passed", "failed"} and (applicability != "applicable" or evidence != "sufficient"):
        fail("authority", f"{label} pass/fail requires applicable and sufficient evidence")
    if outcome == "inapplicable" and applicability != "inapplicable":
        fail("authority", f"{label} inapplicable outcome requires inapplicable applicability")
    if outcome == "cantTell" and (applicability not in {"applicable", "cantTell"} or evidence not in {"conflicting", "insufficient"}):
        fail("authority", f"{label} cantTell requires missing or conflicting evidence")
    if outcome == "untested" and (applicability != "untested" or evidence != "untested"):
        fail("authority", f"{label} untested outcome requires untested applicability and evidence")
    for field in (
        "policyBasis",
        "validAlternativeOrHardNegative",
        "falsePositiveRisk",
        "falseNegativeRisk",
        "rationale",
    ):
        text(judgment[field], f"{label} {field}")
    enum(judgment["confidence"], {"high", "low", "medium"}, f"{label} confidence")
    return judgment_id


def validate_submission(
    submission: dict[str, Any],
    packet: dict[str, Any],
    *,
    require_finalized: bool = False,
) -> None:
    """Strictly validate a reviewer submission and packet binding."""
    validate_packet(packet)
    exact(
        submission,
        {
            "$schema",
            "schemaVersion",
            "documentType",
            "submissionId",
            "submissionDigest",
            "lifecycle",
            "recordPurpose",
            "evidenceStatus",
            "packetBinding",
            "reviewScope",
            "reviewer",
            "declarations",
            "cases",
            "submissionLimitations",
        },
        "reviewer submission",
    )
    if (
        submission["$schema"] != SUBMISSION_SCHEMA
        or submission["schemaVersion"] != VERSION
        or submission["documentType"] != "webReviewerSubmission"
    ):
        fail("version", "reviewer submission uses an unsupported schema or document type")
    submission_id = identifier(submission["submissionId"], "reviewer submission submissionId")
    lifecycle = enum(submission["lifecycle"], {"draft", "finalized"}, "reviewer submission lifecycle")
    purpose = enum(
        submission["recordPurpose"],
        {"fictionalConformance", "humanReviewCandidate"},
        "reviewer submission recordPurpose",
    )
    expected_evidence = "ineligibleConformance" if purpose == "fictionalConformance" else "requiresGovernanceReview"
    if submission["evidenceStatus"] != expected_evidence:
        fail("claims", "reviewer submission has an invalid evidence status")
    binding = exact(submission["packetBinding"], {"packetId", "packetDigest"}, "reviewer submission packetBinding")
    if binding != {"packetId": packet["packetId"], "packetDigest": packet["packetDigest"]}:
        fail("binding", "reviewer submission does not bind the supplied review packet")

    scope = exact(
        submission["reviewScope"],
        {"familyIds", "caseIds", "completeForDeclaredScope"},
        "reviewer submission reviewScope",
    )
    family_ids = sorted_unique_strings(scope["familyIds"], "reviewScope familyIds", len(FAMILIES), identifier)
    case_ids = sorted_unique_strings(scope["caseIds"], "reviewScope caseIds", MAX_CASES, identifier)
    if any(family_id not in {family["familyId"] for family in FAMILIES} for family_id in family_ids):
        fail("inventory", "reviewScope references an unknown fixture family")
    if any(case_id not in CASE_TO_FAMILY for case_id in case_ids):
        fail("inventory", "reviewScope references an unknown packet case")
    if {CASE_TO_FAMILY[case_id] for case_id in case_ids} != set(family_ids):
        fail("binding", "reviewScope familyIds do not match its caseIds")
    boolean(scope["completeForDeclaredScope"], "reviewScope completeForDeclaredScope")

    reviewer = exact(
        submission["reviewer"],
        {
            "stableProjectId",
            "qualification",
            "independentFromAnnotationAuthors",
            "independenceRationale",
            "priorExpectedLabelExposure",
            "conflictOfInterest",
            "reviewedOn",
        },
        "reviewer submission reviewer",
    )
    reviewer_id = identifier(reviewer["stableProjectId"], "reviewer stableProjectId")
    qualification = exact(reviewer["qualification"], {"category", "rationale"}, "reviewer qualification")
    enum(qualification["category"], {"accessibility", "other", "productReview", "webUi"}, "reviewer qualification category")
    text(qualification["rationale"], "reviewer qualification rationale")
    independence = enum(
        reviewer["independentFromAnnotationAuthors"],
        {"declaredFalse", "declaredTrue", "undeclared", "unknown"},
        "reviewer independence",
    )
    text(reviewer["independenceRationale"], "reviewer independence rationale")
    exposure = exact(
        reviewer["priorExpectedLabelExposure"],
        {"status", "caseIds", "rationale"},
        "reviewer prior exposure",
    )
    exposure_status = enum(exposure["status"], {"full", "none", "partial", "undeclared", "unknown"}, "reviewer prior exposure status")
    exposed_cases = sorted_unique_strings(exposure["caseIds"], "reviewer exposed caseIds", MAX_CASES, identifier)
    if any(case_id not in CASE_TO_FAMILY for case_id in exposed_cases):
        fail("inventory", "reviewer prior exposure references an unknown case")
    if exposure_status == "none" and exposed_cases:
        fail("exposure", "reviewer declaring no exposure must not list exposed cases")
    text(exposure["rationale"], "reviewer prior exposure rationale")
    conflict = exact(reviewer["conflictOfInterest"], {"status", "rationale"}, "reviewer conflict of interest")
    conflict_status = enum(conflict["status"], {"declared", "noneDeclared", "undeclared", "unknown"}, "reviewer conflict status")
    text(conflict["rationale"], "reviewer conflict rationale")
    reviewed_on = text(reviewer["reviewedOn"], "reviewer reviewedOn")

    declarations = exact(
        submission["declarations"],
        {
            "sourceFirstNotBlind",
            "sightlintOutputUsedBeforeFinalization",
            "existingOracleViewedBeforeFinalization",
            "generatedCaptureOrReportUsedAsAnswer",
            "implementationOutputUsedAsAnswer",
            "containsProtectedOrPrivateData",
            "containsCredentials",
            "externalProcessingUsed",
            "identityOrQualificationVerifiedBySightLint",
            "signatureVerifiedBySightLint",
        },
        "reviewer submission declarations",
    )
    for name, value in declarations.items():
        boolean(value, f"reviewer declaration {name}")
    prohibited_true = {
        "sightlintOutputUsedBeforeFinalization",
        "existingOracleViewedBeforeFinalization",
        "generatedCaptureOrReportUsedAsAnswer",
        "implementationOutputUsedAsAnswer",
        "containsProtectedOrPrivateData",
        "containsCredentials",
        "externalProcessingUsed",
        "identityOrQualificationVerifiedBySightLint",
        "signatureVerifiedBySightLint",
    }
    if declarations["sourceFirstNotBlind"] is not True or any(declarations[name] for name in prohibited_true):
        fail("declaration", "reviewer submission violates the source-only declaration boundary")

    cases = array(submission["cases"], "reviewer submission cases", 0, MAX_CASES)
    observed_case_ids: list[str] = []
    judgment_ids: list[str] = []
    total_judgments = 0
    for raw_case in cases:
        case = exact(
            raw_case,
            {"caseId", "caseContext", "acquisitionJudgments", "ruleJudgments"},
            "reviewer submission case",
        )
        case_id = identifier(case["caseId"], "reviewer submission caseId")
        observed_case_ids.append(case_id)
        if case_id not in CASE_TO_FAMILY:
            fail("inventory", f"reviewer submission references unknown case {case_id!r}")
        context = exact(case["caseContext"], {"reviewedAs", "rationale"}, f"case {case_id} context")
        enum(context["reviewedAs"], {"ambiguous", "clean", "hardNegative", "other", "targetedMutation"}, f"case {case_id} reviewedAs")
        text(context["rationale"], f"case {case_id} context rationale")
        acquisition = array(case["acquisitionJudgments"], f"case {case_id} acquisitionJudgments", 0, MAX_JUDGMENTS)
        rules = array(case["ruleJudgments"], f"case {case_id} ruleJudgments", 0, MAX_JUDGMENTS)
        total_judgments += len(acquisition) + len(rules)
        for entry in acquisition:
            judgment_ids.append(_acquisition_judgment(entry, f"case {case_id} acquisition judgment"))
        for entry in rules:
            judgment_ids.append(_rule_judgment(entry, f"case {case_id} rule judgment"))
        local_ids = [entry["judgmentId"] for entry in (*acquisition, *rules)]
        if local_ids != sorted(local_ids):
            fail("ordering", f"case {case_id} judgments must be sorted by judgmentId")
    if total_judgments > MAX_JUDGMENTS:
        fail("limit", f"reviewer submission exceeds the {MAX_JUDGMENTS}-judgment limit")
    if observed_case_ids != sorted(observed_case_ids) or len(observed_case_ids) != len(set(observed_case_ids)):
        fail("ordering", "reviewer submission cases must be unique and sorted")
    if len(judgment_ids) != len(set(judgment_ids)):
        fail("ordering", "reviewer submission judgment IDs must be globally unique")
    if observed_case_ids != case_ids:
        fail("binding", "reviewScope caseIds must exactly match submitted cases")

    limitations = array(submission["submissionLimitations"], "submissionLimitations", 1, 16)
    for item in limitations:
        text(item, "submissionLimitations entry")
    _validate_privacy_text(submission, "reviewer submission")

    finalized = lifecycle == "finalized"
    if require_finalized and not finalized:
        fail("lifecycle", "reviewer submission must be finalized before comparison")
    if finalized:
        if (
            submission_id.startswith("replace-")
            or reviewer_id.startswith("replace-")
            or independence in {"undeclared", "unknown"}
            or exposure_status in {"undeclared", "unknown"}
            or conflict_status in {"undeclared", "unknown"}
            or not valid_date(reviewed_on)
            or scope["completeForDeclaredScope"] is not True
            or not cases
            or any(not case["acquisitionJudgments"] and not case["ruleJudgments"] for case in cases)
        ):
            fail("finalization", "reviewer submission has incomplete declarations, scope, or judgments")
        recorded = sha256(submission["submissionDigest"], "reviewer submission submissionDigest")
        if recorded != digest(submission, "submissionDigest"):
            fail("digest", "reviewer submission digest does not match its canonical projection")
        if canonical_bytes(submission) and len(canonical_bytes(submission)) > MAX_INPUT_BYTES:
            fail("input-budget", f"reviewer submission exceeds the {MAX_INPUT_BYTES}-byte limit")
    elif submission["submissionDigest"] is not None:
        fail("lifecycle", "draft reviewer submission must have null submissionDigest")


def finalize_submission(submission: dict[str, Any], packet: dict[str, Any]) -> dict[str, Any]:
    """Finalize without changing reviewer-authored fields other than lifecycle/digest."""
    validate_submission(submission, packet)
    if submission["lifecycle"] != "draft" or submission["submissionDigest"] is not None:
        fail("lifecycle", "only an undigested draft submission can be finalized")
    finalized = dict(submission)
    finalized["lifecycle"] = "finalized"
    finalized["submissionDigest"] = None
    finalized["submissionDigest"] = digest(finalized, "submissionDigest")
    validate_submission(finalized, packet, require_finalized=True)
    return finalized


def valid_date(value: str) -> bool:
    """Return whether a value is a real ISO calendar date rather than a placeholder."""
    if DATE.fullmatch(value) is None or value == "0000-00-00":
        return False
    try:
        date.fromisoformat(value)
    except ValueError:
        return False
    return True


def json_file_digest(path: Path, label: str) -> tuple[dict[str, Any], str]:
    """Load a bounded JSON object and return its raw byte digest."""
    value = load_json(path, label)
    return value, digest_bytes(path.read_bytes())
