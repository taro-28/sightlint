#!/usr/bin/env python3
"""Bounded local iOS instrumented-capture adapter for SightLint."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any, Optional


PROTOCOL_VERSION = "0.1.0"
ADAPTER_VERSION = "0.1.0"
EXTENSION_VERSION = "0.1.0"
CAPTURE_VERSION = "0.1.0"
FIXTURE_SOURCE_SHA256 = "sha256:b058a0ee74be9eb66371f6bebfe5d8c2f1d293af2f4ed1c86534e539640bccba"
MAX_REQUEST_BYTES = 1_048_576
TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
IDENTIFIER = re.compile(r"^[A-Za-z][A-Za-z0-9_.-]{0,127}$")

COVERAGE = {
    "sourceHierarchy": "partial",
    "sourceLayout": "partial",
    "accessibilitySemantics": "partial",
    "touchHitRegions": "cantTell",
    "renderedExtent": "observed",
    "renderedNodeIdentity": "cantTell",
    "safeArea": "observed",
    "swiftUISemantics": "untested",
    "focusNavigation": "untested",
    "dynamicBehavior": "untested",
}

LIMITATIONS = [
    "UIKit layoutBox observations do not prove rendered visibility, ink, touch behavior, accessibility, or usability.",
    "XCUI frames remain platformSemantics and are not promoted to hitBox, renderBox, or inkBox.",
    "Source, screenshot, and XCUI observations are sequential, not atomic; rendered node identity remains cantTell.",
    "Only repository-owned classic UIKit Views on one pinned simulator profile are evaluated; SwiftUI, focus, and dynamic behavior remain untested.",
    "Python, Xcode, simulator, and XCUITest tooling are untrusted process sensors; these limits are not an OS sandbox.",
]


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
        fail("input-invalid", f"{context} is missing {','.join(missing)}")
    if unknown:
        fail("input-invalid", f"{context} has unknown fields {','.join(unknown)}")


def record(value: object, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("input-invalid", f"{context} must be an object")
    return value


def array(value: object, context: str, maximum: int, minimum: int = 0) -> list[Any]:
    if not isinstance(value, list) or not minimum <= len(value) <= maximum:
        fail(
            "input-invalid",
            f"{context} must be an array with {minimum} through {maximum} entries",
        )
    return value


def utf8_text(value: object, context: str, maximum: int) -> str:
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > maximum:
        fail(
            "input-invalid",
            f"{context} must be a nonempty UTF-8 string of at most {maximum} bytes",
        )
    return value


def token(value: object, context: str) -> str:
    result = utf8_text(value, context, 128)
    if TOKEN.fullmatch(result) is None:
        fail("input-invalid", f"{context} is not a stable token")
    return result


def digest(value: object, context: str) -> str:
    result = utf8_text(value, context, 72)
    if DIGEST.fullmatch(result) is None:
        fail("input-invalid", f"{context} must be a lowercase SHA-256 digest")
    return result


def bounded_integer(value: object, context: str, minimum: int, maximum: int) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not minimum <= value <= maximum
    ):
        fail(
            "input-invalid",
            f"{context} must be an integer from {minimum} through {maximum}",
        )
    return value


def boolean(value: object, context: str) -> bool:
    if not isinstance(value, bool):
        fail("input-invalid", f"{context} must be a boolean")
    return value


def one_of(value: object, context: str, values: set[object]) -> object:
    if not any(value == allowed for allowed in values):
        fail("input-invalid", f"{context} has an unsupported value")
    return value


def reference(value: object, context: str, suffix: str) -> str:
    result = utf8_text(value, context, 1024)
    pure = PurePosixPath(result)
    if (
        pure.is_absolute()
        or "\\" in result
        or "\0" in result
        or any(part in {"", ".", ".."} for part in pure.parts)
        or not result.lower().endswith(suffix)
    ):
        fail(
            "input-invalid",
            f"{context} must be a normalized repository-relative {suffix} path",
        )
    return result


def json_depth(raw: bytes, maximum: int, context: str) -> None:
    depth = 0
    in_string = False
    escaped = False
    for byte in raw:
        if in_string:
            if escaped:
                escaped = False
            elif byte == 0x5C:
                escaped = True
            elif byte == 0x22:
                in_string = False
        elif byte == 0x22:
            in_string = True
        elif byte in (0x5B, 0x7B):
            depth += 1
            if depth > maximum:
                fail("input-depth", f"{context} exceeds its nesting budget")
        elif byte in (0x5D, 0x7D):
            depth -= 1
            if depth < 0:
                break


def parse_json_bytes(raw: bytes, context: str, maximum_depth: int) -> object:
    json_depth(raw, maximum_depth, context)

    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                fail("input-json", f"{context} contains duplicate field {key}")
            result[key] = value
        return result

    try:
        return json.loads(raw, object_pairs_hook=no_duplicates)
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError):
        fail("input-json", f"{context} must be one valid UTF-8 JSON value")


def read_bounded(path: Path, context: str, maximum: int) -> bytes:
    try:
        if path.stat().st_size > maximum:
            fail("input-budget", f"{context} exceeds its request byte budget")
        raw = path.read_bytes()
    except OSError as error:
        fail("input-io", f"cannot read {context}: {error.strerror or 'I/O error'}")
    if len(raw) > maximum:
        fail("input-budget", f"{context} exceeds its request byte budget")
    return raw


def parse_request(path: Path) -> dict[str, Any]:
    raw = read_bounded(path, "request", MAX_REQUEST_BYTES)
    root = record(parse_json_bytes(raw, "request", 32), "request")
    exact_keys(
        root,
        {
            "protocolVersion",
            "requestId",
            "artifact",
            "capture",
            "screenshot",
            "privacy",
            "execution",
        },
        set(),
        "request",
    )
    if root["protocolVersion"] != PROTOCOL_VERSION:
        fail("request-version", f"request protocolVersion must be {PROTOCOL_VERSION}")
    token(root["requestId"], "request.requestId")

    artifact = record(root["artifact"], "request.artifact")
    exact_keys(artifact, {"id"}, {"title"}, "request.artifact")
    token(artifact["id"], "request.artifact.id")
    if "title" in artifact:
        utf8_text(artifact["title"], "request.artifact.title", 256)

    for name, suffix in (("capture", ".capture.json"), ("screenshot", ".png")):
        source = record(root[name], f"request.{name}")
        exact_keys(source, {"reference", "sha256"}, set(), f"request.{name}")
        reference(source["reference"], f"request.{name}.reference", suffix)
        digest(source["sha256"], f"request.{name}.sha256")

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
        "contentPolicy": "digestsAndGeometry",
    }:
        fail(
            "request-privacy",
            "protocol 0.1.0 requires local digest-and-geometry processing with no retention",
        )

    execution = record(root["execution"], "request.execution")
    limits = {
        "maxCaptureBytes": (1024, 8_388_608),
        "maxScreenshotBytes": (1024, 67_108_864),
        "maxNodes": (1, 10_000),
        "maxDepth": (1, 64),
        "maxAttributesPerNode": (1, 64),
        "maxStringBytes": (1, 1024),
        "maxOutputBytes": (1024, 16_777_216),
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
        fail(
            "path-boundary",
            f"{context} must resolve to a file below the repository root",
        )
    if not candidate.is_file():
        fail("path-boundary", f"{context} must resolve to a regular file")
    return candidate


def verify_digest(raw: bytes, expected: str, context: str) -> None:
    observed = f"sha256:{hashlib.sha256(raw).hexdigest()}"
    if observed != expected:
        fail("input-digest", f"{context} SHA-256 does not match the request")


def number(value: object, context: str, minimum: float, maximum: float) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
        or not minimum <= float(value) <= maximum
    ):
        fail(
            "capture-invalid",
            f"{context} must be a finite number from {minimum} through {maximum}",
        )
    return float(value)


def rect(value: object, context: str) -> dict[str, Any]:
    result = record(value, context)
    exact_keys(result, {"x", "y", "width", "height"}, set(), context)
    number(result["x"], f"{context}.x", -1_048_576, 1_048_576)
    number(result["y"], f"{context}.y", -1_048_576, 1_048_576)
    number(result["width"], f"{context}.width", 0, 1_048_576)
    number(result["height"], f"{context}.height", 0, 1_048_576)
    return result


def nullable_rect(value: object, context: str) -> Optional[dict[str, Any]]:
    if value is None:
        return None
    return rect(value, context)


def rectangle_intersection(
    value: dict[str, Any], clip: dict[str, Any]
) -> Optional[dict[str, Any]]:
    left = max(value["x"], clip["x"])
    top = max(value["y"], clip["y"])
    right = min(value["x"] + value["width"], clip["x"] + clip["width"])
    bottom = min(value["y"] + value["height"], clip["y"] + clip["height"])
    if right <= left or bottom <= top:
        return None
    return {
        "x": round(left, 3),
        "y": round(top, 3),
        "width": round(right - left, 3),
        "height": round(bottom - top, 3),
    }


def insets(value: object, context: str) -> dict[str, Any]:
    result = record(value, context)
    exact_keys(result, {"left", "top", "right", "bottom"}, set(), context)
    for name in ("left", "top", "right", "bottom"):
        number(result[name], f"{context}.{name}", 0, 32_768)
    return result


def string_fact(
    value: object, context: str, maximum_string_bytes: int
) -> Optional[dict[str, Any]]:
    if value is None:
        return None
    result = record(value, context)
    exact_keys(result, {"utf8ByteLength", "sha256"}, set(), context)
    bounded_integer(
        result["utf8ByteLength"], f"{context}.utf8ByteLength", 1, maximum_string_bytes
    )
    digest(result["sha256"], f"{context}.sha256")
    return result


def identifier(value: object, context: str) -> str:
    result = utf8_text(value, context, 128)
    if IDENTIFIER.fullmatch(result) is None:
        fail("capture-invalid", f"{context} is not a stable iOS identifier")
    return result


def nullable_boolean(value: object, context: str) -> Optional[bool]:
    if value is None:
        return None
    return boolean(value, context)


def validate_source_node(
    value: object, index: int, execution: dict[str, int]
) -> dict[str, Any]:
    context = f"capture.sourceHierarchy.nodes[{index}]"
    result = record(value, context)
    required = {
        "identifier",
        "parentIdentifier",
        "depth",
        "className",
        "layoutBoundsPoints",
        "identityTransform",
        "windowIntersectionPoints",
        "safeAreaIntersectionPoints",
        "state",
        "label",
        "value",
    }
    exact_keys(result, required, set(), context)
    if len(result) > execution["maxAttributesPerNode"]:
        fail("attribute-budget", f"{context} exceeds maxAttributesPerNode")
    identifier(result["identifier"], f"{context}.identifier")
    if result["parentIdentifier"] is not None:
        identifier(result["parentIdentifier"], f"{context}.parentIdentifier")
    bounded_integer(result["depth"], f"{context}.depth", 0, execution["maxDepth"])
    utf8_text(result["className"], f"{context}.className", execution["maxStringBytes"])
    rect(result["layoutBoundsPoints"], f"{context}.layoutBoundsPoints")
    boolean(result["identityTransform"], f"{context}.identityTransform")
    nullable_rect(result["windowIntersectionPoints"], f"{context}.windowIntersectionPoints")
    nullable_rect(result["safeAreaIntersectionPoints"], f"{context}.safeAreaIntersectionPoints")
    state = record(result["state"], f"{context}.state")
    exact_keys(
        state,
        {"hidden", "alpha", "userInteractionEnabled", "enabled", "selected", "windowAttached"},
        set(),
        f"{context}.state",
    )
    boolean(state["hidden"], f"{context}.state.hidden")
    number(state["alpha"], f"{context}.state.alpha", 0, 1)
    boolean(state["userInteractionEnabled"], f"{context}.state.userInteractionEnabled")
    nullable_boolean(state["enabled"], f"{context}.state.enabled")
    nullable_boolean(state["selected"], f"{context}.state.selected")
    boolean(state["windowAttached"], f"{context}.state.windowAttached")
    string_fact(result["label"], f"{context}.label", execution["maxStringBytes"])
    string_fact(result["value"], f"{context}.value", execution["maxStringBytes"])
    return result


def validate_xcui_node(
    value: object, index: int, execution: dict[str, int]
) -> dict[str, Any]:
    context = f"capture.xcuiHierarchy.nodes[{index}]"
    result = record(value, context)
    exact_keys(
        result,
        {
            "identifier", "query", "elementType", "exists", "enabled", "selected",
            "hittable", "focusStatus", "frameStatus", "framePoints", "label", "value",
            "title", "placeholder",
        },
        set(),
        context,
    )
    if len(result) > execution["maxAttributesPerNode"]:
        fail("attribute-budget", f"{context} exceeds maxAttributesPerNode")
    observed_identifier = identifier(result["identifier"], f"{context}.identifier")
    query = utf8_text(result["query"], f"{context}.query", execution["maxStringBytes"])
    expected_query = f"XCUIApplication.descendants(matching: .any)[{observed_identifier}]"
    if query != expected_query:
        fail("capture-invalid", f"{context}.query is not the admitted selector")
    one_of(
        result["elementType"],
        f"{context}.elementType",
        {"application", "window", "other", "scrollView", "staticText", "button", "switch", "textField"},
    )
    for field in ("exists", "enabled", "selected", "hittable"):
        boolean(result[field], f"{context}.{field}")
    one_of(result["focusStatus"], f"{context}.focusStatus", {"focused", "notFocused", "unavailable"})
    frame_status = one_of(result["frameStatus"], f"{context}.frameStatus", {"exact", "unavailable"})
    frame = nullable_rect(result["framePoints"], f"{context}.framePoints")
    if (frame_status == "exact") != (frame is not None):
        fail("capture-invalid", f"{context} frame status and value disagree")
    if not result["exists"] and (frame is not None or result["hittable"]):
        fail("capture-invalid", f"{context} unavailable element exposes geometry or hittability")
    for field in ("label", "value", "title", "placeholder"):
        string_fact(result[field], f"{context}.{field}", execution["maxStringBytes"])
    return result


def validate_hierarchy(
    value: object, name: str, node_validator: Any, execution: dict[str, int]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    context = f"capture.{name}"
    hierarchy = record(value, context)
    if name == "sourceHierarchy":
        exact_keys(hierarchy, {"rootIdentifier", "unidentifiedNodeCount", "nodes"}, set(), context)
        identifier(hierarchy["rootIdentifier"], f"{context}.rootIdentifier")
        bounded_integer(hierarchy["unidentifiedNodeCount"], f"{context}.unidentifiedNodeCount", 0, 10_000)
    else:
        exact_keys(hierarchy, {"queryRoot", "unmatchedQueryCount", "nodes"}, set(), context)
        if hierarchy["queryRoot"] != "XCUIApplication":
            fail("capture-compatibility", "capture XCUI query root is unsupported")
        bounded_integer(hierarchy["unmatchedQueryCount"], f"{context}.unmatchedQueryCount", 0, 10_000)
    if not isinstance(hierarchy["nodes"], list) or not hierarchy["nodes"]:
        fail("capture-invalid", f"{context}.nodes must be a nonempty array")
    if len(hierarchy["nodes"]) > execution["maxNodes"]:
        fail("node-budget", f"{context} node count exceeds maxNodes")
    values = hierarchy["nodes"]
    nodes = [node_validator(item, index, execution) for index, item in enumerate(values)]
    ids = [node["identifier"] for node in nodes]
    if len(ids) != len(set(ids)):
        fail("duplicate-node", f"{context} repeats an identifier")
    return hierarchy, nodes


def validate_capture(raw: bytes, execution: dict[str, int]) -> dict[str, Any]:
    root = record(
        parse_json_bytes(raw, "capture manifest", execution["maxDepth"] + 16),
        "capture",
    )
    exact_keys(
        root,
        {
            "captureVersion",
            "captureId",
            "scenario",
            "application",
            "runner",
            "build",
            "device",
            "capture",
            "sourceHierarchy",
            "xcuiHierarchy",
            "screenshot",
        },
        set(),
        "capture",
    )
    if root["captureVersion"] != CAPTURE_VERSION:
        fail("capture-version", f"captureVersion must be {CAPTURE_VERSION}")
    token(root["captureId"], "capture.captureId")
    one_of(
        root["scenario"],
        "capture.scenario",
        {"clean", "off-canvas-control-mutant", "scroll-offscreen-hard-negative"},
    )
    if root["captureId"] != f"ios-atlas-{root['scenario']}":
        fail("capture-compatibility", "capture ID does not match the admitted scenario")

    application = record(root["application"], "capture.application")
    exact_keys(
        application,
        {"bundleIdentifier", "version", "buildNumber", "testBundleIdentifier"},
        set(),
        "capture.application",
    )
    if application != {
        "bundleIdentifier": "org.sightlint.fixtures.atlas.ios",
        "version": "0.1.0",
        "buildNumber": "1",
        "testBundleIdentifier": "org.sightlint.fixtures.atlas.ios.capture-tests",
    }:
        fail("capture-compatibility", "capture application version is unsupported")

    runner = record(root["runner"], "capture.runner")
    exact_keys(runner, {"name", "version", "captureApi"}, set(), "capture.runner")
    if runner != {
        "name": "sightlint-atlas-ios-capture",
        "version": "0.1.0",
        "captureApi": "instrumented-uikit-and-xcui",
    }:
        fail("capture-compatibility", "capture runner version is unsupported")

    build = record(root["build"], "capture.build")
    exact_keys(
        build,
        {
            "fixtureSourceSha256",
            "xcodeVersion",
            "xcodeBuild",
            "swiftVersion",
            "sdkVersion",
            "deploymentTarget",
        },
        set(),
        "capture.build",
    )
    digest(build["fixtureSourceSha256"], "capture.build.fixtureSourceSha256")
    if build["fixtureSourceSha256"] != FIXTURE_SOURCE_SHA256:
        fail("capture-compatibility", "capture fixture source is unsupported")
    if {
        name: build[name]
        for name in (
            "xcodeVersion",
            "xcodeBuild",
            "swiftVersion",
            "sdkVersion",
            "deploymentTarget",
        )
    } != {
        "xcodeVersion": "26.3",
        "xcodeBuild": "17C529",
        "swiftVersion": "6.2.4",
        "sdkVersion": "26.2",
        "deploymentTarget": "26.0",
    }:
        fail("capture-compatibility", "capture build profile is unsupported")

    device = record(root["device"], "capture.device")
    exact_keys(
        device,
        {
            "runtimeVersion",
            "runtimeBuild",
            "deviceType",
            "modelIdentifier",
            "architecture",
            "display",
            "configuration",
            "safeAreaInsetsPoints",
        },
        set(),
        "capture.device",
    )
    for name in ("runtimeVersion", "runtimeBuild", "deviceType", "modelIdentifier", "architecture"):
        utf8_text(device[name], f"capture.device.{name}", execution["maxStringBytes"])
    display = record(device["display"], "capture.device.display")
    exact_keys(
        display,
        {"widthPoints", "heightPoints", "scale", "widthPixels", "heightPixels", "orientation"},
        set(),
        "capture.device.display",
    )
    number(display["widthPoints"], "capture.device.display.widthPoints", 1, 32_768)
    number(display["heightPoints"], "capture.device.display.heightPoints", 1, 32_768)
    number(display["scale"], "capture.device.display.scale", 0.01, 16)
    bounded_integer(
        display["widthPixels"], "capture.device.display.widthPixels", 1, 32_768
    )
    bounded_integer(
        display["heightPixels"], "capture.device.display.heightPixels", 1, 32_768
    )
    one_of(
        display["orientation"],
        "capture.device.display.orientation",
        {"portrait", "landscapeLeft", "landscapeRight", "portraitUpsideDown"},
    )
    configuration = record(device["configuration"], "capture.device.configuration")
    exact_keys(
        configuration,
        {"locale", "layoutDirection", "contentSizeCategory", "interfaceStyle", "reduceMotion"},
        set(),
        "capture.device.configuration",
    )
    utf8_text(configuration["locale"], "capture.device.configuration.locale", 35)
    one_of(
        configuration["layoutDirection"],
        "capture.device.configuration.layoutDirection",
        {"ltr", "rtl"},
    )
    one_of(
        configuration["contentSizeCategory"],
        "capture.device.configuration.contentSizeCategory",
        {"extraSmall", "small", "medium", "large", "extraLarge", "extraExtraLarge", "extraExtraExtraLarge", "accessibilityMedium", "accessibilityLarge", "accessibilityExtraLarge", "accessibilityExtraExtraLarge", "accessibilityExtraExtraExtraLarge"},
    )
    one_of(
        configuration["interfaceStyle"],
        "capture.device.configuration.interfaceStyle",
        {"light", "dark"},
    )
    boolean(configuration["reduceMotion"], "capture.device.configuration.reduceMotion")
    device_insets = insets(device["safeAreaInsetsPoints"], "capture.device.safeAreaInsetsPoints")
    if {
        "runtimeVersion": device["runtimeVersion"],
        "runtimeBuild": device["runtimeBuild"],
        "deviceType": device["deviceType"],
        "modelIdentifier": device["modelIdentifier"],
        "architecture": device["architecture"],
        "display": display,
        "configuration": configuration,
        "safeAreaInsetsPoints": device_insets,
    } != {
        "runtimeVersion": "26.3.1",
        "runtimeBuild": "23D8133",
        "deviceType": "iPhone 17 Pro",
        "modelIdentifier": "iPhone18,1",
        "architecture": "arm64",
        "display": {
            "widthPoints": 402,
            "heightPoints": 874,
            "scale": 3,
            "widthPixels": 1206,
            "heightPixels": 2622,
            "orientation": "portrait",
        },
        "configuration": {
            "locale": "en-US",
            "layoutDirection": "ltr",
            "contentSizeCategory": "large",
            "interfaceStyle": "light",
            "reduceMotion": False,
        },
        "safeAreaInsetsPoints": {
            "left": 0,
            "top": 62,
            "right": 0,
            "bottom": 34,
        },
    }:
        fail("capture-compatibility", "capture device profile is unsupported")

    capture = record(root["capture"], "capture.capture")
    exact_keys(
        capture,
        {
            "order",
            "atomic",
            "animationsDisabled",
            "testCommand",
            "limitations",
        },
        set(),
        "capture.capture",
    )
    if capture["order"] != [
        "waitForQuiescence",
        "sourceHierarchy",
        "screenshot",
        "xcuiHierarchy",
    ]:
        fail("capture-compatibility", "capture sequence is unsupported")
    if capture["atomic"] is not False or capture["animationsDisabled"] is not True:
        fail(
            "capture-compatibility",
            "capture synchronization declaration is unsupported",
        )
    utf8_text(
        capture["testCommand"],
        "capture.capture.testCommand",
        execution["maxStringBytes"],
    )
    limitations = array(capture["limitations"], "capture.capture.limitations", 16, 1)
    normalized_limitations = [
        utf8_text(
            item, f"capture.capture.limitations[{index}]", execution["maxStringBytes"]
        )
        for index, item in enumerate(limitations)
    ]
    if len(normalized_limitations) != len(set(normalized_limitations)):
        fail("capture-invalid", "capture.capture.limitations must be unique")

    source_hierarchy, nodes = validate_hierarchy(
        root["sourceHierarchy"], "sourceHierarchy", validate_source_node, execution
    )
    xcui_hierarchy, xcui_nodes = validate_hierarchy(
        root["xcuiHierarchy"], "xcuiHierarchy", validate_xcui_node, execution
    )
    ids = [node["identifier"] for node in nodes]
    known = set(ids)
    root_id = source_hierarchy["rootIdentifier"]
    if root_id not in known:
        fail("capture-invalid", "capture source rootIdentifier is missing")
    for node in nodes:
        parent = node["parentIdentifier"]
        if parent is not None and parent not in known:
            fail("dangling-parent", f"capture node {node['identifier']} has an unknown parent")
    parents = {node["identifier"]: node["parentIdentifier"] for node in nodes}
    for identifier in known:
        seen: set[str] = set()
        current: Optional[str] = identifier
        while current is not None:
            if current in seen:
                fail("hierarchy-cycle", "capture hierarchy contains a parent cycle")
            seen.add(current)
            current = parents[current]
    depth_by_id = {node["identifier"]: node["depth"] for node in nodes}
    for node in nodes:
        parent = node["parentIdentifier"]
        expected_depth = 0 if parent is None else depth_by_id[parent] + 1
        if node["depth"] != expected_depth:
            fail("capture-invalid", f"capture node {node['identifier']} has inconsistent depth")
    roots = [node for node in nodes if node["parentIdentifier"] is None]
    if len(roots) != 1 or roots[0]["identifier"] != root_id:
        fail("capture-invalid", "capture source hierarchy must have one declared root")
    screen = {
        "x": 0,
        "y": 0,
        "width": display["widthPoints"],
        "height": display["heightPoints"],
    }
    if roots[0]["layoutBoundsPoints"] != screen:
        fail("capture-conflict", "capture source root and point display extents disagree")
    safe_area = {
        "x": device_insets["left"],
        "y": device_insets["top"],
        "width": display["widthPoints"] - device_insets["left"] - device_insets["right"],
        "height": display["heightPoints"] - device_insets["top"] - device_insets["bottom"],
    }
    if safe_area["width"] <= 0 or safe_area["height"] <= 0:
        fail("capture-invalid", "capture safe area has no positive extent")
    for node in nodes:
        expected_window = rectangle_intersection(node["layoutBoundsPoints"], screen)
        expected_safe = rectangle_intersection(node["layoutBoundsPoints"], safe_area)
        if node["windowIntersectionPoints"] != expected_window:
            fail(
                "capture-conflict",
                f"capture node {node['identifier']} has inconsistent window intersection",
            )
        if node["safeAreaIntersectionPoints"] != expected_safe:
            fail(
                "capture-conflict",
                f"capture node {node['identifier']} has inconsistent safe-area intersection",
            )
    xcui_ids = [node["identifier"] for node in xcui_nodes]
    queries = [node["query"] for node in xcui_nodes]
    if len(queries) != len(set(queries)):
        fail("duplicate-node", "capture XCUI hierarchy repeats a query selector")
    if xcui_hierarchy["unmatchedQueryCount"] != sum(
        not node["exists"] for node in xcui_nodes
    ):
        fail("capture-conflict", "capture XCUI unmatched count disagrees with nodes")

    screenshot = record(root["screenshot"], "capture.screenshot")
    exact_keys(
        screenshot,
        {"reference", "sha256", "widthPixels", "heightPixels", "captureSequence"},
        set(),
        "capture.screenshot",
    )
    reference(screenshot["reference"], "capture.screenshot.reference", ".png")
    digest(screenshot["sha256"], "capture.screenshot.sha256")
    bounded_integer(
        screenshot["widthPixels"], "capture.screenshot.widthPixels", 1, 32_768
    )
    bounded_integer(
        screenshot["heightPixels"], "capture.screenshot.heightPixels", 1, 32_768
    )
    if screenshot["captureSequence"] != 3:
        fail("capture-compatibility", "capture screenshot sequence is unsupported")
    if len(xcui_ids) > execution["maxNodes"]:
        fail("node-budget", "capture XCUI node count exceeds maxNodes")
    return root


def run_sightlint(
    binary: Path, arguments: list[str], stdin: Optional[bytes] = None
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


def png_canvas(binary: Path, screenshot_path: Path) -> tuple[int, int, str]:
    output = run_sightlint(binary, ["adapt-image", str(screenshot_path)])
    try:
        document = json.loads(output)
        canvas = document["canvases"][0]
        width = canvas["size"]["width"]
        height = canvas["size"]["height"]
        header_evidence = next(
            item for item in document["evidence"] if item["id"] == canvas["evidenceId"]
        )
        adapter_version = header_evidence["source"]["adapterVersion"]
    except (KeyError, IndexError, StopIteration, TypeError, json.JSONDecodeError):
        fail(
            "sightlint-output",
            "adapt-image returned an unexpected Artifact IR document",
        )
    if (
        isinstance(width, bool)
        or isinstance(height, bool)
        or not isinstance(width, (int, float))
        or not isinstance(height, (int, float))
        or int(width) != width
        or int(height) != height
    ):
        fail("sightlint-output", "adapt-image returned invalid canvas dimensions")
    return int(width), int(height), str(adapter_version)


def mapping_status(
    node: dict[str, Any], source_nodes: dict[str, dict[str, Any]]
) -> str:
    state = node["state"]
    box = node["layoutBoundsPoints"]
    if state["hidden"]:
        return "notMappedHidden"
    if state["alpha"] <= 0:
        return "notMappedTransparent"
    if not state["windowAttached"]:
        return "notMappedDetached"
    if not node["identityTransform"]:
        return "notMappedUnsupportedTransform"
    if box["width"] == 0 or box["height"] == 0:
        return "notMappedEmptyLayout"
    if node["windowIntersectionPoints"] is None:
        return "notMappedNotWindowVisible"
    parent = node["parentIdentifier"]
    if (
        parent is not None
        and source_nodes[parent]["className"] == "UIScrollView"
        and node["windowIntersectionPoints"] != box
    ):
        return "notMappedClippedScrollContent"
    return "mappedExactLayout"


def core_kind(node: dict[str, Any]) -> str:
    class_name = node["className"]
    if class_name in {"UIButton", "UISwitch"} and node["state"]["enabled"] is True:
        return "control"
    if class_name == "UILabel" and not node["state"]["userInteractionEnabled"]:
        return "text"
    if class_name in {"UIView", "UIStackView", "UIScrollView"}:
        return "container"
    return "other"


def reconciliation(
    source_node: dict[str, Any], xcui_node: Optional[dict[str, Any]]
) -> str:
    if xcui_node is None or xcui_node["frameStatus"] == "unavailable":
        return "xcuiUnavailable"
    if source_node["layoutBoundsPoints"] == xcui_node["framePoints"]:
        return "frameAgreement"
    return "frameConflict"


def adapt(
    request: dict[str, Any], repository_root: Path, binary: Path
) -> tuple[bytes, bytes]:
    execution = request["execution"]
    capture_path = local_file(
        repository_root, request["capture"]["reference"], "capture reference"
    )
    screenshot_path = local_file(
        repository_root, request["screenshot"]["reference"], "screenshot reference"
    )
    capture_raw = read_bounded(
        capture_path, "capture manifest", execution["maxCaptureBytes"]
    )
    screenshot_raw = read_bounded(
        screenshot_path, "screenshot", execution["maxScreenshotBytes"]
    )
    verify_digest(capture_raw, request["capture"]["sha256"], "capture manifest")
    verify_digest(screenshot_raw, request["screenshot"]["sha256"], "screenshot")
    capture = validate_capture(capture_raw, execution)

    if capture["captureId"] != request["artifact"]["id"]:
        fail("capture-conflict", "captureId and request artifact id disagree")
    if capture["screenshot"]["reference"] != request["screenshot"]["reference"]:
        fail("capture-conflict", "capture and request screenshot references disagree")
    if capture["screenshot"]["sha256"] != request["screenshot"]["sha256"]:
        fail("capture-conflict", "capture and request screenshot digests disagree")

    pixel_width, pixel_height, png_adapter_version = png_canvas(binary, screenshot_path)
    display = capture["device"]["display"]
    screenshot = capture["screenshot"]
    if (
        pixel_width != display["widthPixels"]
        or pixel_height != display["heightPixels"]
        or pixel_width != screenshot["widthPixels"]
        or pixel_height != screenshot["heightPixels"]
    ):
        fail("extent-conflict", "capture display and PNG extents disagree")
    if (
        pixel_width != display["widthPoints"] * display["scale"]
        or pixel_height != display["heightPoints"] * display["scale"]
    ):
        fail("extent-conflict", "capture point extent and display scale disagree with PNG")
    if display["orientation"] in {"portrait", "portraitUpsideDown"} and pixel_width > pixel_height:
        fail("orientation-conflict", "portrait orientation conflicts with PNG extent")
    if display["orientation"] in {"landscapeLeft", "landscapeRight"} and pixel_width < pixel_height:
        fail("orientation-conflict", "landscape orientation conflicts with PNG extent")

    display_evidence_id = "e-ios:screen:points:source"
    render_evidence_id = "e-ios:screen:pixels:render"
    mapped_ids: set[str] = set()
    extension_source_nodes: list[dict[str, Any]] = []
    extension_xcui_nodes: list[dict[str, Any]] = []
    evidence: list[dict[str, Any]] = [
        {
            "id": display_evidence_id,
            "class": "exactSource",
            "source": {
                "adapter": "sightlint-ios",
                "adapterVersion": ADAPTER_VERSION,
                "inputDigest": request["capture"]["sha256"],
                "externalProcessing": False,
            },
            "selector": {"type": "nativeId", "nativeId": "ios:screen:points"},
        },
        {
            "id": render_evidence_id,
            "class": "exactRender",
            "source": {
                "adapter": "sightlint-adapter-png",
                "adapterVersion": png_adapter_version,
                "inputDigest": request["screenshot"]["sha256"],
                "externalProcessing": False,
            },
            "selector": {
                "type": "nativeId",
                "nativeId": f"IHDR:{request['screenshot']['reference']}",
            },
        },
    ]

    ordered_source_nodes = sorted(
        capture["sourceHierarchy"]["nodes"], key=lambda item: item["identifier"]
    )
    ordered_xcui_nodes = sorted(
        capture["xcuiHierarchy"]["nodes"], key=lambda item: item["identifier"]
    )
    xcui_by_identifier = {item["identifier"]: item for item in ordered_xcui_nodes}
    source_by_identifier = {item["identifier"]: item for item in ordered_source_nodes}
    for captured in ordered_source_nodes:
        source_identifier = captured["identifier"]
        core_identifier = f"ios:view:{source_identifier}"
        source_evidence_id = f"e-ios:view:{source_identifier}:source"
        status = mapping_status(captured, source_by_identifier)
        if status == "mappedExactLayout":
            mapped_ids.add(source_identifier)
        evidence.append(
            {
                "id": source_evidence_id,
                "class": "exactSource",
                "source": {
                    "adapter": "sightlint-ios",
                    "adapterVersion": ADAPTER_VERSION,
                    "inputDigest": request["capture"]["sha256"],
                    "externalProcessing": False,
                },
                "selector": {"type": "nativeId", "nativeId": core_identifier},
            }
        )
        extension_source_nodes.append(
            {
                "id": core_identifier,
                "identifier": source_identifier,
                "parentIdentifier": captured["parentIdentifier"],
                "depth": captured["depth"],
                "className": captured["className"],
                "mappingStatus": status,
                "layoutBoundsPoints": captured["layoutBoundsPoints"],
                "identityTransform": captured["identityTransform"],
                "windowIntersectionPoints": captured["windowIntersectionPoints"],
                "safeAreaIntersectionPoints": captured["safeAreaIntersectionPoints"],
                "state": captured["state"],
                "label": captured["label"],
                "value": captured["value"],
                "evidenceId": source_evidence_id,
                "xcuiReconciliation": reconciliation(
                    captured, xcui_by_identifier.get(source_identifier)
                ),
            }
        )

    for captured in ordered_xcui_nodes:
        xcui_identifier = captured["identifier"]
        evidence_id = f"e-ios:xcui:{xcui_identifier}:semantics"
        evidence.append(
            {
                "id": evidence_id,
                "class": "platformSemantics",
                "source": {
                    "adapter": "sightlint-ios",
                    "adapterVersion": ADAPTER_VERSION,
                    "inputDigest": request["capture"]["sha256"],
                    "externalProcessing": False,
                },
                "selector": {
                    "type": "nativeId",
                    "nativeId": f"ios:xcui:{xcui_identifier}",
                },
            }
        )
        extension_xcui_nodes.append({**captured, "evidenceId": evidence_id})

    nodes: list[dict[str, Any]] = []
    for captured, extension_node in zip(ordered_source_nodes, extension_source_nodes):
        if extension_node["mappingStatus"] != "mappedExactLayout":
            continue
        node: dict[str, Any] = {
            "id": extension_node["id"],
            "kind": {
                "value": core_kind(captured),
                "evidenceId": extension_node["evidenceId"],
            },
            "coordinateSpaceId": "ios:screen:points",
            "geometry": {
                "layoutBox": {
                    "rect": captured["layoutBoundsPoints"],
                    "coordinateSpaceId": "ios:screen:points",
                    "evidenceId": extension_node["evidenceId"],
                }
            },
        }
        parent = captured["parentIdentifier"]
        if parent in mapped_ids:
            node["parentId"] = f"ios:view:{parent}"
        nodes.append(node)

    capture_details = capture["capture"]
    extension = {
        "extensionVersion": EXTENSION_VERSION,
        "capture": {
            "id": capture["captureId"],
            "scenario": capture["scenario"],
            "sha256": request["capture"]["sha256"],
            "fixtureSourceSha256": capture["build"]["fixtureSourceSha256"],
            "application": capture["application"],
            "runner": capture["runner"],
            "build": {
                name: capture["build"][name]
                for name in (
                    "xcodeVersion",
                    "xcodeBuild",
                    "swiftVersion",
                    "sdkVersion",
                    "deploymentTarget",
                )
            },
            "device": capture["device"],
            "sequence": capture_details["order"],
            "atomic": capture_details["atomic"],
            "animationsDisabled": capture_details["animationsDisabled"],
            "testCommand": capture_details["testCommand"],
            "limitations": sorted(capture_details["limitations"]),
            "sourceRootIdentifier": capture["sourceHierarchy"]["rootIdentifier"],
            "xcuiQueryRoot": capture["xcuiHierarchy"]["queryRoot"],
        },
        "screen": {
            "pointCoordinateSpaceId": "ios:screen:points",
            "display": display,
            "safeAreaInsetsPoints": capture["device"]["safeAreaInsetsPoints"],
            "displayEvidenceId": display_evidence_id,
            "screenshot": {
                "reference": request["screenshot"]["reference"],
                "sha256": request["screenshot"]["sha256"],
                "widthPixels": pixel_width,
                "heightPixels": pixel_height,
                "canvasId": "ios:screen:pixels",
                "evidenceId": render_evidence_id,
                "extentReconciliation": "extentAndScaleAgree",
                "nodeIdentity": "cantTell",
            },
        },
        "sourceNodes": extension_source_nodes,
        "xcuiNodes": extension_xcui_nodes,
        "coverage": COVERAGE,
        "unsupported": {
            "unidentifiedSourceNodeCount": capture["sourceHierarchy"][
                "unidentifiedNodeCount"
            ],
            "unmatchedXcuiQueryCount": capture["xcuiHierarchy"][
                "unmatchedQueryCount"
            ],
            "features": sorted(
                {
                    "dynamicBehavior",
                    "focusNavigation",
                    "occlusionAndInkGeometry",
                    "renderedNodeIdentity",
                    "swiftUISemantics",
                    "touchHitRegions",
                }
            ),
        },
        "privacy": {
            "externalProcessing": False,
            "retention": "none",
            "contentPolicy": "digestsAndGeometry",
            "transmittedFields": [],
        },
    }
    document = {
        "schemaVersion": "0.1.0",
        "artifact": {
            "id": request["artifact"]["id"],
            "kind": "mobile",
            "sourceName": request["capture"]["reference"],
        },
        "canvases": [
            {
                "id": "ios:screen:pixels",
                "size": {"width": pixel_width, "height": pixel_height},
                "unit": "devicePixel",
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": render_evidence_id,
            },
            {
                "id": "ios:screen:points",
                "size": {
                    "width": display["widthPoints"],
                    "height": display["heightPoints"],
                },
                "unit": "point",
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": display_evidence_id,
            },
        ],
        "nodes": nodes,
        "evidence": sorted(evidence, key=lambda item: item["id"]),
        "extensions": {"org.sightlint.ios": extension},
    }
    if "title" in request["artifact"]:
        document["artifact"]["title"] = request["artifact"]["title"]

    normalized = run_sightlint(
        binary, ["normalize", "-"], stdin=canonical_json(document)
    )
    if len(normalized) > execution["maxOutputBytes"]:
        fail("output-budget", "canonical Artifact IR exceeds maxOutputBytes")
    response = {
        "protocolVersion": PROTOCOL_VERSION,
        "requestId": request["requestId"],
        "status": "partial",
        "adapter": {
            "name": "sightlint-ios",
            "version": ADAPTER_VERSION,
            "runtime": {
                "name": "python",
                "version": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
            },
        },
        "captureSha256": request["capture"]["sha256"],
        "screenshotSha256": request["screenshot"]["sha256"],
        "sourceNodeCount": len(extension_source_nodes),
        "xcuiNodeCount": len(extension_xcui_nodes),
        "mappedNodeCount": len(nodes),
        "excludedNodeCount": len(extension_source_nodes) - len(nodes),
        "coverage": COVERAGE,
        "externalProcessing": False,
        "limitations": LIMITATIONS,
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
    parser = argparse.ArgumentParser(prog="sightlint-ios")
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
            try:
                destination.unlink()
            except OSError:
                pass
            fail("output-io", "cannot write Artifact IR output")
        sys.stdout.buffer.write(response)
        return 0
    except AdapterError as error:
        sys.stderr.buffer.write(
            f"sightlint-ios: {error.code}: {error.message}\n".encode("utf-8")
        )
        return 2
    except Exception:
        sys.stderr.buffer.write(
            b"sightlint-ios: execution-error: adapter execution failed\n"
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
