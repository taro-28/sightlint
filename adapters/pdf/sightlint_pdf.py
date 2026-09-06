#!/usr/bin/env python3
"""Bounded local PDF page and link-annotation geometry adapter for SightLint."""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import os
import re
import subprocess
import sys
import warnings
from pathlib import Path, PurePosixPath
from typing import Any

try:
    import pypdf
    from pypdf import PdfReader
    from pypdf.errors import PdfReadError
    from pypdf.generic import (
        ArrayObject,
        DictionaryObject,
        IndirectObject,
        NameObject,
        NumberObject,
    )
except ImportError:  # Reported as a stable dependency error from main().
    pypdf = None  # type: ignore[assignment]
    PdfReader = None  # type: ignore[assignment,misc]
    PdfReadError = Exception  # type: ignore[assignment,misc]
    ArrayObject = DictionaryObject = IndirectObject = NameObject = NumberObject = ()  # type: ignore[assignment,misc]


PROTOCOL_VERSION = "0.1.0"
ADAPTER_VERSION = "0.1.0"
EXTENSION_VERSION = "0.1.0"
PYPDF_VERSION = "6.17.0"
MAX_REQUEST_BYTES = 1_048_576
TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
PDF_HEADER = re.compile(r"^%PDF-[0-9]+\.[0-9]+$")

BASE_UNSUPPORTED_FEATURES = {
    "attachments",
    "contentStreamsAndPaintGeometry",
    "formsAndWidgets",
    "metadata",
    "optionalContent",
    "signatures",
    "sourceTextAndReadingOrder",
    "taggedStructureInterpretation",
}

ACTION_KINDS = {
    "/GoTo": "goTo",
    "/URI": "uri",
    "/GoToR": "remoteGoTo",
    "/Launch": "launch",
    "/JavaScript": "javaScript",
    "/Named": "named",
    "/SubmitForm": "submitForm",
    "/ImportData": "importData",
}


