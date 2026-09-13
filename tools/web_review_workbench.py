#!/usr/bin/env python3
"""Local-only Harbor review workbench contracts and HTTP server."""

from __future__ import annotations

import copy
import json
import os
import secrets
import tempfile
import threading
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs, urlsplit

from web_review_contract import (
    HARBOR_CASES,
    HARBOR_SOURCE_FILES,
    MAX_IDENTIFIER_BYTES,
    MAX_STRING_BYTES,
    ROOT,
    VERSION,
    ContractError,
    array,
    canonical_bytes,
    digest,
    enum,
    exact,
    fail,
    finalize_submission,
    identifier,
    load_json,
    obj,
    sorted_unique_strings,
    text,
    unique_object,
    valid_date,
    validate_harbor_questionnaire,
    validate_packet,
    validate_privacy_text,
    validate_submission,
)

WORKBENCH_VERSION = "1.0.0"
STATE_SCHEMA = "./harbor-review-workbench-state.schema.json"
MAX_HTTP_BODY_BYTES = 262_144
ASSET_DIRECTORY = ROOT / "evaluation" / "web" / "review-workbench"
ASSET_ROUTES = {
    "/": ("index.html", "text/html; charset=utf-8"),
    "/app.js": ("app.js", "text/javascript; charset=utf-8"),
    "/styles.css": ("styles.css", "text/css; charset=utf-8"),
}
FIXTURE_ROUTES = {
    "/fixture/index.html": ("evaluation/web/support-inbox-app/index.html", "text/html; charset=utf-8"),
    "/fixture/app.js": ("evaluation/web/support-inbox-app/app.js", "text/javascript; charset=utf-8"),
    "/fixture/styles.css": ("evaluation/web/support-inbox-app/styles.css", "text/css; charset=utf-8"),
}
STATE_LIMITATIONS = [
    "This local draft is not review evidence and cannot be compared until finalized.",
    "The workbench supplies metadata and serialization only; every substantive judgment remains reviewer-authored.",
    "A clean local browser profile is required because the workbench cannot control extensions, synchronization, or operating-system telemetry.",
    "The Harbor pilot does not review pixels, geometry, other rules, Atlas, protected data, or representative accuracy.",
]
FINAL_LIMITATIONS = [
    "The submission was authored through the local Harbor review workbench using exactly eight reviewer judgments.",
    "Question metadata was fixed before review; the workbench did not supply, recommend, or adjudicate an answer.",
    "Pixels and geometry were not used to infer a native programmatic name, and no other rule was reviewed.",
    "The public Harbor pilot requires governance review and does not establish protected-holdout or representative product performance.",
]
CONFIRMATION_FIELDS = {
    "cleanLocalBrowserProfile",
    "existingOracleNotViewed",
    "generatedCaptureNotUsed",
    "implementationOutputNotUsed",
    "noCredentials",
    "noExternalProcessing",
    "noPrivateData",
    "sightlintOutputNotUsed",
    "sourceFirstNotBlind",
}


def _draft_text(value: Any, label: str, maximum: int = MAX_STRING_BYTES) -> str:
    if not isinstance(value, str):
        fail("shape", f"{label} must be a string")
    if len(value.encode("utf-8")) > maximum:
        fail("limit", f"{label} exceeds the {maximum}-byte string limit")
    return value


def _nullable_enum(value: Any, allowed: set[str], label: str) -> str | None:
    if value is None:
        return None
    return enum(value, allowed, label)


def _questionnaire_binding(questionnaire: dict[str, Any]) -> dict[str, str]:
    return {
        "questionnaireId": questionnaire["questionnaireId"],
        "questionnaireDigest": questionnaire["questionnaireDigest"],
    }


def _packet_binding(packet: dict[str, Any]) -> dict[str, str]:
    return {"packetId": packet["packetId"], "packetDigest": packet["packetDigest"]}


