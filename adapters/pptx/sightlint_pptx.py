#!/usr/bin/env python3
"""Bounded local PPTX source-geometry adapter for SightLint."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import posixpath
import re
import subprocess
import sys
import zipfile
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path, PurePosixPath
from typing import Any
from xml.etree import ElementTree


PROTOCOL_VERSION = "0.1.0"
ADAPTER_VERSION = "0.1.0"
EXTENSION_VERSION = "0.1.0"
MAX_REQUEST_BYTES = 1_048_576
MAX_SAFE_INTEGER = 9_007_199_254_740_991
TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
SLIDE_PART = re.compile(r"^ppt/slides/slide[1-9][0-9]*\.xml$")

P = "http://schemas.openxmlformats.org/presentationml/2006/main"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG_R = "http://schemas.openxmlformats.org/package/2006/relationships"
CT = "http://schemas.openxmlformats.org/package/2006/content-types"
REL_SLIDE = f"{R}/slide"
REL_OFFICE_DOCUMENT = f"{R}/officeDocument"
PRESENTATION_CONTENT_TYPE = "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"


class AdapterError(Exception):
    """Stable categorized adapter failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


@dataclass(frozen=True)
class Matrix:
    sx: Fraction = Fraction(1)
    sy: Fraction = Fraction(1)
    tx: Fraction = Fraction(0)
    ty: Fraction = Fraction(0)


def fail(code: str, message: str) -> None:
    raise AdapterError(code, message)


