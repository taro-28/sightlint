#!/usr/bin/env python3
"""Check the static PPTX evaluation corpus without invoking the adapter."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EVALUATION = ROOT / "evaluation" / "pptx"
DIGEST_PREFIX = "sha256:"


def load_json(path: Path) -> Any:
    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"{path.relative_to(ROOT)} repeats JSON key {key!r}")
            value[key] = item
        return value

    try:
        return json.loads(path.read_bytes(), object_pairs_hook=no_duplicates)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise SystemExit(f"PPTX evaluation error: {error}") from error


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"PPTX evaluation error: {message}")


def repository_file(reference: str) -> Path:
    pure = PurePosixPath(reference)
    require(
        not pure.is_absolute()
        and "\\" not in reference
        and all(part not in {"", ".", ".."} for part in pure.parts),
        f"unsafe repository path {reference!r}",
    )
    path = (ROOT / Path(*pure.parts)).resolve(strict=True)
    try:
        path.relative_to(ROOT)
    except ValueError as error:
        raise SystemExit(f"PPTX evaluation error: path escapes repository: {reference}") from error
    require(path.is_file(), f"not a regular file: {reference}")
    return path


def sha256(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(65_536):
            hasher.update(chunk)
    return DIGEST_PREFIX + hasher.hexdigest()


def indexed(items: list[dict[str, Any]], field: str, context: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for item in items:
        identifier = item.get(field)
        require(isinstance(identifier, str) and identifier, f"{context} has an invalid {field}")
        require(identifier not in result, f"{context} repeats {field} {identifier!r}")
        result[identifier] = item
    return result


def main() -> None:
    corpus = load_json(EVALUATION / "corpus.json")
    acquisitions = load_json(EVALUATION / "annotations" / "acquisition.json")
    rules = load_json(EVALUATION / "annotations" / "rules.json")
    metrics = load_json(EVALUATION / "metric-contract.json")

    require(corpus.get("holdout", {}).get("status") == "notEstablished", "holdout status must remain explicit")
    require(corpus.get("source", {}).get("privacy") == "fictionalNoPersonalOrCustomerData", "privacy provenance is missing")
    require(corpus.get("source", {}).get("license") == "MIT OR Apache-2.0", "fixture license is missing")
    require(acquisitions.get("provenance", {}).get("implementationOutputUsed") is False, "acquisition oracle used implementation output")
    require(rules.get("provenance", {}).get("implementationOutputUsed") is False, "rule oracle used implementation output")
    require(metrics.get("implementationOutputsStoredAsOracle") is False, "metric contract stores implementation output as oracle")

    cases = indexed(corpus.get("cases", []), "id", "corpus cases")
    acquisition_by_id = indexed(acquisitions.get("annotations", []), "id", "acquisition annotations")
    rules_by_id = indexed(rules.get("annotations", []), "id", "rule annotations")
    require(len(cases) == 3, "version 0.1.0 must contain exactly three reviewed cases")
    require({case.get("split") for case in cases.values()} == {"smoke", "development", "challenge"}, "smoke/development/challenge split is incomplete")

    for case_id, case in cases.items():
        for artifact_kind in ("request", "input", "render"):
            artifact = case.get(artifact_kind, {})
            path = repository_file(artifact.get("path", ""))
            require(sha256(path) == artifact.get("sha256"), f"{case_id} {artifact_kind} digest drift")

        acquisition = acquisition_by_id.get(case.get("acquisitionAnnotationId"))
        rule = rules_by_id.get(case.get("ruleAnnotationId"))
        require(acquisition is not None and acquisition.get("caseId") == case_id, f"{case_id} acquisition annotation mismatch")
        require(rule is not None and rule.get("caseId") == case_id, f"{case_id} rule annotation mismatch")

        request = load_json(repository_file(case["request"]["path"]))
        require(request.get("requestId") == f"pptx-{case_id.removeprefix('atlas-')}", f"{case_id} requestId mismatch")
        require(request.get("input") == {"reference": case["input"]["path"], "sha256": case["input"]["sha256"]}, f"{case_id} request input mismatch")
        renders = request.get("renders", [])
        require(len(renders) == 1, f"{case_id} must have one synchronized render")
        require(renders[0].get("reference") == case["render"]["path"] and renders[0].get("sha256") == case["render"]["sha256"], f"{case_id} request render mismatch")
        require(request.get("privacy") == {"externalProcessing": False, "retention": "none", "textPolicy": "digestOnly"}, f"{case_id} privacy contract mismatch")

        nodes = indexed(acquisition.get("nodes", []), "id", f"{case_id} acquisition nodes")
        require(len(nodes) == 5, f"{case_id} must declare five native nodes")
        for node_id, node in nodes.items():
            require(node.get("sourceEvidenceClass") == "exactSource", f"{case_id} {node_id} evidence is not exactSource")
            require(("rectEmu" in node) == (node.get("geometryStatus") == "exact"), f"{case_id} {node_id} geometry/rect mismatch")
            parent = node.get("parentId")
            require(parent is None or parent in nodes, f"{case_id} {node_id} has dangling parent")
            text = node.get("text", {})
            if text.get("status") == "digestOnly":
                require(set(text) == {"status", "sha256", "utf8Bytes"}, f"{case_id} {node_id} text metadata mismatch")
            else:
                require(text == {"status": "absent"}, f"{case_id} {node_id} absent text leaks metadata")

        slide = acquisition.get("slide", {})
        render = acquisition.get("render", {})
        require(render.get("nodeIdentity") == "cantTell", f"{case_id} must abstain from rendered node identity")
        require(render.get("widthPixels") * render.get("emuPerPixel") == slide.get("widthEmu"), f"{case_id} render width mapping mismatch")
        require(render.get("heightPixels") * render.get("emuPerPixel") == slide.get("heightEmu"), f"{case_id} render height mapping mismatch")
        require(len(acquisition.get("abstentions", [])) >= 6, f"{case_id} acquisition abstentions are incomplete")

        expectations = rule.get("expectations", [])
        expected_keys = {(item.get("ruleId"), item.get("targetId"), item.get("aspect")) for item in expectations}
        require(len(expectations) == len(expected_keys) == 5, f"{case_id} rule targets must be unique and complete")
        require({item.get("targetId") for item in expectations} == set(nodes), f"{case_id} rule targets differ from acquisition targets")
        failures = [item for item in expectations if item.get("expectedOutcome") == "failed"]
        role = rule.get("caseRole")
        require((role == "targetedMutation") == (len(failures) == 1), f"{case_id} mutation failure contract mismatch")
        if role != "targetedMutation":
            require(not failures, f"{case_id} clean/hard-negative oracle contains a failure")

    require(set(acquisition_by_id) == {case["acquisitionAnnotationId"] for case in cases.values()}, "orphan acquisition annotation")
    require(set(rules_by_id) == {case["ruleAnnotationId"] for case in cases.values()}, "orphan rule annotation")
    require({item.get("id") for item in metrics.get("metrics", [])} == {"acquisitionFactCoverage", "evaluatedCaseCoverage", "verdictPrecision", "abstentionRetention", "falsePositiveRate", "mutationKillRate"}, "metric set is incomplete")
    print("PPTX evaluation: 3 cases, 15 node facts, 15 rule targets, provenance and digests verified")


if __name__ == "__main__":
    main()
