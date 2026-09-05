#!/usr/bin/env python3
"""Generate the committed SightLint image evaluation corpus without image libraries."""

from __future__ import annotations

import argparse
import binascii
import json
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "fixtures" / "evaluation" / "image"
MANIFEST_PATH = OUTPUT_DIR / "manifest.json"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
Pixel = tuple[int, int, int, int]


@dataclass(frozen=True)
class GeneratedCase:
    case: dict[str, Any]
    content: bytes


def crc32(data: bytes) -> int:
    return binascii.crc32(data) & 0xFFFF_FFFF


def adler32(data: bytes) -> int:
    modulus = 65_521
    a = 1
    b = 0
    for byte in data:
        a = (a + byte) % modulus
        b = (b + a) % modulus
    return (b << 16) | a


def stored_zlib(data: bytes) -> bytes:
    output = bytearray((0x78, 0x01))
    chunks = [data[index : index + 65_535] for index in range(0, len(data), 65_535)]
    if not chunks:
        chunks = [b""]
    for index, block in enumerate(chunks):
        output.append(1 if index + 1 == len(chunks) else 0)
        length = len(block)
        output.extend(struct.pack("<H", length))
        output.extend(struct.pack("<H", (~length) & 0xFFFF))
        output.extend(block)
    output.extend(struct.pack(">I", adler32(data)))
    return bytes(output)


def png_chunk(kind: bytes, data: bytes, *, corrupt_crc: bool = False) -> bytes:
    checksum = crc32(kind + data)
    if corrupt_crc:
        checksum ^= 1
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", checksum)


def png_from_scanlines(
    width: int,
    height: int,
    scanlines: bytes,
    *,
    corrupt_idat_crc: bool = False,
) -> bytes:
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"".join(
        (
            PNG_SIGNATURE,
            png_chunk(b"IHDR", ihdr),
            png_chunk(b"IDAT", stored_zlib(scanlines), corrupt_crc=corrupt_idat_crc),
            png_chunk(b"IEND", b""),
        )
    )


def rgba_png(
    width: int,
    height: int,
    pixels: list[Pixel],
    *,
    corrupt_idat_crc: bool = False,
) -> bytes:
    if len(pixels) != width * height:
        raise ValueError("pixel count does not match dimensions")
    raw = bytearray()
    for row in range(height):
        raw.append(0)
        start = row * width
        for pixel in pixels[start : start + width]:
            raw.extend(pixel)
    return png_from_scanlines(
        width,
        height,
        bytes(raw),
        corrupt_idat_crc=corrupt_idat_crc,
    )


def filled(width: int, height: int, color: Pixel) -> list[Pixel]:
    return [color] * (width * height)


def fill_rect(
    pixels: list[Pixel],
    canvas_width: int,
    x: int,
    y: int,
    width: int,
    height: int,
    color: Pixel,
) -> None:
    for row in range(y, y + height):
        start = row * canvas_width + x
        pixels[start : start + width] = [color] * width


def set_pixel(pixels: list[Pixel], width: int, x: int, y: int, color: Pixel) -> None:
    pixels[y * width + x] = color


def rgba_bytes(pixels: list[Pixel]) -> bytes:
    return bytes(channel for pixel in pixels for channel in pixel)