class AdapterError(Exception):
    """Stable categorized adapter failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def fail(code: str, message: str) -> None:
    raise AdapterError(code, message)


def canonical_json(value: object) -> bytes:
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            allow_nan=False,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def exact_keys(
    value: dict[str, Any], required: set[str], optional: set[str], context: str
) -> None:
    missing = sorted(required - set(value))
    unknown = sorted(set(value) - required - optional)
    if missing:
        fail("request-invalid", f"{context} is missing {','.join(missing)}")
    if unknown:
        fail("request-invalid", f"{context} has unknown fields {','.join(unknown)}")


def record(value: object, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("request-invalid", f"{context} must be an object")
    return value


def text(value: object, context: str, maximum: int) -> str:
    if not isinstance(value, str) or not value or len(value) > maximum:
        fail(
            "request-invalid",
            f"{context} must be a nonempty string of at most {maximum} characters",
        )
    return value


def token(value: object, context: str) -> str:
    result = text(value, context, 128)
    if TOKEN.fullmatch(result) is None:
        fail("request-invalid", f"{context} is not a stable token")
    return result


def digest(value: object, context: str) -> str:
    result = text(value, context, 72)
    if DIGEST.fullmatch(result) is None:
        fail("request-invalid", f"{context} must be a lowercase SHA-256 digest")
    return result


def bounded_integer(
    value: object, context: str, minimum: int, maximum: int
) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not minimum <= value <= maximum
    ):
        fail(
            "request-invalid",
            f"{context} must be an integer from {minimum} through {maximum}",
        )
    return value


def reference(value: object, context: str, suffix: str) -> str:
    result = text(value, context, 1024)
    pure = PurePosixPath(result)
    if (
        pure.is_absolute()
        or "\\" in result
        or "\0" in result
        or any(part in {"", ".", ".."} for part in pure.parts)
        or pure.suffix.lower() != suffix
    ):
        fail(
            "request-invalid",
            f"{context} must be a normalized repository-relative {suffix} path",
        )
    return result


def parse_request(path: Path) -> dict[str, Any]:
    try:
        if path.stat().st_size > MAX_REQUEST_BYTES:
            fail("request-budget", "request exceeds 1048576 bytes")
        raw = path.read_bytes()
    except OSError as error:
        fail("request-io", f"cannot read request: {error.strerror or 'I/O error'}")
    if len(raw) > MAX_REQUEST_BYTES:
        fail("request-budget", "request exceeds 1048576 bytes")

    def no_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                fail("request-json", f"request contains duplicate field {key}")
            result[key] = value
        return result

    try:
        value = json.loads(raw, object_pairs_hook=no_duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError):
        fail("request-json", "request must be one valid UTF-8 JSON object")
    root = record(value, "request")
    exact_keys(
        root,
        {
            "protocolVersion",
            "requestId",
            "artifact",
            "input",
            "renders",
            "privacy",
            "execution",
        },
        set(),
        "request",
    )
    if root["protocolVersion"] != PROTOCOL_VERSION:
        fail(
            "request-version",
            f"request protocolVersion must be {PROTOCOL_VERSION}",
        )
    token(root["requestId"], "request.requestId")

    artifact = record(root["artifact"], "request.artifact")
    exact_keys(artifact, {"id"}, {"title"}, "request.artifact")
    token(artifact["id"], "request.artifact.id")
    if "title" in artifact:
        text(artifact["title"], "request.artifact.title", 256)

    source = record(root["input"], "request.input")
    exact_keys(source, {"reference", "sha256"}, set(), "request.input")
    reference(source["reference"], "request.input.reference", ".pdf")
    digest(source["sha256"], "request.input.sha256")

    renders = root["renders"]
    if not isinstance(renders, list) or len(renders) > 100:
        fail("request-invalid", "request.renders must be an array of at most 100 entries")
    render_indices: list[int] = []
    for index, render_value in enumerate(renders):
        context = f"request.renders[{index}]"
        render = record(render_value, context)
        exact_keys(
            render,
            {"pageIndex", "reference", "sha256", "pdfPointsPerPixel"},
            set(),
            context,
        )
        render_indices.append(
            bounded_integer(render["pageIndex"], f"{context}.pageIndex", 1, 100)
        )
        reference(render["reference"], f"{context}.reference", ".png")
        digest(render["sha256"], f"{context}.sha256")
        ratio = record(render["pdfPointsPerPixel"], f"{context}.pdfPointsPerPixel")
        exact_keys(ratio, {"numerator", "denominator"}, set(), f"{context}.pdfPointsPerPixel")
        bounded_integer(ratio["numerator"], f"{context}.pdfPointsPerPixel.numerator", 1, 14_400)
        bounded_integer(ratio["denominator"], f"{context}.pdfPointsPerPixel.denominator", 1, 14_400)
    if render_indices != sorted(render_indices) or len(render_indices) != len(
        set(render_indices)
    ):
        fail(
            "request-invalid",
            "request.renders must use unique ascending pageIndex values",
        )

    privacy = record(root["privacy"], "request.privacy")
    exact_keys(
        privacy,
        {"externalProcessing", "retention", "contentPolicy"},
        set(),
        "request.privacy",
    )
    if privacy != {
        "externalProcessing": False,
        "retention": "none",
        "contentPolicy": "geometryAndTypeOnly",
    }:
        fail(
            "request-privacy",
            "protocol 0.1.0 requires local geometry-and-type-only processing with no retention",
        )

    execution = record(root["execution"], "request.execution")
    limits = {
        "maxInputBytes": (1, 33_554_432),
        "maxRenderBytes": (1, 67_108_864),
        "maxObjects": (1, 50_000),
        "maxPages": (1, 100),
        "maxAnnotations": (1, 10_000),
        "maxAnnotationsPerPage": (1, 1_000),
        "maxOutputBytes": (1_024, 16_777_216),
    }
    exact_keys(execution, set(limits), set(), "request.execution")
    for name, (minimum, maximum) in limits.items():
        bounded_integer(
            execution[name], f"request.execution.{name}", minimum, maximum
        )
    return root


def local_file(root: Path, value: str, context: str) -> Path:
    try:
        candidate = (root / Path(*PurePosixPath(value).parts)).resolve(strict=True)
        candidate.relative_to(root)
    except (OSError, ValueError):
        fail(
            "path-boundary",
            f"{context} must resolve to a file below the repository root",
        )
    if not candidate.is_file():
        fail("path-boundary", f"{context} must resolve to a regular file")
    return candidate


def verify_digest(path: Path, expected: str, context: str, maximum: int) -> int:
    try:
        size = path.stat().st_size
        if size > maximum:
            fail("input-budget", f"{context} exceeds its request byte budget")
        hasher = hashlib.sha256()
        observed = 0
        with path.open("rb") as stream:
            while chunk := stream.read(65_536):
                observed += len(chunk)
                if observed > maximum:
                    fail("input-budget", f"{context} exceeds its request byte budget")
                hasher.update(chunk)
    except OSError as error:
        fail("input-io", f"cannot read {context}: {error.strerror or 'I/O error'}")
    if observed != size:
        fail("input-changed", f"{context} changed while it was read")
    if f"sha256:{hasher.hexdigest()}" != expected:
        fail("input-digest", f"{context} SHA-256 does not match the request")
    return size


def run_sightlint(
    binary: Path, arguments: list[str], *, stdin: bytes | None = None
) -> bytes:
    try:
        process = subprocess.run(
            [str(binary), *arguments],
            input=stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired):
        fail("sightlint-execution", "cannot complete the public SightLint command")
    if process.returncode != 0:
        diagnostic = process.stderr.decode("utf-8", errors="replace").strip()
        fail(
            "sightlint-rejected",
            diagnostic or "public SightLint command rejected adapter output",
        )
    return process.stdout


def png_canvas(binary: Path, render_path: Path) -> tuple[int, int, str]:
    output = run_sightlint(binary, ["adapt-image", str(render_path)])
    try:
        document = json.loads(output)
        canvas = document["canvases"][0]
        width_value = canvas["size"]["width"]
        height_value = canvas["size"]["height"]
        header_evidence = next(
            item
            for item in document["evidence"]
            if item["id"] == canvas["evidenceId"]
        )
        adapter_version = header_evidence["source"]["adapterVersion"]
    except (KeyError, IndexError, StopIteration, TypeError, json.JSONDecodeError):
        fail(
            "sightlint-output",
            "adapt-image returned an unexpected Artifact IR document",
        )
    if not isinstance(width_value, (int, float)) or not isinstance(
        height_value, (int, float)
    ):
        fail("sightlint-output", "adapt-image returned invalid canvas dimensions")
    if int(width_value) != width_value or int(height_value) != height_value:
        fail(
            "sightlint-output", "adapt-image returned nonintegral canvas dimensions"
        )
    return int(width_value), int(height_value), str(adapter_version)


def rendered_extent_coverage(render_count: int, page_count: int) -> str:
    if render_count == 0:
        return "untested"
    if render_count == page_count:
        return "observed"
    return "partial"


def indirect_reference(value: object, context: str) -> tuple[str, str]:
    if not isinstance(value, IndirectObject):
        fail("source-invalid", f"{context} must be an indirect PDF object")
    number = value.idnum
    generation = value.generation
    if number <= 0 or generation < 0:
        fail("source-invalid", f"{context} has an invalid indirect object reference")
    return f"{number} {generation} R", f"{number}-{generation}"


def integral_number(
    value: object,
    context: str,
    minimum: int = -1_000_000,
    maximum: int = 1_000_000,
) -> int:
    if isinstance(value, bool) or not isinstance(value, NumberObject):
        fail("source-invalid", f"{context} must be an integral PDF number")
    result = int(value)
    if not minimum <= result <= maximum:
        fail("source-invalid", f"{context} is outside the supported integer range")
    return result


def source_rect(value: object, context: str) -> dict[str, int]:
    if not isinstance(value, ArrayObject) or len(value) != 4:
        fail("source-invalid", f"{context} must be a four-number direct array")
    left, bottom, right, top = [
        integral_number(component, f"{context}[{index}]")
        for index, component in enumerate(value)
    ]
    if right <= left or top <= bottom:
        fail("source-invalid", f"{context} must be a nonempty ordered rectangle")
    return {"left": left, "bottom": bottom, "right": right, "top": top}


def normalized_rect(
    rectangle: dict[str, int], crop_box: dict[str, int]
) -> dict[str, int]:
    return {
        "x": rectangle["left"] - crop_box["left"],
        "y": crop_box["top"] - rectangle["top"],
        "width": rectangle["right"] - rectangle["left"],
        "height": rectangle["top"] - rectangle["bottom"],
    }


def raw_dictionary(value: object, context: str) -> DictionaryObject:
    try:
        resolved = value.get_object() if isinstance(value, IndirectObject) else value
    except Exception:
        fail("source-invalid", f"{context} cannot be resolved")
    if not isinstance(resolved, DictionaryObject):
        fail("source-invalid", f"{context} must be a PDF dictionary")
    return resolved


def action_kind(annotation: DictionaryObject) -> str:
    has_destination = "/Dest" in annotation
    has_action = "/A" in annotation
    if has_destination and not has_action:
        return "internalDestination"
    if not has_action:
        return "missing"
    if has_destination:
        return "other"
    action = raw_dictionary(annotation.raw_get("/A"), "annotation action")
    if "/S" not in action:
        return "other"
    kind = action.raw_get("/S")
    if not isinstance(kind, NameObject):
        fail("source-invalid", "annotation action subtype must be a PDF name")
    return ACTION_KINDS.get(str(kind), "other")


def annotation_geometry_status(
    *,
    page_exact: bool,
    subtype: str,
    flags: int,
    action: str,
    has_quad_points: bool,
    has_path: bool,
    rectangle: dict[str, int] | None,
    invalid_rectangle: bool = False,
) -> str:
    if not page_exact:
        return "unsupportedPage"
    if subtype != "/Link":
        return "unsupportedSubtype"
    if flags != 0:
        return "unsupportedFlags"
    if action != "internalDestination":
        return "unsupportedAction"
    if has_path:
        return "unsupportedPath"
    if has_quad_points:
        return "unsupportedQuadPoints"
    if invalid_rectangle:
        return "invalidRect"
    if rectangle is None:
        return "missingRect"
    return "exact"


def object_count(reader: Any) -> int:
    """Count the pinned reader's inventoried indirect-object identities."""
    identities = {
        (number, generation)
        for generation, offsets in reader.xref.items()
        for number in offsets
        if number > 0
    }
    identities.update((number, 0) for number in reader.xref_objStm if number > 0)
    return len(identities)


