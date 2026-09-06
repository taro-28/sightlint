#!/usr/bin/env python3
"""Check the static PDF evaluation corpus and dependency governance."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EVALUATION = ROOT / "evaluation" / "pdf"
DIGEST_PREFIX = "sha256:"


def load_json(path: Path) -> Any:
    """Load UTF-8 JSON while rejecting duplicate object keys."""
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
        raise SystemExit(f"PDF evaluation error: {error}") from error


def require(condition: bool, message: str) -> None:
    """Fail the static contract with one stable prefix."""
    if not condition:
        raise SystemExit(f"PDF evaluation error: {message}")


def repository_file(reference: str) -> Path:
    """Resolve a strict repository-relative regular-file reference."""
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
        raise SystemExit(
            f"PDF evaluation error: path escapes repository: {reference}"
        ) from error
    require(path.is_file(), f"not a regular file: {reference}")
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
    """Index records while rejecting missing and duplicate stable IDs."""
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


def verify_dependency() -> None:
    """Verify the reviewed pypdf lock and hash-pinned requirement agree."""
    lock = load_json(ROOT / "adapters" / "pdf" / "dependency-lock.json")
    require(lock.get("lockVersion") == "0.1.0", "dependency lock version drift")
    packages = lock.get("packages", [])
    require(len(packages) == 1, "PDF adapter must have exactly one package")
    package = packages[0]
    require(package.get("name") == "pypdf", "unexpected PDF dependency")
    require(package.get("version") == "6.17.0", "pypdf version drift")
    require(package.get("license") == "BSD-3-Clause", "pypdf license drift")
    require(package.get("requiresPython") == ">=3.9", "pypdf Python range drift")
    digest = package.get("sha256")
    requirement = (ROOT / "adapters" / "pdf" / "requirements.txt").read_text(
        encoding="utf-8"
    )
    require("--only-binary=:all:" in requirement, "PDF requirement permits source builds")
    require(
        f"pypdf=={package['version']} --hash={digest}" in requirement,
        "PDF requirement and dependency lock differ",
    )


def verify_annotation(
    case_id: str, item: dict[str, Any], page: dict[str, Any]
) -> None:
    """Verify one independently authored source/normalized geometry record."""
    require(
        item.get("sourceEvidenceClass") == "exactSource",
        f"{case_id} annotation evidence is not exactSource",
    )
    require(item.get("subtype") == "/Link", f"{case_id} annotation subtype drift")
    require(
        item.get("actionKind") == "internalDestination",
        f"{case_id} annotation action drift",
    )
    source = item.get("sourceRectPdfPoints", {})
    require(
        source.get("right", 0) > source.get("left", 0)
        and source.get("top", 0) > source.get("bottom", 0),
        f"{case_id} annotation has invalid source Rect",
    )
    normalized = item.get("normalizedHitBoxPdfPoints")
    if item.get("geometryStatus") == "exact":
        crop = page["cropBoxPdfPoints"]
        expected = {
            "x": source["left"] - crop["left"],
            "y": crop["top"] - source["top"],
            "width": source["right"] - source["left"],
            "height": source["top"] - source["bottom"],
        }
        require(normalized == expected, f"{case_id} normalized hit transform drift")
        require(
            not item.get("hasQuadPoints") and not item.get("hasPath"),
            f"{case_id} exact hit geometry has a non-rectangular override",
        )
    else:
        require(
            item.get("geometryStatus") == "unsupportedQuadPoints"
            and item.get("hasQuadPoints") is True,
            f"{case_id} unsupported annotation is not the reviewed QuadPoints case",
        )
        require(
            normalized is None,
            f"{case_id} QuadPoints annotation was promoted to an exact hit box",
        )


def main() -> None:
    """Validate all static PDF data, provenance, and semantic relations."""
    verify_dependency()
    for schema in sorted(
        list((ROOT / "adapters" / "pdf").rglob("*.schema.json"))
        + list(EVALUATION.glob("*.schema.json"))
    ):
        require(isinstance(load_json(schema), dict), f"invalid schema {schema}")

    corpus = load_json(EVALUATION / "corpus.json")
    acquisitions = load_json(EVALUATION / "annotations" / "acquisition.json")
    rules = load_json(EVALUATION / "annotations" / "rules.json")
    metrics = load_json(EVALUATION / "metric-contract.json")

    require(
        corpus.get("holdout", {}).get("status") == "notEstablished",
        "holdout status must remain explicit",
    )
    source = corpus.get("source", {})
    require(
        source.get("privacy") == "fictionalNoPersonalOrCustomerData",
        "privacy provenance is missing",
    )
    require(source.get("license") == "MIT OR Apache-2.0", "fixture license is missing")
    require(
        acquisitions.get("provenance", {}).get("implementationOutputUsed") is False,
        "acquisition oracle used implementation output",
    )
    require(
        rules.get("provenance", {}).get("implementationOutputUsed") is False,
        "rule oracle used implementation output",
    )
    require(
        metrics.get("implementationOutputsStoredAsOracle") is False,
        "metric contract stores implementation output as oracle",
    )

    cases = indexed(corpus.get("cases", []), "id", "corpus cases")
    acquisition_by_id = indexed(
        acquisitions.get("annotations", []), "id", "acquisition annotations"
    )
    rules_by_id = indexed(rules.get("annotations", []), "id", "rule annotations")
    require(len(cases) == 3, "version 0.1.0 must contain exactly three cases")
    require(
        {case.get("split") for case in cases.values()}
        == {"smoke", "development", "challenge"},
        "smoke/development/challenge split is incomplete",
    )
    require(
        {
            case_id: case.get("split")
            for case_id, case in cases.items()
        }
        == {
            "pdf-atlas-clean": "smoke",
            "pdf-atlas-off-page-mutant": "development",
            "pdf-atlas-quadpoints-hard-negative": "challenge",
        },
        "case split roles differ from the reviewed contract",
    )

    exact_nodes = 0
    rule_targets = 0
    for case_id, case in cases.items():
        for artifact_kind in ("request", "input", "render"):
            artifact = case.get(artifact_kind, {})
            path = repository_file(artifact.get("path", ""))
            require(
                sha256(path) == artifact.get("sha256"),
                f"{case_id} {artifact_kind} digest drift",
            )

        acquisition = acquisition_by_id.get(case.get("acquisitionAnnotationId"))
        rule = rules_by_id.get(case.get("ruleAnnotationId"))
        require(
            acquisition is not None and acquisition.get("caseId") == case_id,
            f"{case_id} acquisition annotation mismatch",
        )
        require(
            rule is not None and rule.get("caseId") == case_id,
            f"{case_id} rule annotation mismatch",
        )

        request = load_json(repository_file(case["request"]["path"]))
        require(request.get("requestId") == case_id, f"{case_id} requestId mismatch")
        require(
            request.get("input")
            == {
                "reference": case["input"]["path"],
                "sha256": case["input"]["sha256"],
            },
            f"{case_id} request input mismatch",
        )
        renders = request.get("renders", [])
        require(len(renders) == 1, f"{case_id} must have one synchronized render")
        require(
            renders[0].get("reference") == case["render"]["path"]
            and renders[0].get("sha256") == case["render"]["sha256"],
            f"{case_id} request render mismatch",
        )
        require(
            request.get("privacy")
            == {
                "externalProcessing": False,
                "retention": "none",
                "contentPolicy": "geometryAndTypeOnly",
            },
            f"{case_id} privacy contract mismatch",
        )

        page = acquisition.get("page", {})
        require(page.get("geometryStatus") == "exact", f"{case_id} page geometry drift")
        require(page.get("evidenceClass") == "exactSource", f"{case_id} page evidence drift")
        crop = page.get("cropBoxPdfPoints", {})
        render = acquisition.get("render", {})
        ratio = render.get("pdfPointsPerPixel", {})
        require(
            render.get("widthPixels") * ratio.get("numerator")
            == (crop.get("right") - crop.get("left")) * ratio.get("denominator"),
            f"{case_id} render width mapping mismatch",
        )
        require(
            render.get("heightPixels") * ratio.get("numerator")
            == (crop.get("top") - crop.get("bottom")) * ratio.get("denominator"),
            f"{case_id} render height mapping mismatch",
        )
        require(
            render.get("nodeIdentity") == "cantTell",
            f"{case_id} must abstain from rendered node identity",
        )
        require(
            acquisition.get("taggedStructure", {}).get("interpretation") == "untested",
            f"{case_id} tag interpretation must remain untested",
        )

        source_annotations = indexed(
            acquisition.get("annotations", []), "id", f"{case_id} annotations"
        )
        require(len(source_annotations) == 3, f"{case_id} must declare three annotations")
        for item in source_annotations.values():
            verify_annotation(case_id, item, page)
            if item.get("geometryStatus") == "exact":
                exact_nodes += 1

        expectations = rule.get("expectations", [])
        rule_targets += len(expectations)
        expected_keys = {
            (item.get("ruleId"), item.get("targetId"), item.get("aspect"))
            for item in expectations
        }
        require(
            len(expectations) == len(expected_keys),
            f"{case_id} rule targets are duplicated",
        )
        exact_ids = {
            identifier
            for identifier, item in source_annotations.items()
            if item.get("geometryStatus") == "exact"
        }
        require(
            {item.get("targetId") for item in expectations} == exact_ids,
            f"{case_id} rule targets differ from exact acquisition targets",
        )
        failures = [
            item for item in expectations if item.get("expectedOutcome") == "failed"
        ]
        role = rule.get("caseRole")
        require(
            (role == "targetedMutation") == (len(failures) == 1),
            f"{case_id} mutation failure contract mismatch",
        )
        if role != "targetedMutation":
            require(not failures, f"{case_id} clean/hard-negative contains a failure")

    require(
        cases["pdf-atlas-clean"]["render"]["sha256"]
        == cases["pdf-atlas-off-page-mutant"]["render"]["sha256"],
        "source-only mutation must preserve rendered bytes",
    )
    require(
        set(acquisition_by_id)
        == {case["acquisitionAnnotationId"] for case in cases.values()},
        "orphan acquisition annotation",
    )
    require(
        set(rules_by_id) == {case["ruleAnnotationId"] for case in cases.values()},
        "orphan rule annotation",
    )
    metric_items = metrics.get("metrics", [])
    require(len(metric_items) == 6, "metric set must contain exactly six entries")
    require(
        {item.get("id") for item in metric_items}
        == {
            "acquisitionFactCoverage",
            "evaluatedCaseCoverage",
            "verdictPrecision",
            "abstentionRetention",
            "falsePositiveRate",
            "mutationKillRate",
        },
        "metric set is incomplete",
    )
    print(
        f"PDF evaluation: 3 cases, {exact_nodes} exact link hit boxes, "
        f"{rule_targets} rule targets, provenance, dependency, and digests verified"
    )


if __name__ == "__main__":
    main()
