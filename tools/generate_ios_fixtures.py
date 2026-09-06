#!/usr/bin/env python3
"""Capture or verify the bounded iOS fixture corpus from ADR 0046."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import re
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "fixtures" / "ios" / "atlas-app"
CAPTURES = ROOT / "evaluation" / "ios" / "captures"
DIGEST_PREFIX = "sha256:"
XCODE_VERSION = "26.3"
XCODE_BUILD = "17C529"
SWIFT_VERSION = "6.2.4"
SDK_VERSION = "26.2"
RUNTIME_VERSION = "26.3.1"
RUNTIME_BUILD = "23D8133"
DEVICE_TYPE = "iPhone 17 Pro"
MODEL_IDENTIFIER = "iPhone18,1"
ARCHITECTURE = "arm64"
SCREEN_POINTS = (402, 874)
SCREEN_PIXELS = (1206, 2622)
SCREEN_SCALE = 3
SAFE_AREA = {"left": 0, "top": 62, "right": 0, "bottom": 34}
SOURCE_FILES = (
    "AtlasApp.xcodeproj/project.pbxproj",
    "AtlasApp.xcodeproj/xcshareddata/xcschemes/AtlasApp.xcscheme",
    "AtlasApp/AppDelegate.swift",
    "AtlasApp/AtlasViewController.swift",
    "AtlasApp/Info.plist",
    "AtlasApp/SceneDelegate.swift",
    "AtlasCaptureUITests/AtlasCaptureUITests.swift",
)
SCENARIOS: dict[str, dict[str, Any]] = {
    "clean": {
        "test": "testCaptureClean",
        "manifestSha256": "sha256:78d6567b0359f83b1e67717498c1a793312cfa742784447092633e2d96e29e37",
        "screenshotSha256": "sha256:accaa375f338ead217076ec5242baeec3436e58f8677eeaec4600ae661179d54",
        "sourceNodeCount": 22,
        "xcuiNodeCount": 15,
    },
    "off-canvas-control-mutant": {
        "test": "testCaptureOffCanvasControlMutant",
        "manifestSha256": "sha256:e5b040e303bac15354f79aa3e3466b25529962869d2e836a8b574796b4d3449c",
        "screenshotSha256": "sha256:9d6115b9458f92daa925a0961df6c7ea3fb3c32958f1f93b1a8831d8ce707e9a",
        "sourceNodeCount": 22,
        "xcuiNodeCount": 15,
    },
    "scroll-offscreen-hard-negative": {
        "test": "testCaptureScrollOffscreenHardNegative",
        "manifestSha256": "sha256:d118afcd3fa3e5b2f2ad555e2513e207a6c87010e44091ff09a5cc83e1d7340a",
        "screenshotSha256": "sha256:accaa375f338ead217076ec5242baeec3436e58f8677eeaec4600ae661179d54",
        "sourceNodeCount": 25,
        "xcuiNodeCount": 17,
    },
}


def fail(message: str) -> None:
    """Exit with one stable checker prefix."""
    raise SystemExit(f"iOS fixture error: {message}")


def require(condition: bool, message: str) -> None:
    """Require one fixture invariant."""
    if not condition:
        fail(message)


def sha256_bytes(value: bytes) -> str:
    """Return a prefixed SHA-256 digest."""
    return DIGEST_PREFIX + hashlib.sha256(value).hexdigest()


def sha256(path: Path) -> str:
    """Return a prefixed streaming SHA-256 digest."""
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(65_536):
            hasher.update(chunk)
    return DIGEST_PREFIX + hasher.hexdigest()


def source_digest() -> str:
    """Digest the exact project and capture source inputs."""
    hasher = hashlib.sha256()
    for reference in SOURCE_FILES:
        path = APP / reference
        require(path.is_file(), f"missing fixture source {reference}")
        hasher.update(reference.encode("utf-8"))
        hasher.update(b"\0")
        hasher.update(path.read_bytes())
        hasher.update(b"\0")
    return DIGEST_PREFIX + hasher.hexdigest()


def load_json(path: Path) -> Any:
    """Load UTF-8 JSON while rejecting duplicate keys."""
    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate key {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(path.read_bytes(), object_pairs_hook=no_duplicates)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        try:
            reference = path.relative_to(ROOT)
        except ValueError:
            reference = path
        fail(f"{reference}: {error}")


def canonical_json(value: Any) -> bytes:
    """Encode one canonical fixture JSON document."""
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def png_size(path: Path) -> tuple[int, int]:
    """Read only PNG signature and IHDR dimensions."""
    header = path.read_bytes()[:24]
    require(
        len(header) == 24 and header[:8] == b"\x89PNG\r\n\x1a\n",
        f"{path} is not a PNG",
    )
    require(header[12:16] == b"IHDR", f"{path} lacks IHDR")
    return struct.unpack(">II", header[16:24])


def finite_rect(rect: Any) -> bool:
    """Return whether a value is one bounded nonnegative-size rectangle."""
    if not isinstance(rect, dict) or set(rect) != {"x", "y", "width", "height"}:
        return False
    values = [rect[key] for key in ("x", "y", "width", "height")]
    return (
        all(isinstance(value, (int, float)) and not isinstance(value, bool) for value in values)
        and all(math.isfinite(float(value)) and abs(float(value)) <= 1_048_576 for value in values)
        and rect["width"] >= 0
        and rect["height"] >= 0
    )


def normalize_source(raw: dict[str, Any]) -> dict[str, Any]:
    """Make capture-time optional source facts explicit without inventing values."""
    require(set(raw) == {"nodes", "rootIdentifier", "unidentifiedNodeCount"},
            "raw source hierarchy shape drift")
    nodes = []
    for raw_node in raw["nodes"]:
        node = dict(raw_node)
        for key in (
            "parentIdentifier", "windowIntersectionPoints", "safeAreaIntersectionPoints",
            "label", "value",
        ):
            node.setdefault(key, None)
        state = dict(node["state"])
        state.setdefault("enabled", None)
        state.setdefault("selected", None)
        node["state"] = state
        nodes.append(node)
    return {
        "rootIdentifier": raw["rootIdentifier"],
        "unidentifiedNodeCount": raw["unidentifiedNodeCount"],
        "nodes": sorted(nodes, key=lambda node: node["identifier"]),
    }


def normalize_xcui(raw: dict[str, Any]) -> dict[str, Any]:
    """Make capture-time optional XCUI facts explicit."""
    require(set(raw) == {"nodes", "queryRoot", "unmatchedQueryCount"},
            "raw XCUI hierarchy shape drift")
    nodes = []
    for raw_node in raw["nodes"]:
        node = dict(raw_node)
        for key in ("framePoints", "label", "value", "title", "placeholder"):
            node.setdefault(key, None)
        nodes.append(node)
    return {
        "queryRoot": raw["queryRoot"],
        "unmatchedQueryCount": raw["unmatchedQueryCount"],
        "nodes": sorted(nodes, key=lambda node: node["identifier"]),
    }


def index_nodes(nodes: Any, scenario: str, source: str) -> dict[str, dict[str, Any]]:
    """Index nodes while enforcing stable unique identifiers."""
    require(isinstance(nodes, list), f"{scenario} {source} nodes are not an array")
    result: dict[str, dict[str, Any]] = {}
    for node in nodes:
        require(isinstance(node, dict), f"{scenario} has a non-object {source} node")
        identifier = node.get("identifier")
        require(
            isinstance(identifier, str)
            and re.fullmatch(r"[A-Za-z][A-Za-z0-9_.-]{0,127}", identifier) is not None,
            f"{scenario} has an invalid {source} identifier",
        )
        require(identifier not in result, f"{scenario} repeats {source} identifier {identifier}")
        result[identifier] = node
    return result


def verify_string_fact(value: Any, scenario: str, identifier: str) -> None:
    """Verify one redacted string fact."""
    if value is None:
        return
    require(
        isinstance(value, dict)
        and set(value) == {"sha256", "utf8ByteLength"}
        and isinstance(value["utf8ByteLength"], int)
        and 1 <= value["utf8ByteLength"] <= 1024
        and re.fullmatch(r"sha256:[0-9a-f]{64}", value["sha256"]) is not None,
        f"{scenario} node {identifier} has invalid string fact",
    )


def verify_source_nodes(nodes: dict[str, dict[str, Any]], scenario: str) -> None:
    """Verify exact source geometry and explicit nullable facts."""
    for identifier, node in nodes.items():
        require(finite_rect(node.get("layoutBoundsPoints")),
                f"{scenario} source node {identifier} has invalid layout bounds")
        for key in ("windowIntersectionPoints", "safeAreaIntersectionPoints"):
            value = node.get(key)
            require(value is None or finite_rect(value),
                    f"{scenario} source node {identifier} has invalid {key}")
        state = node.get("state")
        require(
            isinstance(state, dict)
            and set(state) == {
                "alpha", "enabled", "hidden", "selected", "userInteractionEnabled",
                "windowAttached",
            }
            and isinstance(state["alpha"], (int, float))
            and 0 <= state["alpha"] <= 1,
            f"{scenario} source node {identifier} has invalid state",
        )
        verify_string_fact(node.get("label"), scenario, identifier)
        verify_string_fact(node.get("value"), scenario, identifier)


def verify_xcui_nodes(nodes: dict[str, dict[str, Any]], scenario: str) -> None:
    """Verify XCUI facts without promoting them to source geometry."""
    for identifier, node in nodes.items():
        status = node.get("frameStatus")
        frame = node.get("framePoints")
        require(status in {"exact", "unavailable"},
                f"{scenario} XCUI node {identifier} has invalid frame status")
        require((status == "exact" and finite_rect(frame)) or (status == "unavailable" and frame is None),
                f"{scenario} XCUI node {identifier} has inconsistent frame")
        for key in ("label", "value", "title", "placeholder"):
            verify_string_fact(node.get(key), scenario, identifier)


def verify_manifest(scenario: str, expected: dict[str, Any], source: str) -> dict[str, Any]:
    """Verify one committed combined capture and screenshot."""
    manifest_path = CAPTURES / f"{scenario}.capture.json"
    screenshot_path = CAPTURES / f"{scenario}.png"
    require(manifest_path.is_file(), f"missing capture manifest for {scenario}")
    require(screenshot_path.is_file(), f"missing screenshot for {scenario}")
    if expected["manifestSha256"] != "pending":
        require(sha256(manifest_path) == expected["manifestSha256"],
                f"{scenario} capture manifest digest drift")
    if expected["screenshotSha256"] != "pending":
        require(sha256(screenshot_path) == expected["screenshotSha256"],
                f"{scenario} screenshot digest drift")

    manifest = load_json(manifest_path)
    require(manifest.get("captureVersion") == "0.1.0", f"{scenario} capture version drift")
    require(manifest.get("captureId") == f"ios-atlas-{scenario}", f"{scenario} capture ID drift")
    require(manifest.get("scenario") == scenario, f"{scenario} scenario drift")
    require(manifest.get("application") == {
        "bundleIdentifier": "org.sightlint.fixtures.atlas.ios",
        "version": "0.1.0",
        "buildNumber": "1",
        "testBundleIdentifier": "org.sightlint.fixtures.atlas.ios.capture-tests",
    }, f"{scenario} application identity drift")
    require(manifest.get("runner") == {
        "name": "sightlint-atlas-ios-capture",
        "version": "0.1.0",
        "captureApi": "instrumented-uikit-and-xcui",
    }, f"{scenario} runner identity drift")
    require(manifest.get("build") == {
        "fixtureSourceSha256": source,
        "xcodeVersion": XCODE_VERSION,
        "xcodeBuild": XCODE_BUILD,
        "swiftVersion": SWIFT_VERSION,
        "sdkVersion": SDK_VERSION,
        "deploymentTarget": "26.0",
    }, f"{scenario} build profile drift")
    require(manifest.get("device") == device_record(), f"{scenario} device profile drift")
    capture = manifest.get("capture", {})
    require(capture.get("order") == [
        "waitForQuiescence", "sourceHierarchy", "screenshot", "xcuiHierarchy"
    ], f"{scenario} capture order drift")
    require(capture.get("atomic") is False and capture.get("animationsDisabled") is True,
            f"{scenario} synchronization disclosure drift")
    require(isinstance(capture.get("testCommand"), str) and capture["testCommand"],
            f"{scenario} test command missing")
    require(isinstance(capture.get("limitations"), list) and capture["limitations"],
            f"{scenario} limitations missing")

    screenshot = manifest.get("screenshot", {})
    expected_reference = f"evaluation/ios/captures/{scenario}.png"
    require(screenshot.get("reference") == expected_reference,
            f"{scenario} screenshot reference drift")
    require(screenshot.get("sha256") == sha256(screenshot_path),
            f"{scenario} screenshot digest mismatch")
    require(screenshot.get("captureSequence") == 3,
            f"{scenario} screenshot sequence drift")
    require(png_size(screenshot_path) == SCREEN_PIXELS,
            f"{scenario} screenshot extent drift")
    require((screenshot.get("widthPixels"), screenshot.get("heightPixels")) == SCREEN_PIXELS,
            f"{scenario} screenshot metadata extent drift")

    source_hierarchy = manifest.get("sourceHierarchy", {})
    xcui_hierarchy = manifest.get("xcuiHierarchy", {})
    require(source_hierarchy.get("rootIdentifier") == "screen_root",
            f"{scenario} source root drift")
    require(source_hierarchy.get("unidentifiedNodeCount") == 15,
            f"{scenario} unidentified source node count drift")
    require(xcui_hierarchy.get("queryRoot") == "XCUIApplication",
            f"{scenario} XCUI root drift")
    require(xcui_hierarchy.get("unmatchedQueryCount") == 0,
            f"{scenario} unmatched XCUI query drift")
    source_nodes = index_nodes(source_hierarchy.get("nodes"), scenario, "source")
    xcui_nodes = index_nodes(xcui_hierarchy.get("nodes"), scenario, "XCUI")
    require(len(source_nodes) == expected["sourceNodeCount"],
            f"{scenario} source node count drift")
    require(len(xcui_nodes) == expected["xcuiNodeCount"],
            f"{scenario} XCUI node count drift")
    verify_source_nodes(source_nodes, scenario)
    verify_xcui_nodes(xcui_nodes, scenario)
    return manifest


def device_record() -> dict[str, Any]:
    """Return the exact admitted simulator profile."""
    return {
        "runtimeVersion": RUNTIME_VERSION,
        "runtimeBuild": RUNTIME_BUILD,
        "deviceType": DEVICE_TYPE,
        "modelIdentifier": MODEL_IDENTIFIER,
        "architecture": ARCHITECTURE,
        "display": {
            "widthPoints": SCREEN_POINTS[0],
            "heightPoints": SCREEN_POINTS[1],
            "scale": SCREEN_SCALE,
            "widthPixels": SCREEN_PIXELS[0],
            "heightPixels": SCREEN_PIXELS[1],
            "orientation": "portrait",
        },
        "configuration": {
            "locale": "en-US",
            "layoutDirection": "ltr",
            "contentSizeCategory": "large",
            "interfaceStyle": "light",
            "reduceMotion": False,
        },
        "safeAreaInsetsPoints": SAFE_AREA,
    }


def verify_relations(manifests: dict[str, dict[str, Any]]) -> None:
    """Verify the targeted mutation and hard-negative isolation."""
    clean = index_nodes(manifests["clean"]["sourceHierarchy"]["nodes"], "clean", "source")
    mutant = index_nodes(
        manifests["off-canvas-control-mutant"]["sourceHierarchy"]["nodes"],
        "off-canvas-control-mutant",
        "source",
    )
    require(clean.keys() == mutant.keys(), "targeted mutation changed source identity")
    for identifier in clean:
        if identifier != "save_button":
            require(clean[identifier] == mutant[identifier],
                    f"targeted mutation changed unrelated source node {identifier}")
    clean_save = clean["save_button"]
    mutant_save = mutant["save_button"]
    require(clean_save["layoutBoundsPoints"]["x"] == 24,
            "clean save-button source x drift")
    require(mutant_save["layoutBoundsPoints"] == {
        "x": 300, "y": clean_save["layoutBoundsPoints"]["y"], "width": 354, "height": 52
    }, "mutant save-button source layout drift")
    require(mutant_save["windowIntersectionPoints"] == {
        "x": 300, "y": clean_save["layoutBoundsPoints"]["y"], "width": 102, "height": 52
    }, "mutant save-button window intersection drift")
    for field in (
        "parentIdentifier", "className", "identityTransform", "state", "label", "value",
    ):
        require(clean_save[field] == mutant_save[field],
                f"targeted mutation changed save-button {field}")

    hard = index_nodes(
        manifests["scroll-offscreen-hard-negative"]["sourceHierarchy"]["nodes"],
        "scroll-offscreen-hard-negative",
        "source",
    )
    require(set(hard) - set(clean) == {"archived_card", "archived_detail", "archived_title"},
            "hard-negative source node set drift")
    for identifier in ("archived_card", "archived_detail", "archived_title"):
        require(hard[identifier]["windowIntersectionPoints"] is None,
                f"hard-negative {identifier} became window-visible")
    hard_xcui = index_nodes(
        manifests["scroll-offscreen-hard-negative"]["xcuiHierarchy"]["nodes"],
        "scroll-offscreen-hard-negative",
        "XCUI",
    )
    for identifier in ("archived_detail", "archived_title"):
        require(hard_xcui[identifier]["exists"] is True,
                f"hard-negative {identifier} lost XCUI semantics")
        require(hard_xcui[identifier]["hittable"] is False,
                f"hard-negative {identifier} unexpectedly became hittable")


def verify() -> None:
    """Verify committed captures and independently fixed relations."""
    source = source_digest()
    manifests = {
        scenario: verify_manifest(scenario, expected, source)
        for scenario, expected in SCENARIOS.items()
    }
    verify_relations(manifests)
    print(
        "iOS fixture corpus verified: 3 Xcode-26.3/iOS-26.3.1 captures, "
        "one targeted mutation, one offscreen XCUI hard negative"
    )


def run(command: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    """Run one capture command without a shell."""
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        stderr = getattr(error, "stderr", "")
        fail(f"capture command failed: {command[0]}: {stderr or error}")


def exact_version(pattern: str, output: str, expected: str, name: str) -> None:
    """Require one exact capture tool version."""
    match = re.search(pattern, output)
    require(match is not None and match.group(1) == expected,
            f"expected {name} {expected}, got {output.strip()}")


def attachment_path(directory: Path, scenario: str, kind: str) -> Path:
    """Resolve one attachment by its stable suggested-name prefix."""
    manifest = load_json(directory / "manifest.json")
    matches = []
    prefix = f"sightlint-{kind}-{scenario}_"
    for test in manifest:
        for attachment in test.get("attachments", []):
            if attachment.get("suggestedHumanReadableName", "").startswith(prefix):
                matches.append(directory / attachment["exportedFileName"])
    require(len(matches) == 1, f"expected one {kind} attachment for {scenario}")
    return matches[0]


def capture(device_id: str) -> None:
    """Build, run, and combine the explicit iOS fixture captures."""
    require(platform.system() == "Darwin", "--capture requires macOS")
    xcode = run(["xcodebuild", "-version"]).stdout
    exact_version(r"Xcode ([0-9.]+)", xcode, XCODE_VERSION, "Xcode")
    exact_version(r"Build version ([A-Za-z0-9]+)", xcode, XCODE_BUILD, "Xcode build")
    swift = run(["swift", "--version"]).stdout
    exact_version(r"Swift version ([0-9.]+)", swift, SWIFT_VERSION, "Swift")
    sdk = run(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-version"]).stdout.strip()
    require(sdk == SDK_VERSION, f"expected iOS simulator SDK {SDK_VERSION}, got {sdk}")

    devices = load_json_bytes(run(["xcrun", "simctl", "list", "devices", "-j"]).stdout)
    selected = [
        device
        for runtime, entries in devices.get("devices", {}).items()
        if runtime.endswith("iOS-26-3")
        for device in entries
        if device.get("udid") == device_id
    ]
    require(len(selected) == 1 and selected[0].get("name") == DEVICE_TYPE,
            f"device {device_id} is not the pinned {DEVICE_TYPE} runtime")

    run(["xcrun", "simctl", "boot", device_id]) if selected[0].get("state") != "Booted" else None
    run(["xcrun", "simctl", "bootstatus", device_id, "-b"])
    model = run(["xcrun", "simctl", "getenv", device_id, "SIMULATOR_MODEL_IDENTIFIER"]).stdout.strip()
    runtime = run(["xcrun", "simctl", "getenv", device_id, "SIMULATOR_RUNTIME_VERSION"]).stdout.strip()
    runtime_build = run([
        "xcrun", "simctl", "getenv", device_id, "SIMULATOR_RUNTIME_BUILD_VERSION"
    ]).stdout.strip()
    architectures = run(["xcrun", "simctl", "getenv", device_id, "SIMULATOR_ARCHS"]).stdout.strip()
    require((model, runtime, runtime_build, architectures) == (
        MODEL_IDENTIFIER, RUNTIME_VERSION, RUNTIME_BUILD, ARCHITECTURE
    ), "simulator profile drift")

    run(["xcrun", "simctl", "ui", device_id, "appearance", "light"])
    run(["xcrun", "simctl", "ui", device_id, "content_size", "large"])
    run(["xcrun", "simctl", "ui", device_id, "increase_contrast", "disabled"])
    run(["xcrun", "simctl", "status_bar", device_id, "clear"])
    run([
        "xcrun", "simctl", "status_bar", device_id, "override",
        "--time", "9:41", "--dataNetwork", "wifi", "--wifiMode", "active",
        "--wifiBars", "3", "--cellularMode", "active", "--cellularBars", "4",
        "--operatorName", "", "--batteryState", "charged", "--batteryLevel", "100",
    ])

    CAPTURES.mkdir(parents=True, exist_ok=True)
    source = source_digest()
    with tempfile.TemporaryDirectory(prefix="sightlint-ios-capture-") as temporary:
        temporary_path = Path(temporary)
        derived = temporary_path / "DerivedData"
        for scenario, expected in SCENARIOS.items():
            result = temporary_path / f"{scenario}.xcresult"
            attachments = temporary_path / f"{scenario}-attachments"
            test_identifier = (
                "AtlasCaptureUITests/AtlasCaptureUITests/" + expected["test"]
            )
            command = [
                "xcodebuild", "-quiet", "-project", str(APP / "AtlasApp.xcodeproj"),
                "-scheme", "AtlasApp", "-configuration", "Debug",
                "-destination", f"platform=iOS Simulator,id={device_id}",
                "-derivedDataPath", str(derived), "-resultBundlePath", str(result),
                "-only-testing:" + test_identifier, "test",
            ]
            run(command)
            run([
                "xcrun", "xcresulttool", "export", "attachments", "--path", str(result),
                "--output-path", str(attachments),
            ])
            screenshot_input = attachment_path(attachments, scenario, "screen")
            xcui_input = attachment_path(attachments, scenario, "xcui")
            container = run([
                "xcrun", "simctl", "get_app_container", device_id,
                "org.sightlint.fixtures.atlas.ios", "data",
            ]).stdout.strip()
            source_input = Path(container) / "Documents" / "sightlint-source.json"
            require(source_input.is_file(), f"missing source attachment for {scenario}")
            source_hierarchy = normalize_source(load_json(source_input))
            xcui_hierarchy = normalize_xcui(load_json(xcui_input))
            screen_root = index_nodes(source_hierarchy["nodes"], scenario, "source")["screen_root"]
            require(screen_root["layoutBoundsPoints"] == {
                "x": 0, "y": 0, "width": SCREEN_POINTS[0], "height": SCREEN_POINTS[1]
            }, f"{scenario} screen point extent drift")
            require(screen_root["safeAreaIntersectionPoints"] == {
                "x": SAFE_AREA["left"],
                "y": SAFE_AREA["top"],
                "width": SCREEN_POINTS[0] - SAFE_AREA["left"] - SAFE_AREA["right"],
                "height": SCREEN_POINTS[1] - SAFE_AREA["top"] - SAFE_AREA["bottom"],
            }, f"{scenario} safe area drift")
            require(png_size(screenshot_input) == SCREEN_PIXELS,
                    f"{scenario} screenshot profile drift")
            screenshot_output = CAPTURES / f"{scenario}.png"
            shutil.copyfile(screenshot_input, screenshot_output)
            screenshot_digest = sha256(screenshot_output)
            manifest = {
                "captureVersion": "0.1.0",
                "captureId": f"ios-atlas-{scenario}",
                "scenario": scenario,
                "application": {
                    "bundleIdentifier": "org.sightlint.fixtures.atlas.ios",
                    "version": "0.1.0",
                    "buildNumber": "1",
                    "testBundleIdentifier": "org.sightlint.fixtures.atlas.ios.capture-tests",
                },
                "runner": {
                    "name": "sightlint-atlas-ios-capture",
                    "version": "0.1.0",
                    "captureApi": "instrumented-uikit-and-xcui",
                },
                "build": {
                    "fixtureSourceSha256": source,
                    "xcodeVersion": XCODE_VERSION,
                    "xcodeBuild": XCODE_BUILD,
                    "swiftVersion": SWIFT_VERSION,
                    "sdkVersion": SDK_VERSION,
                    "deploymentTarget": "26.0",
                },
                "device": device_record(),
                "capture": {
                    "order": [
                        "waitForQuiescence", "sourceHierarchy", "screenshot", "xcuiHierarchy"
                    ],
                    "atomic": False,
                    "animationsDisabled": True,
                    "testCommand": (
                        "xcodebuild -project fixtures/ios/atlas-app/AtlasApp.xcodeproj "
                        "-scheme AtlasApp -destination 'platform=iOS Simulator,name=iPhone 17 Pro,"
                        "OS=26.3.1' -only-testing:" + test_identifier + " test"
                    ),
                    "limitations": [
                        "The source observation precedes the screenshot and later XCUI queries; capture is not atomic.",
                        "XCUI queries may change scroll state after the screenshot is attached.",
                        "Only named repository-owned UIKit fixture elements are queried.",
                        "XCUI frames are platform semantics, not touch or rendered bounds.",
                    ],
                },
                "sourceHierarchy": source_hierarchy,
                "xcuiHierarchy": xcui_hierarchy,
                "screenshot": {
                    "reference": f"evaluation/ios/captures/{scenario}.png",
                    "sha256": screenshot_digest,
                    "widthPixels": SCREEN_PIXELS[0],
                    "heightPixels": SCREEN_PIXELS[1],
                    "captureSequence": 3,
                },
            }
            manifest_output = CAPTURES / f"{scenario}.capture.json"
            manifest_output.write_bytes(canonical_json(manifest))
            print(
                f"{scenario}: manifest {sha256(manifest_output)}, screenshot {screenshot_digest}"
            )
    print("iOS fixture captures refreshed; pin digests only after source and screenshot review")


def load_json_bytes(value: str) -> Any:
    """Parse JSON returned by one trusted local tool."""
    try:
        return json.loads(value)
    except json.JSONDecodeError as error:
        fail(f"capture tool returned invalid JSON: {error}")


def main() -> None:
    """Parse arguments and capture or verify."""
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--capture", action="store_true")
    parser.add_argument("--device-id")
    arguments = parser.parse_args()
    if arguments.check:
        verify()
        return
    require(arguments.device_id is not None, "--device-id is required for capture")
    capture(arguments.device_id)


if __name__ == "__main__":
    main()