def collect_pages(
    reader: Any, root: DictionaryObject, maximum: int
) -> list[tuple[DictionaryObject, IndirectObject, int]]:
    """Walk the raw page tree without pypdf's inherited-attribute expansion."""
    try:
        root_reference = root.raw_get("/Pages")
    except (KeyError, TypeError):
        fail("pdf-invalid", "PDF catalog has no page tree")
    stack: list[tuple[object, int]] = [(root_reference, 0)]
    visited: set[tuple[int, int]] = set()
    pages: list[tuple[DictionaryObject, IndirectObject, int]] = []
    while stack:
        reference_value, inherited_rotation = stack.pop()
        if not isinstance(reference_value, IndirectObject):
            fail("source-invalid", "page tree entries must be indirect objects")
        identity = (reference_value.idnum, reference_value.generation)
        if identity in visited:
            fail("source-invalid", "page tree contains a repeated or cyclic object")
        visited.add(identity)
        node = raw_dictionary(reference_value, "page tree object")
        node_type = node.raw_get("/Type") if "/Type" in node else None
        if not isinstance(node_type, NameObject):
            fail("source-invalid", "page tree object has no valid Type")
        rotation = inherited_rotation
        if "/Rotate" in node:
            rotation = integral_number(
                node.raw_get("/Rotate"),
                "page tree Rotate",
                -3_600,
                3_600,
            )
        if str(node_type) == "/Page":
            pages.append((node, reference_value, rotation))
            if len(pages) > maximum:
                fail("page-budget", "PDF page count exceeds maxPages")
            continue
        if str(node_type) != "/Pages":
            fail("source-invalid", "page tree contains an unsupported object type")
        try:
            kids_value = node.raw_get("/Kids")
            kids = (
                kids_value.get_object()
                if isinstance(kids_value, IndirectObject)
                else kids_value
            )
        except Exception:
            fail("source-invalid", "page tree Kids cannot be resolved")
        if not isinstance(kids, ArrayObject) or not kids:
            fail("source-invalid", "page tree Kids must be a nonempty array")
        for child in reversed(kids):
            stack.append((child, rotation))
    if not pages:
        fail("pdf-invalid", "PDF contains no pages")
    return pages


