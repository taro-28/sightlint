#!/usr/bin/env python3
"""Check Android evaluation governance, source truth, and cross-file relations."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
from typing import Any

import generate_android_fixtures


ROOT = Path(__file__).resolve().parents[1]
EVALUATION = ROOT / "evaluation" / "android"
DIGEST_PREFIX = "sha256:"


def fail(message: str) -> None:
    """Exit with one stable checker prefix."""
    raise SystemExit(f"Android evaluation error: {message}")


def require(condition: bool, message: str) -> None:
    """Require one evaluation invariant."""
    if not condition:
        fail(message)


def load_json(path: Path) -> Any:
    """Load UTF-8 JSON while rejecting duplicate object keys."""
    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate key {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(path.read_bytes(), object_pairs_hook=no_duplicates)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        fail(f"{path.relative_to(ROOT)}: {error}")


def repository_file(reference: str) -> Path:
    """Resolve one strict repository-relative regular-file reference."""
    pure = PurePosixPath(reference)
    require(
        not pure.is_absolute()
        and "\\" not in reference
        and all(part not in {"", ".", ".."} for part in pure.parts),
        f"unsafe repository path {reference!r}",
    )
    try:
        path = (ROOT / Path(*pure.parts)).resolve(strict=True)
    except OSError as error:
        fail(f"missing repository path {reference!r}: {error}")
    try:
        path.relative_to(ROOT)
    except ValueError:
        fail(f"repository path escapes root: {reference!r}")
    require(path.is_file(), f"not a regular file: {reference!r}")
    return path


def sha256(path: Path) -> str:
    """Return a prefixed streaming SHA-256 digest."""
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(65_536):
            hasher.update(chunk)
    return DIGEST_PREFIX + hasher.hexdigest()


def indexed(
    items: list[dict[str, Any]], field: str, context: str
) -> dict[str, dict[str, Any]]:
    """Index records while rejecting missing or duplicate identifiers."""
    result: dict[str, dict[str, Any]] = {}
    for item in items:
        identifier = item.get(field)
        require(
            isinstance(identifier, str) and identifier,
            f"{context} has an invalid {field}",
        )
        require(identifier not in result, f"{context} repeats {field} {identifier!r}")
        result[identifier] = item
    return result


def expected_mapping(node: dict[str, Any]) -> str:
    """Apply the independently specified ADR 0045 mapping predicate."""
    bounds = node["layoutBoundsDevicePixels"]
    if not node["viewState"]["shown"]:
        return "notMappedNotShown"
    if not node["globalVisible"]["value"]:
        return "notMappedNotGloballyVisible"
    if not node["identityTransform"]:
        return "notMappedUnsupportedTransform"
    if bounds["width"] == 0 or bounds["height"] == 0:
        return "notMappedEmptyLayout"
    return "mappedExactLayout"


def expected_kind(node: dict[str, Any]) -> str:
    """Apply the narrow deterministic core-kind mapping from ADR 0045."""
    if node["viewState"]["clickable"]:
        return "control"
    class_name = node["className"]
    if class_name.endswith("TextView"):
        return "text"
    if class_name.endswith(("Layout", "ScrollView")):
        return "container"
    return "other"


def verify_annotation_node(
    case_id: str,
    expected: dict[str, Any],
    actual: dict[str, Any],
) -> int:
    """Verify one reviewed acquisition node against the platform capture."""
    identifier = expected["resourceId"]
    require(expected["className"] == actual["className"],
            f"{case_id} {identifier} class drift")
    require(expected["layoutBoundsDevicePixels"] == actual["layoutBoundsDevicePixels"],
            f"{case_id} {identifier} layout drift")
    require(expected["globalVisible"] == actual["globalVisible"]["value"],
            f"{case_id} {identifier} global visibility drift")
    mapping = expected_mapping(actual)
    require(expected["mappingStatus"] == mapping,
            f"{case_id} {identifier} mapping annotation drift")
    core_kind = expected_kind(actual) if mapping == "mappedExactLayout" else None
    require(expected["coreKind"] == core_kind,
            f"{case_id} {identifier} core-kind annotation drift")

    accessibility = actual["accessibility"]
    require(expected["accessibilityGeometryStatus"] == accessibility["geometryStatus"],
            f"{case_id} {identifier} accessibility status drift")
    require(expected["accessibilityBoundsDevicePixels"] == accessibility["boundsDevicePixels"],
            f"{case_id} {identifier} accessibility bounds drift")
    if "rawAccessibilityBoundsDevicePixels" in expected:
        require(
            expected["rawAccessibilityBoundsDevicePixels"]
            == accessibility["rawBoundsDevicePixels"],
            f"{case_id} {identifier} raw accessibility bounds drift",
        )
    for field in ("text", "contentDescription"):
        if field in expected:
            require(expected[field] == actual[field], f"{case_id} {identifier} {field} drift")
    return 7 + sum(field in expected for field in ("text", "contentDescription"))


def verify_case(
    case_id: str,
    case: dict[str, Any],
    acquisition: dict[str, Any],
    rule: dict[str, Any],
) -> int:
    """Verify one case's assets, request, source oracle, and rule oracle."""
    artifacts: dict[str, Path] = {}
    for kind in ("capture", "screenshot", "request"):
        item = case.get(kind, {})
        path = repository_file(item.get("path", ""))
        require(sha256(path) == item.get("sha256"), f"{case_id} {kind} digest drift")
        artifacts[kind] = path

    request = load_json(artifacts["request"])
    require(request.get("requestId") == case_id, f"{case_id} request ID drift")
    require(request.get("artifact", {}).get("id") == case_id,
            f"{case_id} artifact ID drift")
    require(
        request.get("capture")
        == {"reference": case["capture"]["path"], "sha256": case["capture"]["sha256"]},
        f"{case_id} request capture binding drift",
    )
    require(
        request.get("screenshot")
        == {
            "reference": case["screenshot"]["path"],
            "sha256": case["screenshot"]["sha256"],
        },
        f"{case_id} request screenshot binding drift",
    )
    require(
        request.get("privacy")
        == {
            "contentPolicy": "digestsAndGeometry",
            "externalProcessing": False,
            "retention": "none",
        },
        f"{case_id} privacy request drift",
    )

    capture = load_json(artifacts["capture"])
    require(capture.get("captureId") == case_id, f"{case_id} capture ID drift")
    require(capture.get("screenshot", {}).get("reference") == case["screenshot"]["path"],
            f"{case_id} capture screenshot path drift")
    require(capture.get("screenshot", {}).get("sha256") == case["screenshot"]["sha256"],
            f"{case_id} capture screenshot digest drift")
    display = capture["device"]["display"]
    require(
        acquisition["display"]
        == {
            "widthPixels": display["widthPixels"],
            "heightPixels": display["heightPixels"],
            "densityDpi": display["densityDpi"],
            "rotationDegrees": display["rotationDegrees"],
        },
        f"{case_id} display annotation drift",
    )
    require(
        acquisition["screenshot"]
        == {
            "widthPixels": capture["screenshot"]["widthPixels"],
            "heightPixels": capture["screenshot"]["heightPixels"],
            "extentReconciliation": "agreement",
            "nodeIdentity": "cantTell",
        },
        f"{case_id} screenshot annotation drift",
    )
    nodes = indexed(capture["hierarchy"]["nodes"], "resourceId", f"{case_id} nodes")
    mapped = sum(expected_mapping(node) == "mappedExactLayout" for node in nodes.values())
    require(
        acquisition["counts"]
        == {
            "capturedNodes": len(nodes),
            "mappedCoreNodes": mapped,
            "unidentifiedNodes": capture["hierarchy"]["unidentifiedNodeCount"],
        },
        f"{case_id} count annotation drift",
    )
    facts = 8
    annotated_ids: set[str] = set()
    for expected in acquisition["nodes"]:
        identifier = expected["resourceId"]
        require(identifier not in annotated_ids, f"{case_id} repeats annotated node {identifier}")
        require(identifier in nodes, f"{case_id} annotated node is absent: {identifier}")
        annotated_ids.add(identifier)
        facts += verify_annotation_node(case_id, expected, nodes[identifier])

    require(rule["caseId"] == case_id, f"{case_id} rule annotation mismatch")
    require(rule["expectedResultCount"] == mapped, f"{case_id} rule result count drift")
    expected_failures = set(rule["expectedFailedTargets"])
    require(
        expected_failures
        == (
            {"android:view:org.sightlint.fixtures.atlas:id/save_button"}
            if case["relation"]["kind"] == "targetedMutation"
            else set()
        ),
        f"{case_id} expected failure contract drift",
    )
    expected_absent = set(rule["expectedAbsentTargets"])
    actual_absent = {
        "android:view:" + identifier
        for identifier, node in nodes.items()
        if expected_mapping(node) != "mappedExactLayout"
    }
    require(expected_absent == actual_absent, f"{case_id} absent-target contract drift")
    return facts


