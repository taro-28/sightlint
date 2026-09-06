#!/usr/bin/env python3
"""Validate perception evaluation governance without generating reviewed oracles."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "evaluation" / "perception"
FAMILIES = ["hierarchy", "peerGroup", "region", "role", "text"]


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"{path.relative_to(ROOT)} must contain a JSON object")
    return value


def repository_entry(value: str, context: str, *, directory: bool = False) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise SystemExit(f"{context} must be a repository-contained relative path")
    resolved = (ROOT / relative).resolve()
    if ROOT not in resolved.parents:
        raise SystemExit(f"{context} escapes the repository")
    exists = resolved.is_dir() if directory else resolved.is_file()
    if not exists:
        kind = "directory" if directory else "file"
        raise SystemExit(f"{context} does not resolve to a repository {kind}: {value}")
    return resolved


def indexed_cases(document: dict[str, Any], context: str) -> dict[str, dict[str, Any]]:
    cases = document.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"{context} must contain cases")
    result: dict[str, dict[str, Any]] = {}
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("caseId"), str):
            raise SystemExit(f"{context} contains an invalid case")
        if case["caseId"] in result:
            raise SystemExit(f"{context} repeats {case['caseId']!r}")
        result[case["caseId"]] = case
    if list(result) != sorted(result):
        raise SystemExit(f"{context} cases must be sorted")
    return result


def validate_acquisition(case: dict[str, Any]) -> None:
    case_id = case["caseId"]
    expectations = case.get("familyExpectations")
    if not isinstance(expectations, list):
        raise SystemExit(f"{case_id} must declare family expectations")
    families = [item.get("family") for item in expectations if isinstance(item, dict)]
    if families != FAMILIES:
        raise SystemExit(f"{case_id} family expectations must be complete and sorted")
    by_family = {item["family"]: item for item in expectations}
    if (
        by_family["region"].get("expectedStatus") != "observed"
        or by_family["region"].get("groundTruthRole") != "acquisition"
    ):
        raise SystemExit(f"{case_id} must distinguish observed region acquisition truth")
    expected = {
        "hierarchy": "untested",
        "peerGroup": "untested",
        "role": "untested",
        "text": "unsupported",
    }
    for family, status in expected.items():
        item = by_family[family]
        if item.get("expectedStatus") != status or item.get("groundTruthRole") != "notAnnotated":
            raise SystemExit(f"{case_id} overclaims {family} acquisition")
    if not case.get("requiredPreservedFacts") or not case.get("abstentions"):
        raise SystemExit(f"{case_id} must retain facts and abstentions")


def main() -> None:
    corpus = load(DIRECTORY / "corpus.json")
    acquisition = load(DIRECTORY / "annotations" / "acquisition.json")
    rules = load(DIRECTORY / "annotations" / "rules.json")
    for name, document in [
        ("corpus", corpus),
        ("acquisition oracle", acquisition),
        ("rule oracle", rules),
    ]:
        if document.get("schemaVersion") != "0.1.0":
            raise SystemExit(f"{name} must use schema version 0.1.0")
    if acquisition.get("documentType") != "acquisitionOracle":
        raise SystemExit("acquisition annotations have the wrong document type")
    if rules.get("documentType") != "ruleOracle":
        raise SystemExit("rule annotations have the wrong document type")

    source = corpus.get("source", {})
    if (
        source.get("kind") != "repositoryOwnedWebFixture"
        or source.get("license") != "MIT OR Apache-2.0"
        or source.get("privacyReview") != "syntheticNoPersonalData"
        or source.get("externalAssets") is not False
        or source.get("fictionalData") is not True
    ):
        raise SystemExit("perception source governance differs")
    origin = source.get("origin")
    if not isinstance(origin, str):
        raise SystemExit("perception fixture origin must be a path")
    repository_entry(origin, "perception fixture application", directory=True)

    governance = corpus.get("governance", {})
    if (
        governance.get("implementationOutputIsGroundTruth") is not False
        or governance.get("capturedArtifactsCommitted") is not False
        or governance.get("externalProcessing") is not False
        or governance.get("telemetryCollected") is not False
        or governance.get("holdoutStatus") != "notEstablished"
    ):
        raise SystemExit("perception evaluation governance differs")
    if corpus.get("splitPolicy", {}).get("holdout", {}).get("status") != "notEstablished":
        raise SystemExit("public perception data must not claim a protected holdout")
    gates = corpus.get("gates", {})
    if gates != {
        "determinismRuns": 2,
        "maximumBlockingFindings": 0,
        "maximumSemanticClaims": 0,
        "requiredAcquisitionMutationObservations": 1,
        "requiredHardNegativeFailures": 0,
    }:
        raise SystemExit("perception evaluation gates differ")

    protocol = corpus.get("protocol", {})
    for field in ["requestSchema", "responseSchema", "runReportSchema", "extensionSchema"]:
        value = protocol.get(field)
        if not isinstance(value, str):
            raise SystemExit(f"perception {field} must be a path")
        repository_entry(value, f"perception {field}")
    annotations = corpus.get("annotations", {})
    for field in ["schema", "acquisition", "rules", "nativeAcquisitionSource", "nativeRuleSource"]:
        value = annotations.get(field)
        if not isinstance(value, str):
            raise SystemExit(f"perception {field} must be a path")
        repository_entry(value, f"perception {field}")

    cases = corpus.get("cases")
    if not isinstance(cases, list) or len(cases) != 3:
        raise SystemExit("perception corpus must contain exactly three reviewed cases")
    case_ids = [case.get("id") for case in cases if isinstance(case, dict)]
    if case_ids != sorted(case_ids) or len(set(case_ids)) != 3:
        raise SystemExit("perception case IDs must be unique and sorted")
    classifications = {case.get("classification") for case in cases}
    splits = {case.get("split") for case in cases}
    if classifications != {"clean", "hardNegative", "targetedMutation"}:
        raise SystemExit("perception corpus must retain clean, mutation, and hard-negative cases")
    if splits != {"smoke", "development", "challenge"}:
        raise SystemExit("perception corpus must keep smoke, development, and challenge separate")
    for case in cases:
        request = case.get("request")
        if not isinstance(request, str):
            raise SystemExit(f"perception request {case.get('id')} must be a path")
        repository_entry(request, f"perception request {case['id']}")
        if case["classification"] == "targetedMutation":
            if case.get("baselineCaseId") not in case_ids:
                raise SystemExit(f"{case['id']} has no reviewed baseline")
        elif "baselineCaseId" in case:
            raise SystemExit(f"{case['id']} must not declare a mutation baseline")

    acquisition_cases = indexed_cases(acquisition, "acquisition oracle")
    rule_cases = indexed_cases(rules, "rule oracle")
    if list(acquisition_cases) != case_ids or list(rule_cases) != case_ids:
        raise SystemExit("corpus, acquisition, and rule case IDs differ")
    for case_id in case_ids:
        validate_acquisition(acquisition_cases[case_id])
        rule = rule_cases[case_id]
        if (
            rule.get("executableRule") is not None
            or rule.get("expectedOutcome") != "untested"
            or rule.get("blockingAllowed") is not False
        ):
            raise SystemExit(f"rule oracle {case_id} exceeds the protocol slice")
        expected = "inapplicable" if case_id.endswith("intentional-grouping") else "cantTell"
        if rule.get("applicabilityGroundTruth") != expected:
            raise SystemExit(f"rule applicability differs for {case_id}")

    print(
        "perception evaluation: 3 cases, 1 acquisition mutation, 1 hard negative; "
        "semantic claims=0, blocking findings=0, protected holdout=not established"
    )


if __name__ == "__main__":
    main()