def parser_runtime() -> dict[str, object]:
    return {
        "name": "sightlint-pdf",
        "version": ADAPTER_VERSION,
        "runtime": {
            "name": "python",
            "version": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        },
        "parser": {"name": "pypdf", "version": PYPDF_VERSION, "strict": True},
    }


def open_reader(source_path: Path) -> Any:
    if pypdf is None or PdfReader is None or pypdf.__version__ != PYPDF_VERSION:
        fail(
            "dependency-error",
            f"sightlint-pdf requires pypdf {PYPDF_VERSION}",
        )
    logger = logging.getLogger("pypdf")
    logger.setLevel(logging.CRITICAL + 1)
    warnings.filterwarnings("error")
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("error")
            reader = PdfReader(
                source_path,
                strict=True,
                root_object_recovery_limit=1,
            )
    except (OSError, PdfReadError, RuntimeError, ValueError, Warning):
        fail("pdf-invalid", "input is not a valid supported strict PDF")
    return reader


def adapt(
    request: dict[str, Any], repository_root: Path, binary: Path
) -> tuple[bytes, bytes]:
    source_path = local_file(
        repository_root, request["input"]["reference"], "input reference"
    )
    execution = request["execution"]
    verify_digest(
        source_path,
        request["input"]["sha256"],
        "PDF input",
        execution["maxInputBytes"],
    )
    renders_by_page: dict[int, tuple[dict[str, Any], Path]] = {}
    for render in request["renders"]:
        page_index = render["pageIndex"]
        render_path = local_file(
            repository_root,
            render["reference"],
            f"render for page {page_index}",
        )
        verify_digest(
            render_path,
            render["sha256"],
            f"render for page {page_index}",
            execution["maxRenderBytes"],
        )
        renders_by_page[page_index] = (render, render_path)

    reader = open_reader(source_path)
    if reader.is_encrypted:
        fail("pdf-encrypted", "encrypted PDF input is unsupported")
    if object_count(reader) > execution["maxObjects"]:
        fail("object-budget", "PDF object count exceeds maxObjects")
    try:
        root = reader.root_object
    except Exception:
        fail("pdf-invalid", "PDF catalog or page tree cannot be resolved")
    if not isinstance(root, DictionaryObject):
        fail("pdf-invalid", "PDF catalog is not a dictionary")
    pages = collect_pages(reader, root, execution["maxPages"])
    page_count = len(pages)
    if any(index > page_count for index in renders_by_page):
        fail("render-reference", "request references a page index that does not exist")
    header = str(reader.pdf_header)
    if PDF_HEADER.fullmatch(header) is None:
        fail("pdf-invalid", "PDF header version is unsupported")

    canvases: list[dict[str, object]] = []
    nodes: list[dict[str, object]] = []
    evidence: list[dict[str, object]] = []
    extension_pages: list[dict[str, object]] = []
    extension_annotations: list[dict[str, object]] = []
    unsupported_features = set(BASE_UNSUPPORTED_FEATURES)
    seen_references: set[str] = set()
    total_annotations = 0

    for page_index, (page, page_reference_value, inherited_rotation) in enumerate(
        pages, start=1
    ):
        try:
            page_reference, page_reference_token = indirect_reference(
                page_reference_value, f"page {page_index}"
            )
        except AdapterError:
            raise
        except Exception:
            fail("source-invalid", f"page {page_index} cannot be resolved")
        if page_reference in seen_references:
            fail("duplicate-object", "PDF repeats an indirect page/annotation reference")
        seen_references.add(page_reference)
        page_id = f"pdf:page:{page_reference_token}"
        page_evidence_id = f"e-pdf:page:{page_reference_token}:source"
        page_extension: dict[str, object] = {
            "id": page_id,
            "index": page_index,
            "objectReference": page_reference,
            "rotationDegrees": 0,
            "geometryStatus": "unsupportedPageBox",
            "sourceEvidenceId": page_evidence_id,
        }
        evidence.append(
            {
                "id": page_evidence_id,
                "class": "exactSource",
                "source": {
                    "adapter": "sightlint-pdf",
                    "adapterVersion": ADAPTER_VERSION,
                    "inputDigest": request["input"]["sha256"],
                    "externalProcessing": False,
                },
                "selector": {
                    "type": "nativeId",
                    "nativeId": f"pdf:{page_reference}",
                },
            }
        )

        try:
            rotation_value = (
                page.raw_get("/Rotate") if "/Rotate" in page else inherited_rotation
            )
            rotation = (
                0
                if rotation_value == 0
                else integral_number(rotation_value, f"page {page_index} Rotate")
            )
        except Exception:
            fail("source-invalid", f"page {page_index} has invalid rotation")
        page_extension["rotationDegrees"] = rotation
        page_exact = False
        crop_box: dict[str, int] | None = None
        if rotation != 0:
            page_extension["geometryStatus"] = "unsupportedRotation"
            unsupported_features.add("rotatedPageGeometry")
        elif "/MediaBox" not in page or "/CropBox" not in page:
            unsupported_features.add("inheritedOrMissingPageBox")
        else:
            try:
                media_box = source_rect(
                    page.raw_get("/MediaBox"), f"page {page_index} MediaBox"
                )
                crop_box = source_rect(
                    page.raw_get("/CropBox"), f"page {page_index} CropBox"
                )
            except AdapterError:
                unsupported_features.add("unsupportedPageBox")
            else:
                page_exact = True
                page_extension["geometryStatus"] = "exact"
                page_extension["mediaBoxPdfPoints"] = media_box
                page_extension["cropBoxPdfPoints"] = crop_box
                canvases.append(
                    {
                        "id": page_id,
                        "size": {
                            "width": crop_box["right"] - crop_box["left"],
                            "height": crop_box["top"] - crop_box["bottom"],
                        },
                        "unit": "pdfPoint",
                        "horizontalDirection": "right",
                        "verticalDirection": "down",
                        "evidenceId": page_evidence_id,
                    }
                )

        render_entry = renders_by_page.get(page_index)
        if render_entry is None:
            page_extension["render"] = {
                "status": "untested",
                "reason": "The request supplied no synchronized rendered page.",
                "nodeIdentity": "cantTell",
            }
        else:
            render, render_path = render_entry
            pixel_width, pixel_height, png_adapter_version = png_canvas(
                binary, render_path
            )
            render_canvas_id = f"{page_id}:render"
            render_evidence_id = f"e-pdf:page:{page_reference_token}:render"
            canvases.append(
                {
                    "id": render_canvas_id,
                    "size": {"width": pixel_width, "height": pixel_height},
                    "unit": "devicePixel",
                    "horizontalDirection": "right",
                    "verticalDirection": "down",
                    "evidenceId": render_evidence_id,
                }
            )
            evidence.append(
                {
                    "id": render_evidence_id,
                    "class": "exactRender",
                    "source": {
                        "adapter": "sightlint-adapter-png",
                        "adapterVersion": png_adapter_version,
                        "inputDigest": render["sha256"],
                        "externalProcessing": False,
                    },
                    "selector": {
                        "type": "nativeId",
                        "nativeId": f"IHDR:{render['reference']}",
                    },
                }
            )
            ratio = render["pdfPointsPerPixel"]
            agreement = (
                page_exact
                and crop_box is not None
                and pixel_width * ratio["numerator"]
                == (crop_box["right"] - crop_box["left"])
                * ratio["denominator"]
                and pixel_height * ratio["numerator"]
                == (crop_box["top"] - crop_box["bottom"])
                * ratio["denominator"]
            )
            page_extension["render"] = {
                "status": "observed",
                "reference": render["reference"],
                "sha256": render["sha256"],
                "widthPixels": pixel_width,
                "heightPixels": pixel_height,
                "pdfPointsPerPixel": ratio,
                "extentReconciliation": (
                    "agreement" if agreement else "conflict" if page_exact else "cantTell"
                ),
                "evidenceId": render_evidence_id,
                "nodeIdentity": "cantTell",
            }

        try:
            annotations_value = (
                page.raw_get("/Annots") if "/Annots" in page else ArrayObject()
            )
            annotations_value = (
                annotations_value.get_object()
                if isinstance(annotations_value, IndirectObject)
                else annotations_value
            )
        except Exception:
            fail("source-invalid", f"page {page_index} Annots cannot be resolved")
        if not isinstance(annotations_value, ArrayObject):
            fail("source-invalid", f"page {page_index} Annots must be an array")
        if len(annotations_value) > execution["maxAnnotationsPerPage"]:
            fail(
                "annotation-budget",
                f"page {page_index} annotation count exceeds maxAnnotationsPerPage",
            )
        total_annotations += len(annotations_value)
        if total_annotations > execution["maxAnnotations"]:
            fail("annotation-budget", "PDF annotation count exceeds maxAnnotations")

        for annotation_index, annotation_reference_value in enumerate(
            annotations_value, start=1
        ):
            if not isinstance(annotation_reference_value, IndirectObject):
                unsupported_features.add("directAnnotationDictionary")
                continue
            annotation_reference, annotation_reference_token = indirect_reference(
                annotation_reference_value,
                f"page {page_index} annotation {annotation_index}",
            )
            if annotation_reference in seen_references:
                fail("duplicate-object", "PDF repeats an indirect page/annotation reference")
            seen_references.add(annotation_reference)
            annotation = raw_dictionary(
                annotation_reference_value,
                f"page {page_index} annotation {annotation_reference}",
            )
            subtype_value = annotation.raw_get("/Subtype") if "/Subtype" in annotation else None
            if not isinstance(subtype_value, NameObject):
                fail("source-invalid", "annotation Subtype must be a PDF name")
            subtype = str(subtype_value)
            if re.fullmatch(r"/[A-Za-z0-9]+", subtype) is None:
                fail("source-invalid", "annotation Subtype is unsupported")
            flag_value = annotation.raw_get("/F") if "/F" in annotation else NumberObject(0)
            flags = integral_number(
                flag_value, "annotation flags", 0, 4_294_967_295
            )
            has_quad_points = "/QuadPoints" in annotation
            has_path = "/Path" in annotation
            action = action_kind(annotation)
            rectangle = None
            invalid_rectangle = False
            if "/Rect" in annotation:
                try:
                    rectangle = source_rect(
                        annotation.raw_get("/Rect"), "annotation Rect"
                    )
                except AdapterError:
                    invalid_rectangle = True
                    unsupported_features.add("invalidAnnotationRect")
            status = annotation_geometry_status(
                page_exact=page_exact,
                subtype=subtype,
                flags=flags,
                action=action,
                has_quad_points=has_quad_points,
                has_path=has_path,
                rectangle=rectangle,
                invalid_rectangle=invalid_rectangle,
            )
            annotation_id = f"pdf:annotation:{annotation_reference_token}"
            annotation_evidence_id = (
                f"e-pdf:annotation:{annotation_reference_token}:source"
            )
            extension_annotation: dict[str, object] = {
                "id": annotation_id,
                "pageId": page_id,
                "objectReference": annotation_reference,
                "subtype": subtype,
                "flags": flags,
                "hasQuadPoints": has_quad_points,
                "hasPath": has_path,
                "actionKind": action,
                "geometryStatus": status,
                "sourceEvidenceId": annotation_evidence_id,
            }
            if rectangle is not None:
                extension_annotation["sourceRectPdfPoints"] = rectangle
            evidence.append(
                {
                    "id": annotation_evidence_id,
                    "class": "exactSource",
                    "source": {
                        "adapter": "sightlint-pdf",
                        "adapterVersion": ADAPTER_VERSION,
                        "inputDigest": request["input"]["sha256"],
                        "externalProcessing": False,
                    },
                    "selector": {
                        "type": "nativeId",
                        "nativeId": f"pdf:{annotation_reference}",
                    },
                }
            )
            if status == "exact" and crop_box is not None and rectangle is not None:
                hit_box = normalized_rect(rectangle, crop_box)
                extension_annotation["normalizedHitBoxPdfPoints"] = hit_box
                extension_annotation["coreNodeId"] = annotation_id
                nodes.append(
                    {
                        "id": annotation_id,
                        "kind": {
                            "value": "control",
                            "evidenceId": annotation_evidence_id,
                        },
                        "coordinateSpaceId": page_id,
                        "role": {
                            "value": "link",
                            "evidenceId": annotation_evidence_id,
                        },
                        "geometry": {
                            "hitBox": {
                                "rect": hit_box,
                                "coordinateSpaceId": page_id,
                                "evidenceId": annotation_evidence_id,
                            }
                        },
                    }
                )
            else:
                unsupported_features.add(f"annotationGeometry:{status}")
            extension_annotations.append(extension_annotation)
        extension_pages.append(page_extension)

    if not canvases:
        fail("unsupported-document", "PDF produced no supported source or render canvas")
    if len(unsupported_features) > 128:
        fail("unsupported-budget", "distinct unsupported feature count exceeds 128")
    adapter = parser_runtime()
    extension = {
        "extensionVersion": EXTENSION_VERSION,
        "protocolVersion": PROTOCOL_VERSION,
        "inputSha256": request["input"]["sha256"],
        "pdfHeader": header,
        "adapter": adapter,
        "privacy": {**request["privacy"], "actionsFollowed": False},
        "taggedStructure": {
            "catalogDeclaresStructTreeRoot": (
                isinstance(root, DictionaryObject) and "/StructTreeRoot" in root
            ),
            "interpretation": "untested",
        },
        "pages": extension_pages,
        "annotations": sorted(extension_annotations, key=lambda item: str(item["id"])),
        "unsupportedFeatures": sorted(unsupported_features),
    }
    document = {
        "schemaVersion": "0.1.0",
        "artifact": {
            "id": request["artifact"]["id"],
            "kind": "pdf",
            "sourceName": request["input"]["reference"],
        },
        "canvases": sorted(canvases, key=lambda item: str(item["id"])),
        "nodes": sorted(nodes, key=lambda item: str(item["id"])),
        "evidence": sorted(evidence, key=lambda item: str(item["id"])),
        "extensions": {"org.sightlint.pdf": extension},
    }
    if "title" in request["artifact"]:
        document["artifact"]["title"] = request["artifact"]["title"]
    normalized = run_sightlint(binary, ["normalize", "-"], stdin=canonical_json(document))
    if len(normalized) > execution["maxOutputBytes"]:
        fail("output-budget", "canonical Artifact IR exceeds maxOutputBytes")
    response = {
        "protocolVersion": PROTOCOL_VERSION,
        "requestId": request["requestId"],
        "status": "partial",
        "adapter": {
            "name": adapter["name"],
            "version": adapter["version"],
            "runtime": adapter["runtime"],
            "parser": {
                "name": adapter["parser"]["name"],
                "version": adapter["parser"]["version"],
            },
        },
        "inputSha256": request["input"]["sha256"],
        "pageCount": page_count,
        "nodeCount": len(nodes),
        "renderCount": len(renders_by_page),
        "coverage": {
            "pageGeometry": "partial",
            "linkAnnotations": "partial",
            "taggedStructure": "untested",
            "sourceText": "untested",
            "paintGeometry": "untested",
            "renderedExtent": rendered_extent_coverage(
                len(renders_by_page), page_count
            ),
            "renderedNodeIdentity": "cantTell",
        },
        "externalProcessing": False,
        "limitations": [
            "Link activation rectangles are source hit geometry, not proof of visible styling or usable interaction.",
            "Text, reading order, tagged structure, content streams, paint, and ink geometry remain untested.",
            "Rendered node identity, clipping, occlusion, and viewer hit testing remain cantTell.",
            "QuadPoints, Path, rotation, inherited page boxes, actions, widgets, and non-Link annotations are not approximated.",
            "pypdf and Python are an untrusted process sensor; these limits are not an OS sandbox.",
        ],
    }
    response_bytes = canonical_json(response)
    if len(response_bytes) > execution["maxOutputBytes"]:
        fail("output-budget", "canonical response exceeds maxOutputBytes")
    return response_bytes, normalized


