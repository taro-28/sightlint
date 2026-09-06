#!/usr/bin/env python3
"""Generate repository-owned transparent UI assets without using SightLint."""

from __future__ import annotations

import argparse
import binascii
from dataclasses import dataclass
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "evaluation" / "image-alpha" / "assets"


@dataclass(frozen=True)
class Asset:
    name: str
    width: int
    height: int
    rgba: bytes


def chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", binascii.crc32(kind + data))
    )


def stored_zlib(data: bytes) -> bytes:
    if not 0 < len(data) <= 65_535:
        raise ValueError("fixture scanlines must fit one stored DEFLATE block")
    return (
        b"\x78\x01\x01"
        + struct.pack("<HH", len(data), 65_535 - len(data))
        + data
        + struct.pack(">I", zlib.adler32(data))
    )


def png(asset: Asset) -> bytes:
    rows = b"".join(
        b"\0" + asset.rgba[y * asset.width * 4 : (y + 1) * asset.width * 4]
        for y in range(asset.height)
    )
    header = struct.pack(">IIBBBBB", asset.width, asset.height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", stored_zlib(rows))
        + chunk(b"IEND", b"")
    )


def transparent_canvas(width: int, height: int, hidden_variant: int) -> bytearray:
    pixels = bytearray()
    for y in range(height):
        for x in range(width):
            if hidden_variant == 0:
                rgb = (19, 31, 49)
            else:
                rgb = ((37 * x + 11 * y) % 256, (17 * x + 29 * y) % 256, 211)
            pixels.extend((*rgb, 0))
    return pixels


def set_pixel(
    pixels: bytearray,
    width: int,
    x: int,
    y: int,
    rgba: tuple[int, int, int, int],
) -> None:
    start = (y * width + x) * 4
    pixels[start : start + 4] = bytes(rgba)


def compass_mark(hidden_variant: int = 0) -> Asset:
    width = height = 48
    pixels = transparent_canvas(width, height, hidden_variant)

    # A soft offset source-alpha shadow. These are final encoded samples, not compositing output.
    for y in range(13, 37):
        half_width = 12 - abs(y - 25)
        if half_width >= 0:
            for x in range(25 - half_width, 26 + half_width):
                set_pixel(pixels, width, x, y, (40, 53, 91, 64))

    # Compass diamond with a translucent one-pixel antialias fringe.
    for y in range(7, 36):
        half_width = 14 - abs(y - 21)
        for x in range(23 - half_width, 24 + half_width):
            alpha = 128 if x in {23 - half_width, 23 + half_width} else 255
            set_pixel(pixels, width, x, y, (74, 93, 230, alpha))

    # Transparent center cutout preserves nonzero hidden RGB.
    for y in range(17, 26):
        half_width = 4 - abs(y - 21)
        for x in range(23 - half_width, 24 + half_width):
            set_pixel(pixels, width, x, y, (121, 135, 241, 0))

    # Disconnected status sparkle with its own translucent fringe.
    for y in range(6, 12):
        for x in range(36, 42):
            alpha = 255 if 38 <= x <= 40 and 8 <= y <= 10 else 128
            set_pixel(pixels, width, x, y, (45, 202, 181, alpha))

    return Asset("northstar-compass", width, height, bytes(pixels))


def padded_compass() -> Asset:
    source = compass_mark()
    width = height = 56
    pixels = transparent_canvas(width, height, 0)
    for y in range(source.height):
        source_start = y * source.width * 4
        target_start = ((y + 4) * width + 4) * 4
        pixels[target_start : target_start + source.width * 4] = source.rgba[
            source_start : source_start + source.width * 4
        ]
    return Asset("northstar-compass-padded", width, height, bytes(pixels))


def edge_badge() -> Asset:
    width, height = 40, 24
    pixels = transparent_canvas(width, height, 0)
    for y in range(3, 21):
        for x in range(5, 40):
            in_rounded_shape = (9 <= x) or (7 <= y <= 16)
            if not in_rounded_shape:
                continue
            fringe = y in {3, 20} or x == 5
            set_pixel(pixels, width, x, y, (239, 91, 115, 128 if fringe else 255))
    # A transparent counter inside the fictional badge.
    for y in range(9, 15):
        for x in range(21, 25):
            set_pixel(pixels, width, x, y, (255, 255, 255, 0))
    return Asset("northstar-edge-badge", width, height, bytes(pixels))


def invisible_placeholder() -> Asset:
    width = height = 24
    return Asset(
        "northstar-invisible-placeholder",
        width,
        height,
        bytes(transparent_canvas(width, height, 1)),
    )


def assets() -> list[Asset]:
    changed_hidden_rgb = compass_mark(hidden_variant=1)
    changed_hidden_rgb = Asset(
        "northstar-compass-hidden-rgb",
        changed_hidden_rgb.width,
        changed_hidden_rgb.height,
        changed_hidden_rgb.rgba,
    )
    return [
        compass_mark(),
        changed_hidden_rgb,
        padded_compass(),
        edge_badge(),
        invisible_placeholder(),
    ]


def source_alpha_summary(asset: Asset) -> str:
    points = []
    opaque = []
    edge = {"top": 0, "right": 0, "bottom": 0, "left": 0}
    counts = {"visible": 0, "opaque": 0, "translucent": 0, "transparent": 0}
    for y in range(asset.height):
        for x in range(asset.width):
            alpha = asset.rgba[(y * asset.width + x) * 4 + 3]
            if alpha == 0:
                counts["transparent"] += 1
                continue
            counts["visible"] += 1
            points.append((x, y))
            if alpha == 255:
                counts["opaque"] += 1
                opaque.append((x, y))
            else:
                counts["translucent"] += 1
            if y == 0:
                edge["top"] += 1
            if x + 1 == asset.width:
                edge["right"] += 1
            if y + 1 == asset.height:
                edge["bottom"] += 1
            if x == 0:
                edge["left"] += 1

    def bounds(values: list[tuple[int, int]]) -> list[int] | None:
        if not values:
            return None
        xs = [value[0] for value in values]
        ys = [value[1] for value in values]
        return [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]

    return (
        f"{asset.name}: canvas={asset.width}x{asset.height} "
        f"visible={bounds(points)} opaque={bounds(opaque)} counts={counts} edges={edge}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = assets()
    if args.check:
        expected_names = {f"{asset.name}.png" for asset in generated}
        actual_names = {path.name for path in OUTPUT.glob("*.png")} if OUTPUT.is_dir() else set()
        if actual_names != expected_names:
            raise SystemExit("source-alpha asset set differs; regenerate and review")
        for asset in generated:
            path = OUTPUT / f"{asset.name}.png"
            if not path.is_file() or path.read_bytes() != png(asset):
                raise SystemExit(f"{path.relative_to(ROOT)} differs; regenerate and review")
    else:
        OUTPUT.mkdir(parents=True, exist_ok=True)
        for asset in generated:
            (OUTPUT / f"{asset.name}.png").write_bytes(png(asset))
    for asset in generated:
        print(source_alpha_summary(asset))


if __name__ == "__main__":
    main()