def build_empty_state(
    packet: dict[str, Any], questionnaire: dict[str, Any], record_purpose: str
) -> dict[str, Any]:
    """Build an answer-free, resumable Harbor workbench draft."""
    validate_harbor_questionnaire(questionnaire, packet)
    enum(record_purpose, {"fictionalConformance", "humanReviewCandidate"}, "record purpose")
    state: dict[str, Any] = {
        "$schema": STATE_SCHEMA,
        "schemaVersion": WORKBENCH_VERSION,
        "documentType": "harborReviewWorkbenchState",
        "stateDigest": None,
        "recordPurpose": record_purpose,
        "questionnaireBinding": _questionnaire_binding(questionnaire),
        "packetBinding": _packet_binding(packet),
        "submissionId": "",
        "reviewer": {
            "stableProjectId": "",
            "qualificationCategory": None,
            "qualificationRationale": "",
            "independence": None,
            "independenceRationale": "",
            "priorExposureStatus": None,
            "priorExposureCaseIds": [],
            "priorExposureRationale": "",
            "conflictStatus": None,
            "conflictRationale": "",
            "reviewedOn": "",
        },
        "confirmations": {field: False for field in sorted(CONFIRMATION_FIELDS)},
        "answers": [
            {
                "caseId": case_id,
                "acquisition": {
                    "status": None,
                    "observedValueKind": None,
                    "value": None,
                    "confidence": None,
                    "rationale": "",
                },
                "rule": {
                    "outcome": None,
                    "requiredEvidence": None,
                    "confidence": None,
                    "rationale": "",
                },
            }
            for case_id in sorted(HARBOR_CASES)
        ],
        "stateLimitations": list(STATE_LIMITATIONS),
    }
    state["stateDigest"] = digest(state, "stateDigest")
    validate_state(state, packet, questionnaire)
    return state


