#!/usr/bin/env python3
"""Validate public Web holdout status and the non-evidentiary conformance chain.

This checker is deliberately read-only. It validates manifests, digests, joins, and
publication rules; it does not execute SightLint, access a protected store, verify
human identity, or promote public conformance data to holdout evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "evaluation" / "web"
DEFAULT_ADMISSION = WEB / "holdout-admission.json"
DEFAULT_RECORD = WEB / "holdout-run.json"
MAXIMUM_MANIFEST_BYTES = 1_048_576
MAXIMUM_FAMILIES = 128
MAXIMUM_CASES = 4_096
MAXIMUM_FILES_PER_CASE = 64
MAXIMUM_METRICS = 512
MINIMUM_PUBLISHED_DENOMINATOR = 5
SHA256 = re.compile(r"sha256:[0-9a-f]{64}\Z")
IDENTIFIER = re.compile(r"[a-z0-9][a-z0-9._-]{0,127}\Z")


class ContractError(Exception):
    """A stable, user-facing contract validation failure."""

    def __init__(self, category: str, message: str) -> None:
        super().__init__(message)
        self.category = category


def fail(category: str, message: str) -> NoReturn:
    raise ContractError(category, message)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail("json", "a manifest contains a duplicate object key")
        result[key] = value
    return result


def load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        size = path.stat().st_size
    except OSError:
        fail("input", f"{label} is unavailable")
    if size > MAXIMUM_MANIFEST_BYTES:
        fail("input-budget", f"{label} exceeds the 1048576-byte manifest limit")
    try:
        raw = path.read_bytes()
    except OSError:
        fail("input", f"{label} is unavailable")
    if raw.startswith(b"\xef\xbb\xbf"):
        fail("json", f"{label} must not contain a UTF-8 byte-order mark")
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    except UnicodeDecodeError:
        fail("json", f"{label} is not UTF-8")
    except json.JSONDecodeError:
        fail("json", f"{label} is not valid JSON")
    if not isinstance(value, dict):
        fail("shape", f"{label} must contain a JSON object")
    validate_scalar_domain(value, label)
    return value


def validate_scalar_domain(value: Any, label: str) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            validate_ascii(key, label)
            validate_scalar_domain(child, label)
    elif isinstance(value, list):
        for child in value:
            validate_scalar_domain(child, label)
    elif isinstance(value, str):
        validate_ascii(value, label)
    elif isinstance(value, float):
        fail("scalar-domain", f"{label} must use integer numeric values")
    elif value is not None and not isinstance(value, (bool, int)):
        fail("scalar-domain", f"{label} contains an unsupported JSON value")


def validate_ascii(value: str, label: str) -> None:
    if len(value) > 512 or any(ord(character) < 32 or ord(character) > 126 for character in value):
        fail("scalar-domain", f"{label} contains a non-printable-ASCII or overlong string")


def obj(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("shape", f"{label} must be an object")
    return value


def items(value: Any, label: str, minimum: int = 0, maximum: int | None = None) -> list[Any]:
    if not isinstance(value, list):
        fail("shape", f"{label} must be an array")
    if len(value) < minimum or (maximum is not None and len(value) > maximum):
        fail("limit", f"{label} has an invalid item count")
    return value


def integer(value: Any, label: str, minimum: int = 0, maximum: int = 1_000_000) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not minimum <= value <= maximum:
        fail("shape", f"{label} must be an integer from {minimum} through {maximum}")
    return value


def text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail("shape", f"{label} must be a non-empty string")
    return value


def identifier(value: Any, label: str) -> str:
    result = text(value, label)
    if not IDENTIFIER.fullmatch(result):
        fail("shape", f"{label} must be a stable lowercase identifier")
    return result


def boolean(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        fail("shape", f"{label} must be a boolean")
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


def canonical_bytes(value: dict[str, Any], omit: str | None = None) -> bytes:
    projected = dict(value)
    if omit is not None:
        projected.pop(omit, None)
    return json.dumps(
        projected, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def digest(value: dict[str, Any], omit: str | None = None) -> str:
    return f"sha256:{hashlib.sha256(canonical_bytes(value, omit)).hexdigest()}"


def validate_sha256(value: Any, label: str) -> str:
    result = text(value, label)
    if not SHA256.fullmatch(result):
        fail("digest", f"{label} must be a lowercase SHA-256 digest")
    return result


def validate_manifest_digest(value: dict[str, Any], label: str) -> None:
    recorded = validate_sha256(value.get("manifestDigest"), f"{label} manifestDigest")
    if recorded != digest(value, "manifestDigest"):
        fail("digest", f"{label} manifestDigest does not match its canonical projection")


def sorted_unique_ids(values: list[Any], field: str, label: str) -> list[str]:
    result = [identifier(obj(value, label).get(field), f"{label} {field}") for value in values]
    if result != sorted(result) or len(result) != len(set(result)):
        fail("ordering", f"{label} must have unique entries sorted by {field}")
    return result


def validate_binding(value: Any, expected: dict[str, Any], label: str) -> None:
    binding = exact(value, {"id", "version", "manifestDigest"}, label)
    if binding != {
        "id": expected["id"],
        "version": expected["version"],
        "manifestDigest": expected["manifestDigest"],
    }:
        fail("binding", f"{label} does not match the referenced manifest")


def validate_relative_file(root: Path, value: Any, expected_digest: Any, expected_length: Any, label: str) -> Path:
    relative_text = text(value, f"{label} path")
    relative = PurePosixPath(relative_text)
    if (
        relative.is_absolute()
        or "\\" in relative_text
        or ":" in relative_text
        or "//" in relative_text
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        fail("path", f"{label} must use a contained relative POSIX path")
    path = root.joinpath(*relative.parts)
    try:
        if path.is_symlink() or not path.is_file():
            fail("path", f"{label} does not resolve to a regular file")
        resolved = path.resolve()
        if not resolved.is_relative_to(root.resolve()):
            fail("path", f"{label} escapes its bundle directory")
        raw = path.read_bytes()
    except OSError:
        fail("path", f"{label} is unavailable")
    if len(raw) > MAXIMUM_MANIFEST_BYTES:
        fail("input-budget", f"{label} exceeds the 1048576-byte file limit")
    length = integer(expected_length, f"{label} byteLength", 0, MAXIMUM_MANIFEST_BYTES)
    if len(raw) != length:
        fail("digest", f"{label} byteLength does not match the referenced file")
    recorded = validate_sha256(expected_digest, f"{label} sha256")
    if recorded != f"sha256:{hashlib.sha256(raw).hexdigest()}":
        fail("digest", f"{label} sha256 does not match the referenced file")
    return path


def validate_admission(path: Path) -> tuple[dict[str, Any], str]:
    admission = load_json(path, "holdout admission")
    if admission.get("status") != "notOperational":
        fail("admission", "this public checker requires a separately verified operational admission")
    if admission.get("publicCasesEligible") is not False:
        fail("admission", "public repository cases must remain ineligible for protected evidence")
    if admission.get("implementationOutputUsedAsOracle") is not False:
        fail("oracle", "implementation output must not be used as holdout truth")
    blockers = items(admission.get("blockers"), "admission blockers", 1, 32)
    if not all(isinstance(item, str) and item for item in blockers):
        fail("shape", "admission blockers must be non-empty strings")
    return admission, digest(admission)


DISCLOSURE_FIELDS = {
    "authorityId",
    "minimumPublishedDenominator",
    "containsArtifactPathsOrUrls",
    "containsCaseMembership",
    "containsLabelsOrSelectors",
    "containsPerCaseResults",
    "containsCredentialsOrPersonalData",
    "universalScore",
}


def validate_disclosure(value: Any, label: str) -> int:
    disclosure = exact(value, DISCLOSURE_FIELDS, label)
    identifier(disclosure["authorityId"], f"{label} authorityId")
    threshold = integer(
        disclosure["minimumPublishedDenominator"],
        f"{label} minimumPublishedDenominator",
        MINIMUM_PUBLISHED_DENOMINATOR,
    )
    for field in DISCLOSURE_FIELDS - {"authorityId", "minimumPublishedDenominator"}:
        if boolean(disclosure[field], f"{label} {field}"):
            fail("disclosure", f"{label} {field} must remain false")
    return threshold


def validate_current_record(path: Path, admission_digest: str) -> None:
    record = load_json(path, "current holdout record")
    exact(
        record,
        {
            "$schema",
            "schemaVersion",
            "documentType",
            "manifestDigest",
            "recordPurpose",
            "lifecycle",
            "dataClassification",
            "evidenceEligible",
            "admission",
            "disclosure",
            "nonClaims",
            "blockers",
        },
        "current holdout record",
    )
    expected = {
        "$schema": "urn:sightlint:schema:web-holdout-run:1.0.0",
        "schemaVersion": "1.0.0",
        "documentType": "sanitizedHoldoutRunAttestation",
        "recordPurpose": "currentStatus",
        "lifecycle": "notRun",
        "dataClassification": "publicMetadataOnly",
        "evidenceEligible": False,
    }
    for field, wanted in expected.items():
        if record.get(field) != wanted:
            fail("lifecycle", f"current holdout record {field} must be {wanted!r}")
    admission = exact(record["admission"], {"schemaVersion", "status", "recordDigest"}, "record admission")
    if admission != {
        "schemaVersion": "1.0.0",
        "status": "notOperational",
        "recordDigest": admission_digest,
    }:
        fail("binding", "current holdout record does not match holdout admission")
    validate_disclosure(record["disclosure"], "current disclosure")
    items(record["nonClaims"], "current nonClaims", 1, 32)
    items(record["blockers"], "current blockers", 1, 32)
    validate_manifest_digest(record, "current holdout record")


def validate_bundle(root: Path) -> tuple[dict[str, Any], list[str]]:
    bundle = load_json(root / "bundle-manifest.json", "bundle manifest")
    exact(
        bundle,
        {"$schema", "schemaVersion", "documentType", "manifestDigest", "dataClassification", "bundle", "provenance", "privacy", "limits", "families", "cases"},
        "bundle manifest",
    )
    if bundle.get("dataClassification") != "publicConformanceOnly":
        fail("classification", "the committed conformance bundle must remain publicConformanceOnly")
    metadata = exact(
        bundle["bundle"],
        {"id", "version", "frozenAt", "storageAuthorityId", "tuningVisible", "implementationOutputUsedAsOracle"},
        "bundle metadata",
    )
    for field in ("id", "storageAuthorityId"):
        identifier(metadata[field], f"bundle {field}")
    if metadata["tuningVisible"] is not True:
        fail("classification", "public conformance data must be tuning-visible")
    if metadata["implementationOutputUsedAsOracle"] is not False:
        fail("oracle", "implementation output must not be used as bundle truth")
    provenance = exact(
        bundle["provenance"],
        {"sourceAuthorityId", "ownershipBasis", "licenseId", "redistribution"},
        "bundle provenance",
    )
    identifier(provenance["sourceAuthorityId"], "bundle sourceAuthorityId")
    if (
        provenance["licenseId"] != "MIT OR Apache-2.0"
        or provenance["redistribution"] != "permitted"
        or "repository-authored fictional" not in text(provenance["ownershipBasis"], "bundle ownershipBasis").lower()
    ):
        fail("provenance", "public conformance data must retain its fictional dual-licensed provenance")
    privacy = exact(
        bundle["privacy"],
        {"reviewed", "containsPersonalData", "containsCustomerData", "containsCredentials", "externalProcessing", "retentionPolicyVersion"},
        "bundle privacy",
    )
    if privacy != {
        "reviewed": True,
        "containsPersonalData": False,
        "containsCustomerData": False,
        "containsCredentials": False,
        "externalProcessing": False,
        "retentionPolicyVersion": "1.0.0",
    }:
        fail("privacy", "public conformance data must remain reviewed, fictional, credential-free, and local")
    limits = exact(bundle["limits"], {"maximumFamilies", "maximumCases", "maximumFilesPerCase", "maximumManifestBytes"}, "bundle limits")
    maximum_families = integer(limits["maximumFamilies"], "maximumFamilies", 1, MAXIMUM_FAMILIES)
    maximum_cases = integer(limits["maximumCases"], "maximumCases", 1, MAXIMUM_CASES)
    maximum_files = integer(limits["maximumFilesPerCase"], "maximumFilesPerCase", 1, MAXIMUM_FILES_PER_CASE)
    if integer(limits["maximumManifestBytes"], "maximumManifestBytes", 1, MAXIMUM_MANIFEST_BYTES) != MAXIMUM_MANIFEST_BYTES:
        fail("limit", "bundle maximumManifestBytes must pin the checker limit")
    families = items(bundle["families"], "bundle families", 1, maximum_families)
    cases = items(bundle["cases"], "bundle cases", 1, maximum_cases)
    family_ids = sorted_unique_ids(families, "id", "bundle families")
    case_ids = sorted_unique_ids(cases, "id", "bundle cases")
    membership: dict[str, str] = {}
    for family_value in families:
        family = exact(family_value, {"id", "caseIds"}, "bundle family")
        ids = items(family["caseIds"], "family caseIds", 1, MAXIMUM_CASES)
        if ids != sorted(ids) or len(ids) != len(set(ids)) or not all(isinstance(value, str) for value in ids):
            fail("ordering", "family caseIds must be unique and sorted")
        for case_id in ids:
            identifier(case_id, "family caseId")
            if case_id in membership:
                fail("join", "a case is assigned to more than one family")
            membership[case_id] = family["id"]
    if sorted(membership) != case_ids:
        fail("join", "family membership must cover exactly the bundle cases")
    classifications: list[str] = []
    for case_value in cases:
        case = exact(case_value, {"id", "familyId", "classification", "files"}, "bundle case")
        case_id = identifier(case["id"], "bundle case id")
        family_id = identifier(case["familyId"], "bundle case familyId")
        if family_id not in family_ids or membership.get(case_id) != family_id:
            fail("join", "bundle case family membership is inconsistent")
        classification = text(case["classification"], "bundle case classification")
        if classification not in {"clean", "targetedMutation", "hardNegative", "ambiguous", "malformed", "resourceBoundary"}:
            fail("shape", "bundle case classification is unsupported")
        classifications.append(classification)
        files = items(case["files"], "bundle case files", 1, maximum_files)
        roles: list[str] = []
        for file_value in files:
            file = exact(file_value, {"role", "path", "sha256", "byteLength", "mediaType"}, "bundle file")
            role = text(file["role"], "bundle file role")
            if role not in {"source", "request", "supportingAsset"}:
                fail("shape", "bundle file role is unsupported")
            roles.append(role)
            validate_relative_file(root, file["path"], file["sha256"], file["byteLength"], "bundle file")
        if roles.count("source") != 1 or roles.count("request") != 1 or len(roles) != len(set(roles)):
            fail("shape", "each conformance case must have one source, one request, and unique file roles")
    if Counter(classifications) != Counter({name: 1 for name in ("clean", "targetedMutation", "hardNegative", "ambiguous", "malformed", "resourceBoundary")}):
        fail("coverage", "the conformance bundle must exercise each required case class once")
    lowered = canonical_bytes(bundle).decode("ascii").lower()
    if "atlas" in lowered or "harbor" in lowered:
        fail("leakage", "the conformance bundle must not copy tuning fixture family identities")
    validate_manifest_digest(bundle, "bundle manifest")
    return bundle, case_ids


def validate_count_array(value: Any, key: str, expected: dict[str, int], label: str) -> None:
    rows = items(value, label, 1, len(expected))
    names = [text(exact(row, {key, "count"}, label)[key], f"{label} {key}") for row in rows]
    if names != sorted(names) or len(names) != len(set(names)):
        fail("ordering", f"{label} must be uniquely sorted by {key}")
    actual = {row[key]: integer(row["count"], f"{label} count", 0, 1_000_000) for row in rows}
    if actual != expected:
        fail("oracle", f"{label} does not match its reviewed source")


def validate_oracle(root: Path, bundle: dict[str, Any], case_ids: list[str]) -> dict[str, Any]:
    oracle = load_json(root / "oracle-manifest.json", "oracle manifest")
    exact(
        oracle,
        {"$schema", "schemaVersion", "documentType", "manifestDigest", "dataClassification", "bundleBinding", "oracle", "caseIds", "acquisitionDocuments", "ruleDocuments", "classificationCounts", "acquisitionExpectationCounts", "ruleOutcomeCounts"},
        "oracle manifest",
    )
    if oracle.get("dataClassification") != "publicConformanceOnly":
        fail("classification", "the committed conformance oracle must remain publicConformanceOnly")
    validate_binding(oracle["bundleBinding"], {**bundle["bundle"], "manifestDigest": bundle["manifestDigest"]}, "oracle bundle binding")
    metadata = exact(
        oracle["oracle"],
        {"id", "version", "frozenAt", "authoringBasis", "implementationOutputUsedAsOracle", "reviewStatus", "reviewers", "unresolvedDisagreements"},
        "oracle metadata",
    )
    if metadata["authoringBasis"] != "publicConformanceContract" or metadata["reviewStatus"] != "conformanceOnly":
        fail("assurance", "public conformance truth must not claim independent review")
    if metadata["implementationOutputUsedAsOracle"] is not False:
        fail("oracle", "implementation output must not be used as oracle truth")
    reviewers = items(metadata["reviewers"], "oracle reviewers", 1, 64)
    reviewer_ids: list[str] = []
    for reviewer_value in reviewers:
        reviewer = exact(reviewer_value, {"id", "role", "qualification", "independentFromAnnotationAuthors"}, "oracle reviewer")
        reviewer_ids.append(identifier(reviewer["id"], "oracle reviewer id"))
        if reviewer["independentFromAnnotationAuthors"] is not False:
            fail("assurance", "conformance authors must not be represented as independent reviewers")
    if len(reviewer_ids) != len(set(reviewer_ids)):
        fail("ordering", "oracle reviewer IDs must be unique")
    integer(metadata["unresolvedDisagreements"], "unresolvedDisagreements")
    declared_case_ids = items(oracle["caseIds"], "oracle caseIds", 1, MAXIMUM_CASES)
    if declared_case_ids != case_ids:
        fail("join", "oracle caseIds must exactly match sorted bundle case IDs")

    documents: dict[str, dict[str, Any]] = {}
    paths: set[str] = set()
    for group in ("acquisitionDocuments", "ruleDocuments"):
        rows = items(oracle[group], f"oracle {group}", 1, MAXIMUM_FAMILIES)
        sorted_unique_ids(rows, "id", f"oracle {group}")
        for row_value in rows:
            row = exact(row_value, {"id", "path", "sha256", "byteLength", "caseCount"}, "oracle document")
            path_text = text(row["path"], "oracle document path")
            if path_text in paths:
                fail("oracle", "acquisition and rule truth must use distinct documents")
            paths.add(path_text)
            if integer(row["caseCount"], "oracle document caseCount", 1, MAXIMUM_CASES) != len(case_ids):
                fail("join", "oracle document caseCount must match the bundle")
            path = validate_relative_file(root, path_text, row["sha256"], row["byteLength"], "oracle document")
            documents[group] = load_json(path, f"{group} payload")

    acquisition_payload = documents["acquisitionDocuments"]
    rule_payload = documents["ruleDocuments"]
    if acquisition_payload.get("authority") != "acquisition" or "outcomes" in acquisition_payload:
        fail("oracle", "acquisition truth must remain separate from rule verdict truth")
    if rule_payload.get("authority") != "rule" or "states" in rule_payload:
        fail("oracle", "rule verdict truth must remain separate from acquisition truth")
    if acquisition_payload.get("caseCount") != len(case_ids) or rule_payload.get("caseCount") != len(case_ids):
        fail("join", "oracle payload caseCount must match the bundle")
    classifications = Counter(case["classification"] for case in bundle["cases"])
    validate_count_array(oracle["classificationCounts"], "classification", dict(classifications), "classification counts")
    validate_count_array(oracle["acquisitionExpectationCounts"], "state", obj(acquisition_payload.get("states"), "acquisition states"), "acquisition counts")
    validate_count_array(oracle["ruleOutcomeCounts"], "outcome", obj(rule_payload.get("outcomes"), "rule outcomes"), "rule outcome counts")
    validate_manifest_digest(oracle, "oracle manifest")
    return oracle


def validate_invocation(root: Path, bundle: dict[str, Any], oracle: dict[str, Any], case_count: int) -> dict[str, Any]:
    invocation = load_json(root / "invocation-manifest.json", "invocation manifest")
    exact(
        invocation,
        {"$schema", "schemaVersion", "documentType", "manifestDigest", "dataClassification", "source", "bundleBinding", "oracleBinding", "commands", "evaluationContract", "environment", "resourceLimits", "createdAt", "createdBy"},
        "invocation manifest",
    )
    if invocation.get("dataClassification") != "publicConformanceOnly":
        fail("classification", "the conformance invocation must remain publicConformanceOnly")
    validate_binding(invocation["bundleBinding"], {**bundle["bundle"], "manifestDigest": bundle["manifestDigest"]}, "invocation bundle binding")
    validate_binding(invocation["oracleBinding"], {**oracle["oracle"], "manifestDigest": oracle["manifestDigest"]}, "invocation oracle binding")
    source = exact(invocation["source"], {"commit", "tree", "buildInputArchiveDigest", "binaryDigest", "adapterLockDigest", "adapterEntryDigest"}, "invocation source")
    for field in ("commit", "tree"):
        if not re.fullmatch(r"[0-9a-f]{40}", text(source[field], f"source {field}")):
            fail("shape", f"source {field} must be a full Git object ID")
    for field in ("buildInputArchiveDigest", "binaryDigest", "adapterLockDigest", "adapterEntryDigest"):
        validate_sha256(source[field], f"source {field}")
    commands = exact(invocation["commands"], {"commandDigest", "allowedPlaceholders", "captureArgv", "checkArgv", "shellInterpolation"}, "invocation commands")
    allowed = items(commands["allowedPlaceholders"], "allowed placeholders", 1, 16)
    expected_allowed = ["{artifactIr}", "{repositoryRoot}", "{request}", "{screenshot}"]
    if allowed != expected_allowed:
        fail("command", "allowed placeholders must be complete, unique, and sorted")
    expected_capture = ["node", "adapters/playwright/dist/src/cli.js", "--request", "{request}", "--repository-root", "{repositoryRoot}", "--artifact-ir-out", "{artifactIr}", "--screenshot-out", "{screenshot}"]
    expected_check = ["target/release/sightlint", "check", "{artifactIr}", "--profile", "recommended", "--format", "json"]
    if commands["captureArgv"] != expected_capture or commands["checkArgv"] != expected_check:
        fail("command", "invocation must pin the public adapter and sightlint command surfaces")
    if commands["shellInterpolation"] is not False:
        fail("command", "holdout commands must not use shell interpolation")
    if validate_sha256(commands["commandDigest"], "command digest") != digest(commands, "commandDigest"):
        fail("digest", "commandDigest does not match its canonical projection")
    evaluation = exact(
        invocation["evaluationContract"],
        {"profileId", "profileVersion", "configurationDigest", "rules", "expectedExitCodes"},
        "evaluation contract",
    )
    if evaluation["profileId"] != "sightlint:recommended" or evaluation["profileVersion"] != "0.1.0":
        fail("command", "evaluation contract must pin the current recommended profile")
    validate_sha256(evaluation["configurationDigest"], "evaluation configurationDigest")
    rules = items(evaluation["rules"], "evaluation rule bindings", 1, 128)
    if sorted_unique_ids(rules, "id", "evaluation rule bindings") != [
        "web.accessibility.interactive-name",
        "web.interaction.ancestor-clip",
        "web.interaction.center-hit",
    ]:
        fail("command", "evaluation contract must pin the current recommended rule set")
    for rule_value in rules:
        rule = exact(rule_value, {"id", "version"}, "evaluation rule binding")
        if rule["version"] != "0.1.0":
            fail("command", "evaluation contract must pin current rule versions")
    if evaluation["expectedExitCodes"] != [0, 1, 2]:
        fail("command", "evaluation contract must preserve public exit-code semantics")
    environment = exact(
        invocation["environment"],
        {"manifestDigest", "operatingSystem", "architecture", "rustVersion", "nodeVersion", "playwrightVersion", "chromiumRevision", "locale", "timezone", "theme", "reducedMotion", "viewportWidthCssPixels", "viewportHeightCssPixels", "deviceScaleFactor", "textScale"},
        "invocation environment",
    )
    if validate_sha256(environment["manifestDigest"], "environment digest") != digest(environment, "manifestDigest"):
        fail("digest", "environment manifestDigest does not match its canonical projection")
    for field in ("viewportWidthCssPixels", "viewportHeightCssPixels", "deviceScaleFactor", "textScale"):
        integer(environment[field], f"environment {field}", 1, 32768)
    limits = exact(invocation["resourceLimits"], {"maximumCases", "maximumCaseSeconds", "maximumOutputBytes", "maximumManifestBytes"}, "invocation resource limits")
    if integer(limits["maximumCases"], "invocation maximumCases", 1, MAXIMUM_CASES) != case_count:
        fail("limit", "invocation maximumCases must equal the admitted conformance case count")
    integer(limits["maximumCaseSeconds"], "maximumCaseSeconds", 1, 3600)
    integer(limits["maximumOutputBytes"], "maximumOutputBytes", 1, 1_073_741_824)
    if integer(limits["maximumManifestBytes"], "maximumManifestBytes", 1, MAXIMUM_MANIFEST_BYTES) != MAXIMUM_MANIFEST_BYTES:
        fail("limit", "invocation maximumManifestBytes must pin the checker limit")
    validate_manifest_digest(invocation, "invocation manifest")
    return invocation


METRIC_FIELDS = {"id", "measure", "scope", "numerator", "denominator"}
SCOPE_FIELDS = {"opaqueCohortId", "split", "ruleId", "ruleVersion", "evidenceClass"}


def validate_scope(value: Any, label: str) -> dict[str, Any]:
    scope = exact(value, SCOPE_FIELDS, label)
    for field in ("opaqueCohortId", "ruleId", "evidenceClass"):
        identifier(scope[field], f"{label} {field}")
    if scope["split"] != "holdout":
        fail("shape", f"{label} split must be holdout")
    return scope


def validate_result(root: Path, bundle: dict[str, Any], oracle: dict[str, Any], invocation: dict[str, Any], case_ids: list[str]) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    result = load_json(root / "private-result-manifest.json", "private result manifest")
    exact(
        result,
        {"$schema", "schemaVersion", "documentType", "manifestDigest", "dataClassification", "bundleBinding", "oracleBinding", "invocationBinding", "execution", "caseResults", "metricCells"},
        "private result manifest",
    )
    if result.get("dataClassification") != "publicConformanceOnly":
        fail("classification", "the conformance private result must remain publicConformanceOnly")
    validate_binding(result["bundleBinding"], {**bundle["bundle"], "manifestDigest": bundle["manifestDigest"]}, "result bundle binding")
    validate_binding(result["oracleBinding"], {**oracle["oracle"], "manifestDigest": oracle["manifestDigest"]}, "result oracle binding")
    if exact(result["invocationBinding"], {"manifestDigest"}, "result invocation binding")["manifestDigest"] != invocation["manifestDigest"]:
        fail("binding", "result invocation binding does not match the invocation manifest")
    execution = exact(result["execution"], {"status", "startedAt", "completedAt", "evaluatorId", "attemptedCases", "completedCases", "executionErrors"}, "private execution")
    attempted = integer(execution["attemptedCases"], "attemptedCases", 0, MAXIMUM_CASES)
    completed = integer(execution["completedCases"], "completedCases", 0, MAXIMUM_CASES)
    errors = integer(execution["executionErrors"], "executionErrors", 0, MAXIMUM_CASES)
    if attempted != len(case_ids) or completed + errors != attempted or execution["startedAt"] > execution["completedAt"]:
        fail("execution", "private execution counts or timestamps are inconsistent")
    if execution["status"] == "succeeded" and errors != 0:
        fail("execution", "a succeeded private execution cannot contain execution errors")
    case_results = items(result["caseResults"], "private case results", 1, MAXIMUM_CASES)
    if sorted_unique_ids(case_results, "caseId", "private case results") != case_ids:
        fail("join", "private case results must match the bundle case IDs")
    for case_value in case_results:
        case = exact(case_value, {"caseId", "captureResponseDigest", "artifactIrDigest", "screenshotDigest", "checkReportDigest", "diagnosticsDigest", "exitCode"}, "private case result")
        for field in ("captureResponseDigest", "artifactIrDigest", "screenshotDigest", "checkReportDigest", "diagnosticsDigest"):
            validate_sha256(case[field], f"case result {field}")
        integer(case["exitCode"], "case result exitCode", 0, 2)
    metric_values = items(result["metricCells"], "private metric cells", 1, MAXIMUM_METRICS)
    metric_ids = sorted_unique_ids(metric_values, "id", "private metric cells")
    metrics: dict[str, dict[str, Any]] = {}
    for metric_value in metric_values:
        metric = exact(metric_value, METRIC_FIELDS, "private metric cell")
        validate_scope(metric["scope"], "private metric scope")
        numerator = integer(metric["numerator"], "metric numerator")
        denominator = integer(metric["denominator"], "metric denominator")
        if numerator > denominator:
            fail("metric", "metric numerator must not exceed denominator")
        metrics[metric["id"]] = metric
    if list(metrics) != metric_ids:
        fail("ordering", "private metric cells must remain sorted")
    validate_manifest_digest(result, "private result manifest")
    return result, metrics


PUBLIC_METRIC_BASE = {"id", "measure", "scope", "publication"}


def validate_attestation(root: Path, admission_digest: str, bundle: dict[str, Any], oracle: dict[str, Any], invocation: dict[str, Any], result: dict[str, Any], private_metrics: dict[str, dict[str, Any]]) -> dict[str, Any]:
    attestation = load_json(root / "public-attestation.json", "public attestation")
    exact(
        attestation,
        {"$schema", "schemaVersion", "documentType", "manifestDigest", "recordPurpose", "lifecycle", "dataClassification", "evidenceEligible", "admission", "bindings", "execution", "assurance", "disclosure", "metrics", "nonClaims"},
        "public attestation",
    )
    expected = {"recordPurpose": "conformanceExample", "lifecycle": "valid", "dataClassification": "publicConformanceOnly", "evidenceEligible": False}
    for field, wanted in expected.items():
        if attestation.get(field) != wanted:
            fail("lifecycle", f"conformance attestation {field} must be {wanted!r}")
    admission = exact(attestation["admission"], {"schemaVersion", "status", "recordDigest"}, "attestation admission")
    if admission != {"schemaVersion": "1.0.0", "status": "notOperational", "recordDigest": admission_digest}:
        fail("binding", "conformance attestation must bind the non-operational public admission")
    bindings = exact(attestation["bindings"], {"bundle", "oracle", "invocationManifestDigest", "privateResultManifestDigest"}, "attestation bindings")
    validate_binding(bindings["bundle"], {**bundle["bundle"], "manifestDigest": bundle["manifestDigest"]}, "attestation bundle binding")
    validate_binding(bindings["oracle"], {**oracle["oracle"], "manifestDigest": oracle["manifestDigest"]}, "attestation oracle binding")
    if bindings["invocationManifestDigest"] != invocation["manifestDigest"] or bindings["privateResultManifestDigest"] != result["manifestDigest"]:
        fail("binding", "attestation manifest bindings do not match the private chain")
    execution = exact(attestation["execution"], {"status", "startedAt", "completedAt", "attemptedCases", "completedCases", "executionErrors"}, "public execution")
    private_execution = dict(result["execution"])
    private_execution.pop("evaluatorId")
    if execution != private_execution:
        fail("execution", "public execution summary does not match the private result")
    assurance = exact(attestation["assurance"], {"evaluator", "secondVerifier", "detachedSignaturesVerifiedBy", "cryptographicVerificationPerformedBySightLint", "exposureLogVersion", "exposureLogDigest"}, "attestation assurance")
    declarations: list[dict[str, Any]] = []
    for role in ("evaluator", "secondVerifier"):
        declaration = exact(assurance[role], {"id", "qualification", "independentFromTuning", "conflictOfInterestReviewed", "declarationDigest"}, f"{role} declaration")
        identifier(declaration["id"], f"{role} id")
        if declaration["independentFromTuning"] is not True or declaration["conflictOfInterestReviewed"] is not True:
            fail("assurance", f"{role} declaration must explicitly assert both controls")
        validate_sha256(declaration["declarationDigest"], f"{role} declarationDigest")
        declarations.append(declaration)
    if declarations[0]["id"] == declarations[1]["id"]:
        fail("assurance", "evaluator and second verifier must be different identities")
    if declarations[0]["id"] != result["execution"]["evaluatorId"]:
        fail("assurance", "public evaluator must match the private execution evaluator")
    if assurance["cryptographicVerificationPerformedBySightLint"] is not False:
        fail("assurance", "SightLint must not claim cryptographic identity verification")
    validate_sha256(assurance["exposureLogDigest"], "exposure log digest")
    threshold = validate_disclosure(attestation["disclosure"], "attestation disclosure")
    if threshold != MINIMUM_PUBLISHED_DENOMINATOR:
        fail("disclosure", "the v1 conformance threshold must remain 5")

    public_values = items(attestation["metrics"], "public metrics", 1, MAXIMUM_METRICS)
    if sorted_unique_ids(public_values, "id", "public metrics") != list(private_metrics):
        fail("join", "public metrics must match sorted private metric IDs")
    for public_value in public_values:
        publication = text(obj(public_value, "public metric").get("publication"), "metric publication")
        allowed = set(PUBLIC_METRIC_BASE)
        if publication in {"reported", "zeroDenominator"}:
            allowed |= {"numerator", "denominator"}
        elif publication == "suppressed":
            allowed |= {"suppressionReason", "privateCellDigest"}
        else:
            fail("metric", "public metric publication is unsupported")
        public = exact(public_value, allowed, "public metric")
        private = private_metrics[public["id"]]
        if public["measure"] != private["measure"] or public["scope"] != private["scope"]:
            fail("metric", "public metric scope or measure does not match the private cell")
        numerator = private["numerator"]
        denominator = private["denominator"]
        if publication == "reported":
            if denominator < threshold or public["numerator"] != numerator or public["denominator"] != denominator:
                fail("disclosure", "reported metrics must meet the threshold and match private counts")
        elif publication == "zeroDenominator":
            if (numerator, denominator, public["numerator"], public["denominator"]) != (0, 0, 0, 0):
                fail("metric", "zeroDenominator metrics must preserve an explicit 0/0 cell")
        else:
            if public["suppressionReason"] != "smallCell" or not 0 < denominator < threshold:
                fail("disclosure", "small-cell suppression requires a nonzero denominator below the threshold")
            if public["privateCellDigest"] != digest(private):
                fail("digest", "suppressed metric does not bind the matching private cell")

    serialized_strings = [value.lower() for value in walk_strings(attestation)]
    sensitive_fragments = ("atlas", "harbor", "http://", "https://", "/users/", "c:\\")
    if any(fragment in value for value in serialized_strings for fragment in sensitive_fragments):
        fail("leakage", "public attestation contains a prohibited fixture identity, path, or URL")
    items(attestation["nonClaims"], "attestation nonClaims", 1, 32)
    validate_manifest_digest(attestation, "public attestation")
    return attestation


def walk_strings(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, dict):
        return [item for child in value.values() for item in walk_strings(child)]
    if isinstance(value, list):
        return [item for child in value for item in walk_strings(child)]
    return []


def validate_conformance(root: Path, admission_digest: str) -> tuple[int, int]:
    bundle, case_ids = validate_bundle(root)
    oracle = validate_oracle(root, bundle, case_ids)
    invocation = validate_invocation(root, bundle, oracle, len(case_ids))
    result, private_metrics = validate_result(root, bundle, oracle, invocation, case_ids)
    validate_attestation(root, admission_digest, bundle, oracle, invocation, result, private_metrics)
    return len(case_ids), len(private_metrics)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", type=Path, default=DEFAULT_RECORD, help="public current-status record")
    parser.add_argument("--admission", type=Path, default=DEFAULT_ADMISSION, help="holdout admission record")
    parser.add_argument("--conformance-dir", type=Path, help="validate a public, non-evidentiary conformance chain")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        _admission, admission_digest = validate_admission(args.admission)
        if args.conformance_dir is None:
            validate_current_record(args.record, admission_digest)
            print("web holdout foundation: lifecycle=notRun, admission=notOperational, evidence_eligible=false")
        else:
            case_count, metric_count = validate_conformance(args.conformance_dir, admission_digest)
            print(
                "web holdout foundation: "
                f"conformance_chain=valid, cases={case_count}, metrics={metric_count}, "
                "evidence_eligible=false"
            )
    except ContractError as error:
        print(f"web-holdout-foundation: {error.category}: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
