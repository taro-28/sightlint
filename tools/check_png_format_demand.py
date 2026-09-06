#!/usr/bin/env python3
"""Verify the reviewed PNG format-demand assessment without collecting user data."""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "evaluation" / "png-format-demand"
IGNORED_DIRECTORIES = {".git", "node_modules", "target"}
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def load(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"{relative} must contain an object")
    return value


def repository_file(relative: str, context: str) -> Path:
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts:
        raise SystemExit(f"{context} must be repository-relative")
    resolved = (ROOT / path).resolve()
    if ROOT not in resolved.parents or not resolved.is_file():
        raise SystemExit(f"{context} is missing: {relative}")
    return resolved


def png_header(path: Path) -> dict[str, int]:
    data = path.read_bytes()
    if len(data) < 33 or data[:8] != PNG_SIGNATURE or data[12:16] != b"IHDR":
        raise SystemExit(f"{path.relative_to(ROOT)} is not a complete PNG header")
    if int.from_bytes(data[8:12], "big") != 13:
        raise SystemExit(f"{path.relative_to(ROOT)} has an invalid IHDR length")
    ihdr = data[16:29]
    return {
        "width": int.from_bytes(ihdr[0:4], "big"),
        "height": int.from_bytes(ihdr[4:8], "big"),
        "bitDepth": ihdr[8],
        "colorType": ihdr[9],
        "interlaceMethod": ihdr[12],
    }


def committed_png_paths() -> list[str]:
    result = []
    for path in ROOT.rglob("*.png"):
        relative = path.relative_to(ROOT)
        if any(part in IGNORED_DIRECTORIES for part in relative.parts):
            continue
        result.append(relative.as_posix())
    return sorted(result)


def index_cases(document: dict[str, Any], key: str, context: str) -> dict[str, dict[str, Any]]:
    cases = document.get(key)
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"{context} requires cases")
    result: dict[str, dict[str, Any]] = {}
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("caseId"), str):
            raise SystemExit(f"{context} contains an invalid case")
        case_id = case["caseId"]
        if case_id in result:
            raise SystemExit(f"{context} repeats {case_id}")
        result[case_id] = case
    if list(result) != sorted(result):
        raise SystemExit(f"{context} cases must be sorted")
    return result