def validate_state(
    state: dict[str, Any],
    packet: dict[str, Any],
    questionnaire: dict[str, Any],
    *,
    require_complete: bool = False,
) -> None:
    """Validate a local Harbor draft and optionally require finalizable input."""
    validate_harbor_questionnaire(questionnaire, packet)
    exact(
        state,
        {
            "$schema",
            "schemaVersion",
            "documentType",
            "stateDigest",
            "recordPurpose",
            "questionnaireBinding",
            "packetBinding",
            "submissionId",
            "reviewer",
            "confirmations",
            "answers",
            "stateLimitations",
        },
        "Harbor workbench state",
    )
    if (
        state["$schema"] != STATE_SCHEMA
        or state["schemaVersion"] != WORKBENCH_VERSION
        or state["documentType"] != "harborReviewWorkbenchState"
    ):
        fail("version", "Harbor workbench state uses an unsupported version or document type")
    record_purpose = enum(
        state["recordPurpose"],
        {"fictionalConformance", "humanReviewCandidate"},
        "Harbor workbench record purpose",
    )
    if state["questionnaireBinding"] != _questionnaire_binding(questionnaire):
        fail("binding", "Harbor workbench state does not bind the current questionnaire")
    if state["packetBinding"] != _packet_binding(packet):
        fail("binding", "Harbor workbench state does not bind the current packet")
    if state["stateLimitations"] != STATE_LIMITATIONS:
        fail("claims", "Harbor workbench state changes its fixed limitations")
    if state["stateDigest"] != digest(state, "stateDigest"):
        fail("digest", "Harbor workbench state digest does not match its canonical projection")

    submission_id = _draft_text(state["submissionId"], "workbench submissionId", MAX_IDENTIFIER_BYTES)
    if submission_id:
        identifier(submission_id, "workbench submissionId")
    reviewer = exact(
        state["reviewer"],
        {
            "stableProjectId",
            "qualificationCategory",
            "qualificationRationale",
            "independence",
            "independenceRationale",
            "priorExposureStatus",
            "priorExposureCaseIds",
            "priorExposureRationale",
            "conflictStatus",
            "conflictRationale",
            "reviewedOn",
        },
        "workbench reviewer",
    )
    reviewer_id = _draft_text(reviewer["stableProjectId"], "workbench reviewer ID", MAX_IDENTIFIER_BYTES)
    if reviewer_id:
        identifier(reviewer_id, "workbench reviewer ID")
    qualification = _nullable_enum(
        reviewer["qualificationCategory"],
        {"accessibility", "other", "productReview", "webUi"},
        "workbench reviewer qualification",
    )
    independence = _nullable_enum(
        reviewer["independence"], {"declaredFalse", "declaredTrue"}, "workbench reviewer independence"
    )
    exposure = _nullable_enum(
        reviewer["priorExposureStatus"], {"full", "none", "partial"}, "workbench prior exposure"
    )
    conflict = _nullable_enum(
        reviewer["conflictStatus"], {"declared", "noneDeclared"}, "workbench conflict status"
    )
    qualification_rationale = _draft_text(
        reviewer["qualificationRationale"], "workbench qualification rationale"
    )
    independence_rationale = _draft_text(
        reviewer["independenceRationale"], "workbench independence rationale"
    )
    exposure_rationale = _draft_text(
        reviewer["priorExposureRationale"], "workbench prior-exposure rationale"
    )
    conflict_rationale = _draft_text(reviewer["conflictRationale"], "workbench conflict rationale")
    reviewed_on = _draft_text(reviewer["reviewedOn"], "workbench review date", 10)
    if reviewed_on and not valid_date(reviewed_on):
        fail("date", "workbench review date must be a real YYYY-MM-DD calendar date")
    exposed_cases = sorted_unique_strings(
        reviewer["priorExposureCaseIds"], "workbench prior-exposure cases", len(HARBOR_CASES), identifier
    )
    if any(case_id not in HARBOR_CASES for case_id in exposed_cases):
        fail("inventory", "workbench prior exposure references a case outside the Harbor pilot")
    if exposure in {None, "none"} and exposed_cases:
        fail("exposure", "workbench no/undeclared exposure must not list case IDs")
    if exposure == "partial" and (not exposed_cases or len(exposed_cases) == len(HARBOR_CASES)):
        fail("exposure", "workbench partial exposure must list a non-empty proper Harbor subset")
    if exposure == "full" and exposed_cases != sorted(HARBOR_CASES):
        fail("exposure", "workbench full exposure must list every Harbor case")

    confirmations = exact(state["confirmations"], CONFIRMATION_FIELDS, "workbench confirmations")
    if any(not isinstance(value, bool) for value in confirmations.values()):
        fail("shape", "workbench confirmations must be booleans")

    answers = array(state["answers"], "workbench answers", len(HARBOR_CASES), len(HARBOR_CASES))
    observed_case_ids: list[str] = []
    for answer in answers:
        record = exact(answer, {"caseId", "acquisition", "rule"}, "workbench answer")
        case_id = identifier(record["caseId"], "workbench answer caseId")
        observed_case_ids.append(case_id)
        acquisition = exact(
            record["acquisition"],
            {"status", "observedValueKind", "value", "confidence", "rationale"},
            f"workbench case {case_id} acquisition",
        )
        status = _nullable_enum(
            acquisition["status"], {"cantTell", "observed", "untested"}, f"case {case_id} acquisition status"
        )
        value_kind = _nullable_enum(
            acquisition["observedValueKind"], {"absent", "text"}, f"case {case_id} observed value kind"
        )
        value = acquisition["value"]
        if value is not None:
            value = _draft_text(value, f"case {case_id} acquisition value")
        acquisition_confidence = _nullable_enum(
            acquisition["confidence"], {"high", "low", "medium"}, f"case {case_id} acquisition confidence"
        )
        acquisition_rationale = _draft_text(
            acquisition["rationale"], f"case {case_id} acquisition rationale"
        )
        if status in {None, "cantTell", "untested"} and (value_kind is not None or value is not None):
            fail("authority", f"case {case_id} unavailable acquisition must not contain an observed value")
        if value_kind == "absent" and value is not None:
            fail("authority", f"case {case_id} observed-absent acquisition must have a null value")
        if value_kind == "text" and value is None:
            fail("authority", f"case {case_id} observed-text acquisition must have a string value")
        if status == "observed" and value_kind not in {None, "absent", "text"}:
            fail("authority", f"case {case_id} observed acquisition has an invalid value kind")

        rule = exact(
            record["rule"],
            {"outcome", "requiredEvidence", "confidence", "rationale"},
            f"workbench case {case_id} rule",
        )
        outcome = _nullable_enum(
            rule["outcome"],
            {"cantTell", "failed", "inapplicable", "passed", "untested"},
            f"case {case_id} rule outcome",
        )
        required_evidence = _nullable_enum(
            rule["requiredEvidence"],
            {"conflicting", "insufficient", "sufficient", "untested"},
            f"case {case_id} rule required evidence",
        )
        rule_confidence = _nullable_enum(
            rule["confidence"], {"high", "low", "medium"}, f"case {case_id} rule confidence"
        )
        rule_rationale = _draft_text(rule["rationale"], f"case {case_id} rule rationale")
        if outcome in {"passed", "failed", "inapplicable"} and required_evidence not in {None, "sufficient"}:
            fail("authority", f"case {case_id} conclusive rule outcome requires sufficient evidence")
        if outcome == "cantTell" and required_evidence not in {None, "conflicting", "insufficient"}:
            fail("authority", f"case {case_id} cantTell requires insufficient or conflicting evidence")
        if outcome == "untested" and required_evidence not in {None, "untested"}:
            fail("authority", f"case {case_id} untested outcome requires untested evidence")

        if require_complete:
            if (
                status is None
                or (value_kind is None and status == "observed")
                or acquisition_confidence is None
                or not acquisition_rationale
                or outcome is None
                or required_evidence is None
                or rule_confidence is None
                or not rule_rationale
            ):
                fail("finalization", f"case {case_id} does not contain both complete reviewer judgments")
            if status == "observed" and value_kind == "text" and not value:
                fail("finalization", f"case {case_id} observed accessible-name text must not be empty")
    if observed_case_ids != sorted(HARBOR_CASES):
        fail("ordering", "workbench answers must contain the exact sorted Harbor case inventory")

    validate_privacy_text(state, "Harbor workbench state")
    if require_complete:
        if (
            not submission_id
            or not reviewer_id
            or qualification is None
            or not qualification_rationale
            or independence is None
            or not independence_rationale
            or exposure is None
            or not exposure_rationale
            or conflict is None
            or not conflict_rationale
            or not reviewed_on
            or not all(confirmations.values())
        ):
            fail("finalization", "workbench reviewer declaration and confirmations are incomplete")
        if record_purpose == "fictionalConformance" and exposure != "full":
            fail("claims", "fictional workbench conformance must declare full expected-label exposure")