def output_path(value: str) -> Path:
    path = Path(value)
    if path.exists() or path.is_symlink():
        fail("output-collision", "artifact IR output already exists")
    if not path.parent.is_dir():
        fail("output-path", "artifact IR output parent directory does not exist")
    return path


def main() -> int:
    parser = argparse.ArgumentParser(prog="sightlint-pdf")
    parser.add_argument("--request", required=True)
    parser.add_argument("--repository-root", required=True)
    parser.add_argument("--sightlint-binary", required=True)
    parser.add_argument("--artifact-ir-out", required=True)
    try:
        arguments = parser.parse_args()
        repository_root = Path(arguments.repository_root).resolve(strict=True)
        if not repository_root.is_dir():
            fail("path-boundary", "repository root must be a directory")
        binary = Path(arguments.sightlint_binary).resolve(strict=True)
        if not binary.is_file() or not os.access(binary, os.X_OK):
            fail("sightlint-binary", "sightlint binary must be an executable file")
        destination = output_path(arguments.artifact_ir_out)
        request = parse_request(Path(arguments.request))
        response, artifact_ir = adapt(request, repository_root, binary)
        try:
            with destination.open("xb") as output:
                output.write(artifact_ir)
        except FileExistsError:
            fail("output-collision", "artifact IR output already exists")
        except OSError:
            fail("output-io", "cannot write artifact IR output")
        sys.stdout.buffer.write(response)
        return 0
    except AdapterError as error:
        sys.stderr.buffer.write(
            f"sightlint-pdf: {error.code}: {error.message}\n".encode("utf-8")
        )
        return 2
    except Exception:
        sys.stderr.buffer.write(
            b"sightlint-pdf: execution-error: adapter execution failed\n"
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
