#!/usr/bin/env python3
"""Validate the reviewed Web evaluation corpus without generating its oracles."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "evaluation" / "web"
CORPUS = WEB / "corpus.json"
ACQUISITION = WEB / "annotations" / "acquisition.json"
RULES = WEB / "annotations" / "rules.json"
SCHEMA_VERSION = "0.1.0"


def load(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit(f"{path.relative_to(ROOT)} must contain a JSON object")
    return value


def exact_fields(
    value: Any, allowed: set[str], required: set[str], context: str
) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SystemExit(f"{context} must be a JSON object")
    unknown = set(value) - allowed
    missing = required - set(value)
    if unknown:
        raise SystemExit(f"{context} contains unsupported fields: {sorted(unknown)}")
    if missing:
        raise SystemExit(f"{context} is missing required fields: {sorted(missing)}")
    return value


def repository_path(value: str, context: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise SystemExit(f"{context} must be a repository-contained relative path")
    resolved = (ROOT / relative).resolve()
    if ROOT not in resolved.parents or not resolved.is_file():
        raise SystemExit(f"{context} does not resolve to a repository file: {value}")
    return resolved


def indexed_cases(document: dict[str, Any], context: str) -> dict[str, dict[str, Any]]:
    cases = document.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"{context} must contain a non-empty cases array")
    result: dict[str, dict[str, Any]] = {}
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("caseId"), str):
            raise SystemExit(f"{context} contains a case without a string caseId")
        identifier = case["caseId"]
        if identifier in result:
            raise SystemExit(f"{context} contains duplicate caseId {identifier!r}")
        result[identifier] = case
    if list(result) != sorted(result):
        raise SystemExit(f"{context} cases must be sorted by caseId")
    return result


def validate_review(value: Any, context: str) -> None:
    review = exact_fields(
        value,
        {"status", "qualification", "guideVersion", "reviewedAt", "rationale"},
        {"status", "qualification", "guideVersion", "reviewedAt", "rationale"},
        context,
    )
    if review["guideVersion"] != SCHEMA_VERSION:
        raise SystemExit(f"{context} uses an unsupported guide version")
    if review["status"] not in {
        "maintainerReviewed",
        "dualReviewed",
        "expertReviewed",
    }:
        raise SystemExit(f"{context} uses an unsupported review status")


def validate_acquisition_case(case: dict[str, Any]) -> set[str]:
    case_id = case["caseId"]
    exact_fields(
        case,
        {"caseId", "nodes", "relations", "unavailableAspects", "review"},
        {"caseId", "nodes", "relations", "unavailableAspects", "review"},
        f"acquisition case {case_id!r}",
    )
    node_ids: set[str] = set()
    parents: list[str] = []
    for node in case["nodes"]:
        exact_fields(
            node,
            {"id", "selector", "kind", "role", "parentId"},
            {"id", "selector", "kind", "role", "parentId"},
            f"acquisition node in {case_id!r}",
        )
        exact_fields(
            node["selector"],
            {"type", "value"},
            {"type", "value"},
            f"acquisition selector in {case_id!r}",
        )
        if node["id"] in node_ids:
            raise SystemExit(f"acquisition case {case_id!r} repeats node {node['id']!r}")
        node_ids.add(node["id"])
        if node["parentId"] is not None:
            parents.append(node["parentId"])
    dangling_parents = set(parents) - node_ids
    if dangling_parents:
        raise SystemExit(
            f"acquisition case {case_id!r} has dangling parents {sorted(dangling_parents)}"
        )

    relation_ids: set[str] = set()
    for relation in case["relations"]:
        exact_fields(
            relation,
            {"id", "type", "status", "nodeIds", "axis", "evidenceBasis", "alternatives"},
            {"id", "type", "status", "nodeIds", "axis", "evidenceBasis", "alternatives"},
            f"acquisition relation in {case_id!r}",
        )
        if relation["id"] in relation_ids:
            raise SystemExit(
                f"acquisition case {case_id!r} repeats relation {relation['id']!r}"
            )
        relation_ids.add(relation["id"])
        members = relation["nodeIds"]
        if len(members) < 2 or len(set(members)) != len(members):
            raise SystemExit(f"acquisition relation {relation['id']!r} has invalid members")
        dangling_nodes = set(members) - node_ids
        if dangling_nodes:
            raise SystemExit(
                f"acquisition relation {relation['id']!r} has dangling nodes {sorted(dangling_nodes)}"
            )

    for unavailable in case["unavailableAspects"]:
        exact_fields(
            unavailable,
            {"aspect", "status", "reason", "trackingIssue"},
            {"aspect", "status", "reason", "trackingIssue"},
            f"unavailable aspect in {case_id!r}",
        )
    validate_review(case["review"], f"acquisition review in {case_id!r}")
    return relation_ids


def validate_rule_case(case: dict[str, Any], acquisition_relations: set[str]) -> None:
    case_id = case["caseId"]
    exact_fields(
        case,
        {
            "caseId",
            "ruleId",
            "ruleVersion",
            "targetRelationId",
            "applicability",
            "policy",
            "expectedOutcome",
            "minimumEvidence",
            "validAlternatives",
            "severityInputs",
            "maturity",
            "blocking",
            "falsePositiveRisk",
            "review",
        },
        {
            "caseId",
            "ruleId",
            "ruleVersion",
            "targetRelationId",
            "applicability",
            "policy",
            "expectedOutcome",
            "minimumEvidence",
            "validAlternatives",
            "severityInputs",
            "maturity",
            "blocking",
            "falsePositiveRisk",
            "review",
        },
        f"rule case {case_id!r}",
    )
    applicability = exact_fields(
        case["applicability"],
        {"status", "rationale"},
        {"status", "rationale"},
        f"rule applicability in {case_id!r}",
    )
    if applicability["status"] not in {"applicable", "inapplicable", "cantTell"}:
        raise SystemExit(f"rule case {case_id!r} has invalid applicability")
    target = case["targetRelationId"]
    if target is not None and target not in acquisition_relations:
        raise SystemExit(f"rule case {case_id!r} references missing acquisition relation")
    if case["policy"] is not None:
        exact_fields(
            case["policy"],
            {"id", "source", "expectation", "value", "unit", "tolerance"},
            {"id", "source", "expectation", "value", "unit", "tolerance"},
            f"rule policy in {case_id!r}",
        )
    exact_fields(
        case["severityInputs"],
        {"userHarm", "affectedScope", "reversibility", "accessibilityImpact"},
        {"userHarm", "affectedScope", "reversibility", "accessibilityImpact"},
        f"severity inputs in {case_id!r}",
    )
    validate_review(case["review"], f"rule review in {case_id!r}")


def bundle_digest(source_files: list[str]) -> str:
    digest = hashlib.sha256()
    for value in source_files:
        path = repository_path(value, "fixture source file")
        digest.update(value.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
    return f"sha256:{digest.hexdigest()}"


def validate() -> tuple[int, int, int, int]:
    corpus = load(CORPUS)
    acquisition = load(ACQUISITION)
    rules = load(RULES)

    exact_fields(
        corpus,
        {
            "$schema",
            "schemaVersion",
            "corpus",
            "annotationGuide",
            "sources",
            "splitPolicy",
            "gates",
            "cases",
        },
        {
            "$schema",
            "schemaVersion",
            "corpus",
            "annotationGuide",
            "sources",
            "splitPolicy",
            "gates",
            "cases",
        },
        "Web evaluation corpus",
    )
    for document, document_type, context in [
        (acquisition, "acquisitionOracle", "acquisition annotations"),
        (rules, "ruleOracle", "rule annotations"),
    ]:
        exact_fields(
            document,
            {"$schema", "schemaVersion", "documentType", "cases"},
            {"$schema", "schemaVersion", "documentType", "cases"},
            context,
        )
        if document.get("documentType") != document_type:
            raise SystemExit(f"{context} has the wrong document type")

    for name, document in [
        ("corpus", corpus),
        ("acquisition annotations", acquisition),
        ("rule annotations", rules),
    ]:
        if document.get("schemaVersion") != SCHEMA_VERSION:
            raise SystemExit(f"{name} must use schema version {SCHEMA_VERSION}")

    acquisition_cases = indexed_cases(acquisition, "acquisition annotations")
    rule_cases = indexed_cases(rules, "rule annotations")
    acquisition_relations = {
        case_id: validate_acquisition_case(case)
        for case_id, case in acquisition_cases.items()
    }
    for case_id, case in rule_cases.items():
        validate_rule_case(case, acquisition_relations[case_id])
    cases = corpus.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit("corpus must contain a non-empty cases array")

    case_ids: list[str] = []
    source_ids: set[str] = set()
    for source in corpus.get("sources", []):
        exact_fields(
            source,
            {
                "id",
                "kind",
                "origin",
                "ownership",
                "license",
                "redistribution",
                "privacyReview",
                "externalAssets",
            },
            {
                "id",
                "kind",
                "origin",
                "ownership",
                "license",
                "redistribution",
                "privacyReview",
                "externalAssets",
            },
            "Web evaluation source",
        )
        if source["id"] in source_ids:
            raise SystemExit(f"duplicate Web evaluation source {source['id']!r}")
        source_ids.add(source["id"])
    runnable = 0
    smoke = 0
    mutations = 0
    hard_negatives = 0
    abstentions = 0

    for case in cases:
        identifier = case.get("id")
        if not isinstance(identifier, str) or not identifier:
            raise SystemExit("corpus case IDs must be non-empty strings")
        case_ids.append(identifier)
        exact_fields(
            case,
            {
                "id",
                "version",
                "split",
                "medium",
                "sourceId",
                "fixture",
                "environment",
                "capture",
                "acquisitionOracle",
                "ruleOracle",
                "execution",
                "mutation",
                "hardNegative",
            },
            {
                "id",
                "version",
                "split",
                "medium",
                "sourceId",
                "fixture",
                "environment",
                "capture",
                "acquisitionOracle",
                "ruleOracle",
                "execution",
            },
            f"Web evaluation case {identifier!r}",
        )
        if case.get("sourceId") not in source_ids:
            raise SystemExit(f"case {identifier!r} references an unknown source")
        if identifier not in acquisition_cases or identifier not in rule_cases:
            raise SystemExit(f"case {identifier!r} is missing one of its independent oracles")

        fixture = case.get("fixture", {})
        exact_fields(
            fixture,
            {"entrypoint", "state", "sourceFiles", "sourceDigest"},
            {"entrypoint", "state", "sourceFiles", "sourceDigest"},
            f"fixture in case {identifier!r}",
        )
        source_files = fixture.get("sourceFiles")
        if not isinstance(source_files, list) or source_files != sorted(set(source_files)):
            raise SystemExit(f"case {identifier!r} sourceFiles must be unique and sorted")
        repository_path(fixture.get("entrypoint", ""), f"case {identifier!r} entrypoint")
        expected_digest = bundle_digest(source_files)
        if fixture.get("sourceDigest") != expected_digest:
            raise SystemExit(
                f"case {identifier!r} fixture source drift: expected {expected_digest}"
            )

        capture = case.get("capture", {})
        exact_fields(
            capture,
            {"status", "reason", "trackingIssue", "externalProcessing"},
            {"status", "reason", "trackingIssue", "externalProcessing"},
            f"capture in case {identifier!r}",
        )
        if (
            capture.get("status") != "untested"
            or capture.get("trackingIssue") != 23
            or capture.get("externalProcessing") is not False
        ):
            raise SystemExit(
                f"case {identifier!r} must preserve untested local acquisition for issue 23"
            )

        for reference_name, expected_path in [
            ("acquisitionOracle", ACQUISITION),
            ("ruleOracle", RULES),
        ]:
            reference = case.get(reference_name, {})
            resolved = repository_path(
                reference.get("document", ""), f"case {identifier!r} {reference_name}"
            )
            if resolved != expected_path or reference.get("caseId") != identifier:
                raise SystemExit(f"case {identifier!r} has an inconsistent {reference_name}")

        unavailable = acquisition_cases[identifier].get("unavailableAspects", [])
        if not unavailable or any(item.get("status") != "untested" for item in unavailable):
            raise SystemExit(f"case {identifier!r} must expose unavailable acquisition aspects")

        expected_outcome = rule_cases[identifier].get("expectedOutcome")
        execution = case.get("execution", {})
        if execution.get("status") == "runnable":
            exact_fields(
                execution,
                {"status", "inputKind", "inputPath"},
                {"status", "inputKind", "inputPath"},
                f"execution in case {identifier!r}",
            )
            repository_path(execution.get("inputPath", ""), f"case {identifier!r} input")
            runnable += 1
        elif execution.get("status") == "untested":
            exact_fields(
                execution,
                {"status", "reason"},
                {"status", "reason"},
                f"execution in case {identifier!r}",
            )
            if expected_outcome not in {"cantTell", "untested"}:
                raise SystemExit(
                    f"case {identifier!r} is unexecuted without an abstaining rule oracle"
                )
            abstentions += 1
        else:
            raise SystemExit(f"case {identifier!r} has an unsupported execution status")

        if case.get("split") == "smoke":
            smoke += 1
            if execution.get("status") != "runnable":
                raise SystemExit(f"smoke case {identifier!r} must be runnable")
        if "mutation" in case:
            mutation = exact_fields(
                case["mutation"],
                {"baselineCaseId", "targetRuleId", "changedProperty", "preservedProperties"},
                {"baselineCaseId", "targetRuleId", "changedProperty", "preservedProperties"},
                f"mutation in case {identifier!r}",
            )
            if mutation["targetRuleId"] != rule_cases[identifier]["ruleId"]:
                raise SystemExit(f"mutation in case {identifier!r} targets the wrong rule")
            mutations += 1
        if "hardNegative" in case:
            exact_fields(
                case["hardNegative"],
                {"category", "rationale"},
                {"category", "rationale"},
                f"hard negative in case {identifier!r}",
            )
            hard_negatives += 1

    if case_ids != sorted(set(case_ids)):
        raise SystemExit("corpus cases must be unique and sorted by ID")
    if set(case_ids) != set(acquisition_cases) or set(case_ids) != set(rule_cases):
        raise SystemExit("corpus and independent oracle case inventories must match exactly")
    if any(case.get("split") == "holdout" for case in cases):
        raise SystemExit("web corpus 0.1 must not imply that a holdout has been established")
    if (smoke, runnable, mutations, hard_negatives, abstentions) != (3, 3, 1, 1, 3):
        raise SystemExit(
            "web corpus 0.1 inventory changed; review its evidence claim before updating counts"
        )

    for case in cases:
        mutation = case.get("mutation")
        if mutation is not None and mutation["baselineCaseId"] not in case_ids:
            raise SystemExit(
                f"mutation {case['id']!r} references missing baseline {mutation['baselineCaseId']!r}"
            )

    return len(cases), smoke, mutations, hard_negatives


def main() -> None:
    total, smoke, mutations, hard_negatives = validate()
    print(
        "web evaluation: "
        f"{total} cases, {smoke} smoke, {mutations} mutation, "
        f"{hard_negatives} hard negative; acquisition remains untested"
    )


if __name__ == "__main__":
    main()
