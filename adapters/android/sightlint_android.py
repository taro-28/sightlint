#!/usr/bin/env python3
"""Bounded local Android instrumented-capture adapter for SightLint."""

from __future__ import annotations

import argparse
import hashlib
import json
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
MAX_REQUEST_BYTES = 1_048_576
TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
RESOURCE_ID = re.compile(r"^[A-Za-z0-9_.]+:id/[A-Za-z0-9_]+$")
VERSION = re.compile(r"^[0-9]+(?:\.[0-9]+){1,3}$")
LOCALE = re.compile(r"^[A-Za-z0-9-]{2,35}$")

COVERAGE = {
    "viewHierarchy": "partial",
    "viewLayout": "partial",
    "accessibilitySemantics": "partial",
    "touchHitRegions": "cantTell",
    "renderedExtent": "observed",
    "renderedNodeIdentity": "cantTell",
    "composeSemantics": "untested",
    "dynamicBehavior": "untested",
}

LIMITATIONS = [
    "View layoutBox observations do not prove rendered visibility, ink, touch behavior, accessibility, or usability.",
    "Accessibility bounds remain platformSemantics and are not promoted to hitBox or renderBox.",
    "The native hierarchy and screenshot are sequential, not atomic; rendered node identity remains cantTell.",
    "Only repository-owned classic Android Views on the pinned capture profile are evaluated; Compose and dynamic behavior remain untested.",
    "Python and Android capture tooling are untrusted process sensors; these limits are not an OS sandbox.",
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


def rect(value: object, context: str) -> dict[str, int]:
    result = record(value, context)
    exact_keys(result, {"x", "y", "width", "height"}, set(), context)
    bounded_integer(result["x"], f"{context}.x", -1_048_576, 1_048_576)
    bounded_integer(result["y"], f"{context}.y", -1_048_576, 1_048_576)
    bounded_integer(result["width"], f"{context}.width", 0, 1_048_576)
    bounded_integer(result["height"], f"{context}.height", 0, 1_048_576)
    return result


def raw_bounds(value: object, context: str) -> dict[str, int]:
    result = record(value, context)
    exact_keys(result, {"left", "top", "right", "bottom"}, set(), context)
    for name in ("left", "top", "right", "bottom"):
        bounded_integer(result[name], f"{context}.{name}", -1_048_576, 1_048_576)
    return result


def insets(value: object, context: str) -> dict[str, int]:
    result = record(value, context)
    exact_keys(result, {"left", "top", "right", "bottom"}, set(), context)
    for name in ("left", "top", "right", "bottom"):
        bounded_integer(result[name], f"{context}.{name}", 0, 32_768)
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


def booleans(value: object, context: str, names: tuple[str, ...]) -> dict[str, bool]:
    result = record(value, context)
    exact_keys(result, set(names), set(), context)
    for name in names:
        boolean(result[name], f"{context}.{name}")
    return result  # type: ignore[return-value]


VIEW_STATE_FIELDS = (
    "shown",
    "enabled",
    "clickable",
    "focusable",
    "focused",
    "selected",
    "checkable",
    "checked",
    "scrollable",
    "longClickable",
)
ACCESSIBILITY_BOOLEAN_FIELDS = (
    "enabled",
    "clickable",
    "focusable",
    "focused",
    "selected",
    "checkable",
    "checked",
    "scrollable",
    "longClickable",
    "visibleToUser",
)


def nullable_text(value: object, context: str, maximum: int) -> Optional[str]:
    if value is None:
        return None
    return utf8_text(value, context, maximum)


def resource_id(value: object, context: str) -> str:
    result = utf8_text(value, context, 256)
    if RESOURCE_ID.fullmatch(result) is None:
        fail("capture-invalid", f"{context} is not an Android resource ID")
    return result


def validate_accessibility(
    value: object, context: str, max_string_bytes: int, max_attributes: int
) -> dict[str, Any]:
    result = record(value, context)
    required = {
        "className",
        "packageName",
        "viewIdResourceName",
        "geometryStatus",
        "rawBoundsDevicePixels",
        "boundsDevicePixels",
        "actionIds",
        *ACCESSIBILITY_BOOLEAN_FIELDS,
    }
    exact_keys(result, required, set(), context)
    if len(result) > max_attributes:
        fail("attribute-budget", f"{context} exceeds maxAttributesPerNode")
    nullable_text(result["className"], f"{context}.className", max_string_bytes)
    nullable_text(result["packageName"], f"{context}.packageName", max_string_bytes)
    if result["viewIdResourceName"] is not None:
        resource_id(result["viewIdResourceName"], f"{context}.viewIdResourceName")
    status = one_of(
        result["geometryStatus"],
        f"{context}.geometryStatus",
        {"exact", "invalidPlatformBounds", "unavailable"},
    )
    raw = result["rawBoundsDevicePixels"]
    bounds = result["boundsDevicePixels"]
    if status == "exact":
        raw_value = raw_bounds(raw, f"{context}.rawBoundsDevicePixels")
        bounds_value = rect(bounds, f"{context}.boundsDevicePixels")
        if (
            raw_value["right"] < raw_value["left"]
            or raw_value["bottom"] < raw_value["top"]
        ):
            fail("capture-invalid", f"{context} exact bounds are inverted")
        expected = {
            "x": raw_value["left"],
            "y": raw_value["top"],
            "width": raw_value["right"] - raw_value["left"],
            "height": raw_value["bottom"] - raw_value["top"],
        }
        if bounds_value != expected:
            fail(
                "capture-conflict",
                f"{context} exact raw and normalized bounds disagree",
            )
    elif status == "invalidPlatformBounds":
        raw_value = raw_bounds(raw, f"{context}.rawBoundsDevicePixels")
        if bounds is not None:
            fail("capture-invalid", f"{context} invalid bounds must not be normalized")
        if (
            raw_value["right"] >= raw_value["left"]
            and raw_value["bottom"] >= raw_value["top"]
        ):
            fail("capture-invalid", f"{context} invalid bounds are actually ordered")
    elif raw is not None or bounds is not None:
        fail("capture-invalid", f"{context} unavailable geometry must be null")

    actions = array(result["actionIds"], f"{context}.actionIds", 128)
    observed_actions = [
        bounded_integer(action, f"{context}.actionIds[{index}]", 0, 2_147_483_647)
        for index, action in enumerate(actions)
    ]
    if observed_actions != sorted(set(observed_actions)):
        fail("capture-invalid", f"{context}.actionIds must be sorted and unique")
    for name in ACCESSIBILITY_BOOLEAN_FIELDS:
        boolean(result[name], f"{context}.{name}")
    return result


def validate_node(
    value: object,
    index: int,
    execution: dict[str, int],
) -> dict[str, Any]:
    context = f"capture.hierarchy.nodes[{index}]"
    result = record(value, context)
    exact_keys(
        result,
        {
            "resourceId",
            "parentResourceId",
            "depth",
            "className",
            "layoutBoundsDevicePixels",
            "identityTransform",
            "globalVisible",
            "viewState",
            "text",
            "contentDescription",
            "accessibility",
        },
        set(),
        context,
    )
    if len(result) > execution["maxAttributesPerNode"]:
        fail("attribute-budget", f"{context} exceeds maxAttributesPerNode")
    resource_id(result["resourceId"], f"{context}.resourceId")
    if result["parentResourceId"] is not None:
        resource_id(result["parentResourceId"], f"{context}.parentResourceId")
    bounded_integer(result["depth"], f"{context}.depth", 0, execution["maxDepth"])
    utf8_text(result["className"], f"{context}.className", execution["maxStringBytes"])
    rect(result["layoutBoundsDevicePixels"], f"{context}.layoutBoundsDevicePixels")
    boolean(result["identityTransform"], f"{context}.identityTransform")
    visible = record(result["globalVisible"], f"{context}.globalVisible")
    exact_keys(
        visible, {"value", "boundsDevicePixels"}, set(), f"{context}.globalVisible"
    )
    is_visible = boolean(visible["value"], f"{context}.globalVisible.value")
    if is_visible:
        visible_bounds = rect(
            visible["boundsDevicePixels"],
            f"{context}.globalVisible.boundsDevicePixels",
        )
        if visible_bounds["width"] == 0 or visible_bounds["height"] == 0:
            fail(
                "capture-invalid",
                f"{context}.globalVisible true bounds must be nonempty",
            )
    elif visible["boundsDevicePixels"] is not None:
        fail("capture-invalid", f"{context}.globalVisible false bounds must be null")
    booleans(result["viewState"], f"{context}.viewState", VIEW_STATE_FIELDS)
    string_fact(result["text"], f"{context}.text", execution["maxStringBytes"])
    string_fact(
        result["contentDescription"],
        f"{context}.contentDescription",
        execution["maxStringBytes"],
    )
    validate_accessibility(
        result["accessibility"],
        f"{context}.accessibility",
        execution["maxStringBytes"],
        execution["maxAttributesPerNode"],
    )
    return result


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
            "hierarchy",
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

    application = record(root["application"], "capture.application")
    exact_keys(
        application,
        {"packageName", "versionName", "versionCode"},
        set(),
        "capture.application",
    )
    if application != {
        "packageName": "org.sightlint.fixtures.atlas",
        "versionName": "0.1.0",
        "versionCode": 1,
    }:
        fail("capture-compatibility", "capture application version is unsupported")

    runner = record(root["runner"], "capture.runner")
    exact_keys(runner, {"name", "version", "captureApi"}, set(), "capture.runner")
    if runner != {
        "name": "sightlint-atlas-android-capture",
        "version": "0.1.0",
        "captureApi": "instrumentation-view-accessibility",
    }:
        fail("capture-compatibility", "capture runner version is unsupported")

    build = record(root["build"], "capture.build")
    exact_keys(
        build,
        {
            "fixtureSourceSha256",
            "gradleVersion",
            "androidGradlePluginVersion",
            "javaLanguageVersion",
            "compileSdk",
        },
        set(),
        "capture.build",
    )
    digest(build["fixtureSourceSha256"], "capture.build.fixtureSourceSha256")
    if {
        name: build[name]
        for name in (
            "gradleVersion",
            "androidGradlePluginVersion",
            "javaLanguageVersion",
            "compileSdk",
        )
    } != {
        "gradleVersion": "8.13",
        "androidGradlePluginVersion": "8.10.1",
        "javaLanguageVersion": 17,
        "compileSdk": 35,
    }:
        fail("capture-compatibility", "capture build profile is unsupported")

    device = record(root["device"], "capture.device")
    exact_keys(
        device,
        {
            "apiLevel",
            "buildFingerprint",
            "manufacturer",
            "model",
            "device",
            "display",
            "configuration",
            "systemBarInsetsDevicePixels",
        },
        set(),
        "capture.device",
    )
    bounded_integer(device["apiLevel"], "capture.device.apiLevel", 28, 100)
    for name in ("buildFingerprint", "manufacturer", "model", "device"):
        utf8_text(device[name], f"capture.device.{name}", execution["maxStringBytes"])
    display = record(device["display"], "capture.device.display")
    exact_keys(
        display,
        {"widthPixels", "heightPixels", "densityDpi", "rotationDegrees"},
        set(),
        "capture.device.display",
    )
    bounded_integer(
        display["widthPixels"], "capture.device.display.widthPixels", 1, 32_768
    )
    bounded_integer(
        display["heightPixels"], "capture.device.display.heightPixels", 1, 32_768
    )
    bounded_integer(
        display["densityDpi"], "capture.device.display.densityDpi", 1, 4_096
    )
    one_of(
        display["rotationDegrees"],
        "capture.device.display.rotationDegrees",
        {0, 90, 180, 270},
    )
    configuration = record(device["configuration"], "capture.device.configuration")
    exact_keys(
        configuration,
        {"fontScale", "locale", "layoutDirection", "nightMode"},
        set(),
        "capture.device.configuration",
    )
    scale = configuration["fontScale"]
    if (
        isinstance(scale, bool)
        or not isinstance(scale, (int, float))
        or not 0 < scale <= 10
    ):
        fail("capture-invalid", "capture.device.configuration.fontScale is invalid")
    locale = utf8_text(
        configuration["locale"], "capture.device.configuration.locale", 35
    )
    if LOCALE.fullmatch(locale) is None:
        fail("capture-invalid", "capture.device.configuration.locale is invalid")
    one_of(
        configuration["layoutDirection"],
        "capture.device.configuration.layoutDirection",
        {"ltr", "rtl"},
    )
    one_of(
        configuration["nightMode"],
        "capture.device.configuration.nightMode",
        {"light", "dark"},
    )
    device_insets = insets(
        device["systemBarInsetsDevicePixels"],
        "capture.device.systemBarInsetsDevicePixels",
    )
    if {
        "apiLevel": device["apiLevel"],
        "buildFingerprint": device["buildFingerprint"],
        "manufacturer": device["manufacturer"],
        "model": device["model"],
        "device": device["device"],
        "display": display,
        "configuration": configuration,
        "systemBarInsetsDevicePixels": device_insets,
    } != {
        "apiLevel": 35,
        "buildFingerprint": "google/sdk_gphone64_arm64/emu64a:15/AE3A.240806.043/12960925:userdebug/dev-keys",
        "manufacturer": "Google",
        "model": "sdk_gphone64_arm64",
        "device": "emu64a",
        "display": {
            "widthPixels": 1080,
            "heightPixels": 2400,
            "densityDpi": 420,
            "rotationDegrees": 0,
        },
        "configuration": {
            "fontScale": 1.0,
            "locale": "en-US",
            "layoutDirection": "ltr",
            "nightMode": "light",
        },
        "systemBarInsetsDevicePixels": {
            "left": 0,
            "top": 0,
            "right": 0,
            "bottom": 0,
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
            "adbVersion",
            "emulatorVersion",
            "avdName",
            "instrumentationCommand",
            "limitations",
        },
        set(),
        "capture.capture",
    )
    if capture["order"] != [
        "waitForIdle",
        "viewAndAccessibilityHierarchy",
        "screenshot",
    ]:
        fail("capture-compatibility", "capture sequence is unsupported")
    if capture["atomic"] is not False or capture["animationsDisabled"] is not True:
        fail(
            "capture-compatibility",
            "capture synchronization declaration is unsupported",
        )
    for name in ("adbVersion", "emulatorVersion"):
        value = utf8_text(capture[name], f"capture.capture.{name}", 32)
        if VERSION.fullmatch(value) is None:
            fail("capture-invalid", f"capture.capture.{name} is invalid")
    token(capture["avdName"], "capture.capture.avdName")
    if {
        "adbVersion": capture["adbVersion"],
        "emulatorVersion": capture["emulatorVersion"],
        "avdName": capture["avdName"],
    } != {
        "adbVersion": "1.0.41",
        "emulatorVersion": "36.4.10.0",
        "avdName": "Pixel_8",
    }:
        fail("capture-compatibility", "capture Android tool profile is unsupported")
    utf8_text(
        capture["instrumentationCommand"],
        "capture.capture.instrumentationCommand",
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

    hierarchy = record(root["hierarchy"], "capture.hierarchy")
    exact_keys(
        hierarchy,
        {"rootResourceId", "unidentifiedNodeCount", "nodes"},
        set(),
        "capture.hierarchy",
    )
    root_id = resource_id(
        hierarchy["rootResourceId"], "capture.hierarchy.rootResourceId"
    )
    bounded_integer(
        hierarchy["unidentifiedNodeCount"],
        "capture.hierarchy.unidentifiedNodeCount",
        0,
        10_000,
    )
    if not isinstance(hierarchy["nodes"], list) or not hierarchy["nodes"]:
        fail("capture-invalid", "capture.hierarchy.nodes must be a nonempty array")
    if len(hierarchy["nodes"]) > execution["maxNodes"]:
        fail("node-budget", "capture node count exceeds maxNodes")
    values = hierarchy["nodes"]
    nodes = [
        validate_node(value, index, execution) for index, value in enumerate(values)
    ]
    ids = [node["resourceId"] for node in nodes]
    if len(ids) != len(set(ids)):
        fail("duplicate-node", "capture hierarchy repeats a resource ID")
    known = set(ids)
    if root_id not in known:
        fail("capture-invalid", "capture hierarchy rootResourceId is missing")
    for node in nodes:
        parent = node["parentResourceId"]
        if parent is not None and parent not in known:
            fail(
                "dangling-parent",
                f"capture node {node['resourceId']} has an unknown parent",
            )
    parents = {node["resourceId"]: node["parentResourceId"] for node in nodes}
    for identifier in known:
        seen: set[str] = set()
        current: Optional[str] = identifier
        while current is not None:
            if current in seen:
                fail("hierarchy-cycle", "capture hierarchy contains a parent cycle")
            seen.add(current)
            current = parents[current]

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
    if device_insets != device["systemBarInsetsDevicePixels"]:
        fail("capture-invalid", "capture device insets are invalid")
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


def mapping_status(node: dict[str, Any]) -> str:
    box = node["layoutBoundsDevicePixels"]
    if not node["viewState"]["shown"]:
        return "notMappedNotShown"
    if not node["globalVisible"]["value"]:
        return "notMappedNotGloballyVisible"
    if not node["identityTransform"]:
        return "notMappedUnsupportedTransform"
    if box["width"] == 0 or box["height"] == 0:
        return "notMappedEmptyLayout"
    return "mappedExactLayout"


def core_kind(node: dict[str, Any]) -> str:
    if node["viewState"]["clickable"]:
        return "control"
    class_name = node["className"]
    if class_name.endswith("TextView"):
        return "text"
    if class_name.endswith("Layout") or class_name.endswith("ScrollView"):
        return "container"
    return "other"


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
    if display["rotationDegrees"] in {0, 180} and pixel_width > pixel_height:
        fail("orientation-conflict", "portrait rotation conflicts with PNG extent")
    if display["rotationDegrees"] in {90, 270} and pixel_width < pixel_height:
        fail("orientation-conflict", "landscape rotation conflicts with PNG extent")

    display_evidence_id = "e-android:display:0:source"
    render_evidence_id = "e-android:display:0:render"
    mapped_ids: set[str] = set()
    extension_nodes: list[dict[str, Any]] = []
    evidence: list[dict[str, Any]] = [
        {
            "id": display_evidence_id,
            "class": "exactSource",
            "source": {
                "adapter": "sightlint-android",
                "adapterVersion": ADAPTER_VERSION,
                "inputDigest": request["capture"]["sha256"],
                "externalProcessing": False,
            },
            "selector": {"type": "nativeId", "nativeId": "android:display:0"},
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

    ordered_capture_nodes = sorted(
        capture["hierarchy"]["nodes"], key=lambda item: item["resourceId"]
    )
    for captured in ordered_capture_nodes:
        identifier = f"android:view:{captured['resourceId']}"
        view_evidence_id = f"e-android:view:{captured['resourceId']}:source"
        accessibility_evidence_id = (
            f"e-android:view:{captured['resourceId']}:accessibility"
        )
        status = mapping_status(captured)
        if status == "mappedExactLayout":
            mapped_ids.add(captured["resourceId"])
        evidence.extend(
            [
                {
                    "id": view_evidence_id,
                    "class": "exactSource",
                    "source": {
                        "adapter": "sightlint-android",
                        "adapterVersion": ADAPTER_VERSION,
                        "inputDigest": request["capture"]["sha256"],
                        "externalProcessing": False,
                    },
                    "selector": {"type": "nativeId", "nativeId": identifier},
                },
                {
                    "id": accessibility_evidence_id,
                    "class": "platformSemantics",
                    "source": {
                        "adapter": "sightlint-android",
                        "adapterVersion": ADAPTER_VERSION,
                        "inputDigest": request["capture"]["sha256"],
                        "externalProcessing": False,
                    },
                    "selector": {
                        "type": "nativeId",
                        "nativeId": f"android:accessibility:{captured['resourceId']}",
                    },
                },
            ]
        )
        extension_nodes.append(
            {
                "id": identifier,
                "resourceId": captured["resourceId"],
                "parentResourceId": captured["parentResourceId"],
                "depth": captured["depth"],
                "className": captured["className"],
                "mappingStatus": status,
                "layoutBoundsDevicePixels": captured["layoutBoundsDevicePixels"],
                "identityTransform": captured["identityTransform"],
                "globalVisible": captured["globalVisible"],
                "viewState": captured["viewState"],
                "text": captured["text"],
                "contentDescription": captured["contentDescription"],
                "viewEvidenceId": view_evidence_id,
                "accessibilityEvidenceId": accessibility_evidence_id,
                "accessibility": captured["accessibility"],
            }
        )

    nodes: list[dict[str, Any]] = []
    for captured, extension_node in zip(ordered_capture_nodes, extension_nodes):
        if extension_node["mappingStatus"] != "mappedExactLayout":
            continue
        node: dict[str, Any] = {
            "id": extension_node["id"],
            "kind": {
                "value": core_kind(captured),
                "evidenceId": extension_node["viewEvidenceId"],
            },
            "coordinateSpaceId": "android:screen:display-0",
            "geometry": {
                "layoutBox": {
                    "rect": captured["layoutBoundsDevicePixels"],
                    "coordinateSpaceId": "android:screen:display-0",
                    "evidenceId": extension_node["viewEvidenceId"],
                }
            },
        }
        parent = captured["parentResourceId"]
        if parent in mapped_ids:
            node["parentId"] = f"android:view:{parent}"
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
            "runner": {
                **capture["runner"],
                "adbVersion": capture_details["adbVersion"],
                "emulatorVersion": capture_details["emulatorVersion"],
                "avdName": capture_details["avdName"],
            },
            "build": {
                name: capture["build"][name]
                for name in (
                    "gradleVersion",
                    "androidGradlePluginVersion",
                    "javaLanguageVersion",
                    "compileSdk",
                )
            },
            "device": capture["device"],
            "sequence": capture_details["order"],
            "atomic": capture_details["atomic"],
            "animationsDisabled": capture_details["animationsDisabled"],
            "instrumentationCommand": capture_details["instrumentationCommand"],
            "limitations": sorted(capture_details["limitations"]),
            "rootResourceId": capture["hierarchy"]["rootResourceId"],
        },
        "screen": {
            "coordinateSpaceId": "android:screen:display-0",
            "display": display,
            "systemBarInsetsDevicePixels": capture["device"][
                "systemBarInsetsDevicePixels"
            ],
            "displayEvidenceId": display_evidence_id,
            "screenshot": {
                "reference": request["screenshot"]["reference"],
                "sha256": request["screenshot"]["sha256"],
                "widthPixels": pixel_width,
                "heightPixels": pixel_height,
                "canvasId": "android:render:display-0",
                "evidenceId": render_evidence_id,
                "extentReconciliation": "agreement",
                "nodeIdentity": "cantTell",
            },
        },
        "nodes": extension_nodes,
        "coverage": COVERAGE,
        "unsupported": {
            "unidentifiedNodeCount": capture["hierarchy"]["unidentifiedNodeCount"],
            "features": sorted(
                {
                    "composeSemantics",
                    "dynamicBehavior",
                    "occlusionAndInkGeometry",
                    "renderedNodeIdentity",
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
                "id": "android:render:display-0",
                "size": {"width": pixel_width, "height": pixel_height},
                "unit": "devicePixel",
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": render_evidence_id,
            },
            {
                "id": "android:screen:display-0",
                "size": {
                    "width": display["widthPixels"],
                    "height": display["heightPixels"],
                },
                "unit": "devicePixel",
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": display_evidence_id,
            },
        ],
        "nodes": nodes,
        "evidence": sorted(evidence, key=lambda item: item["id"]),
        "extensions": {"org.sightlint.android": extension},
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
            "name": "sightlint-android",
            "version": ADAPTER_VERSION,
            "runtime": {
                "name": "python",
                "version": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
            },
        },
        "captureSha256": request["capture"]["sha256"],
        "screenshotSha256": request["screenshot"]["sha256"],
        "nodeCount": len(extension_nodes),
        "mappedNodeCount": len(nodes),
        "excludedNodeCount": len(extension_nodes) - len(nodes),
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
    parser = argparse.ArgumentParser(prog="sightlint-android")
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
            fail("output-io", "cannot write Artifact IR output")
        sys.stdout.buffer.write(response)
        return 0
    except AdapterError as error:
        sys.stderr.buffer.write(
            f"sightlint-android: {error.code}: {error.message}\n".encode("utf-8")
        )
        return 2
    except Exception:
        sys.stderr.buffer.write(
            b"sightlint-android: execution-error: adapter execution failed\n"
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