def _rule_applicability(outcome: str) -> str:
    if outcome in {"failed", "passed"}:
        return "applicable"
    if outcome == "cantTell":
        return "cantTell"
    return outcome


def state_to_submission(
    state: dict[str, Any], packet: dict[str, Any], questionnaire: dict[str, Any]
) -> dict[str, Any]:
    """Convert complete human inputs into the existing reviewer-submission contract."""
    validate_state(state, packet, questionnaire, require_complete=True)
    reviewer = state["reviewer"]
    questions = {question["questionId"]: question for question in questionnaire["questions"]}
    cases: list[dict[str, Any]] = []
    for answer in state["answers"]:
        case_id = answer["caseId"]
        acquisition_answer = answer["acquisition"]
        rule_answer = answer["rule"]
        acquisition_question = questions[f"{case_id}.acquisition.name"]
        rule_question = questions[f"{case_id}.rule.interactive-name"]
        if acquisition_answer["status"] == "observed":
            acquisition_value = (
                acquisition_answer["value"] if acquisition_answer["observedValueKind"] == "text" else None
            )
            native_status = "available"
            unavailable: list[str] = []
        else:
            acquisition_value = None
            native_status = "unavailable" if acquisition_answer["status"] == "cantTell" else "untested"
            unavailable = [acquisition_answer["rationale"]]
        cases.append(
            {
                "caseId": case_id,
                "caseContext": {
                    "reviewedAs": "other",
                    "rationale": (
                        "The fixed Harbor pilot does not pre-classify this case for the reviewer; "
                        "the two judgments carry the substantive conclusion."
                    ),
                },
                "acquisitionJudgments": [
                    {
                        "judgmentId": acquisition_question["questionId"],
                        "subject": acquisition_question["subject"],
                        "aspect": acquisition_question["aspect"],
                        "status": acquisition_answer["status"],
                        "value": acquisition_value,
                        "unitOrCoordinateSpace": None,
                        "confidence": acquisition_answer["confidence"],
                        "rationale": acquisition_answer["rationale"],
                        "nativeEvidence": {
                            "status": native_status,
                            "rationale": acquisition_answer["rationale"],
                        },
                        "pixelEvidence": {
                            "status": "notApplicable",
                            "rationale": (
                                "The pilot reviews browser-native programmatic-name evidence; "
                                "pixels are not used to infer the name."
                            ),
                        },
                        "nativePixelRelationship": "notCompared",
                        "unavailableEvidence": unavailable,
                    }
                ],
                "ruleJudgments": [
                    {
                        "judgmentId": rule_question["questionId"],
                        "ruleId": rule_question["ruleId"],
                        "ruleVersion": rule_question["ruleVersion"],
                        "targetKind": rule_question["targetKind"],
                        "targetId": rule_question["targetId"],
                        "targetAspect": rule_question["targetAspect"],
                        "applicability": _rule_applicability(rule_answer["outcome"]),
                        "requiredEvidence": rule_answer["requiredEvidence"],
                        "outcome": rule_answer["outcome"],
                        "policyBasis": rule_question["policyBasis"],
                        "validAlternativeOrHardNegative": rule_answer["rationale"],
                        "falsePositiveRisk": rule_question["falsePositiveRisk"],
                        "falseNegativeRisk": rule_question["falseNegativeRisk"],
                        "confidence": rule_answer["confidence"],
                        "rationale": rule_answer["rationale"],
                    }
                ],
            }
        )
    purpose = state["recordPurpose"]
    submission = {
        "$schema": "./reviewer-submission.schema.json",
        "schemaVersion": VERSION,
        "documentType": "webReviewerSubmission",
        "submissionId": state["submissionId"],
        "submissionDigest": None,
        "lifecycle": "draft",
        "recordPurpose": purpose,
        "evidenceStatus": (
            "ineligibleConformance" if purpose == "fictionalConformance" else "requiresGovernanceReview"
        ),
        "packetBinding": _packet_binding(packet),
        "reviewScope": {
            "familyIds": ["harbor-support-inbox-v1"],
            "caseIds": sorted(HARBOR_CASES),
            "completeForDeclaredScope": True,
        },
        "reviewer": {
            "stableProjectId": reviewer["stableProjectId"],
            "qualification": {
                "category": reviewer["qualificationCategory"],
                "rationale": reviewer["qualificationRationale"],
            },
            "independentFromAnnotationAuthors": reviewer["independence"],
            "independenceRationale": reviewer["independenceRationale"],
            "priorExpectedLabelExposure": {
                "status": reviewer["priorExposureStatus"],
                "caseIds": reviewer["priorExposureCaseIds"],
                "rationale": reviewer["priorExposureRationale"],
            },
            "conflictOfInterest": {
                "status": reviewer["conflictStatus"],
                "rationale": reviewer["conflictRationale"],
            },
            "reviewedOn": reviewer["reviewedOn"],
        },
        "declarations": {
            "sourceFirstNotBlind": True,
            "sightlintOutputUsedBeforeFinalization": False,
            "existingOracleViewedBeforeFinalization": False,
            "generatedCaptureOrReportUsedAsAnswer": False,
            "implementationOutputUsedAsAnswer": False,
            "containsProtectedOrPrivateData": False,
            "containsCredentials": False,
            "externalProcessingUsed": False,
            "identityOrQualificationVerifiedBySightLint": False,
            "signatureVerifiedBySightLint": False,
        },
        "cases": cases,
        "submissionLimitations": [
            *FINAL_LIMITATIONS,
            (
                f"Questionnaire {questionnaire['questionnaireId']} is bound at "
                f"{questionnaire['questionnaireDigest']}."
            ),
        ],
    }
    validate_submission(submission, packet)
    return submission


