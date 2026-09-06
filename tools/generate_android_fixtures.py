#!/usr/bin/env python3
"""Capture or verify the bounded Android fixture corpus from ADR 0045."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "fixtures" / "android" / "atlas-app"
CAPTURES = ROOT / "evaluation" / "android" / "captures"
DIGEST_PREFIX = "sha256:"
GRADLE_VERSION = "8.13"
AGP_VERSION = "8.10.1"
ADB_VERSION = "1.0.41"
EMULATOR_VERSION = "36.4.10.0"
AVD_NAME = "Pixel_8"
BUILD_FINGERPRINT = (
    "google/sdk_gphone64_arm64/emu64a:15/AE3A.240806.043/12960925:userdebug/dev-keys"
)
SOURCE_FILES = (
    "app/build.gradle",
    "app/src/androidTest/java/org/sightlint/fixtures/atlas/CaptureInstrumentation.java",
    "app/src/main/AndroidManifest.xml",
    "app/src/main/java/org/sightlint/fixtures/atlas/MainActivity.java",
    "app/src/main/res/values/colors.xml",
    "app/src/main/res/values/ids.xml",
    "app/src/main/res/values/strings.xml",
    "app/src/main/res/values/styles.xml",
    "build.gradle",
    "settings.gradle",
)
SCENARIOS = {
    "clean": {
        "manifestSha256": "sha256:97735fa0446f07dfbf71f660f1dbab9fd4f736a20ab4a113ba6b6075a896c96f",
        "screenshotSha256": "sha256:2b492399b8f2739f9bac0aa19c06db74758806fcf5cc88b26503f01d3a02cfd5",
        "nodeCount": 18,
    },
    "off-canvas-control-mutant": {
        "manifestSha256": "sha256:0d21e9057760d6c523efd13d198607f8309d657f8001cfdc3657311d0c3291ec",
        "screenshotSha256": "sha256:d2274785a84c71f80af9e7e9815cf3dc9ff0d9937f68d7b5a40e3b0f03486767",
        "nodeCount": 18,
    },
    "scroll-offscreen-hard-negative": {
        "manifestSha256": "sha256:c98fe9ff92dbeb6b4e9a3da580ce3e2a48a872265b50fef96dcb5a24cf0efdaa",
        "screenshotSha256": "sha256:afe150689f40cbd2e7fdd5cd5e32e7d1bd61a29943228b3fb9a5b744e529d332",
        "nodeCount": 21,
    },
}


def fail(message: str) -> None:
    """Exit with one stable checker prefix."""
    raise SystemExit(f"Android fixture error: {message}")


def require(condition: bool, message: str) -> None:
    """Require a static corpus invariant."""
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
    """Digest the exact source/build inputs used to produce the fixture APKs."""
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
        fail(f"{path.relative_to(ROOT)}: {error}")


def png_size(path: Path) -> tuple[int, int]:
    """Read only the PNG signature and IHDR dimensions."""
    try:
        header = path.read_bytes()[:24]
    except OSError as error:
        fail(f"could not read {path.relative_to(ROOT)}: {error}")
    require(
        len(header) == 24 and header[:8] == b"\x89PNG\r\n\x1a\n",
        f"{path.relative_to(ROOT)} is not a PNG",
    )
    require(header[12:16] == b"IHDR", f"{path.relative_to(ROOT)} lacks IHDR")
    return struct.unpack(">II", header[16:24])


def index_nodes(manifest: dict[str, Any], scenario: str) -> dict[str, dict[str, Any]]:
    """Index captured nodes while enforcing stable unique resource IDs."""
    nodes = manifest.get("hierarchy", {}).get("nodes", [])
    require(isinstance(nodes, list), f"{scenario} hierarchy nodes are not an array")
    result: dict[str, dict[str, Any]] = {}
    for node in nodes:
        require(isinstance(node, dict), f"{scenario} has a non-object node")
        identifier = node.get("resourceId")
        require(
            isinstance(identifier, str)
            and re.fullmatch(r"[A-Za-z0-9_.]+:id/[A-Za-z0-9_]+", identifier) is not None,
            f"{scenario} has an invalid resource ID",
        )
        require(identifier not in result, f"{scenario} repeats resource ID {identifier}")
        result[identifier] = node
    return result


def verify_node_geometry(nodes: dict[str, dict[str, Any]], scenario: str) -> None:
    """Verify bounded rectangles and platform conflict representation."""
    for identifier, node in nodes.items():
        require(
            isinstance(node.get("depth"), int) and 0 <= node["depth"] <= 64,
            f"{scenario} node {identifier} has invalid depth",
        )
        layout = node.get("layoutBoundsDevicePixels", {})
        require(
            all(isinstance(layout.get(key), int) for key in ("x", "y", "width", "height"))
            and layout["width"] >= 0
            and layout["height"] >= 0,
            f"{scenario} node {identifier} has invalid layout bounds",
        )
        accessibility = node.get("accessibility", {})
        status = accessibility.get("geometryStatus")
        raw = accessibility.get("rawBoundsDevicePixels")
        normalized = accessibility.get("boundsDevicePixels")
        require(
            status in {"exact", "invalidPlatformBounds", "unavailable"},
            f"{scenario} node {identifier} has unknown accessibility geometry status",
        )
        if status == "exact":
            require(isinstance(raw, dict) and isinstance(normalized, dict),
                    f"{scenario} node {identifier} exact accessibility bounds are missing")
            require(
                raw["right"] >= raw["left"] and raw["bottom"] >= raw["top"],
                f"{scenario} node {identifier} exact raw bounds are inverted",
            )
            require(
                normalized
                == {
                    "x": raw["left"],
                    "y": raw["top"],
                    "width": raw["right"] - raw["left"],
                    "height": raw["bottom"] - raw["top"],
                },
                f"{scenario} node {identifier} accessibility normalization drift",
            )
        elif status == "invalidPlatformBounds":
            require(isinstance(raw, dict) and normalized is None,
                    f"{scenario} node {identifier} invalid bounds were normalized")
            require(
                raw["right"] < raw["left"] or raw["bottom"] < raw["top"],
                f"{scenario} node {identifier} invalid bounds are not inverted",
            )
        else:
            require(raw is None and normalized is None,
                    f"{scenario} unavailable accessibility bounds carry geometry")


def verify_manifest(scenario: str, expected: dict[str, Any], source: str) -> dict[str, Any]:
    """Verify one capture manifest and its paired screenshot."""
    manifest_path = CAPTURES / f"{scenario}.capture.json"
    screenshot_path = CAPTURES / f"{scenario}.png"
    require(manifest_path.is_file(), f"missing capture manifest for {scenario}")
    require(screenshot_path.is_file(), f"missing screenshot for {scenario}")
    if expected["manifestSha256"] != "pending":
        require(
            sha256(manifest_path) == expected["manifestSha256"],
            f"{scenario} capture manifest digest drift",
        )
    require(
        sha256(screenshot_path) == expected["screenshotSha256"],
        f"{scenario} screenshot digest drift",
    )

    manifest = load_json(manifest_path)
    require(manifest.get("captureVersion") == "0.1.0", f"{scenario} capture version drift")
    require(manifest.get("captureId") == f"android-atlas-{scenario}",
            f"{scenario} capture ID drift")
    require(manifest.get("scenario") == scenario, f"{scenario} scenario drift")
    require(
        manifest.get("application")
        == {
            "packageName": "org.sightlint.fixtures.atlas",
            "versionName": "0.1.0",
            "versionCode": 1,
        },
        f"{scenario} application identity drift",
    )
    require(
        manifest.get("runner")
        == {
            "name": "sightlint-atlas-android-capture",
            "version": "0.1.0",
            "captureApi": "instrumentation-view-accessibility",
        },
        f"{scenario} runner identity drift",
    )
    build = manifest.get("build", {})
    require(build.get("fixtureSourceSha256") == source,
            f"{scenario} capture does not match fixture source")
    require(build.get("gradleVersion") == GRADLE_VERSION,
            f"{scenario} Gradle version drift")
    require(build.get("androidGradlePluginVersion") == AGP_VERSION,
            f"{scenario} Android Gradle Plugin version drift")
    require(build.get("javaLanguageVersion") == 17 and build.get("compileSdk") == 35,
            f"{scenario} Java/SDK build contract drift")

    device = manifest.get("device", {})
    require(device.get("apiLevel") == 35, f"{scenario} API level drift")
    require(device.get("buildFingerprint") == BUILD_FINGERPRINT,
            f"{scenario} build fingerprint drift")
    require(
        device.get("display")
        == {
            "widthPixels": 1080,
            "heightPixels": 2400,
            "densityDpi": 420,
            "rotationDegrees": 0,
        },
        f"{scenario} display profile drift",
    )
    require(
        device.get("configuration")
        == {
            "fontScale": 1.0,
            "locale": "en-US",
            "layoutDirection": "ltr",
            "nightMode": "light",
        },
        f"{scenario} configuration drift",
    )
    require(
        device.get("systemBarInsetsDevicePixels")
        == {"left": 0, "top": 0, "right": 0, "bottom": 0},
        f"{scenario} system-bar inset drift",
    )

    capture = manifest.get("capture", {})
    require(capture.get("order") == [
        "waitForIdle", "viewAndAccessibilityHierarchy", "screenshot"
    ], f"{scenario} capture order drift")
    require(capture.get("atomic") is False and capture.get("animationsDisabled") is True,
            f"{scenario} synchronization disclosure drift")
    require(capture.get("adbVersion") == ADB_VERSION,
            f"{scenario} adb version drift")
    require(capture.get("emulatorVersion") == EMULATOR_VERSION,
            f"{scenario} emulator version drift")
    require(capture.get("avdName") == AVD_NAME, f"{scenario} AVD drift")

    screenshot = manifest.get("screenshot", {})
    reference = f"evaluation/android/captures/{scenario}.png"
    require(screenshot.get("reference") == reference,
            f"{scenario} screenshot reference drift")
    require(screenshot.get("sha256") == expected["screenshotSha256"],
            f"{scenario} manifest screenshot digest drift")
    require(screenshot.get("captureSequence") == 3,
            f"{scenario} screenshot sequence drift")
    require(png_size(screenshot_path) == (1080, 2400),
            f"{scenario} PNG extent drift")
    require(
        (screenshot.get("widthPixels"), screenshot.get("heightPixels")) == (1080, 2400),
        f"{scenario} screenshot metadata extent drift",
    )

    hierarchy = manifest.get("hierarchy", {})
    require(hierarchy.get("rootResourceId") == "org.sightlint.fixtures.atlas:id/atlas_root",
            f"{scenario} hierarchy root drift")
    require(hierarchy.get("unidentifiedNodeCount") == 5,
            f"{scenario} unidentified node count drift")
    nodes = index_nodes(manifest, scenario)
    require(len(nodes) == expected["nodeCount"], f"{scenario} node count drift")
    verify_node_geometry(nodes, scenario)
    return manifest


def verify_relations(manifests: dict[str, dict[str, Any]]) -> None:
    """Verify the clean mutation and hard-negative properties."""
    clean = index_nodes(manifests["clean"], "clean")
    mutant = index_nodes(manifests["off-canvas-control-mutant"], "off-canvas-control-mutant")
    require(clean.keys() == mutant.keys(), "targeted mutation changed node identity")
    save = "org.sightlint.fixtures.atlas:id/save_button"
    for identifier in clean:
        if identifier != save:
            require(clean[identifier] == mutant[identifier],
                    f"targeted mutation changed unrelated node {identifier}")
    clean_save = clean[save]
    mutant_save = mutant[save]
    require(clean_save["layoutBoundsDevicePixels"] == {
        "x": 63, "y": 1105, "width": 578, "height": 147
    }, "clean save-button layout drift")
    require(mutant_save["layoutBoundsDevicePixels"] == {
        "x": 798, "y": 1105, "width": 578, "height": 147
    }, "mutant save-button layout drift")
    require(mutant_save["globalVisible"]["boundsDevicePixels"] == {
        "x": 798, "y": 1105, "width": 219, "height": 147
    }, "mutant clipped visible bounds drift")
    for field in ("parentResourceId", "className", "identityTransform", "viewState", "text",
                  "contentDescription"):
        require(clean_save[field] == mutant_save[field],
                f"targeted mutation changed save-button {field}")

    hard = index_nodes(
        manifests["scroll-offscreen-hard-negative"], "scroll-offscreen-hard-negative"
    )
    extra = set(hard) - set(clean)
    require(extra == {
        "org.sightlint.fixtures.atlas:id/archive_spacer",
        "org.sightlint.fixtures.atlas:id/archived_label",
        "org.sightlint.fixtures.atlas:id/archived_section",
    }, "hard-negative node set drift")
    for suffix in ("archived_label", "archived_section"):
        node = hard[f"org.sightlint.fixtures.atlas:id/{suffix}"]
        require(node["globalVisible"] == {"value": False, "boundsDevicePixels": None},
                f"hard-negative {suffix} became globally visible")
        require(node["accessibility"]["geometryStatus"] == "invalidPlatformBounds",
                f"hard-negative {suffix} lost platform conflict evidence")
        require(node["accessibility"]["boundsDevicePixels"] is None,
                f"hard-negative {suffix} invalid accessibility bounds were promoted")


def verify() -> None:
    """Verify committed captures and independently fixed scenario relations."""
    source = source_digest()
    manifests = {
        scenario: verify_manifest(scenario, expected, source)
        for scenario, expected in SCENARIOS.items()
    }
    verify_relations(manifests)
    print(
        "Android fixture corpus verified: 3 API-35 captures, one targeted mutation, "
        "one platform-conflict hard negative"
    )


def command_output(command: list[str]) -> str:
    """Run a capture-time tool without a shell and return UTF-8 output."""
    try:
        result = subprocess.run(command, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f"capture command failed: {command[0]}: {error}")
    return result.stdout + result.stderr


def extract_version(pattern: str, output: str, tool: str) -> str:
    """Extract a pinned tool version."""
    match = re.search(pattern, output)
    if match is None:
        fail(f"could not determine {tool} version")
    return match.group(1)


def capture(gradle: Path, adb: Path, emulator: Path, serial: str) -> None:
    """Build, run, and pull the explicit fixture captures."""
    gradle_version = extract_version(
        r"Gradle ([0-9.]+)", command_output([str(gradle), "--version"]), "Gradle"
    )
    adb_version = extract_version(
        r"Android Debug Bridge version ([0-9.]+)",
        command_output([str(adb), "version"]),
        "adb",
    )
    emulator_version = extract_version(
        r"Android emulator version ([0-9.]+)",
        command_output([str(emulator), "-version"]),
        "emulator",
    )
    require(gradle_version == GRADLE_VERSION, f"expected Gradle {GRADLE_VERSION}")
    require(adb_version == ADB_VERSION, f"expected adb {ADB_VERSION}")
    require(emulator_version == EMULATOR_VERSION,
            f"expected emulator {EMULATOR_VERSION}")
    adb_prefix = [str(adb), "-s", serial]
    avd_name = command_output(adb_prefix + ["shell", "getprop", "ro.boot.qemu.avd_name"]).strip()
    require(avd_name == AVD_NAME, f"expected running AVD {AVD_NAME}")

    environment = os.environ.copy()
    sdk_root = adb.resolve().parents[1]
    environment["ANDROID_HOME"] = str(sdk_root)
    environment["ANDROID_SDK_ROOT"] = str(sdk_root)
    subprocess.run(
        [
            str(gradle), "--no-daemon", "-p", str(APP), "clean",
            ":app:assembleDebug", ":app:assembleDebugAndroidTest",
        ],
        check=True,
        env=environment,
    )
    app_apk = APP / "app/build/outputs/apk/debug/app-debug.apk"
    test_apk = APP / "app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
    subprocess.run(adb_prefix + ["install", "-r", str(app_apk)], check=True)
    subprocess.run(adb_prefix + ["install", "-r", str(test_apk)], check=True)
    for setting in (
        "window_animation_scale", "transition_animation_scale", "animator_duration_scale"
    ):
        subprocess.run(adb_prefix + ["shell", "settings", "put", "global", setting, "0"],
                       check=True)

    source = source_digest()
    CAPTURES.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="sightlint-android-capture-") as temporary:
        temporary_path = Path(temporary)
        for scenario in SCENARIOS:
            subprocess.run(
                adb_prefix + ["shell", "am", "force-stop", "org.sightlint.fixtures.atlas"],
                check=True,
            )
            output = command_output(
                adb_prefix
                + [
                    "shell", "am", "instrument", "-w",
                    "-e", "scenario", scenario,
                    "-e", "fixtureSourceSha256", source,
                    "-e", "gradleVersion", gradle_version,
                    "-e", "adbVersion", adb_version,
                    "-e", "emulatorVersion", emulator_version,
                    "-e", "avdName", avd_name,
                    "org.sightlint.fixtures.atlas.test/"
                    "org.sightlint.fixtures.atlas.CaptureInstrumentation",
                ]
            )
            require("INSTRUMENTATION_CODE: -1" in output,
                    f"instrumentation did not complete for {scenario}: {output.strip()}")
            remote = "/storage/emulated/0/Android/data/org.sightlint.fixtures.atlas/files/capture"
            for suffix in ("capture.json", "png"):
                destination = temporary_path / f"{scenario}.{suffix}"
                subprocess.run(
                    adb_prefix + ["pull", f"{remote}/{scenario}.{suffix}", str(destination)],
                    check=True,
                )
                os.replace(destination, CAPTURES / destination.name)
    print("Android fixture captures refreshed; update pinned digests only after review")


def main() -> None:
    """Parse arguments and capture or verify."""
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--capture", action="store_true")
    parser.add_argument("--gradle-bin", type=Path)
    parser.add_argument("--adb-bin", type=Path)
    parser.add_argument("--emulator-bin", type=Path)
    parser.add_argument("--serial", default="emulator-5554")
    arguments = parser.parse_args()
    if arguments.check:
        verify()
        return
    for name in ("gradle_bin", "adb_bin", "emulator_bin"):
        value = getattr(arguments, name)
        require(value is not None and value.is_file(), f"--{name.replace('_', '-')} is required")
    capture(arguments.gradle_bin, arguments.adb_bin, arguments.emulator_bin, arguments.serial)


if __name__ == "__main__":
    main()
