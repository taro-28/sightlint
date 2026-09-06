#!/usr/bin/env python3
"""Validate GitHub Actions evaluation governance without generating its oracles."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
EVALUATION = ROOT / "evaluation" / "github-actions"
CORPUS = EVALUATION / "corpus.json"
RULES = EVALUATION / "annotations" / "rules.json"
PROJECTION = EVALUATION / "annotations" / "projection.json"
METRICS = EVALUATION / "metric-contract.json"
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


def repository_file(value: str, context: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise SystemExit(f"{context} must be a repository-contained relative path")
    path = (ROOT / relative).resolve()
    if ROOT not in path.parents or not path.is_file():
        raise SystemExit(f"{context} does not resolve to a repository file: {value}")
    return path


def indexed_cases(document: dict[str, Any], context: str) -> dict[str, dict[str, Any]]:
    cases = document.get("cases")
    if not isinstance(cases, list) or not cases:
        raise SystemExit(f"{context} must contain a non-empty cases array")
    result: dict[str, dict[str, Any]] = {}
    source_order: list[str] = []
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("caseId"), str):
            raise SystemExit(f"{context} contains a case without a string caseId")
        case_id = case["caseId"]
        source_order.append(case_id)
        if case_id in result:
            raise SystemExit(f"{context} contains duplicate caseId {case_id!r}")
        result[case_id] = case
    if source_order != sorted(source_order):
        raise SystemExit(f"{context} cases must be sorted by caseId")
    return result


def finding_tuple(value: dict[str, Any], *, flattened: bool) -> tuple[str, ...]:
    if flattened:
        exact_fields(
            value,
            {"ruleId", "ruleVersion", "targetKind", "targetId", "targetAspect"},
            {"ruleId", "ruleVersion", "targetKind", "targetId", "targetAspect"},
            "projection finding",
        )
        return (
            value["ruleId"],
            value["ruleVersion"],
            value["targetKind"],
            value["targetId"],
            value["targetAspect"] or "",
        )

    exact_fields(
        value,
        {"ruleId", "ruleVersion", "target"},
        {"ruleId", "ruleVersion", "target"},
        "source-map finding",
    )
    target = exact_fields(
        value["target"],
        {"kind", "id", "aspect"},
        {"kind", "id"},
        "source-map target",
    )
    return (
        value["ruleId"],
        value["ruleVersion"],
        target["kind"],
        target["id"],
        target.get("aspect") or "",
    )


def rule_expectation_tuple(value: dict[str, Any]) -> tuple[str, ...]:
    exact_fields(
        value,
        {"ruleId", "ruleVersion", "outcome", "enforcement", "target"},
        {"ruleId", "ruleVersion", "outcome", "enforcement", "target"},
        "rule expectation",
    )
    target = exact_fields(
        value["target"],
        {"kind", "id", "aspect"},
        {"kind", "id", "aspect"},
        "rule target",
    )
    return (
        value["ruleId"],
        value["ruleVersion"],
        target["kind"],
        target["id"],
        target["aspect"] or "",
    )


def validate_source_map(path_value: str, input_document: dict[str, Any]) -> dict[tuple[str, ...], dict[str, Any]]:
    path = repository_file(path_value, "source map")
    document = load(path)
    exact_fields(
        document,
        {"sourceMapSchemaVersion", "artifactId", "provenance", "entries"},
        {"sourceMapSchemaVersion", "artifactId", "provenance", "entries"},
        f"source map {path_value}",
    )
    if document["sourceMapSchemaVersion"] != SCHEMA_VERSION:
        raise SystemExit(f"source map {path_value} uses an unsupported version")
    artifact_id = input_document.get("artifact", {}).get("id")
    if document["artifactId"] != artifact_id:
        raise SystemExit(f"source map {path_value} does not match artifact {artifact_id!r}")
    provenance = exact_fields(
        document["provenance"],
        {"authoringBasis", "implementationOutputUsedAsOracle", "externalProcessing"},
        {"authoringBasis", "implementationOutputUsedAsOracle", "externalProcessing"},
        f"source map provenance {path_value}",
    )
    if provenance != {
        "authoringBasis": "declaredExactSource",
        "implementationOutputUsedAsOracle": False,
        "externalProcessing": False,
    }:
        raise SystemExit(f"source map {path_value} violates provenance policy")

    entries = document["entries"]
    if not isinstance(entries, list) or not entries or len(entries) > 512:
        raise SystemExit(f"source map {path_value} must contain 1..512 entries")
    result: dict[tuple[str, ...], dict[str, Any]] = {}
    source_order: list[tuple[str, ...]] = []
    for entry in entries:
        exact_fields(entry, {"finding", "location"}, {"finding", "location"}, "source-map entry")
        key = finding_tuple(entry["finding"], flattened=False)
        source_order.append(key)
        if key in result:
            raise SystemExit(f"source map {path_value} repeats finding {key!r}")
        location = exact_fields(
            entry["location"],
            {"attribution", "path", "startLine", "endLine", "anchorLine", "anchorText"},
            {"attribution", "path", "startLine", "endLine", "anchorLine", "anchorText"},
            "source-map location",
        )
        if location["attribution"] != "declaredExactSourceLine":
            raise SystemExit(f"source map {path_value} uses unsupported attribution")
        start = location["startLine"]
        end = location["endLine"]
        anchor = location["anchorLine"]
        if not all(isinstance(number, int) and not isinstance(number, bool) for number in (start, end, anchor)):
            raise SystemExit(f"source map {path_value} line numbers must be integers")
        if start < 1 or end < start or end - start >= 200 or not start <= anchor <= end:
            raise SystemExit(f"source map {path_value} contains an invalid line range")
        source = repository_file(location["path"], "source-map location")
        lines = source.read_text(encoding="utf-8").splitlines()
        if end > len(lines) or lines[anchor - 1] != location["anchorText"]:
            raise SystemExit(f"source map {path_value} has a stale line or anchor")
        if not location["anchorText"] or len(location["anchorText"].encode("utf-8")) > 4096:
            raise SystemExit(f"source map {path_value} anchor is empty or over budget")
        result[key] = location
    if source_order != sorted(source_order):
        raise SystemExit(f"source map {path_value} entries must be sorted by finding identity")
    return result


def expected_level(outcome: str, enforcement: str) -> str | None:
    if outcome == "failed":
        return "error" if enforcement == "blocking" else "warning"
    if outcome in {"cantTell", "untested"}:
        return "notice"
    return None


def validate() -> None:
    corpus = load(CORPUS)
    rules = load(RULES)
    projection = load(PROJECTION)
    metrics = load(METRICS)

    for schema_name, schema_id in [
        ("corpus.schema.json", "urn:sightlint:schema:github-actions-evaluation-corpus:0.1.0"),
        ("rule-annotation.schema.json", "urn:sightlint:schema:github-actions-rule-oracle:0.1.0"),
        ("projection-annotation.schema.json", "urn:sightlint:schema:github-actions-projection-oracle:0.1.0"),
        ("metric-contract.schema.json", "urn:sightlint:schema:github-actions-metric-contract:0.1.0"),
    ]:
        schema = load(EVALUATION / schema_name)
        if schema.get("$id") != schema_id or schema.get("additionalProperties") is not False:
            raise SystemExit(f"{schema_name} is not the expected strict schema")

    exact_fields(
        corpus,
        {"$schema", "schemaVersion", "corpus", "provenance", "splitPolicy", "metricContract", "projectionOracle", "cases", "limitations"},
        {"$schema", "schemaVersion", "corpus", "provenance", "splitPolicy", "metricContract", "projectionOracle", "cases", "limitations"},
        "corpus",
    )
    if corpus["schemaVersion"] != SCHEMA_VERSION:
        raise SystemExit("corpus uses an unsupported version")
    provenance = corpus["provenance"]
    if provenance.get("implementationOutputUsedAsOracle") is not False:
        raise SystemExit("corpus must not use implementation output as an oracle")
    if provenance.get("license") != "MIT OR Apache-2.0" or provenance.get("privacyReview") != "syntheticNoPersonalData":
        raise SystemExit("corpus license/privacy policy is incomplete")
    if provenance.get("externalAssets") is not False or provenance.get("externalProcessing") is not False:
        raise SystemExit("corpus must remain repository-owned and local")
    split_policy = corpus["splitPolicy"]
    if split_policy.get("visibility") != "public" or split_policy.get("holdoutStatus") != "notHoldout":
        raise SystemExit("corpus must disclose public non-holdout status")

    rule_cases = indexed_cases(rules, "rule oracle")
    projection_cases = indexed_cases(projection, "projection oracle")
    corpus_cases = indexed_cases(corpus, "corpus")
    if set(corpus_cases) != set(rule_cases) or set(corpus_cases) != set(projection_cases):
        raise SystemExit("corpus, rule, and projection case identifiers must match one-to-one")
    for document, context in [(rules, "rule oracle"), (projection, "projection oracle")]:
        if document.get("schemaVersion") != SCHEMA_VERSION:
            raise SystemExit(f"{context} uses an unsupported version")
        if document.get("provenance", {}).get("implementationOutputUsedAsOracle") is not False:
            raise SystemExit(f"{context} must not use implementation output as an oracle")

    observed = {
        "executedCases": 0,
        "reviewedFailures": 0,
        "exactSourceAnnotations": 0,
        "preservedAbstentions": 0,
        "summaryOnlyAbstentions": 0,
        "killedMutations": 0,
        "cleanCases": 0,
        "hardNegatives": 0,
        "unexpectedFailures": 0,
        "falsePositiveFailures": 0,
        "hardNegativeFailures": 0,
        "unexpectedAnnotations": 0,
    }

    for case_id, case in corpus_cases.items():
        exact_fields(
            case,
            {"caseId", "sourceId", "split", "classification", "input", "profile", "sourceMap", "ruleOracle", "pairedCleanCase", "falsePositiveRisk", "nonClaim"},
            {"caseId", "sourceId", "split", "classification", "input", "profile", "sourceMap", "ruleOracle", "pairedCleanCase", "falsePositiveRisk", "nonClaim"},
            f"corpus case {case_id}",
        )
        input_document = load(repository_file(case["input"], f"input for {case_id}"))
        source_entries = (
            validate_source_map(case["sourceMap"], input_document)
            if case["sourceMap"] is not None
            else {}
        )
        reference = case["ruleOracle"]
        if reference != {
            "document": "evaluation/github-actions/annotations/rules.json",
            "collection": "cases",
            "caseId": case_id,
        }:
            raise SystemExit(f"case {case_id} must use the separate current rule oracle")

        rule_case = rule_cases[case_id]
        if rule_case.get("profile") != case["profile"] or rule_case.get("forbidUnexpectedFailures") is not True:
            raise SystemExit(f"case {case_id} rule profile or failure policy disagrees")
        expectations = rule_case.get("expectations")
        if not isinstance(expectations, list) or not expectations:
            raise SystemExit(f"case {case_id} has no rule expectations")
        expected_by_key: dict[tuple[str, ...], dict[str, Any]] = {}
        for expectation in expectations:
            key = rule_expectation_tuple(expectation)
            if key in expected_by_key:
                raise SystemExit(f"case {case_id} repeats rule finding {key!r}")
            expected_by_key[key] = expectation
        if list(expected_by_key) != sorted(expected_by_key):
            raise SystemExit(f"case {case_id} rule expectations must be sorted")

        projection_case = projection_cases[case_id]
        if projection_case.get("forbidUnexpectedProjectedResults") is not True:
            raise SystemExit(f"case {case_id} must forbid unexpected projected results")
        dispositions = projection_case.get("dispositions")
        if not isinstance(dispositions, list):
            raise SystemExit(f"case {case_id} dispositions must be an array")
        disposition_by_key: dict[tuple[str, ...], dict[str, Any]] = {}
        for disposition in dispositions:
            exact_fields(
                disposition,
                {"finding", "status", "level", "location", "reason"},
                {"finding", "status", "level", "location", "reason"},
                f"projection disposition in {case_id}",
            )
            key = finding_tuple(disposition["finding"], flattened=True)
            if key in disposition_by_key:
                raise SystemExit(f"case {case_id} repeats projection finding {key!r}")
            disposition_by_key[key] = disposition
        if list(disposition_by_key) != sorted(disposition_by_key):
            raise SystemExit(f"case {case_id} projection dispositions must be sorted")

        actionable = {
            key: value
            for key, value in expected_by_key.items()
            if value["outcome"] in {"failed", "cantTell", "untested"}
        }
        if set(actionable) != set(disposition_by_key):
            raise SystemExit(f"case {case_id} projection does not match actionable rule truth")
        blocking = any(
            value["outcome"] == "failed" and value["enforcement"] == "blocking"
            for value in expectations
        )
        expected_exit = int(blocking)
        if rule_case.get("expectedExit") != expected_exit or projection_case.get("expectedExit") != expected_exit:
            raise SystemExit(f"case {case_id} exit oracles disagree with rule truth")

        for key, disposition in disposition_by_key.items():
            expectation = actionable[key]
            level = expected_level(expectation["outcome"], expectation["enforcement"])
            if key in source_entries:
                source = source_entries[key]
                expected_location = {
                    "path": source["path"],
                    "startLine": source["startLine"],
                    "endLine": source["endLine"],
                }
                if disposition["status"] != "emitted" or disposition["level"] != level or disposition["location"] != expected_location or disposition["reason"] is not None:
                    raise SystemExit(f"case {case_id} emitted disposition disagrees with exact source truth")
                observed["exactSourceAnnotations"] += 1
            else:
                expected_reason = "sourceMapNotProvided" if case["sourceMap"] is None else "sourceLocationNotDeclared"
                if disposition != {
                    "finding": disposition["finding"],
                    "status": "sourceUnavailable",
                    "level": None,
                    "location": None,
                    "reason": expected_reason,
                }:
                    raise SystemExit(f"case {case_id} must preserve unavailable source attribution")
                if expectation["outcome"] in {"cantTell", "untested"}:
                    observed["summaryOnlyAbstentions"] += 1
            if expectation["outcome"] in {"cantTell", "untested"}:
                observed["preservedAbstentions"] += 1
            if expectation["outcome"] == "failed":
                observed["reviewedFailures"] += 1

        classification = case["classification"]
        observed["executedCases"] += 1
        if classification == "targetedMutation":
            if not any(value["outcome"] == "failed" for value in expectations):
                raise SystemExit(f"targeted mutation {case_id} is not killed by its rule oracle")
            paired = case["pairedCleanCase"]
            if paired not in corpus_cases or corpus_cases[paired]["classification"] != "clean":
                raise SystemExit(f"targeted mutation {case_id} lacks a clean rerun pair")
            observed["killedMutations"] += 1
        elif case["pairedCleanCase"] is not None:
            raise SystemExit(f"non-mutation {case_id} must not declare a clean pair")
        if classification == "clean":
            observed["cleanCases"] += 1
        if classification == "hardNegative":
            observed["hardNegatives"] += 1
            if any(value["outcome"] == "failed" for value in expectations):
                observed["hardNegativeFailures"] += 1

    if metrics.get("schemaVersion") != SCHEMA_VERSION or metrics.get("caseCount") != len(corpus_cases):
        raise SystemExit("metric contract version or case count disagrees with corpus")
    if metrics.get("aggregateScore") is not False:
        raise SystemExit("metric contract must not introduce an aggregate score")
    for name, minimum in metrics.get("minimums", {}).items():
        if observed.get(name, -1) < minimum:
            raise SystemExit(f"metric {name}={observed.get(name)} is below minimum {minimum}")
    for name, maximum in metrics.get("maximums", {}).items():
        if observed.get(name, -1) > maximum:
            raise SystemExit(f"metric {name}={observed.get(name)} exceeds maximum {maximum}")

    print(
        "github-actions evaluation contract: "
        f"cases={observed['executedCases']}/{len(corpus_cases)}, "
        f"reviewed_failures={observed['reviewedFailures']}/{metrics['minimums']['reviewedFailures']}, "
        f"exact_source_annotations={observed['exactSourceAnnotations']}/{metrics['minimums']['exactSourceAnnotations']}, "
        f"abstentions={observed['preservedAbstentions']}/{metrics['minimums']['preservedAbstentions']}, "
        f"summary_only_abstentions={observed['summaryOnlyAbstentions']}/{metrics['minimums']['summaryOnlyAbstentions']}, "
        f"mutation_kill={observed['killedMutations']}/{metrics['minimums']['killedMutations']}, "
        f"hard_negative_failures={observed['hardNegativeFailures']}/{observed['hardNegatives']}, "
        "aggregate_score=none, holdout=none"
    )


if __name__ == "__main__":
    validate()