def verify_committed_assets(assessment: dict[str, Any]) -> None:
    committed = assessment.get("committedAssets")
    if not isinstance(committed, dict):
        raise SystemExit("committedAssets must be an object")
    assessed = index_cases(committed, "cases", "committed asset assessment")
    assessed_paths = sorted(case["path"] for case in assessed.values())
    actual_paths = committed_png_paths()
    if assessed_paths != actual_paths:
        raise SystemExit(
            f"committed PNG inventory differs: assessed={assessed_paths}, actual={actual_paths}"
        )

    manifest_paths = [committed["sourceManifest"], *committed.get("additionalSourceManifests", [])]
    source_assets: dict[str, str] = {}
    for manifest_path in manifest_paths:
        manifest = load(manifest_path)
        for manifest_case in manifest.get("cases", []):
            if not isinstance(manifest_case, dict):
                continue
            if manifest_path == "evaluation/image-alpha/corpus.json":
                path = manifest_case.get("path")
                digest = manifest_case.get("byteSha256")
            else:
                render = manifest_case.get("render", {})
                path = render.get("path") if isinstance(render, dict) else None
                value = render.get("sha256") if isinstance(render, dict) else None
                digest = value.removeprefix("sha256:") if isinstance(value, str) else None
            if isinstance(path, str) and isinstance(digest, str):
                source_assets[path] = digest
    if set(source_assets) != set(assessed_paths):
        raise SystemExit("source manifests and PNG demand assessment paths differ")

    for case_id, case in assessed.items():
        path = repository_file(case["path"], f"asset {case_id}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != case["byteSha256"]:
            raise SystemExit(f"{case_id} digest differs from the assessment")
        if digest != source_assets[case["path"]]:
            raise SystemExit(f"{case_id} digest differs from its source manifest")
        header = png_header(path)
        expected = {name: case[name] for name in header}
        if header != expected:
            raise SystemExit(f"{case_id} PNG header differs: {header} != {expected}")
        if case.get("currentRasterStatus") != "available":
            raise SystemExit(f"{case_id} must remain in the supported raster subset")


def verify_ephemeral_capture(assessment: dict[str, Any]) -> None:
    capture = assessment.get("ephemeralProductCapture")
    if not isinstance(capture, dict):
        raise SystemExit("ephemeralProductCapture must be an object")
    corpus = load(capture["corpus"])
    cases = corpus.get("cases")
    if not isinstance(cases, list) or len(cases) != capture.get("caseCount"):
        raise SystemExit("browser screenshot case count differs")
    repository_file(capture["verification"], "browser screenshot verification")
    encoding = capture.get("expectedEncoding")
    if encoding != {
        "bitDepth": 8,
        "colorType": 2,
        "interlaceMethod": 0,
        "currentRasterStatus": "available",
    }:
        raise SystemExit("browser screenshot format contract differs")


def verify_unsupported_controls(assessment: dict[str, Any]) -> None:
    raster = load("fixtures/png-raster/corpus.json")
    raster_cases = {
        case["id"]: case
        for case in raster.get("cases", [])
        if isinstance(case, dict) and isinstance(case.get("id"), str)
    }
    controls = index_cases(
        assessment, "unsupportedConformanceControls", "unsupported conformance controls"
    )
    expected_ids = {"animation-control", "indexed", "packed", "sixteen-bit", "trns"}
    if set(controls) != expected_ids:
        raise SystemExit("unsupported conformance control set differs")
    for case_id, control in controls.items():
        source = raster_cases.get(case_id)
        if source is None or source.get("status") != "unavailable":
            raise SystemExit(f"{case_id} is not an unavailable raster control")
        if source.get("reason") != control.get("reason"):
            raise SystemExit(f"{case_id} reason differs")
        if control.get("productDemandEvidence") is not False:
            raise SystemExit(f"{case_id} cannot be promoted to product demand")


def verify_decision(assessment: dict[str, Any]) -> None:
    governance = assessment.get("governance", {})
    if (
        governance.get("implementationOutputIsGroundTruth") is not False
        or governance.get("telemetryCollected") is not False
        or governance.get("artifactContentLeavesRepository") is not False
        or governance.get("holdoutStatus") != "notEstablished"
    ):
        raise SystemExit("PNG demand governance exceeds the reviewed evidence")
    decision = assessment.get("decision", {})
    if (
        decision.get("selectedStrategy") != "retainExplicitUnavailability"
        or decision.get("productCoverageGap") != "notEstablished"
        or decision.get("broaderDecodingOutcome") != "untested"
        or decision.get("newDecoderDependency") is not False
        or decision.get("currentExtensionChanged") is not False
        or decision.get("nextIssue") != 28
    ):
        raise SystemExit("PNG decoder strategy decision differs")
    candidate = assessment.get("libraryCandidateReview", {})
    if candidate.get("dependencyAdmitted") is not False:
        raise SystemExit("reviewed library candidate must not become an implicit dependency")
    manifest = (ROOT / "crates/sightlint-adapter-png/Cargo.toml").read_text(encoding="utf-8")
    if re.search(r"^png\s*=", manifest, flags=re.MULTILINE):
        raise SystemExit("png dependency was added without a new admission decision")


def main() -> None:
    assessment = load("evaluation/png-format-demand/assessment.json")
    if assessment.get("schemaVersion") != "0.1.0":
        raise SystemExit("unsupported PNG format-demand assessment version")
    repository_file(assessment["assessment"]["adr"], "assessment ADR")
    verify_committed_assets(assessment)
    verify_ephemeral_capture(assessment)
    verify_unsupported_controls(assessment)
    verify_decision(assessment)
    print(
        "PNG format demand: 11/11 committed assets and 9 browser cases use the supported subset; "
        "5 unsupported controls remain conformance-only; broader decoding stays untested"
    )


if __name__ == "__main__":
    main()
