#!/usr/bin/env python3
"""Validate multi-family Web evaluation governance without generating oracles."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "evaluation" / "web"
REGISTRY = WEB / "evaluation-v1.json"
HOLDOUT = WEB / "holdout-admission.json"
VERSION = "1.0.0"


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"{path.relative_to(ROOT)} must contain an object")
    return value


def fields(
    value: Any, allowed: set[str], required: set[str], context: str
) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SystemExit(f"{context} must be an object")
    if unknown := set(value) - allowed:
        raise SystemExit(f"{context} has unsupported fields: {sorted(unknown)}")
    if missing := required - set(value):
        raise SystemExit(f"{context} is missing fields: {sorted(missing)}")
    return value


def repo_path(value: Any, context: str, *, directory: bool = False) -> Path:
    if not isinstance(value, str) or not value:
        raise SystemExit(f"{context} must be a repository-relative path")
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise SystemExit(f"{context} must remain inside the repository")
    resolved = (ROOT / relative).resolve()
    if ROOT not in resolved.parents:
        raise SystemExit(f"{context} escapes the repository")
    exists = resolved.is_dir() if directory else resolved.is_file()
    if not exists:
        kind = "directory" if directory else "file"
        raise SystemExit(f"{context} is not a {kind}: {value}")
    return resolved


def index(values: Any, key: str, context: str) -> dict[str, dict[str, Any]]:
    if not isinstance(values, list) or not values:
        raise SystemExit(f"{context} must be a non-empty array")
    result: dict[str, dict[str, Any]] = {}
    for item in values:
        if not isinstance(item, dict) or not isinstance(item.get(key), str):
            raise SystemExit(f"{context} has an entry without {key}")
        identifier = item[key]
        if identifier in result:
            raise SystemExit(f"{context} repeats {identifier!r}")
        result[identifier] = item
    if list(result) != sorted(result):
        raise SystemExit(f"{context} must be sorted by {key}")
    return result


def source_digest(source_files: Any, context: str) -> str:
    if not isinstance(source_files, list) or source_files != sorted(set(source_files)):
        raise SystemExit(f"{context} sourceFiles must be unique and sorted")
    digest = hashlib.sha256()
    for value in source_files:
        path = repo_path(value, f"{context} source file")
        digest.update(value.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
    return f"sha256:{digest.hexdigest()}"


def validate_review(value: Any, context: str) -> None:
    review = fields(
        value,
        {"status", "reviewers", "agreement", "adjudication", "rationale"},
        {"status", "reviewers", "agreement", "adjudication", "rationale"},
        context,
    )
    reviewers = index(review["reviewers"], "id", f"{context} reviewers")
    roles: set[str] = set()
    for reviewer_id, reviewer in reviewers.items():
        fields(
            reviewer,
            {"id", "role", "qualification", "independentFromAnnotationAuthor"},
            {"id", "role", "qualification", "independentFromAnnotationAuthor"},
            f"{context} reviewer {reviewer_id!r}",
        )
        role = reviewer["role"]
        roles.add(role)
        independent = reviewer["independentFromAnnotationAuthor"]
        if role == "annotationAuthor" and independent is not False:
            raise SystemExit(f"{context} author cannot claim independence")
        if role in {"independentReviewer", "adjudicator"} and independent is not True:
            raise SystemExit(f"{context} independent role must declare independence")
    if "annotationAuthor" not in roles:
        raise SystemExit(f"{context} must identify an annotation author")
    status = review["status"]
    if status == "maintainerOnly":
        if roles != {"annotationAuthor"}:
            raise SystemExit(f"{context} maintainerOnly review has independent roles")
        if review["agreement"] != "notMeasured" or review["adjudication"] != "notPerformed":
            raise SystemExit(f"{context} maintainerOnly review overclaims agreement")
    elif status == "independentlyReviewed":
        if "independentReviewer" not in roles or review["agreement"] == "notMeasured":
            raise SystemExit(f"{context} independent review is incomplete")
    elif status == "adjudicated":
        if not {"independentReviewer", "adjudicator"}.issubset(roles):
            raise SystemExit(f"{context} adjudication lacks independent roles")
        if review["adjudication"] != "completed":
            raise SystemExit(f"{context} adjudication is incomplete")
    else:
        raise SystemExit(f"{context} has unsupported status {status!r}")


def validate_family(family: dict[str, Any]) -> None:
    family_id = family["id"]
    fields(
        family,
        {
            "id", "version", "name", "productContext", "reviewedTasks", "sourceRoot",
            "sourceFiles", "sourceDigest", "sourceRevisionBasis", "governance", "exposure",
            "review", "samplingLimitations",
        },
        {
            "id", "version", "name", "productContext", "reviewedTasks", "sourceRoot",
            "sourceFiles", "sourceDigest", "sourceRevisionBasis", "governance", "exposure",
            "review", "samplingLimitations",
        },
        f"family {family_id!r}",
    )
    repo_path(family["sourceRoot"], f"family {family_id!r} sourceRoot", directory=True)
    observed_digest = source_digest(family["sourceFiles"], f"family {family_id!r}")
    if family["sourceDigest"] != observed_digest:
        raise SystemExit(
            f"family {family_id!r} source drift: expected {observed_digest}"
        )
    governance = fields(
        family["governance"],
        {
            "kind", "ownership", "license", "redistribution", "privacyReview",
            "personalOrCustomerData", "externalAssets", "externalNetwork", "externalProcessing",
        },
        {
            "kind", "ownership", "license", "redistribution", "privacyReview",
            "personalOrCustomerData", "externalAssets", "externalNetwork", "externalProcessing",
        },
        f"family {family_id!r} governance",
    )
    expected = {
        "kind": "repositoryOwnedFixture",
        "ownership": "sightlintRepository",
        "license": "MIT OR Apache-2.0",
        "privacyReview": "syntheticNoPersonalData",
        "personalOrCustomerData": False,
        "externalAssets": False,
        "externalNetwork": False,
        "externalProcessing": False,
    }
    if any(governance[name] != value for name, value in expected.items()):
        raise SystemExit(f"family {family_id!r} violates data governance")
    exposure = fields(
        family["exposure"],
        {"classification", "tuningVisible", "implementationOutputUsedAsOracle"},
        {"classification", "tuningVisible", "implementationOutputUsedAsOracle"},
        f"family {family_id!r} exposure",
    )
    if exposure != {
        "classification": "publicDevelopmentData",
        "tuningVisible": True,
        "implementationOutputUsedAsOracle": False,
    }:
        raise SystemExit(f"family {family_id!r} is not honest public development data")
    validate_review(family["review"], f"family {family_id!r} review")


def validate_dataset(
    dataset: dict[str, Any], families: dict[str, dict[str, Any]]
) -> int:
    dataset_id = dataset["id"]
    fields(
        dataset,
        {"id", "version", "familyId", "commandSurface", "acquisitionOracle", "ruleOracle", "cases"},
        {"id", "version", "familyId", "commandSurface", "acquisitionOracle", "ruleOracle", "cases"},
        f"dataset {dataset_id!r}",
    )
    if dataset["familyId"] not in families:
        raise SystemExit(f"dataset {dataset_id!r} references an unknown family")
    documents: dict[str, dict[str, Any]] = {}
    for authority, expected_type in (
        ("acquisitionOracle", "browserAcquisitionOracle"),
        ("ruleOracle", "browserRuleOracle"),
    ):
        reference = fields(
            dataset[authority],
            {"document", "schema", "documentType"},
            {"document", "schema", "documentType"},
            f"dataset {dataset_id!r} {authority}",
        )
        if reference["documentType"] != expected_type:
            raise SystemExit(f"dataset {dataset_id!r} reverses oracle authority")
        repo_path(reference["schema"], f"dataset {dataset_id!r} {authority} schema")
        document = load(repo_path(reference["document"], f"dataset {dataset_id!r} {authority}"))
        if document.get("documentType") != expected_type:
            raise SystemExit(f"dataset {dataset_id!r} {authority} has the wrong type")
        documents[authority] = document
    provenance = fields(
        documents["acquisitionOracle"].get("provenance"),
        {
            "authoringBasis", "implementationOutputUsedAsOracle", "sourceId", "ownership",
            "license", "privacyReview", "externalAssets", "externalProcessing", "holdoutStatus",
        },
        {
            "authoringBasis", "implementationOutputUsedAsOracle", "sourceId", "ownership",
            "license", "privacyReview", "externalAssets", "externalProcessing", "holdoutStatus",
        },
        f"dataset {dataset_id!r} acquisition provenance",
    )
    if (
        provenance["authoringBasis"] != "humanReviewedFixtureContract"
        or provenance["implementationOutputUsedAsOracle"] is not False
        or provenance["holdoutStatus"] != "publicDevelopmentData"
    ):
        raise SystemExit(f"dataset {dataset_id!r} has invalid oracle provenance")
    inventory = index(dataset["cases"], "caseId", f"dataset {dataset_id!r} cases")
    acquisition = index(
        documents["acquisitionOracle"].get("cases"),
        "caseId",
        f"dataset {dataset_id!r} acquisition cases",
    )
    rules = index(
        documents["ruleOracle"].get("cases"),
        "caseId",
        f"dataset {dataset_id!r} rule cases",
    )
    if set(inventory) != set(acquisition) or set(inventory) != set(rules):
        raise SystemExit(f"dataset {dataset_id!r} oracle inventories disagree")
    for case_id, registered in inventory.items():
        fields(
            registered,
            {"caseId", "request", "split", "classification"},
            {"caseId", "request", "split", "classification"},
            f"dataset {dataset_id!r} case {case_id!r}",
        )
        repo_path(registered["request"], f"dataset {dataset_id!r} case request")
        for observed in (acquisition[case_id], rules[case_id]):
            for name in ("request", "classification"):
                if observed.get(name) != registered[name]:
                    raise SystemExit(f"dataset {dataset_id!r} {case_id!r} disagrees on {name}")
        if acquisition[case_id].get("split") != registered["split"]:
            raise SystemExit(f"dataset {dataset_id!r} {case_id!r} disagrees on split")
    if dataset_id == "harbor-support-browser-v1":
        categories = {case["classification"] for case in inventory.values()}
        if len(inventory) != 4 or categories != {
            "clean", "targetedMutation", "hardNegative", "ambiguous"
        }:
            raise SystemExit("support-inbox dataset must keep its four focused case classes")
    return len(inventory)


def validate_holdout(registry: dict[str, Any]) -> str:
    reference = fields(
        registry["holdoutAdmission"],
        {"version", "path", "status"},
        {"version", "path", "status"},
        "holdout reference",
    )
    if repo_path(reference["path"], "holdout document") != HOLDOUT:
        raise SystemExit("registry must reference the canonical holdout admission document")
    holdout = load(HOLDOUT)
    fields(
        holdout,
        {
            "$schema", "schemaVersion", "status", "publicRepositoryRole",
            "implementationOutputUsedAsOracle", "publicCasesEligible", "requiredControls",
            "forbiddenClaims", "blockers", "operationalRecord",
        },
        {
            "$schema", "schemaVersion", "status", "publicRepositoryRole",
            "implementationOutputUsedAsOracle", "publicCasesEligible", "requiredControls",
            "forbiddenClaims",
        },
        "holdout admission",
    )
    if reference["version"] != VERSION or holdout["schemaVersion"] != VERSION:
        raise SystemExit("holdout admission uses an unsupported version")
    if reference["status"] != holdout["status"]:
        raise SystemExit("registry and holdout status disagree")
    if (
        holdout["publicRepositoryRole"] != "admissionMetadataOnly"
        or holdout["implementationOutputUsedAsOracle"] is not False
        or holdout["publicCasesEligible"] is not False
    ):
        raise SystemExit("holdout admission weakens the public-data boundary")
    controls = holdout["requiredControls"]
    expected_controls = {
        "freeze", "access", "evaluator", "leakage", "execution", "oracleCorrection", "reporting"
    }
    if not isinstance(controls, dict) or set(controls) != expected_controls:
        raise SystemExit("holdout admission is missing a required control")
    if holdout["status"] == "notOperational":
        if not holdout.get("blockers") or "operationalRecord" in holdout:
            raise SystemExit("non-operational holdout must expose blockers and no result record")
    elif holdout["status"] == "operational":
        if "blockers" in holdout or not isinstance(holdout.get("operationalRecord"), dict):
            raise SystemExit("operational holdout requires a complete admission record")
    else:
        raise SystemExit("holdout admission has an unsupported status")
    return holdout["status"]


def validate() -> tuple[int, int, int, str]:
    registry = load(REGISTRY)
    fields(
        registry,
        {
            "$schema", "schemaVersion", "registry", "annotationGuide", "families", "datasets",
            "splitPolicy", "holdoutAdmission", "metricContract", "nonClaims",
        },
        {
            "$schema", "schemaVersion", "registry", "annotationGuide", "families", "datasets",
            "splitPolicy", "holdoutAdmission", "metricContract", "nonClaims",
        },
        "Web evaluation v1 registry",
    )
    if registry["schemaVersion"] != VERSION:
        raise SystemExit(f"Web evaluation registry must use version {VERSION}")
    repo_path(registry["annotationGuide"]["path"], "annotation guide")
    families = index(registry["families"], "id", "fixture families")
    if len(families) < 2:
        raise SystemExit("Web evaluation v1 must contain at least two fixture families")
    for family in families.values():
        validate_family(family)
    datasets = index(registry["datasets"], "id", "evaluation datasets")
    cases = sum(validate_dataset(dataset, families) for dataset in datasets.values())
    holdout = validate_holdout(registry)
    metric_contract = registry["metricContract"]
    if not isinstance(metric_contract, dict) or metric_contract.get("universalScore") is not False:
        raise SystemExit("Web evaluation must not introduce a universal score")
    return len(families), len(datasets), cases, holdout


def main() -> None:
    families, datasets, cases, holdout = validate()
    print(
        "web evaluation v1 governance: "
        f"families={families}, datasets={datasets}, public_cases={cases}, "
        f"holdout={holdout}; public results remain nonrepresentative"
    )


if __name__ == "__main__":
    main()
