#!/usr/bin/env python3
"""Generate native PNG bytes and independently declared region/gap oracles (no dependencies)."""
import argparse
import binascii
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'fixtures/image-inspection/corpus.json'
BG = [240, 240, 240, 255]
FG = [10, 20, 30, 255]


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', binascii.crc32(kind + data))


def encode(width, height, pixels):
    rows = b''.join(b'\0' + bytes(pixels[y * width * 4:(y + 1) * width * 4]) for y in range(height))
    compressor = zlib.compressobj(9, zlib.DEFLATED, 15, 8, zlib.Z_FIXED)
    stream = compressor.compress(rows) + compressor.flush()
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', stream) + chunk(b'IEND', b''))


def group(axis, gaps):
    return {'axis': axis, 'gaps': gaps, 'pattern': 'uniform' if min(gaps) == max(gaps) else 'unequal'}


def image(identifier, width, height, shapes=(), *, groups=(), bounds=None, background=BG, edits=(), reason=None):
    pixels = background * (width * height)
    for shape in shapes:
        x, y, w, h = shape[:4]
        color = shape[4] if len(shape) == 5 else FG
        for row in range(y, y + h):
            for column in range(x, x + w):
                offset = (row * width + column) * 4
                pixels[offset:offset + 4] = color
    for x, y, color in edits:
        offset = (y * width + x) * 4
        pixels[offset:offset + 4] = color
    expected = {'status': 'unavailable' if reason else 'observed', 'groups': list(groups)}
    if reason:
        expected['reason'] = reason
        expected['bounds'] = []
    else:
        expected['bounds'] = sorted([list(s[:4]) for s in shapes] if bounds is None else bounds, key=lambda r: (r[1], r[0]))
    return {'id': identifier, 'pngHex': encode(width, height, pixels).hex(), 'expected': expected}


def generate():
    clean = [[1, 1, 2, 3], [4, 1, 2, 3], [7, 1, 2, 3]]
    mutant = [[1, 1, 2, 3], [4, 1, 2, 3], [8, 1, 2, 3]]
    cases = [
        {'id': 'cards-clean', 'rasterCase': 'cards-clean', 'expected': {'status': 'observed', 'bounds': clean, 'groups': [group('horizontal', [1, 1])]}},
        {'id': 'cards-mutated', 'rasterCase': 'cards-mutated', 'expected': {'status': 'observed', 'bounds': mutant, 'groups': [group('horizontal', [1, 2])]}},
        {'id': 'intentional-grouping', 'rasterCase': 'cards-mutated', 'designIntent': 'The same pixels may intentionally separate groups; no blocking verdict is justified.', 'expected': {'status': 'observed', 'bounds': mutant, 'groups': [group('horizontal', [1, 2])]}}
    ]
    cases += [
        image('vertical-clean', 5, 12, [(1, 1, 3, 2), (1, 4, 3, 2), (1, 7, 3, 2)], groups=[group('vertical', [1, 1])]),
        image('vertical-unequal', 5, 12, [(1, 1, 3, 2), (1, 4, 3, 2), (1, 8, 3, 2)], groups=[group('vertical', [1, 2])]),
        image('translated', 15, 7, [(2, 2, 2, 3), (5, 2, 2, 3), (9, 2, 2, 3)], groups=[group('horizontal', [1, 2])]),
        image('scaled', 24, 10, [(2, 2, 4, 6), (8, 2, 4, 6), (16, 2, 4, 6)], groups=[group('horizontal', [2, 4])]),
        image('recolored', 12, 5, [(*r, [240, 210, 160, 255]) for r in mutant], background=[0, 0, 0, 255], groups=[group('horizontal', [1, 2])]),
        image('two-rows', 12, 10, mutant + [[1, 6, 2, 3], [4, 6, 2, 3], [7, 6, 2, 3]], groups=[group('horizontal', [1, 2]), group('horizontal', [1, 1])]),
        image('blocker', 18, 5, [(1, 1, 2, 3), (7, 1, 2, 3), (13, 1, 2, 3), (5, 2, 1, 1)]),
        image('different-size', 12, 5, [(1, 1, 2, 3), (4, 1, 3, 3), (8, 1, 2, 3)]),
        image('different-color', 12, 5, [clean[0], clean[1], (*clean[2], [90, 80, 70, 255])]),
        image('one-region', 5, 5, [(1, 1, 2, 3)]),
        image('two-regions', 8, 5, clean[:2]),
        image('touching', 5, 5, [(1, 1, 1, 2), (2, 1, 1, 2)], bounds=[[1, 1, 2, 2]]),
        image('diagonal', 5, 5, [(1, 1, 1, 1), (2, 2, 1, 1)]),
        image('hollow', 7, 7, [(1, 1, 5, 5), (2, 2, 3, 3, BG)], bounds=[[1, 1, 5, 5]]),
        image('mixed-region', 5, 5, [(1, 1, 3, 3)], edits=[(2, 2, [90, 80, 70, 255])]),
        image('uniform', 5, 5),
        image('border-noise', 12, 5, clean, edits=[(0, 2, FG)], reason='nonUniformBorder'),
        image('alpha-zero', 12, 5, clean, edits=[(2, 2, [10, 20, 30, 0])], reason='nonOpaqueRaster'),
        image('alpha-one', 12, 5, clean, edits=[(2, 2, [10, 20, 30, 1])], reason='nonOpaqueRaster'),
        image('alpha-254', 12, 5, clean, edits=[(2, 2, [10, 20, 30, 254])], reason='nonOpaqueRaster'),
    ]
    for identifier in ['indexed', 'packed', 'sixteen-bit', 'trns', 'animation-control']:
        reason = {'indexed': 'indexedColor', 'packed': 'unsupportedBitDepth', 'sixteen-bit': 'unsupportedBitDepth', 'trns': 'transparencyChunk', 'animation-control': 'animationChunks'}[identifier]
        cases.append({'id': identifier, 'rasterCase': identifier, 'expected': {'status': 'unavailable', 'reason': reason, 'bounds': [], 'groups': []}})
    for identifier in ['invalid-filter', 'invalid-crc']:
        cases.append({'id': identifier, 'rasterCase': identifier, 'expected': {'exitCode': 2}})
    assert len({case['id'] for case in cases}) == len(cases)
    return {'version': '0.1.0', 'origin': 'procedural-no-external-assets', 'reviewStatus': 'synthetic-not-human-validated', 'scope': 'conditional-region-and-gap-observations-not-UX-verdicts', 'cases': cases}


def serialize(corpus):
    cases = corpus.pop('cases')
    header = json.dumps(corpus, sort_keys=True)[:-1]
    return header + ', "cases": [\n' + ',\n'.join(json.dumps(c, sort_keys=True) for c in cases) + '\n]}\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = serialize(generate()).encode()
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_bytes() != data:
            raise SystemExit('inspection corpus drift; regenerate and review input and oracle changes')
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(data)
    print(f"inspection corpus: {len(generate()['cases'])} cases")


if __name__ == '__main__':
    main()