def update_state_from_request(
    current: dict[str, Any], payload: dict[str, Any], packet: dict[str, Any], questionnaire: dict[str, Any]
) -> dict[str, Any]:
    """Apply only human-editable fields using optimistic digest locking."""
    request = exact(
        payload,
        {"expectedStateDigest", "submissionId", "reviewer", "confirmations", "answers"},
        "workbench save request",
    )
    if request["expectedStateDigest"] != current["stateDigest"]:
        fail("stale", "workbench save request does not match the current state digest")
    updated = copy.deepcopy(current)
    for field in ("submissionId", "reviewer", "confirmations", "answers"):
        updated[field] = request[field]
    updated["stateDigest"] = digest(updated, "stateDigest")
    validate_state(updated, packet, questionnaire)
    return updated


def atomic_replace(path: Path, raw: bytes) -> None:
    """Atomically replace one draft using a closed same-directory temporary file."""
    temporary: Path | None = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.sightlint-", dir=path.parent)
        temporary = Path(temporary_name)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def exclusive_promote(path: Path, raw: bytes) -> None:
    """Atomically create finalized bytes without replacing an existing file."""
    temporary: Path | None = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.sightlint-", dir=path.parent)
        temporary = Path(temporary_name)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            fail("output", "finalized output already exists and will not be overwritten")
        temporary.unlink()
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def resolve_output_path(path: Path, label: str) -> Path:
    """Resolve one explicit output below a real directory but outside the repository."""
    if path.name in {"", ".", ".."}:
        fail("output", f"{label} must name a file")
    if path.parent.is_symlink():
        fail("output", f"{label} parent must not be a symlink")
    try:
        parent = path.parent.resolve(strict=True)
    except OSError:
        fail("output", f"{label} parent must be an existing directory")
    if not parent.is_dir():
        fail("output", f"{label} parent must be an existing directory")
    resolved = parent / path.name
    if resolved.is_relative_to(ROOT.resolve()):
        fail("output", f"{label} must be outside the repository")
    if resolved.is_symlink():
        fail("output", f"{label} must not be a symlink")
    if resolved.exists() and not resolved.is_file():
        fail("output", f"{label} existing path must be a regular non-symlink file")
    return resolved


