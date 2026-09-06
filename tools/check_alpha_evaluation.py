#!/usr/bin/env python3
"""Validate source-alpha evaluation governance without generating its oracles."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "evaluation" / "image-alpha"


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"{path.relative_to(ROOT)} must contain an object")
    return value


def exact_fields(value: Any, fields: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        actual = sorted(value) if isinstance(value, dict) else type(value).__name__
        raise SystemExit(f"{context} fields differ: {actual}")
    return value


def repository_file(value: str, context: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise SystemExit(f"{context} must be repository-contained")
    resolved = (ROOT / relative).resolve()
    if ROOT not in resolved.parents or not resolved.is_file():
        raise SystemExit(f"{context} is missing: {value}")
    return resolved


def indexed_cases(document: dict[str, Any], context: str) -> dict[str, dict[str, Any]]:
    cases = document.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"{context} needs cases")
    result: dict[str, dict[str, Any]] = {}
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("caseId"), str):
            raise SystemExit(f"{context} has invalid case")
        if case["caseId"] in result:
            raise SystemExit(f"{context} repeats {case['caseId']}")
        result[case["caseId"]] = case
    if list(result) != sorted(result):
        raise SystemExit(f"{context} cases must be sorted")
    return result


def bounds(value: Any, canvas: tuple[int, int], context: str) -> list[int] | None:
    if value is None:
        return None
    if (
        not isinstance(value, list)
        or len(value) != 4
        or not all(isinstance(item, int) for item in value)
    ):
        raise SystemExit(f"{context} must be null or integer xywh")
    x, y, width, height = value
    if min(x, y) < 0 or min(width, height) < 1:
        raise SystemExit(f"{context} has invalid dimensions")
    if x + width > canvas[0] or y + height > canvas[1]:
        raise SystemExit(f"{context} exceeds canvas")
    return value


def validate_acquisition(case: dict[str, Any]) -> None:
    case_id = case["caseId"]
    exact_fields(
        case,
        {"caseId", "canvas", "expectedStatus", "alpha", "annotationBasis", "abstentions"},
        f"acquisition {case_id}",
    )
    canvas_record = exact_fields(case["canvas"], {"width", "height", "unit"}, f"canvas {case_id}")
    canvas = (canvas_record["width"], canvas_record["height"])
    if not all(isinstance(item, int) and item > 0 for item in canvas):
        raise SystemExit(f"{case_id} canvas dimensions are invalid")
    if canvas_record["unit"] != "devicePixel" or case["expectedStatus"] != "available":
        raise SystemExit(f"{case_id} uses an unsupported acquisition contract")
    alpha = exact_fields(
        case["alpha"],
        {
            "visibleBounds", "opaqueBounds", "transparentInsets", "pixelCounts",
            "visibleEdgePixels", "entirelyTransparent", "allPixelsVisible",
            "expectedInkBox", "evidenceId", "unit", "boundsFormat",
        },
        f"alpha {case_id}",
    )
    visible = bounds(alpha["visibleBounds"], canvas, f"{case_id} visible bounds")
    bounds(alpha["opaqueBounds"], canvas, f"{case_id} opaque bounds")
    ink = bounds(alpha["expectedInkBox"], canvas, f"{case_id} ink box")
    if ink != visible:
        raise SystemExit(f"{case_id} ink box must equal visible bounds")
    counts = exact_fields(
        alpha["pixelCounts"],
        {"total", "visible", "opaque", "translucent", "transparent"},
        f"counts {case_id}",
    )
    if counts["total"] != canvas[0] * canvas[1]:
        raise SystemExit(f"{case_id} total count differs from canvas")
    if counts["visible"] != counts["opaque"] + counts["translucent"]:
        raise SystemExit(f"{case_id} visible count does not partition")
    if counts["total"] != counts["visible"] + counts["transparent"]:
        raise SystemExit(f"{case_id} total count does not partition")
    if alpha["entirelyTransparent"] != (counts["visible"] == 0):
        raise SystemExit(f"{case_id} entirelyTransparent disagrees")
    if alpha["allPixelsVisible"] != (counts["visible"] == counts["total"]):
        raise SystemExit(f"{case_id} allPixelsVisible disagrees")
    if visible is None:
        if alpha["transparentInsets"] is not None:
            raise SystemExit(f"{case_id} empty visibility must omit insets")
    else:
        x, y, width, height = visible
        expected = {
            "top": y,
            "right": canvas[0] - x - width,
            "bottom": canvas[1] - y - height,
            "left": x,
        }
        if alpha["transparentInsets"] != expected:
            raise SystemExit(f"{case_id} insets disagree with visible bounds")
    edges = exact_fields(
        alpha["visibleEdgePixels"], {"top", "right", "bottom", "left"}, f"edges {case_id}"
    )
    denominators = {"top": canvas[0], "right": canvas[1], "bottom": canvas[0], "left": canvas[1]}
    for name, denominator in denominators.items():
        edge = exact_fields(edges[name], {"count", "denominator"}, f"{case_id} {name} edge")
        if edge["denominator"] != denominator or not 0 <= edge["count"] <= denominator:
            raise SystemExit(f"{case_id} {name} edge is invalid")
    if alpha["evidenceId"] != "evidence:png-alpha":
        raise SystemExit(f"{case_id} uses unexpected evidence")
    if alpha["unit"] != "devicePixel" or alpha["boundsFormat"] != "xywh-half-open":
        raise SystemExit(f"{case_id} geometry contract differs")
    if not isinstance(case["abstentions"], list) or not case["abstentions"]:
        raise SystemExit(f"{case_id} must preserve abstention")


def main() -> None:
    corpus = load(DIRECTORY / "corpus.json")
    acquisition = load(DIRECTORY / "annotations" / "acquisition.json")
    rules = load(DIRECTORY / "annotations" / "rules.json")
    if corpus.get("schemaVersion") != "0.1.0":
        raise SystemExit("unsupported image-alpha corpus version")
    governance = corpus.get("dataGovernance", {})
    if governance.get("implementationOutputIsOracle") is not False:
        raise SystemExit("implementation output must not be an oracle")
    source = corpus.get("source", {})
    if (
        source.get("ownership") != "sightlintRepository"
        or source.get("license") != "MIT OR Apache-2.0"
        or source.get("privacyReview") != "syntheticNoPersonalData"
        or source.get("externalAssets") is not False
    ):
        raise SystemExit("source governance differs")
    repository_file(source["generator"], "asset generator")
    if corpus.get("splitPolicy", {}).get("holdout", {}).get("status") != "notEstablished":
        raise SystemExit("public corpus must not claim a holdout")

    cases = corpus.get("cases")
    if not isinstance(cases, list) or len(cases) != corpus.get("gates", {}).get("requiredAcquisitionCases"):
        raise SystemExit("required acquisition case count differs")
    case_ids = [case.get("id") for case in cases]
    if case_ids != sorted(case_ids) or len(set(case_ids)) != len(case_ids):
        raise SystemExit("corpus case IDs must be unique and sorted")
    indexed = {case["id"]: case for case in cases}
    for case in cases:
        path = repository_file(case["path"], f"asset {case['id']}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != case["byteSha256"]:
            raise SystemExit(f"asset digest differs for {case['id']}")
        if case["sourceId"] != source["id"]:
            raise SystemExit(f"source ID differs for {case['id']}")
        relation = case.get("relation")
        if relation is not None and relation["baselineCaseId"] not in indexed:
            raise SystemExit(f"{case['id']} has missing baseline")
        if case["classification"] == "targetedMutation" and relation is None:
            raise SystemExit(f"{case['id']} mutation lacks relation")
        if case["classification"] == "hardNegative" and "hardNegative" not in case:
            raise SystemExit(f"{case['id']} hard negative lacks rationale")

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
            raise SystemExit(f"rule oracle {case_id} exceeds this slice")
        expected_applicability = "inapplicable" if case_id == "northstar-invisible-placeholder" else "cantTell"
        if rule.get("applicabilityGroundTruth") != expected_applicability:
            raise SystemExit(f"rule applicability differs for {case_id}")
    print("source-alpha evaluation: 5 cases, 1 mutation, 2 hard negatives; rule truth remains untested")


if __name__ == "__main__":
    main()