def alpha_facts(width: int, height: int, pixels: list[Pixel]) -> dict[str, Any]:
    visible = [(index % width, index // width) for index, pixel in enumerate(pixels) if pixel[3] > 0]
    opaque = [(index % width, index // width) for index, pixel in enumerate(pixels) if pixel[3] == 255]

    def bounds(points: list[tuple[int, int]]) -> dict[str, int] | None:
        if not points:
            return None
        xs = [point[0] for point in points]
        ys = [point[1] for point in points]
        minimum_x = min(xs)
        maximum_x = max(xs)
        minimum_y = min(ys)
        maximum_y = max(ys)
        return {
            "x": minimum_x,
            "y": minimum_y,
            "width": maximum_x - minimum_x + 1,
            "height": maximum_y - minimum_y + 1,
        }

    visible_bounds = bounds(visible)
    insets = None
    if visible_bounds is not None:
        insets = {
            "top": visible_bounds["y"],
            "right": width - visible_bounds["x"] - visible_bounds["width"],
            "bottom": height - visible_bounds["y"] - visible_bounds["height"],
            "left": visible_bounds["x"],
        }
    return {
        "visibleBounds": visible_bounds,
        "opaqueBounds": bounds(opaque),
        "transparentInsets": insets,
        "visiblePixelCount": len(visible),
        "opaquePixelCount": len(opaque),
        "transparentPixelCount": sum(pixel[3] == 0 for pixel in pixels),
        "translucentPixelCount": sum(0 < pixel[3] < 255 for pixel in pixels),
        "allTransparent": not visible,
        "allVisible": len(visible) == width * height,
    }


def assertion(pointer: str, value: Any) -> dict[str, Any]:
    return {"pointer": pointer, "equals": value}


def missing_assertion(pointer: str) -> dict[str, Any]:
    return {"pointer": pointer, "exists": False}


def length_assertion(pointer: str, length: int) -> dict[str, Any]:
    return {"pointer": pointer, "length": length}


def png_pointer(suffix: str) -> str:
    return f"/extensions/org.sightlint.adapter.png/{suffix}"


def common_valid_assertions(width: int, height: int, pixels: list[Pixel]) -> list[dict[str, Any]]:
    encoded = rgba_bytes(pixels)
    return [
        assertion("/artifact/kind", "image"),
        assertion("/canvases/0/size/width", float(width)),
        assertion("/canvases/0/size/height", float(height)),
        assertion(png_pointer("version"), "0.3.0"),
        assertion(png_pointer("pixelEncoding"), "pngEncodedRgba8"),
        assertion(png_pointer("colorManagementApplied"), False),
        assertion(png_pointer("rgba8Bytes"), len(encoded)),
        assertion(png_pointer("rgba8Crc32"), f"{crc32(encoded):08x}"),
    ]


def transparent_padding_case() -> GeneratedCase:
    width = 32
    height = 24
    pixels = filled(width, height, (0, 0, 0, 0))
    fill_rect(pixels, width, 6, 4, 20, 16, (30, 100, 220, 128))
    fill_rect(pixels, width, 7, 5, 18, 14, (30, 100, 220, 255))
    facts = alpha_facts(width, height, pixels)
    assertions = common_valid_assertions(width, height, pixels)
    for key in (
        "visibleBounds",
        "opaqueBounds",
        "transparentInsets",
        "visiblePixelCount",
        "opaquePixelCount",
        "transparentPixelCount",
        "translucentPixelCount",
        "allTransparent",
        "allVisible",
    ):
        assertions.append(assertion(png_pointer(f"alphaAnalysis/{key}"), facts[key]))
    assertions.extend(
        (
            assertion("/nodes/0/geometry/inkBox/rect", facts["visibleBounds"]),
            assertion(png_pointer("backgroundCandidateAnalysis/applicability"), "requiresFullyOpaquePixels"),
            length_assertion(png_pointer("backgroundCandidateAnalysis/candidates"), 0),
        )
    )
    case = {
        "id": "transparent-symbol-padding",
        "file": "transparent-symbol-padding.png",
        "description": "Transparent padding around translucent and opaque source ink.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 0,
        "currentAssertions": assertions,
        "groundTruth": {
            "regions": [
                {"id": "visible-symbol", "role": "image-content", "rect": facts["visibleBounds"]},
                {"id": "opaque-core", "role": "opaque-content", "rect": facts["opaqueBounds"]},
            ],
            "peerGroups": [],
            "defects": [],
            "targetCapabilities": ["pixel.alpha-visible-geometry"],
        },
    }
    return GeneratedCase(case, rgba_png(width, height, pixels))


def all_transparent_case() -> GeneratedCase:
    width = 16
    height = 12
    pixels = filled(width, height, (0, 0, 0, 0))
    assertions = common_valid_assertions(width, height, pixels)
    assertions.extend(
        (
            assertion(png_pointer("alphaAnalysis/visibleBounds"), None),
            assertion(png_pointer("alphaAnalysis/opaqueBounds"), None),
            assertion(png_pointer("alphaAnalysis/allTransparent"), True),
            assertion(png_pointer("alphaAnalysis/visiblePixelCount"), 0),
            missing_assertion("/nodes/0/geometry/inkBox"),
            assertion(png_pointer("backgroundCandidateAnalysis/applicability"), "requiresFullyOpaquePixels"),
        )
    )
    case = {
        "id": "all-transparent",
        "file": "all-transparent.png",
        "description": "An image with no alpha-visible source pixels.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 0,
        "currentAssertions": assertions,
        "groundTruth": {
            "regions": [],
            "peerGroups": [],
            "defects": [],
            "targetCapabilities": ["pixel.alpha-visible-geometry"],
        },
    }
    return GeneratedCase(case, rgba_png(width, height, pixels))


def dashboard_pixels(*, mutated_spacing: bool) -> tuple[int, int, list[Pixel], list[dict[str, Any]]]:
    width = 240
    height = 160
    background = (246, 247, 249, 255)
    navigation = (43, 52, 69, 255)
    card = (255, 255, 255, 255)
    title = (48, 58, 74, 255)
    body = (145, 154, 168, 255)
    action = (53, 105, 225, 255)
    pixels = filled(width, height, background)
    fill_rect(pixels, width, 16, 16, 208, 20, navigation)
    card_xs = [20, 88, 163 if mutated_spacing else 156]
    regions: list[dict[str, Any]] = [
        {"id": "top-navigation", "role": "navigation", "rect": {"x": 16, "y": 16, "width": 208, "height": 20}}
    ]
    for index, x in enumerate(card_xs, start=1):
        fill_rect(pixels, width, x, 56, 56, 72, card)
        fill_rect(pixels, width, x + 8, 66, 32, 5, title)
        fill_rect(pixels, width, x + 8, 79, 40, 4, body)
        fill_rect(pixels, width, x + 8, 88, 34, 4, body)
        fill_rect(pixels, width, x + 8, 108, 40, 10, action)
        regions.append(
            {
                "id": f"card-{index}",
                "role": "card",
                "rect": {"x": x, "y": 56, "width": 56, "height": 72},
            }
        )
    return width, height, pixels, regions


def dashboard_case(*, mutated_spacing: bool) -> GeneratedCase:
    width, height, pixels, regions = dashboard_pixels(mutated_spacing=mutated_spacing)
    case_id = "opaque-dashboard-spacing-mutation" if mutated_spacing else "opaque-dashboard-clean"
    filename = f"{case_id}.png"
    assertions = common_valid_assertions(width, height, pixels)
    assertions.extend(
        (
            assertion(png_pointer("alphaAnalysis/allVisible"), True),
            assertion("/nodes/0/geometry/inkBox/rect", {"x": 0.0, "y": 0.0, "width": 240.0, "height": 160.0}),
            assertion(png_pointer("backgroundCandidateAnalysis/applicability"), "fullyOpaque"),
            assertion(png_pointer("backgroundCandidateAnalysis/candidates/0/rgba"), "#f6f7f9ff"),
            assertion(
                png_pointer("backgroundCandidateAnalysis/candidates/0/nonCandidateBounds"),
                {"x": 16, "y": 16, "width": 208, "height": 112},
            ),
        )
    )
    gaps = [12, 19] if mutated_spacing else [12, 12]
    defects: list[dict[str, Any]] = []
    if mutated_spacing:
        defects.append(
            {
                "id": "unequal-card-horizontal-gap",
                "ruleFamily": "visual.spacing.peer-horizontal-gap",
                "targetRelation": "dashboard-cards",
                "measurements": {"unit": "devicePixel", "gaps": gaps},
                "rationale": "Equivalent dashboard cards use inconsistent horizontal gaps.",
            }
        )
    ground_truth: dict[str, Any] = {
        "regions": regions,
        "peerGroups": [
            {
                "id": "dashboard-cards",
                "axis": "horizontal",
                "memberIds": ["card-1", "card-2", "card-3"],
                "gaps": {"unit": "devicePixel", "values": gaps},
            }
        ],
        "defects": defects,
        "targetCapabilities": [
            "pixel.opaque-background-candidates",
            "structure.repeated-region-groups",
            "visual.spacing.peer-consistency",
        ],
    }
    case: dict[str, Any] = {
        "id": case_id,
        "file": filename,
        "description": "Dashboard-like synthetic UI with a targeted peer-spacing mutation."
        if mutated_spacing
        else "Dashboard-like synthetic UI with equal peer-card gaps.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 0,
        "currentAssertions": assertions,
        "groundTruth": ground_truth,
    }
    if mutated_spacing:
        case["mutation"] = {
            "baselineCaseId": "opaque-dashboard-clean",
            "kind": "peer-gap",
            "targetRelation": "dashboard-cards",
            "changedMeasurement": {
                "index": 1,
                "from": 12,
                "to": 19,
                "unit": "devicePixel",
            },
        }
    return GeneratedCase(case, rgba_png(width, height, pixels))


def border_tie_case() -> GeneratedCase:
    width = 8
    height = 6
    pixels = filled(width, height, (0, 0, 0, 255))
    edge_colors = {
        "top": (10, 0, 0, 255),
        "bottom": (20, 0, 0, 255),
        "left": (30, 0, 0, 255),
        "right": (40, 0, 0, 255),
    }
    for x in range(1, width - 1):
        set_pixel(pixels, width, x, 0, edge_colors["top"])
        set_pixel(pixels, width, x, height - 1, edge_colors["bottom"])
    for y in range(1, height - 1):
        set_pixel(pixels, width, 0, y, edge_colors["left"])
        set_pixel(pixels, width, width - 1, y, edge_colors["right"])
    corners = ((1, 0, 0, 255), (2, 0, 0, 255), (3, 0, 0, 255), (4, 0, 0, 255))
    set_pixel(pixels, width, 0, 0, corners[0])
    set_pixel(pixels, width, width - 1, 0, corners[1])
    set_pixel(pixels, width, 0, height - 1, corners[2])
    set_pixel(pixels, width, width - 1, height - 1, corners[3])
    expected_colors = [
        "#010000ff",
        "#020000ff",
        "#030000ff",
        "#040000ff",
        "#0a0000ff",
        "#140000ff",
        "#1e0000ff",
        "#280000ff",
    ]
    assertions = common_valid_assertions(width, height, pixels)
    assertions.extend(
        (
            assertion(png_pointer("backgroundCandidateAnalysis/edgeSampleCount"), 24),
            length_assertion(png_pointer("backgroundCandidateAnalysis/candidates"), 8),
        )
    )
    assertions.extend(
        assertion(png_pointer(f"backgroundCandidateAnalysis/candidates/{index}/rgba"), color)
        for index, color in enumerate(expected_colors)
    )
    case = {
        "id": "opaque-border-tie",
        "file": "opaque-border-tie.png",
        "description": "Opaque border colors with deterministic corner and frequency ties.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 0,
        "currentAssertions": assertions,
        "groundTruth": {
            "regions": [],
            "peerGroups": [],
            "defects": [],
            "targetCapabilities": ["pixel.opaque-background-candidates"],
        },
    }
    return GeneratedCase(case, rgba_png(width, height, pixels))


def translucent_overlay_case() -> GeneratedCase:
    width = 80
    height = 60
    pixels = filled(width, height, (245, 245, 247, 255))
    fill_rect(pixels, width, 20, 15, 40, 30, (20, 60, 160, 128))
    assertions = common_valid_assertions(width, height, pixels)
    assertions.extend(
        (
            assertion(png_pointer("alphaAnalysis/allVisible"), True),
            assertion(png_pointer("alphaAnalysis/translucentPixelCount"), 1_200),
            assertion(png_pointer("backgroundCandidateAnalysis/applicability"), "requiresFullyOpaquePixels"),
            length_assertion(png_pointer("backgroundCandidateAnalysis/candidates"), 0),
        )
    )
    case = {
        "id": "translucent-overlay",
        "file": "translucent-overlay.png",
        "description": "An uncomposited translucent source region over otherwise opaque pixels.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 0,
        "currentAssertions": assertions,
        "groundTruth": {
            "regions": [
                {"id": "overlay", "role": "translucent-overlay", "rect": {"x": 20, "y": 15, "width": 40, "height": 30}}
            ],
            "peerGroups": [],
            "defects": [],
            "targetCapabilities": ["pixel.alpha-visible-geometry", "pixel.opaque-background-candidates"],
        },
    }
    return GeneratedCase(case, rgba_png(width, height, pixels))


def bad_crc_case() -> GeneratedCase:
    width = 10
    height = 10
    pixels = filled(width, height, (255, 255, 255, 255))
    case = {
        "id": "malformed-idat-crc",
        "file": "malformed-idat-crc.png",
        "description": "A valid raster whose IDAT CRC is intentionally corrupted.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 2,
        "stderrContains": "IDAT chunk CRC-32",
        "currentAssertions": [],
        "groundTruth": {
            "regions": [],
            "peerGroups": [],
            "defects": [{"id": "invalid-idat-crc", "ruleFamily": "parser.png.chunk-crc"}],
            "targetCapabilities": ["parser.png.chunk-validation"],
        },
    }
    return GeneratedCase(case, rgba_png(width, height, pixels, corrupt_idat_crc=True))


def invalid_filter_case() -> GeneratedCase:
    width = 4
    height = 2
    scanlines = bytes([5, *([0] * (width * 4)), 0, *([0] * (width * 4))])
    case = {
        "id": "malformed-filter-selector",
        "file": "malformed-filter-selector.png",
        "description": "A length-valid raster whose first scanline uses undefined filter 5.",
        "dimensions": {"width": width, "height": height},
        "expectedExit": 2,
        "stderrContains": "filter type 5 is invalid at pass 0, row 0",
        "currentAssertions": [],
        "groundTruth": {
            "regions": [],
            "peerGroups": [],
            "defects": [{"id": "invalid-filter-5", "ruleFamily": "parser.png.scanline-filter"}],
            "targetCapabilities": ["parser.png.filter-reconstruction"],
        },
    }
    return GeneratedCase(case, png_from_scanlines(width, height, scanlines))


def generate_cases() -> list[GeneratedCase]:
    return [
        transparent_padding_case(),
        all_transparent_case(),
        dashboard_case(mutated_spacing=False),
        dashboard_case(mutated_spacing=True),
        border_tie_case(),
        translucent_overlay_case(),
        bad_crc_case(),
        invalid_filter_case(),
    ]


def manifest_bytes(cases: list[GeneratedCase]) -> bytes:
    manifest = {
        "schemaVersion": "0.1.0",
        "generator": {
            "name": "tools/generate_image_evaluation_corpus.py",
            "version": "0.1.0",
            "determinism": "stdlib stored-DEFLATE PNG; no fonts, randomness, network, or clock",
        },
        "assertionSemantics": {
            "equals": "The JSON pointer must exist and equal the supplied JSON value.",
            "exists": "The JSON pointer must be present or absent as specified.",
            "length": "The pointed JSON array or object must have the supplied length.",
        },
        "cases": [generated.case for generated in cases],
    }
    return (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()


def expected_outputs() -> dict[Path, bytes]:
    cases = generate_cases()
    outputs = {OUTPUT_DIR / generated.case["file"]: generated.content for generated in cases}
    outputs[MANIFEST_PATH] = manifest_bytes(cases)
    return outputs


def check_outputs(outputs: dict[Path, bytes]) -> int:
    failures: list[str] = []
    for path, expected in outputs.items():
        if not path.exists():
            failures.append(f"missing generated file: {path.relative_to(ROOT)}")
        elif path.read_bytes() != expected:
            failures.append(f"generated file differs: {path.relative_to(ROOT)}")

    expected_paths = set(outputs)
    if OUTPUT_DIR.exists():
        for path in OUTPUT_DIR.iterdir():
            if path.is_file() and path not in expected_paths:
                failures.append(f"unexpected generated file: {path.relative_to(ROOT)}")

    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


def write_outputs(outputs: dict[Path, bytes]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    expected_paths = set(outputs)
    for path in OUTPUT_DIR.iterdir():
        if path.is_file() and path not in expected_paths:
            path.unlink()
    for path, content in outputs.items():
        path.write_bytes(content)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail unless committed generated files exactly match the generator",
    )
    arguments = parser.parse_args()
    outputs = expected_outputs()
    if arguments.check:
        return check_outputs(outputs)
    write_outputs(outputs)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