class WorkbenchSession:
    """Mutable local session whose persisted state remains digest-bound."""

    def __init__(
        self,
        packet: dict[str, Any],
        questionnaire: dict[str, Any],
        draft_path: Path,
        final_path: Path,
        record_purpose: str,
        resume: bool,
    ) -> None:
        self.packet = packet
        self.questionnaire = questionnaire
        self.draft_path = resolve_output_path(draft_path, "draft path")
        self.final_path = resolve_output_path(final_path, "final path")
        if self.draft_path == self.final_path:
            fail("output", "draft and finalized output paths must differ")
        if self.final_path.exists():
            fail("output", "finalized output already exists and will not be overwritten")
        if resume:
            if not self.draft_path.exists():
                fail("resume", "resume requires an existing draft file")
            self.state = load_json(self.draft_path, "Harbor workbench state", MAX_HTTP_BODY_BYTES)
            validate_state(self.state, packet, questionnaire)
            if self.state["recordPurpose"] != record_purpose:
                fail("resume", "draft record purpose does not match the requested workbench mode")
        else:
            if self.draft_path.exists():
                fail("output", "draft output exists; use --resume to open it explicitly")
            self.state = build_empty_state(packet, questionnaire, record_purpose)
        self.locked = False
        self.token = secrets.token_urlsafe(32)
        packet_files = {entry["path"]: entry["contentUtf8"] for entry in packet["files"]}
        self.fixture_files = {path: packet_files[path] for path in HARBOR_SOURCE_FILES}

    def save(self, payload: dict[str, Any]) -> dict[str, Any]:
        if self.locked:
            fail("lifecycle", "workbench is locked after finalization")
        updated = update_state_from_request(self.state, payload, self.packet, self.questionnaire)
        atomic_replace(self.draft_path, canonical_bytes(updated))
        self.state = updated
        return self.state

    def finalize(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.save(payload)
        validate_state(state, self.packet, self.questionnaire, require_complete=True)
        submission = state_to_submission(state, self.packet, self.questionnaire)
        finalized = finalize_submission(submission, self.packet)
        exclusive_promote(self.final_path, canonical_bytes(finalized))
        self.locked = True
        return finalized


def _json_error(category: str, message: str) -> bytes:
    return canonical_bytes({"error": {"category": category, "message": message}})


def handler_for(session: WorkbenchSession) -> type[BaseHTTPRequestHandler]:
    """Create a request handler bound to one local workbench session."""

    class WorkbenchHandler(BaseHTTPRequestHandler):
        server_version = "SightLintReviewWorkbench/1.0"
        sys_version = ""

        def log_message(self, _format: str, *_args: Any) -> None:
            return

        def _origin(self) -> str:
            return f"http://127.0.0.1:{self.server.server_port}"

        def _base_headers(self, content_type: str, length: int, *, fixture: bool = False) -> None:
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(length))
            self.send_header("Cache-Control", "no-store")
            self.send_header("Referrer-Policy", "no-referrer")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Cross-Origin-Resource-Policy", "same-origin")
            self.send_header("Permissions-Policy", "camera=(), geolocation=(), microphone=()")
            if fixture:
                self.send_header("X-Frame-Options", "SAMEORIGIN")
                self.send_header(
                    "Content-Security-Policy",
                    "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'none'; "
                    "img-src 'none'; font-src 'none'; object-src 'none'; base-uri 'none'; "
                    "form-action 'none'; frame-ancestors 'self'",
                )
            else:
                self.send_header("X-Frame-Options", "DENY")
                self.send_header(
                    "Content-Security-Policy",
                    "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; "
                    "img-src 'none'; font-src 'none'; object-src 'none'; base-uri 'none'; "
                    "form-action 'none'; frame-src 'self'; frame-ancestors 'none'",
                )

        def _send(self, status: int, raw: bytes, content_type: str, *, fixture: bool = False) -> None:
            self.send_response(status)
            self._base_headers(content_type, len(raw), fixture=fixture)
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(raw)

        def _fail(self, status: int, category: str, message: str) -> None:
            self._send(status, _json_error(category, message), "application/json; charset=utf-8")

        def _single_header(self, name: str) -> str | None:
            values = self.headers.get_all(name, failobj=[])
            if len(values) != 1:
                return None
            return values[0]

        def _valid_host(self) -> bool:
            return self._single_header("Host") == f"127.0.0.1:{self.server.server_port}"

        def _authorized_api(self, *, mutation: bool) -> bool:
            if not self._valid_host():
                self._fail(HTTPStatus.BAD_REQUEST, "host", "request Host does not match the loopback origin")
                return False
            if self._single_header("X-SightLint-Review-Token") != session.token:
                self._fail(HTTPStatus.FORBIDDEN, "token", "request token is missing or invalid")
                return False
            if mutation and self._single_header("Origin") != self._origin():
                self._fail(HTTPStatus.FORBIDDEN, "origin", "mutation request Origin does not match the loopback origin")
                return False
            return True

        def _api_state(self, finalized: dict[str, Any] | None = None) -> dict[str, Any]:
            source_entries = [
                {"path": path, "contentUtf8": session.fixture_files[path]}
                for path in sorted(session.fixture_files)
            ]
            fixture_routes = {
                question["caseId"]: f"/fixture/index.html?case={question['fixtureState']}"
                for question in session.questionnaire["questions"]
                if question["authority"] == "acquisition"
            }
            value: dict[str, Any] = {
                "workbenchVersion": WORKBENCH_VERSION,
                "locked": session.locked,
                "state": session.state,
                "questionnaire": session.questionnaire,
                "sources": source_entries,
                "fixtureRoutes": fixture_routes,
                "maximumRequestBytes": MAX_HTTP_BODY_BYTES,
                "finalization": None,
            }
            if finalized is not None:
                value["finalization"] = {
                    "submissionDigest": finalized["submissionDigest"],
                    "evidenceStatus": finalized["evidenceStatus"],
                    "finalPath": str(session.final_path),
                    "comparisonArgv": [
                        "python3",
                        "tools/compare_web_review.py",
                        "--submission",
                        str(session.final_path),
                    ],
                }
            return value

        def do_GET(self) -> None:
            parsed = urlsplit(self.path)
            path = parsed.path
            if not self._valid_host():
                self._fail(HTTPStatus.BAD_REQUEST, "host", "request Host does not match the loopback origin")
                return
            if path == "/api/state":
                if not self._authorized_api(mutation=False):
                    return
                self._send(
                    HTTPStatus.OK,
                    canonical_bytes(self._api_state()),
                    "application/json; charset=utf-8",
                )
                return
            if path in ASSET_ROUTES:
                if parsed.query:
                    self._fail(HTTPStatus.BAD_REQUEST, "route", "workbench asset routes do not accept queries")
                    return
                filename, content_type = ASSET_ROUTES[path]
                raw = (ASSET_DIRECTORY / filename).read_bytes()
                self._send(HTTPStatus.OK, raw, content_type)
                return
            if path in FIXTURE_ROUTES:
                source_path, content_type = FIXTURE_ROUTES[path]
                if path == "/fixture/index.html":
                    query = parse_qs(parsed.query, strict_parsing=True)
                    states = {
                        question["fixtureState"]
                        for question in session.questionnaire["questions"]
                        if question["authority"] == "acquisition"
                    }
                    if set(query) != {"case"} or len(query["case"]) != 1 or query["case"][0] not in states:
                        self._fail(HTTPStatus.BAD_REQUEST, "route", "fixture route requires one admitted Harbor case")
                        return
                elif parsed.query:
                    self._fail(HTTPStatus.BAD_REQUEST, "route", "fixture asset routes do not accept queries")
                    return
                self._send(
                    HTTPStatus.OK,
                    session.fixture_files[source_path].encode("utf-8"),
                    content_type,
                    fixture=True,
                )
                return
            self._fail(HTTPStatus.NOT_FOUND, "route", "request route is not available")

        def _request_json(self) -> dict[str, Any]:
            if self.headers.get_all("Transfer-Encoding", failobj=[]):
                fail("http", "chunked or transformed request bodies are not accepted")
            if self._single_header("Content-Type") != "application/json":
                fail("content-type", "mutation requests require application/json")
            length_text = self._single_header("Content-Length")
            if length_text is None or not length_text.isascii() or not length_text.isdecimal():
                fail("http", "mutation requests require a decimal Content-Length")
            length = int(length_text)
            if length <= 0 or length > MAX_HTTP_BODY_BYTES:
                fail("request-budget", f"mutation body must be 1..{MAX_HTTP_BODY_BYTES} bytes")
            raw = self.rfile.read(length)
            if len(raw) != length:
                fail("http", "mutation request body ended before Content-Length")
            try:
                value = json.loads(raw, object_pairs_hook=unique_object, parse_constant=lambda value: fail("json", f"invalid JSON constant {value}"))
            except UnicodeDecodeError:
                fail("json", "mutation request must be UTF-8 JSON")
            except json.JSONDecodeError:
                fail("json", "mutation request is not valid JSON")
            return obj(value, "mutation request")

        def do_POST(self) -> None:
            parsed = urlsplit(self.path)
            if parsed.query or parsed.path not in {"/api/finalize", "/api/save"}:
                self._fail(HTTPStatus.NOT_FOUND, "route", "request route is not available")
                return
            if not self._authorized_api(mutation=True):
                return
            try:
                payload = self._request_json()
                if parsed.path == "/api/save":
                    state = session.save(payload)
                    response = {"locked": False, "state": state}
                else:
                    finalized = session.finalize(payload)
                    response = self._api_state(finalized)
            except ContractError as error:
                self._fail(HTTPStatus.BAD_REQUEST, error.category, str(error))
                return
            except OSError as error:
                self._fail(HTTPStatus.INTERNAL_SERVER_ERROR, "output", str(error))
                return
            self._send(HTTPStatus.OK, canonical_bytes(response), "application/json; charset=utf-8")
            if parsed.path == "/api/finalize":
                threading.Thread(target=self.server.shutdown, daemon=True).start()

        def _method_not_allowed(self) -> None:
            if not self._valid_host():
                self._fail(HTTPStatus.BAD_REQUEST, "host", "request Host does not match the loopback origin")
                return
            self._fail(HTTPStatus.METHOD_NOT_ALLOWED, "method", "request method is not allowed")

        do_CONNECT = _method_not_allowed
        do_DELETE = _method_not_allowed
        do_HEAD = _method_not_allowed
        do_OPTIONS = _method_not_allowed
        do_PATCH = _method_not_allowed
        do_PUT = _method_not_allowed
        do_TRACE = _method_not_allowed

    return WorkbenchHandler


def serve(session: WorkbenchSession, host: str, port: int) -> tuple[HTTPServer, str]:
    """Create the fixed loopback HTTP server and fragment-token launch URL."""
    if host != "127.0.0.1":
        fail("host", "workbench host must be exactly 127.0.0.1")
    if port < 0 or port > 65_535:
        fail("port", "workbench port must be between 0 and 65535")
    server = HTTPServer((host, port), handler_for(session))
    server.timeout = 1
    return server, f"http://127.0.0.1:{server.server_port}/#token={session.token}"