def verify_non_leakage() -> None:
    """Ensure selected fictional UI strings are not serialized in native captures."""
    sensitive = (
        b"Account settings",
        b"Alex Morgan",
        b"Workspace plan",
        b"Product notifications",
        b"Profile visibility",
        b"Save changes",
        b"Archived preferences",
    )
    for path in sorted((EVALUATION / "captures").glob("*.capture.json")):
        payload = path.read_bytes()
        for value in sensitive:
            require(value not in payload, f"{path.name} leaks full UI text {value!r}")


def main() -> None:
    """Validate all static Android evaluation contracts."""
    generate_android_fixtures.verify()
    for schema in sorted(
        list((ROOT / "adapters" / "android").rglob("*.schema.json"))
        + list(EVALUATION.glob("*.schema.json"))
    ):
        require(isinstance(load_json(schema), dict), f"invalid schema {schema.relative_to(ROOT)}")

    corpus = load_json(EVALUATION / "corpus.json")
    acquisitions = load_json(EVALUATION / "annotations" / "acquisition.json")
    rules = load_json(EVALUATION / "annotations" / "rules.json")
    metrics = load_json(EVALUATION / "metric-contract.json")
    require(corpus.get("fixtureSourceSha256") == generate_android_fixtures.source_digest(),
            "fixture source digest drift")
    require(corpus.get("holdout", {}).get("status") == "notEstablished",
            "holdout status must remain explicit")
    source = corpus.get("source", {})
    require(source.get("license") == "MIT OR Apache-2.0", "fixture license is missing")
    require(source.get("privacy") == "fictionalNoPersonalOrCustomerData",
            "fixture privacy provenance is missing")
    require(acquisitions.get("provenance", {}).get("implementationOutputUsed") is False,
            "acquisition oracle used implementation output")
    require(acquisitions.get("provenance", {}).get("captureIsAdapterOutput") is False,
            "platform capture was mislabeled as adapter output")
    require(rules.get("provenance", {}).get("implementationOutputUsed") is False,
            "rule oracle used implementation output")
    require(metrics.get("implementationOutputsStoredAsOracle") is False,
            "metric contract stores implementation output as oracle")

    cases = indexed(corpus.get("cases", []), "id", "corpus cases")
    acquisition_by_id = indexed(
        acquisitions.get("annotations", []), "id", "acquisition annotations"
    )
    rule_by_id = indexed(rules.get("annotations", []), "id", "rule annotations")
    require(set(cases) == {
        "android-atlas-clean",
        "android-atlas-off-canvas-control-mutant",
        "android-atlas-scroll-offscreen-hard-negative",
    }, "Android corpus case set drift")
    require(
        {identifier: case["split"] for identifier, case in cases.items()}
        == {
            "android-atlas-clean": "smoke",
            "android-atlas-off-canvas-control-mutant": "development",
            "android-atlas-scroll-offscreen-hard-negative": "challenge",
        },
        "Android corpus split drift",
    )

    acquisition_facts = 0
    for case_id, case in cases.items():
        acquisition = acquisition_by_id.get(case.get("acquisitionAnnotationId"))
        rule = rule_by_id.get(case.get("ruleAnnotationId"))
        require(acquisition is not None and acquisition["caseId"] == case_id,
                f"{case_id} acquisition annotation mismatch")
        require(rule is not None, f"{case_id} rule annotation is missing")
        acquisition_facts += verify_case(case_id, case, acquisition, rule)

    metric_ids = {metric.get("id") for metric in metrics.get("metrics", [])}
    require(metric_ids == {
        "acquisitionFactCoverage",
        "evaluatedCaseCoverage",
        "failurePrecision",
        "abstentionRetention",
        "falsePositiveRate",
        "mutationKillRate",
    }, "Android metric contract is incomplete")
    verify_non_leakage()
    print(
        "Android evaluation contract verified: "
        f"3 public cases, {acquisition_facts} reviewed acquisition facts, "
        "1 targeted mutation, 1 abstention hard negative, no holdout"
    )


if __name__ == "__main__":
    main()