def canonical_json(value: object, *, pretty: bool = False) -> bytes:
    options: dict[str, object] = {
        "ensure_ascii": False,
        "sort_keys": True,
        "allow_nan": False,
    }
    if pretty:
        options["indent"] = 2
    else:
        options["separators"] = (",", ":")
    return (json.dumps(value, **options) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return f"sha256:{hashlib.sha256(value).hexdigest()}"


def exact_keys(value: dict[str, Any], required: set[str], optional: set[str], context: str) -> None:
    keys = set(value)
    missing = sorted(required - keys)
    unknown = sorted(keys - required - optional)
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
        fail("request-invalid", f"{context} must be a nonempty string of at most {maximum} characters")
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


def bounded_integer(value: object, context: str, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not minimum <= value <= maximum:
        fail("request-invalid", f"{context} must be an integer from {minimum} through {maximum}")
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
        fail("request-invalid", f"{context} must be a normalized repository-relative {suffix} path")
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
    exact_keys(root, {"protocolVersion", "requestId", "artifact", "input", "renders", "privacy", "execution"}, set(), "request")
    if root["protocolVersion"] != PROTOCOL_VERSION:
        fail("request-version", f"request protocolVersion must be {PROTOCOL_VERSION}")
    token(root["requestId"], "request.requestId")

    artifact = record(root["artifact"], "request.artifact")
    exact_keys(artifact, {"id"}, {"title"}, "request.artifact")
    token(artifact["id"], "request.artifact.id")
    if "title" in artifact:
        text(artifact["title"], "request.artifact.title", 256)

    source = record(root["input"], "request.input")
    exact_keys(source, {"reference", "sha256"}, set(), "request.input")
    reference(source["reference"], "request.input.reference", ".pptx")
    digest(source["sha256"], "request.input.sha256")

    renders = root["renders"]
    if not isinstance(renders, list) or len(renders) > 100:
        fail("request-invalid", "request.renders must be an array of at most 100 entries")
    render_indices: list[int] = []
    for index, item_value in enumerate(renders):
        item = record(item_value, f"request.renders[{index}]")
        exact_keys(item, {"slideIndex", "reference", "sha256", "emuPerPixel"}, set(), f"request.renders[{index}]")
        render_indices.append(bounded_integer(item["slideIndex"], f"request.renders[{index}].slideIndex", 1, 100))
        reference(item["reference"], f"request.renders[{index}].reference", ".png")
        digest(item["sha256"], f"request.renders[{index}].sha256")
        bounded_integer(item["emuPerPixel"], f"request.renders[{index}].emuPerPixel", 1, 914_400)
    if render_indices != sorted(render_indices) or len(render_indices) != len(set(render_indices)):
        fail("request-invalid", "request.renders must use unique ascending slideIndex values")

    privacy = record(root["privacy"], "request.privacy")
    exact_keys(privacy, {"externalProcessing", "retention", "textPolicy"}, set(), "request.privacy")
    if privacy != {"externalProcessing": False, "retention": "none", "textPolicy": "digestOnly"}:
        fail("request-privacy", "protocol 0.1.0 requires local digest-only processing with no retention")

    execution = record(root["execution"], "request.execution")
    limits = {
        "maxArchiveBytes": (1, 67_108_864),
        "maxRenderBytes": (1, 67_108_864),
        "maxEntries": (1, 2_048),
        "maxExpandedBytes": (1, 134_217_728),
        "maxXmlBytes": (1, 8_388_608),
        "maxCompressionRatio": (1, 100),
        "maxSlides": (1, 100),
        "maxNodes": (1, 10_000),
        "maxGroupDepth": (1, 32),
        "maxOutputBytes": (1_024, 16_777_216),
    }
    exact_keys(execution, set(limits), set(), "request.execution")
    for name, (minimum, maximum) in limits.items():
        bounded_integer(execution[name], f"request.execution.{name}", minimum, maximum)
    return root


def local_file(root: Path, value: str, context: str) -> Path:
    try:
        candidate = (root / Path(*PurePosixPath(value).parts)).resolve(strict=True)
        candidate.relative_to(root)
    except (OSError, ValueError):
        fail("path-boundary", f"{context} must resolve to a file below the repository root")
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


def safe_member_name(name: str) -> str:
    if "\\" in name or "\0" in name or name.startswith("/"):
        fail("archive-member", "archive contains an unsafe member name")
    pure = PurePosixPath(name)
    if not pure.parts or any(part in {"", ".", ".."} for part in pure.parts):
        fail("archive-member", "archive contains an unsafe member name")
    return pure.as_posix()


def inventory(archive: zipfile.ZipFile, execution: dict[str, int]) -> dict[str, zipfile.ZipInfo]:
    infos = archive.infolist()
    if len(infos) > execution["maxEntries"]:
        fail("archive-budget", "archive member count exceeds the request budget")
    members: dict[str, zipfile.ZipInfo] = {}
    expanded = 0
    for info in infos:
        name = safe_member_name(info.filename.rstrip("/"))
        if name in members:
            fail("archive-member", "archive contains duplicate normalized member names")
        if info.flag_bits & 1:
            fail("archive-encryption", "encrypted archive members are unsupported")
        if info.compress_type not in {zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED}:
            fail("archive-compression", "archive member uses unsupported compression")
        if info.file_size < 0 or info.compress_size < 0:
            fail("archive-member", "archive member size is invalid")
        expanded += info.file_size
        if expanded > execution["maxExpandedBytes"]:
            fail("archive-budget", "declared expanded archive size exceeds the request budget")
        if info.file_size > 0 and (
            info.compress_size == 0
            or info.file_size > info.compress_size * execution["maxCompressionRatio"]
        ):
            fail("archive-ratio", "archive member compression ratio exceeds the request budget")
        members[name] = info
    return members


def xml_part(archive: zipfile.ZipFile, members: dict[str, zipfile.ZipInfo], name: str, maximum: int) -> ElementTree.Element:
    info = members.get(name)
    if info is None or info.is_dir():
        fail("package-part", f"required package part is missing: {name}")
    if info.file_size > maximum:
        fail("xml-budget", f"XML part exceeds the request budget: {name}")
    try:
        raw = archive.read(info)
    except (OSError, RuntimeError, zipfile.BadZipFile):
        fail("archive-read", f"cannot read package part: {name}")
    upper = raw.upper()
    if b"<!DOCTYPE" in upper or b"<!ENTITY" in upper:
        fail("xml-entity", f"DTD or entity declarations are forbidden: {name}")
    try:
        return ElementTree.fromstring(raw)
    except ElementTree.ParseError:
        fail("xml-invalid", f"package part is not valid XML: {name}")


def attribute_integer(
    element: ElementTree.Element | None,
    name: str,
    context: str,
    *,
    positive: bool = False,
    maximum: int = MAX_SAFE_INTEGER,
) -> int:
    if element is None or name not in element.attrib:
        fail("source-invalid", f"{context} is missing {name}")
    try:
        value = int(element.attrib[name], 10)
    except ValueError:
        fail("source-invalid", f"{context}.{name} must be an integer")
    minimum = 1 if positive else -MAX_SAFE_INTEGER
    if not minimum <= value <= maximum:
        fail("source-invalid", f"{context}.{name} is outside the supported integer range")
    return value


def relationships(root: ElementTree.Element, context: str) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    if root.tag != f"{{{PKG_R}}}Relationships":
        fail("source-invalid", f"{context} has an unsupported root element")
    for item in root.findall(f"{{{PKG_R}}}Relationship"):
        identifier = item.attrib.get("Id", "")
        relation_type = item.attrib.get("Type", "")
        target = item.attrib.get("Target", "")
        if not identifier or not relation_type or not target or identifier in result:
            fail("source-invalid", f"{context} has an invalid relationship")
        result[identifier] = {
            "type": relation_type,
            "target": target,
            "mode": item.attrib.get("TargetMode", "Internal"),
        }
    return result


def resolve_part(base: str, target: str, context: str) -> str:
    if "\\" in target or "\0" in target or target.startswith("/"):
        fail("source-invalid", f"{context} target is not a safe package-relative path")
    resolved = posixpath.normpath(posixpath.join(posixpath.dirname(base), target))
    if resolved.startswith("../") or resolved == "..":
        fail("source-invalid", f"{context} target escapes the package root")
    return safe_member_name(resolved)


def fraction_number(value: Fraction) -> int | float:
    if value.denominator == 1:
        return value.numerator
    result = float(value)
    if not math.isfinite(result):
        fail("geometry-range", "derived geometry is not finite")
    return result


def transform_rect(matrix: Matrix, x: int, y: int, width: int, height: int) -> dict[str, int | float]:
    return {
        "x": fraction_number(matrix.tx + matrix.sx * x),
        "y": fraction_number(matrix.ty + matrix.sy * y),
        "width": fraction_number(matrix.sx * width),
        "height": fraction_number(matrix.sy * height),
    }


def xfrm(element: ElementTree.Element | None, context: str, *, group: bool) -> tuple[str, dict[str, int] | None]:
    if element is None:
        return "missing", None
    if element.attrib.get("rot") not in {None, "0"} or any(
        element.attrib.get(name) not in {None, "0", "false"} for name in ("flipH", "flipV")
    ):
        return "unsupportedTransform", None
    off = element.find(f"{{{A}}}off")
    ext = element.find(f"{{{A}}}ext")
    if off is None or ext is None:
        return "missing", None
    values = {
        "x": attribute_integer(off, "x", f"{context}.off"),
        "y": attribute_integer(off, "y", f"{context}.off"),
        "width": attribute_integer(ext, "cx", f"{context}.ext"),
        "height": attribute_integer(ext, "cy", f"{context}.ext"),
    }
    if values["width"] < 0 or values["height"] < 0:
        fail("source-invalid", f"{context} extents must be nonnegative")
    if group:
        child_off = element.find(f"{{{A}}}chOff")
        child_ext = element.find(f"{{{A}}}chExt")
        if child_off is None or child_ext is None:
            return "missing", None
        values.update({
            "childX": attribute_integer(child_off, "x", f"{context}.chOff"),
            "childY": attribute_integer(child_off, "y", f"{context}.chOff"),
            "childWidth": attribute_integer(child_ext, "cx", f"{context}.chExt", positive=True),
            "childHeight": attribute_integer(child_ext, "cy", f"{context}.chExt", positive=True),
        })
    return "exact", values


def child_matrix(parent: Matrix, transform: dict[str, int]) -> Matrix:
    scale_x = Fraction(transform["width"], transform["childWidth"])
    scale_y = Fraction(transform["height"], transform["childHeight"])
    return Matrix(
        sx=parent.sx * scale_x,
        sy=parent.sy * scale_y,
        tx=parent.tx + parent.sx * (transform["x"] - scale_x * transform["childX"]),
        ty=parent.ty + parent.sy * (transform["y"] - scale_y * transform["childY"]),
    )


def text_record(shape: ElementTree.Element) -> dict[str, object]:
    body = shape.find(f"{{{P}}}txBody")
    if body is None:
        return {"status": "absent"}
    paragraphs = []
    for paragraph in body.findall(f"{{{A}}}p"):
        paragraphs.append("".join((item.text or "") for item in paragraph.iter(f"{{{A}}}t")))
    raw = "\n".join(paragraphs).encode("utf-8")
    return {"status": "digestOnly", "sha256": sha256_bytes(raw), "utf8Bytes": len(raw)}


def placeholder_record(shape: ElementTree.Element) -> dict[str, object] | None:
    placeholder = shape.find(f"{{{P}}}nvSpPr/{{{P}}}nvPr/{{{P}}}ph")
    if placeholder is None:
        return None
    result: dict[str, object] = {}
    if "type" in placeholder.attrib:
        placeholder_type = placeholder.attrib["type"]
        if not placeholder_type or len(placeholder_type) > 64:
            fail("source-invalid", "placeholder.type must contain 1 through 64 characters")
        result["type"] = placeholder_type
    if "idx" in placeholder.attrib:
        result["index"] = attribute_integer(placeholder, "idx", "placeholder", maximum=4_294_967_295)
    return result


def native_id(element: ElementTree.Element, native_type: str, context: str) -> int:
    path = f"{{{P}}}nvSpPr/{{{P}}}cNvPr" if native_type == "shape" else f"{{{P}}}nvGrpSpPr/{{{P}}}cNvPr"
    return attribute_integer(element.find(path), "id", context, positive=True, maximum=4_294_967_295)


def parse_slide_nodes(
    slide: ElementTree.Element,
    slide_identifier: str,
    slide_part: str,
    canvas_id: str,
    maximum_nodes: int,
    maximum_depth: int,
) -> tuple[list[dict[str, object]], list[dict[str, object]], list[dict[str, object]], set[str]]:
    tree = slide.find(f"{{{P}}}cSld/{{{P}}}spTree")
    if tree is None:
        fail("source-invalid", f"slide has no shape tree: {slide_part}")
    core_nodes: list[dict[str, object]] = []
    extension_nodes: list[dict[str, object]] = []
    evidence: list[dict[str, object]] = []
    unsupported: set[str] = set()
    identifiers: set[int] = set()

    def visit(parent: ElementTree.Element, parent_id: str | None, matrix: Matrix | None, depth: int) -> None:
        if depth > maximum_depth:
            fail("group-depth", f"group hierarchy exceeds maxGroupDepth on {slide_part}")
        z_order = 0
        for element in list(parent):
            if z_order > 10_000:
                fail("z-order-budget", "shape-tree z-order exceeds the supported range")
            local_name = element.tag.rsplit("}", 1)[-1]
            if local_name in {"nvGrpSpPr", "grpSpPr", "extLst"}:
                continue
            if local_name not in {"sp", "grpSp"}:
                unsupported.add(
                    f"unsupportedObject:{local_name}" if len(local_name) <= 128 else "unsupportedObject:other"
                )
                if len(unsupported) > 128:
                    fail("unsupported-budget", "distinct unsupported feature count exceeds 128")
                z_order += 1
                continue
            native_type = "shape" if local_name == "sp" else "group"
            item_native_id = native_id(element, native_type, f"{slide_part}:{native_type}")
            if item_native_id in identifiers:
                fail("duplicate-native-id", f"slide contains duplicate cNvPr id {item_native_id}: {slide_part}")
            identifiers.add(item_native_id)
            node_id = f"pptx:slide:{slide_identifier}:node:{item_native_id}"
            evidence_id = f"e-pptx:slide:{slide_identifier}:node:{item_native_id}"
            if len(core_nodes) >= maximum_nodes:
                fail("node-budget", "mapped node count exceeds maxNodes")
            transform_element = (
                element.find(f"{{{P}}}spPr/{{{A}}}xfrm")
                if native_type == "shape"
                else element.find(f"{{{P}}}grpSpPr/{{{A}}}xfrm")
            )
            geometry_status, transform = xfrm(transform_element, node_id, group=native_type == "group")
            if matrix is None and geometry_status == "exact":
                geometry_status = "unsupportedTransform"
            geometry: dict[str, object] = {}
            if geometry_status == "exact" and transform is not None and matrix is not None:
                geometry = {
                    "layoutBox": {
                        "rect": transform_rect(matrix, transform["x"], transform["y"], transform["width"], transform["height"]),
                        "coordinateSpaceId": canvas_id,
                        "evidenceId": evidence_id,
                    }
                }
            elif geometry_status == "unsupportedTransform":
                unsupported.add("unsupportedTransform")
            core_node: dict[str, object] = {
                "id": node_id,
                "kind": {"value": "shape" if native_type == "shape" else "container", "evidenceId": evidence_id},
                "coordinateSpaceId": canvas_id,
                "geometry": geometry,
            }
            if parent_id is not None:
                core_node["parentId"] = parent_id
            core_nodes.append(core_node)
            evidence.append({
                "id": evidence_id,
                "class": "exactSource",
                "source": {
                    "adapter": "sightlint-pptx",
                    "adapterVersion": ADAPTER_VERSION,
                    "inputDigest": "__INPUT_DIGEST__",
                    "externalProcessing": False,
                },
                "selector": {"type": "nativeId", "nativeId": f"pptx:{slide_part}#cNvPr={item_native_id}"},
            })
            extension_node: dict[str, object] = {
                "id": node_id,
                "slideId": canvas_id,
                "nativeId": item_native_id,
                "nativeType": native_type,
                "zOrder": z_order,
                "geometryStatus": geometry_status,
                "sourceEvidenceId": evidence_id,
                "text": text_record(element) if native_type == "shape" else {"status": "absent"},
            }
            if parent_id is not None:
                extension_node["parentId"] = parent_id
            if native_type == "shape":
                placeholder = placeholder_record(element)
                if placeholder is not None:
                    extension_node["placeholder"] = placeholder
            extension_nodes.append(extension_node)
            if native_type == "group":
                next_matrix = None
                if geometry_status == "exact" and transform is not None and matrix is not None:
                    next_matrix = child_matrix(matrix, transform)
                visit(element, node_id, next_matrix, depth + 1)
            z_order += 1

    visit(tree, None, Matrix(), 1)
    core_nodes.sort(key=lambda item: str(item["id"]))
    extension_nodes.sort(key=lambda item: str(item["id"]))
    evidence.sort(key=lambda item: str(item["id"]))
    return core_nodes, extension_nodes, evidence, unsupported


def run_sightlint(binary: Path, arguments: list[str], *, stdin: bytes | None = None) -> bytes:
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
        fail("sightlint-rejected", diagnostic or "public SightLint command rejected adapter output")
    return process.stdout


def png_canvas(binary: Path, render_path: Path) -> tuple[int, int, str]:
    output = run_sightlint(binary, ["adapt-image", str(render_path)])
    try:
        document = json.loads(output)
        canvas = document["canvases"][0]
        width_value = canvas["size"]["width"]
        height_value = canvas["size"]["height"]
        header_evidence = next(item for item in document["evidence"] if item["id"] == canvas["evidenceId"])
        adapter_version = header_evidence["source"]["adapterVersion"]
    except (KeyError, IndexError, StopIteration, TypeError, json.JSONDecodeError):
        fail("sightlint-output", "adapt-image returned an unexpected Artifact IR document")
    if not isinstance(width_value, (int, float)) or not isinstance(height_value, (int, float)):
        fail("sightlint-output", "adapt-image returned invalid canvas dimensions")
    if int(width_value) != width_value or int(height_value) != height_value:
        fail("sightlint-output", "adapt-image returned nonintegral canvas dimensions")
    return int(width_value), int(height_value), str(adapter_version)


def rendered_extent_coverage(render_count: int, slide_count: int) -> str:
    if render_count == 0:
        return "untested"
    if render_count == slide_count:
        return "observed"
    return "partial"


def adapt(request: dict[str, Any], repository_root: Path, binary: Path) -> tuple[bytes, bytes]:
    source_path = local_file(repository_root, request["input"]["reference"], "input reference")
    execution = request["execution"]
    verify_digest(source_path, request["input"]["sha256"], "PPTX input", execution["maxArchiveBytes"])
    renders_by_slide: dict[int, tuple[dict[str, Any], Path]] = {}
    for render in request["renders"]:
        render_path = local_file(repository_root, render["reference"], f"render for slide {render['slideIndex']}")
        verify_digest(
            render_path,
            render["sha256"],
            f"render for slide {render['slideIndex']}",
            execution["maxRenderBytes"],
        )
        renders_by_slide[render["slideIndex"]] = (render, render_path)

    try:
        archive = zipfile.ZipFile(source_path, "r")
    except (OSError, zipfile.BadZipFile, zipfile.LargeZipFile):
        fail("archive-invalid", "input is not a valid supported ZIP package")
    with archive:
        members = inventory(archive, execution)
        content_types = xml_part(archive, members, "[Content_Types].xml", execution["maxXmlBytes"])
        presentation_overrides = [
            item
            for item in content_types.findall(f"{{{CT}}}Override")
            if item.attrib.get("PartName") == "/ppt/presentation.xml"
        ]
        if (
            len(presentation_overrides) != 1
            or presentation_overrides[0].attrib.get("ContentType") != PRESENTATION_CONTENT_TYPE
        ):
            fail("package-type", "package is not a macro-free PPTX presentation")
        root_relations = relationships(xml_part(archive, members, "_rels/.rels", execution["maxXmlBytes"]), "package relationships")
        office_relations = [item for item in root_relations.values() if item["type"] == REL_OFFICE_DOCUMENT]
        if len(office_relations) != 1 or office_relations[0]["mode"] != "Internal":
            fail("package-type", "package must have one internal Office presentation relationship")
        root_target = office_relations[0]["target"]
        if root_target.startswith("/"):
            root_target = root_target[1:]
        if safe_member_name(posixpath.normpath(root_target)) != "ppt/presentation.xml":
            fail("package-type", "Office presentation relationship target is unsupported")

        presentation = xml_part(archive, members, "ppt/presentation.xml", execution["maxXmlBytes"])
        if presentation.tag != f"{{{P}}}presentation":
            fail("source-invalid", "presentation part has an unsupported root element")
        size = presentation.find(f"{{{P}}}sldSz")
        width = attribute_integer(size, "cx", "presentation.slideSize", positive=True)
        height = attribute_integer(size, "cy", "presentation.slideSize", positive=True)
        presentation_relations = relationships(
            xml_part(archive, members, "ppt/_rels/presentation.xml.rels", execution["maxXmlBytes"]),
            "presentation relationships",
        )
        slide_list = presentation.find(f"{{{P}}}sldIdLst")
        slide_items = [] if slide_list is None else slide_list.findall(f"{{{P}}}sldId")
        if not slide_items:
            fail("source-invalid", "presentation contains no slides")
        if len(slide_items) > execution["maxSlides"]:
            fail("slide-budget", "slide count exceeds maxSlides")
        if any(index > len(slide_items) for index in renders_by_slide):
            fail("render-reference", "request references a slide index that does not exist")

        all_core_nodes: list[dict[str, object]] = []
        all_extension_nodes: list[dict[str, object]] = []
        all_evidence: list[dict[str, object]] = []
        canvases: list[dict[str, object]] = []
        extension_slides: list[dict[str, object]] = []
        unsupported_features: set[str] = {"masterAndLayoutObjects", "themeResolvedStyles"}
        seen_slide_ids: set[int] = set()
        seen_slide_parts: set[str] = set()

        for slide_index, slide_item in enumerate(slide_items, start=1):
            slide_native_id = attribute_integer(
                slide_item,
                "id",
                f"presentation.slide[{slide_index}]",
                positive=True,
                maximum=4_294_967_295,
            )
            if slide_native_id in seen_slide_ids:
                fail("duplicate-slide-id", "presentation contains duplicate slide IDs")
            seen_slide_ids.add(slide_native_id)
            relation_id = slide_item.attrib.get(f"{{{R}}}id", "")
            relation = presentation_relations.get(relation_id)
            if relation is None or relation["type"] != REL_SLIDE:
                fail("slide-relationship", f"slide {slide_index} has no valid slide relationship")
            if relation["mode"] != "Internal":
                fail("slide-relationship", f"slide {slide_index} uses an external relationship")
            slide_part = resolve_part("ppt/presentation.xml", relation["target"], f"slide {slide_index} relationship")
            if SLIDE_PART.fullmatch(slide_part) is None:
                fail("slide-relationship", f"slide {slide_index} target is outside the supported slide part pattern")
            if slide_part in seen_slide_parts:
                fail("duplicate-slide-part", "presentation references the same slide part more than once")
            seen_slide_parts.add(slide_part)
            slide = xml_part(archive, members, slide_part, execution["maxXmlBytes"])
            if slide.tag != f"{{{P}}}sld":
                fail("source-invalid", f"slide part has an unsupported root element: {slide_part}")
            slide_identifier = str(slide_native_id)
            canvas_id = f"pptx:slide:{slide_identifier}"
            canvas_evidence_id = f"e-pptx:slide:{slide_identifier}:canvas"
            canvases.append({
                "id": canvas_id,
                "size": {"width": width, "height": height},
                "unit": "emu",
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": canvas_evidence_id,
            })
            all_evidence.append({
                "id": canvas_evidence_id,
                "class": "exactSource",
                "source": {
                    "adapter": "sightlint-pptx",
                    "adapterVersion": ADAPTER_VERSION,
                    "inputDigest": request["input"]["sha256"],
                    "externalProcessing": False,
                },
                "selector": {"type": "nativeId", "nativeId": f"pptx:ppt/presentation.xml#sldId={slide_native_id}"},
            })
            core_nodes, extension_nodes, node_evidence, unsupported = parse_slide_nodes(
                slide,
                slide_identifier,
                slide_part,
                canvas_id,
                execution["maxNodes"] - len(all_core_nodes),
                execution["maxGroupDepth"],
            )
            for evidence in node_evidence:
                evidence["source"]["inputDigest"] = request["input"]["sha256"]  # type: ignore[index]
            all_core_nodes.extend(core_nodes)
            all_extension_nodes.extend(extension_nodes)
            all_evidence.extend(node_evidence)
            unsupported_features.update(unsupported)
            if len(unsupported_features) > 128:
                fail("unsupported-budget", "distinct unsupported feature count exceeds 128")
            slide_extension: dict[str, object] = {
                "id": canvas_id,
                "index": slide_index,
                "part": slide_part,
                "widthEmu": width,
                "heightEmu": height,
                "sourceEvidenceId": canvas_evidence_id,
            }
            render_entry = renders_by_slide.get(slide_index)
            if render_entry is None:
                slide_extension["render"] = {
                    "status": "untested",
                    "reason": "The request supplied no synchronized rendered slide.",
                    "nodeIdentity": "cantTell",
                }
            else:
                render, render_path = render_entry
                pixel_width, pixel_height, png_adapter_version = png_canvas(binary, render_path)
                render_canvas_id = f"{canvas_id}:render"
                render_evidence_id = f"e-pptx:slide:{slide_identifier}:render"
                canvases.append({
                    "id": render_canvas_id,
                    "size": {"width": pixel_width, "height": pixel_height},
                    "unit": "devicePixel",
                    "horizontalDirection": "right",
                    "verticalDirection": "down",
                    "evidenceId": render_evidence_id,
                })
                all_evidence.append({
                    "id": render_evidence_id,
                    "class": "exactRender",
                    "source": {
                        "adapter": "sightlint-adapter-png",
                        "adapterVersion": png_adapter_version,
                        "inputDigest": render["sha256"],
                        "externalProcessing": False,
                    },
                    "selector": {"type": "nativeId", "nativeId": f"IHDR:{render['reference']}"},
                })
                agreement = (
                    pixel_width * render["emuPerPixel"] == width
                    and pixel_height * render["emuPerPixel"] == height
                )
                slide_extension["render"] = {
                    "status": "observed",
                    "reference": render["reference"],
                    "sha256": render["sha256"],
                    "widthPixels": pixel_width,
                    "heightPixels": pixel_height,
                    "emuPerPixel": render["emuPerPixel"],
                    "extentReconciliation": "agreement" if agreement else "conflict",
                    "evidenceId": render_evidence_id,
                    "nodeIdentity": "cantTell",
                }
            extension_slides.append(slide_extension)

    if len(all_core_nodes) > execution["maxNodes"]:
        fail("node-budget", "mapped node count exceeds maxNodes")
    runtime = {"name": "python", "version": platform_version()}
    extension = {
        "extensionVersion": EXTENSION_VERSION,
        "protocolVersion": PROTOCOL_VERSION,
        "inputSha256": request["input"]["sha256"],
        "adapter": {"name": "sightlint-pptx", "version": ADAPTER_VERSION, "runtime": runtime},
        "privacy": request["privacy"],
        "slides": extension_slides,
        "nodes": sorted(all_extension_nodes, key=lambda item: str(item["id"])),
        "unsupportedFeatures": sorted(unsupported_features),
    }
    document = {
        "schemaVersion": "0.1.0",
        "artifact": {
            "id": request["artifact"]["id"],
            "kind": "slide",
            "sourceName": request["input"]["reference"],
        },
        "canvases": sorted(canvases, key=lambda item: str(item["id"])),
        "nodes": sorted(all_core_nodes, key=lambda item: str(item["id"])),
        "evidence": sorted(all_evidence, key=lambda item: str(item["id"])),
        "extensions": {"org.sightlint.pptx": extension},
    }
    if "title" in request["artifact"]:
        document["artifact"]["title"] = request["artifact"]["title"]
    candidate = canonical_json(document)
    normalized = run_sightlint(binary, ["normalize", "-"], stdin=candidate)
    if len(normalized) > execution["maxOutputBytes"]:
        fail("output-budget", "canonical Artifact IR exceeds maxOutputBytes")
    response = {
        "protocolVersion": PROTOCOL_VERSION,
        "requestId": request["requestId"],
        "status": "partial",
        "adapter": {"name": "sightlint-pptx", "version": ADAPTER_VERSION, "runtime": runtime},
        "inputSha256": request["input"]["sha256"],
        "slideCount": len(extension_slides),
        "nodeCount": len(all_core_nodes),
        "renderCount": len(renders_by_slide),
        "coverage": {
            "sourceGeometry": "partial",
            "sourceText": "digestOnly",
            "renderedExtent": rendered_extent_coverage(
                len(renders_by_slide), len(extension_slides)
            ),
            "renderedNodeIdentity": "cantTell",
        },
        "externalProcessing": False,
        "limitations": [
            "Source layout geometry is not rendered or visible-ink geometry.",
            "Text content and shape names are retained only as digest/count metadata.",
            "Unsalted text digests can disclose low-entropy source strings and remain sensitive metadata.",
            "Rendered node identity, font substitution, effects, clipping, and text layout remain cantTell.",
            "Pictures, charts, tables, connectors, media, animations, and unsupported transforms are not mapped in protocol 0.1.0.",
            "Master/layout objects and theme-resolved styles are not mapped; source geometry coverage is partial.",
        ],
    }
    response_bytes = canonical_json(response)
    if len(response_bytes) > execution["maxOutputBytes"]:
        fail("output-budget", "canonical response exceeds maxOutputBytes")
    return response_bytes, normalized


def platform_version() -> str:
    return f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"


def output_path(value: str) -> Path:
    path = Path(value)
    if path.exists() or path.is_symlink():
        fail("output-collision", "artifact IR output already exists")
    if not path.parent.is_dir():
        fail("output-path", "artifact IR output parent directory does not exist")
    return path


def main() -> int:
    parser = argparse.ArgumentParser(prog="sightlint-pptx")
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
        sys.stderr.write(f"sightlint-pptx: {error.code}: {error.message}\n")
        return 2
    except Exception:
        sys.stderr.write("sightlint-pptx: execution-error: adapter execution failed\n")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
