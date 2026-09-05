#!/usr/bin/env python3
"""Generate reviewable PNG bytes and independent source-pixel oracles. No dependencies."""
import argparse
import binascii
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "fixtures/png-raster/corpus.json"
PASSES = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8),
          (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", binascii.crc32(kind + data))


def stored(data):
    assert 0 < len(data) <= 65535
    return b"\x78\x01\x01" + struct.pack("<HH", len(data), 65535 - len(data)) + data + struct.pack(">I", zlib.adler32(data))


def png(width, height, color, data, *, depth=8, interlace=0, extra=(), split=False):
    result = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, depth, color, 0, 0, interlace))
    if color == 3:
        result += chunk(b"PLTE", b"\x00\x00\x00\xff\xff\xff")
    for kind, payload in extra:
        result += chunk(kind, payload)
    stream = stored(data)
    if split:
        # Split even the zlib framing and trailer into separate IDAT chunks.
        for part in (stream[:1], stream[1:-2], stream[-2:], b""):
            result += chunk(b"IDAT", part)
    else:
        result += chunk(b"IDAT", stream)
    return result + chunk(b"IEND", b"")


def encode_row(row, previous, bpp, selector):
    output = bytearray([selector])
    for i, value in enumerate(row):
        left = row[i - bpp] if i >= bpp else 0
        up = previous[i] if previous else 0
        corner = previous[i - bpp] if previous and i >= bpp else 0
        estimate = left + up - corner
        paeth = min((left, up, corner), key=lambda p: abs(estimate - p))
        predictor = (0, left, up, (left + up) // 2, paeth)[selector]
        output.append((value - predictor) % 256)
    return bytes(output)


def sample_bytes(width, height, color):
    # Different values in every coordinate/channel catch scatter and channel-order mistakes.
    result = bytearray()
    for y in range(height):
        for x in range(width):
            for c in range(CHANNELS[color]):
                value = (241 + 37 * x + 61 * y + 29 * c) % 256
                if color in (4, 6) and c == CHANNELS[color] - 1:
                    value = (0, 1, 127, 254, 255)[(x + y) % 5]
                result.append(value)
    return bytes(result)


def expected_rgba(source, color):
    channels = CHANNELS[color]
    result = bytearray()
    for start in range(0, len(source), channels):
        pixel = source[start:start + channels]
        if color == 0:
            result.extend((pixel[0], pixel[0], pixel[0], 255))
        elif color == 2:
            result.extend((*pixel, 255))
        elif color == 4:
            result.extend((pixel[0], pixel[0], pixel[0], pixel[1]))
        else:
            assert color == 6
            result.extend(pixel)
    return bytes(result)


def scanlines(source, width, height, channels, selector, interlace):
    result = bytearray()
    for sx, sy, dx, dy in PASSES if interlace else [(0, 0, 1, 1)]:
        xs = range(sx, width, dx)
        if not xs:
            continue
        previous = None
        for y in range(sy, height, dy):
            row = b"".join(source[(y * width + x) * channels:(y * width + x + 1) * channels] for x in xs)
            result.extend(encode_row(row, previous, channels, selector))
            previous = row
    return bytes(result)


def available(identifier, width, height, color, selector, interlace=0, *, source=None, extra=()):
    source = sample_bytes(width, height, color) if source is None else source
    rgba = expected_rgba(source, color)
    data = scanlines(source, width, height, CHANNELS[color], selector, interlace)
    input_bytes = png(width, height, color, data, interlace=interlace, extra=extra, split=True)
    return {"id": identifier, "pngHex": input_bytes.hex(), "width": width, "height": height,
            "colorType": color, "filter": selector, "interlace": interlace, "exitCode": 0,
            "status": "available", "rgbaHex": rgba.hex(),
            "byteCrc32": f"{binascii.crc32(rgba):08x}"}


def dashboard(mutated):
    width, height = 12, 5
    source = bytearray([240, 240, 240, 255] * width * height)
    starts = [1, 4, 8 if mutated else 7]
    for left in starts:
        for y in range(1, 4):
            for x in range(left, left + 2):
                start = (y * width + x) * 4
                source[start:start + 4] = bytes([10, 20, 30, 255])
    case = available("cards-mutated" if mutated else "cards-clean", width, height, 6, 1, source=bytes(source))
    case["future"] = {"capability": "peer-spacing", "status": "untested",
                      "peerBounds": [[left, 1, 2, 3] for left in starts],
                      "gaps": [1, 2 if mutated else 1], "expectedDefect": mutated}
    if mutated:
        case["future"]["baseline"] = "cards-clean"
    return case


def generate():
    cases = [available(f"color-{color}-filter-{f}", 3, 2, color, f)
             for color in (0, 2, 4, 6) for f in range(5)]
    cases += [available(f"adam7-color-{color}", 3, 3, color, 4, 1) for color in (0, 2, 4, 6)]
    cases += [available(f"adam7-{w}x{h}", w, h, 6, 3, 1) for w, h in ((1, 1), (1, 5), (5, 1), (8, 8))]
    cases += [available("unmanaged-gamma", 3, 2, 6, 0, extra=[(b"gAMA", struct.pack(">I", 100000))])]
    cases += [dashboard(False), dashboard(True)]
    unsupported = [
        ("indexed", png(2, 1, 3, b"\0\0\1"), "indexedColor"),
        ("packed", png(8, 1, 0, b"\0\xa5", depth=1), "unsupportedBitDepth"),
        ("sixteen-bit", png(1, 1, 6, b"\0\0\1\0\2\0\3\0\4", depth=16), "unsupportedBitDepth"),
        ("trns", png(1, 1, 2, b"\0\1\2\3", extra=[(b"tRNS", b"\0\1\0\2\0\3")]), "transparencyChunk"),
        ("animation-control", png(1, 1, 6, b"\0\1\2\3\xff", extra=[(b"acTL", struct.pack(">II", 1, 0))]), "animationChunks"),
    ]
    cases += [{"id": name, "pngHex": data.hex(), "status": "unavailable", "reason": reason, "exitCode": 0}
              for name, data, reason in unsupported]
    invalid = png(1, 1, 0, b"\5\x12")
    bad_crc = bytearray(png(1, 1, 0, b"\0\x12"))
    bad_crc[-13] ^= 1
    cases += [{"id": "invalid-filter", "pngHex": invalid.hex(), "exitCode": 2, "errorContains": "invalid filter type 5"},
              {"id": "invalid-crc", "pngHex": bad_crc.hex(), "exitCode": 2, "errorContains": "CRC"}]
    assert len({case["id"] for case in cases}) == len(cases)
    return {"version": "0.1.0", "origin": "procedural-no-external-assets",
            "reviewStatus": "synthetic-not-human-validated", "scope": "source-raster-conformance",
            "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content = json.dumps(generate(), sort_keys=True, indent=2) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_bytes() != content.encode("utf-8"):
            raise SystemExit("raster corpus differs; regenerate and review input/oracle changes")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(content.encode("utf-8"))
    print(f"raster corpus: {len(generate()['cases'])} cases verified")


if __name__ == "__main__":
    main()
