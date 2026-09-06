#!/usr/bin/env python3
"""Generate deterministic repository-owned PDF source fixtures for ADR 0044."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = ROOT / "fixtures" / "pdf"
PAGE_WIDTH = 612
PAGE_HEIGHT = 792


def stream_object(payload: bytes) -> bytes:
    """Wrap exact bytes in an unfiltered PDF stream object."""
    return (
        f"<< /Length {len(payload)} >>\nstream\n".encode("ascii")
        + payload
        + b"\nendstream"
    )


def content_stream(asymmetric: bool) -> bytes:
    """Return one realistic fictional report page description."""
    left_card_width = 246 if not asymmetric else 214
    right_card_x = 318 if not asymmetric else 286
    right_card_width = 246 if not asymmetric else 278
    delivery_note = (
        "Three milestones remain inside the reviewed scope."
        if not asymmetric
        else "Three milestones remain."
    )
    commands = f"""q
1 1 1 rg 0 0 {PAGE_WIDTH} {PAGE_HEIGHT} re f
0.055 0.075 0.125 rg 0 656 {PAGE_WIDTH} 136 re f
0.15 0.20 0.31 rg 40 676 68 30 re f
0.15 0.20 0.31 rg 116 676 84 30 re f
0.13 0.48 0.88 rg 430 54 134 36 re f
0.96 0.97 0.99 rg 40 424 {left_card_width} 188 re f
0.94 0.97 1 rg {right_card_x} 424 {right_card_width} 188 re f
0.96 0.97 0.99 rg 40 174 524 208 re f
0.22 0.76 0.55 rg 58 536 12 12 re f
0.98 0.67 0.24 rg {right_card_x + 18} 536 12 12 re f
0.13 0.48 0.88 rg 58 302 456 10 re f
0.40 0.47 0.58 rg 58 276 402 10 re f
0.66 0.71 0.78 rg 58 250 348 10 re f
1 1 1 rg
BT /F2 24 Tf 40 744 Td (Atlas Operations Review) Tj ET
BT /F1 10 Tf 40 726 Td (Fictional repository-owned report - September planning cycle) Tj ET
BT /F2 10 Tf 56 687 Td (Overview) Tj ET
BT /F2 10 Tf 128 687 Td (Decisions) Tj ET
BT /F2 12 Tf 462 67 Td (Open plan) Tj ET
0.055 0.075 0.125 rg
BT /F2 14 Tf 58 574 Td (Delivery confidence) Tj ET
BT /F2 30 Tf 58 520 Td (84 percent) Tj ET
BT /F1 10 Tf 58 486 Td ({delivery_note}) Tj ET
BT /F2 14 Tf {right_card_x + 18} 574 Td (Review queue) Tj ET
BT /F2 30 Tf {right_card_x + 18} 520 Td (7 items) Tj ET
BT /F1 10 Tf {right_card_x + 18} 486 Td (Two items require a documented abstention.) Tj ET
BT /F2 16 Tf 58 344 Td (Evidence notes) Tj ET
BT /F1 11 Tf 58 326 Td (Native structure and rendered pixels remain separate.) Tj ET
BT /F1 11 Tf 58 216 Td (SIGHTLINT-PDF-CONTENT-SENTINEL is fictional private test text.) Tj ET
Q"""
    return commands.encode("ascii")


def annotation(rect: tuple[int, int, int, int], quad_points: bool = False) -> bytes:
    """Build one internal Link annotation without executable actions."""
    left, bottom, right, top = rect
    parts = [
        "<< /Type /Annot /Subtype /Link",
        f"/Rect [{left} {bottom} {right} {top}]",
        "/Border [0 0 0]",
        "/Dest [3 0 R /Fit]",
    ]
    if quad_points:
        parts.append(
            "/QuadPoints [430 54 492 54 492 90 430 90 "
            "500 54 564 54 564 90 500 90]"
        )
    parts.append(">>")
    return " ".join(parts).encode("ascii")


def build_pdf(*, mutated: bool, asymmetric: bool, quad_points: bool) -> bytes:
    """Build one PDF 1.7 file with stable indirect objects and classic xref."""
    final_rect = (540, 54, 674, 90) if mutated else (430, 54, 564, 90)
    objects = {
        1: b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 10 0 R >>",
        2: b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        3: (
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/CropBox [0 0 612 792] /Rotate 0 "
            b"/Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> "
            b"/Contents 6 0 R /Annots [7 0 R 8 0 R 9 0 R] >>"
        ),
        4: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        5: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>",
        6: stream_object(content_stream(asymmetric)),
        7: annotation((40, 676, 108, 706)),
        8: annotation((116, 676, 200, 706)),
        9: annotation(final_rect, quad_points=quad_points),
        10: b"<< /Type /StructTreeRoot /K [] >>",
        11: (
            b"<< /Title (Atlas Internal Review) "
            b"/Subject (SIGHTLINT-PDF-METADATA-SENTINEL) "
            b"/Creator (SightLint fixture generator) >>"
        ),
    }
    output = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for number in range(1, len(objects) + 1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode("ascii"))
        output.extend(objects[number])
        output.extend(b"\nendobj\n")
    xref_offset = len(output)
    output.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    output.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        output.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    output.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R /Info 11 0 R >>\n"
            f"startxref\n{xref_offset}\n%%EOF\n"
        ).encode("ascii")
    )
    return bytes(output)


def expected_files() -> dict[Path, bytes]:
    """Return every generated fixture and its exact bytes."""
    return {
        FIXTURE_ROOT / "atlas-clean.pdf": build_pdf(
            mutated=False, asymmetric=False, quad_points=False
        ),
        FIXTURE_ROOT / "atlas-off-page-mutant.pdf": build_pdf(
            mutated=True, asymmetric=False, quad_points=False
        ),
        FIXTURE_ROOT / "atlas-quadpoints-hard-negative.pdf": build_pdf(
            mutated=False, asymmetric=True, quad_points=True
        ),
    }


def run(check: bool) -> int:
    """Write fixtures or verify that committed bytes match the generator."""
    failed = False
    for path, expected in expected_files().items():
        if check:
            if not path.is_file() or path.read_bytes() != expected:
                print(f"fixture drift: {path.relative_to(ROOT)}", file=sys.stderr)
                failed = True
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(expected)
            print(path.relative_to(ROOT))
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    return run(arguments.check)


if __name__ == "__main__":
    raise SystemExit(main())
