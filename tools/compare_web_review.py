#!/usr/bin/env python3
"""Compare a finalized Web review submission with current public oracles read-only."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

from web_review_contract import (
    COMPARISON_SCHEMA,
    MAX_CASES,
    MAX_JUDGMENTS,
    MAX_OUTPUT_BYTES,
    MAX_PACKET_BYTES,
    PACKET_PATH,
    ROOT,
    VERSION,
    ContractError,
    array,
    canonical_bytes,
    digest,
    digest_bytes,
    exact,
    identifier,
    json_file_digest,
    load_json,
    nested_value,
    obj,
    relative_path,
    validate_packet,
    validate_submission,
)

REGISTRY_PATH = ROOT / "evaluation" / "web" / "evaluation-v1.json"


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--packet", type=Path, default=PACKET_PATH)
    root.add_argument("--submission", type=Path, required=True)
    root.add_argument("--registry", type=Path, default=REGISTRY_PATH)
    return root


def repository_json_path(value: Any, label: str) -> Path:
    path_text = relative_path(value, label)
    path = ROOT.joinpath(*path_text.split("/"))
    try:
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(ROOT.resolve()):
            raise OSError
    except OSError:
        raise ContractError("path", f"{label} is not a contained regular repository file")
    return path


def case_index(value: Any, label: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for raw in array(value, label, 1, MAX_CASES):
        item = obj(raw, f"{label} entry")
        case_id = identifier(item.get("caseId"), f"{label} caseId")
        if case_id in result:
            raise ContractError("ordering", f"{label} repeats case {case_id!r}")
        result[case_id] = item
    return result


def load_oracles(registry_path: Path) -> tuple[dict[str, Any], str, dict[str, dict[str, Any]], list[dict[str, str]]]:
    try:
        if registry_path.resolve(strict=True) != REGISTRY_PATH.resolve(strict=True):
            raise OSError
    except OSError:
        raise ContractError("path", "comparison requires the current public Web evaluation registry")
    registry, registry_digest = json_file_digest(registry_path, "Web evaluation registry")
    if (
        registry.get("$schema") != "./evaluation-v1.schema.json"
        or registry.get("schemaVersion") != "1.0.0"
    ):
        raise ContractError("version", "Web evaluation registry uses an unsupported version")
    datasets = array(registry.get("datasets"), "Web evaluation registry datasets", 1, 16)
    resolved: dict[str, dict[str, Any]] = {}
    bindings: dict[str, dict[str, str]] = {}
    for dataset in datasets:
        record = obj(dataset, "Web evaluation dataset")
        family_id = identifier(record.get("familyId"), "Web evaluation dataset familyId")
        acquisition_ref = obj(record.get("acquisitionOracle"), "acquisition oracle reference")
        rule_ref = obj(record.get("ruleOracle"), "rule oracle reference")
        acquisition_path = repository_json_path(acquisition_ref.get("document"), "acquisition oracle path")
        rule_path = repository_json_path(rule_ref.get("document"), "rule oracle path")
        acquisition, acquisition_digest = json_file_digest(acquisition_path, "acquisition oracle")
        rules, rule_digest = json_file_digest(rule_path, "rule oracle")
        acquisition_cases = case_index(acquisition.get("cases"), "acquisition oracle cases")
        rule_cases = case_index(rules.get("cases"), "rule oracle cases")
        if set(acquisition_cases) != set(rule_cases):
            raise ContractError("authority", "acquisition and rule oracle case inventories disagree")
        bindings[acquisition_path.relative_to(ROOT).as_posix()] = {
            "authority": "acquisition",
            "path": acquisition_path.relative_to(ROOT).as_posix(),
            "sha256": acquisition_digest,
        }
        bindings[rule_path.relative_to(ROOT).as_posix()] = {
            "authority": "rule",
            "path": rule_path.relative_to(ROOT).as_posix(),
            "sha256": rule_digest,
        }
        for case_id in sorted(acquisition_cases):
            if case_id in resolved:
                raise ContractError("inventory", f"multiple datasets contain case {case_id!r}")
            resolved[case_id] = {
                "familyId": family_id,
                "acquisition": acquisition_cases[case_id],
                "acquisitionPath": acquisition_path.relative_to(ROOT).as_posix(),
                "rule": rule_cases[case_id],
                "rulePath": rule_path.relative_to(ROOT).as_posix(),
            }
    return registry, registry_digest, resolved, [bindings[key] for key in sorted(bindings)]


def acquisition_oracle_value(case: dict[str, Any], judgment: dict[str, Any]) -> tuple[Any, str]:
    subject = obj(judgment["subject"], "acquisition subject")
    kind = subject["kind"]
    subject_id = subject["id"]
    aspect = judgment["aspect"]
    if kind == "case":
        base = obj(case.get("expectations"), "acquisition case expectations")
        source = f"expectations.{aspect}"
    elif kind == "node":
        nodes = array(obj(case.get("expectations"), "acquisition case expectations").get("nodes"), "acquisition nodes", 0, MAX_JUDGMENTS)
        matches = [item for item in nodes if isinstance(item, dict) and item.get("id") == subject_id]
        if len(matches) != 1:
            raise KeyError(f"node:{subject_id}")
        base = matches[0]
        source = f"expectations.nodes[id={subject_id}].{aspect}"
    else:
        abstentions = array(case.get("abstentions"), "acquisition abstentions", 0, MAX_JUDGMENTS)
        matches = [item for item in abstentions if isinstance(item, dict) and item.get("aspect") == subject_id]
        if len(matches) != 1:
            raise KeyError(f"abstention:{subject_id}")
        base = matches[0]
        source = f"abstentions[aspect={subject_id}].{aspect}"
    return nested_value(base, aspect), source


def rule_oracle_value(case: dict[str, Any], judgment: dict[str, Any]) -> tuple[Any, str, dict[str, Any]]:
    results = array(case.get("expectedResults"), "rule expectedResults", 0, MAX_JUDGMENTS)
    key_fields = ("ruleId", "ruleVersion", "targetKind", "targetId", "targetAspect")
    matches = [
        item
        for item in results
        if isinstance(item, dict) and all(item.get(field) == judgment.get(field) for field in key_fields)
    ]
    if len(matches) != 1:
        raise KeyError(judgment["judgmentId"])
    return matches[0].get("outcome"), "expectedResults[rule/version/target].outcome", matches[0]


def rationale_for_acquisition(case: dict[str, Any], judgment: dict[str, Any]) -> str:
    subject = obj(judgment["subject"], "acquisition subject")
    if subject["kind"] == "abstention":
        for item in array(case.get("abstentions"), "acquisition abstentions", 0, MAX_JUDGMENTS):
            if isinstance(item, dict) and item.get("aspect") == subject["id"]:
                return str(item.get("rationale", "No oracle rationale recorded."))
    review = case.get("review")
    if isinstance(review, dict) and isinstance(review.get("rationale"), str):
        return review["rationale"]
    hard_negative = case.get("hardNegative")
    if isinstance(hard_negative, dict) and isinstance(hard_negative.get("rationale"), str):
        return hard_negative["rationale"]
    return "The public acquisition oracle records the compared value without a field-specific rationale."


def comparison_row(
    authority: str,
    case_id: str,
    judgment: dict[str, Any],
    oracle_case: dict[str, Any],
    oracle_path: str,
) -> tuple[dict[str, Any], bool, bool, bool]:
    reviewer_value: Any
    if authority == "acquisition":
        reviewer_value = judgment["value"] if judgment["status"] == "observed" else judgment["status"]
        try:
            oracle_value, source = acquisition_oracle_value(oracle_case, judgment)
            oracle_rationale = rationale_for_acquisition(oracle_case, judgment)
        except KeyError:
            oracle_value = None
            source = "unresolved"
            oracle_rationale = "No unique current acquisition-oracle item matches the submitted subject and aspect."
            row_status = "unresolved"
        else:
            row_status = "agreement" if reviewer_value == oracle_value else "disagreement"
    else:
        reviewer_value = judgment["outcome"]
        try:
            oracle_value, source, _ = rule_oracle_value(oracle_case, judgment)
            oracle_rationale = str(oracle_case.get("review", {}).get("rationale", oracle_case.get("falsePositiveRisk", "No oracle rationale recorded.")))
        except KeyError:
            oracle_value = None
            source = "unresolved"
            oracle_rationale = "No unique current rule-oracle item matches the submitted rule/version/target key."
            row_status = "unresolved"
        else:
            row_status = "agreement" if reviewer_value == oracle_value else "disagreement"
    unresolved = row_status in {"disagreement", "unresolved"}
    abstention_agreement = row_status == "agreement" and reviewer_value in {"cantTell", "untested"}
    row = {
        "authority": authority,
        "caseId": case_id,
        "judgmentId": judgment["judgmentId"],
        "status": row_status,
        "unresolved": unresolved,
        "reviewer": {
            "value": reviewer_value,
            "confidence": judgment["confidence"],
            "rationale": judgment["rationale"],
        },
        "oracle": {
            "value": oracle_value,
            "document": oracle_path,
            "source": source,
            "rationale": oracle_rationale,
        },
        "adjudication": {
            "status": "notPerformed",
            "rationale": "Version 1.0.0 preserves the comparison for separate human adjudication and never resolves it automatically.",
        },
    }
    return row, row_status == "agreement", row_status == "disagreement", abstention_agreement


def compare(packet: dict[str, Any], submission: dict[str, Any], registry_path: Path) -> dict[str, Any]:
    validate_packet(packet)
    validate_submission(submission, packet, require_finalized=True)
    # Oracle files are intentionally opened only after finalized digest validation above.
    _, registry_digest, oracles, oracle_bindings = load_oracles(registry_path)
    rows: list[dict[str, Any]] = []
    counts = {
        "acquisitionAgreement": 0,
        "ruleAgreement": 0,
        "disagreement": 0,
        "unresolved": 0,
        "adjudicated": 0,
        "abstentionAgreement": 0,
    }
    for case in submission["cases"]:
        case_id = case["caseId"]
        if case_id not in oracles:
            raise ContractError("inventory", f"current registry has no oracle binding for {case_id!r}")
        oracle = oracles[case_id]
        if oracle["familyId"] != next(item["familyId"] for item in packet["cases"] if item["caseId"] == case_id):
            raise ContractError("binding", f"current registry family differs for {case_id!r}")
        for judgment in case["acquisitionJudgments"]:
            row, agreed, disagreed, abstention = comparison_row(
                "acquisition", case_id, judgment, oracle["acquisition"], oracle["acquisitionPath"]
            )
            rows.append(row)
            counts["acquisitionAgreement"] += int(agreed)
            counts["disagreement"] += int(disagreed)
            counts["unresolved"] += int(row["unresolved"])
            counts["abstentionAgreement"] += int(abstention)
        for judgment in case["ruleJudgments"]:
            row, agreed, disagreed, abstention = comparison_row(
                "rule", case_id, judgment, oracle["rule"], oracle["rulePath"]
            )
            rows.append(row)
            counts["ruleAgreement"] += int(agreed)
            counts["disagreement"] += int(disagreed)
            counts["unresolved"] += int(row["unresolved"])
            counts["abstentionAgreement"] += int(abstention)
    rows.sort(key=lambda row: (row["caseId"], row["authority"], row["judgmentId"]))
    if len(rows) > MAX_JUDGMENTS:
        raise ContractError("limit", f"comparison exceeds the {MAX_JUDGMENTS}-row limit")
    report: dict[str, Any] = {
        "$schema": COMPARISON_SCHEMA,
        "schemaVersion": VERSION,
        "documentType": "webReviewComparison",
        "comparisonDigest": None,
        "recordPurpose": submission["recordPurpose"],
        "evidenceStatus": "ineligibleConformance" if submission["recordPurpose"] == "fictionalConformance" else "requiresGovernanceReview",
        "packetBinding": {
            "packetId": packet["packetId"],
            "packetDigest": packet["packetDigest"],
        },
        "submissionBinding": {
            "submissionId": submission["submissionId"],
            "submissionDigest": submission["submissionDigest"],
        },
        "oracleBindings": {
            "registry": {
                "path": registry_path.relative_to(ROOT).as_posix(),
                "sha256": registry_digest,
            },
            "documents": oracle_bindings,
        },
        "counts": counts,
        "comparisons": rows,
        "adjudication": {
            "performedByTool": False,
            "adjudicatedCount": 0,
            "rationale": "Comparison preserves disagreements and unresolved items for a separately responsible human adjudicator.",
        },
        "nonClaims": [
            "Structural equality does not prove that either reviewer or oracle is correct.",
            "SightLint does not verify reviewer identity, qualification, independence, conflicts, or signatures.",
            "Public source-first agreement is not protected-holdout evidence or representative accuracy.",
            "This comparison does not establish WCAG conformance, universal UI and UX quality, or blocking maturity.",
        ],
    }
    report["comparisonDigest"] = digest(report, "comparisonDigest")
    if len(canonical_bytes(report)) > MAX_OUTPUT_BYTES:
        raise ContractError("output-budget", f"comparison exceeds the {MAX_OUTPUT_BYTES}-byte limit")
    return report


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        packet = load_json(arguments.packet, "review packet", MAX_PACKET_BYTES)
        submission = load_json(arguments.submission, "reviewer submission")
        report = compare(packet, submission, arguments.registry)
        sys.stdout.buffer.write(canonical_bytes(report))
    except (ContractError, OSError, StopIteration) as error:
        category = error.category if isinstance(error, ContractError) else "input"
        print(f"web-review-compare: {category}: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
