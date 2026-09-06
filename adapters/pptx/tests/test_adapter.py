#!/usr/bin/env python3
"""Unit tests for bounded archive and DrawingML primitives."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from xml.etree import ElementTree


ADAPTER_PATH = Path(__file__).resolve().parents[1] / "sightlint_pptx.py"
SPEC = importlib.util.spec_from_file_location("sightlint_pptx", ADAPTER_PATH)
assert SPEC is not None and SPEC.loader is not None
ADAPTER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ADAPTER
SPEC.loader.exec_module(ADAPTER)


def limits() -> dict[str, int]:
    return {
        "maxEntries": 8,
        "maxExpandedBytes": 1_024,
        "maxCompressionRatio": 20,
    }


class ArchiveTests(unittest.TestCase):
    def test_inventory_rejects_parent_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "unsafe.zip"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("../outside.xml", b"x")
            with zipfile.ZipFile(path) as archive:
                with self.assertRaisesRegex(ADAPTER.AdapterError, "unsafe member") as raised:
                    ADAPTER.inventory(archive, limits())
                self.assertEqual(raised.exception.code, "archive-member")

    def test_inventory_rejects_duplicate_normalized_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "duplicate.zip"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("ppt//slide.xml", b"a")
                archive.writestr("ppt/slide.xml", b"b")
            with zipfile.ZipFile(path) as archive:
                with self.assertRaisesRegex(ADAPTER.AdapterError, "duplicate normalized") as raised:
                    ADAPTER.inventory(archive, limits())
                self.assertEqual(raised.exception.code, "archive-member")

    def test_xml_part_rejects_dtd_and_entity_declarations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "entity.zip"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("part.xml", b'<!DOCTYPE x [<!ENTITY y "z">]><x>&y;</x>')
            with zipfile.ZipFile(path) as archive:
                members = ADAPTER.inventory(archive, limits())
                with self.assertRaisesRegex(ADAPTER.AdapterError, "forbidden") as raised:
                    ADAPTER.xml_part(archive, members, "part.xml", 1_024)
                self.assertEqual(raised.exception.code, "xml-entity")

    def test_inventory_rejects_excessive_compression_ratio(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "ratio.zip"
            with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                archive.writestr("large.xml", b"0" * 1_000)
            with zipfile.ZipFile(path) as archive:
                with self.assertRaisesRegex(ADAPTER.AdapterError, "compression ratio") as raised:
                    ADAPTER.inventory(archive, limits())
                self.assertEqual(raised.exception.code, "archive-ratio")


class GeometryTests(unittest.TestCase):
    def test_explicit_false_flips_remain_exact(self) -> None:
        element = ElementTree.fromstring(
            '<a:xfrm xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" flipH="0" flipV="false">'
            '<a:off x="1" y="2"/><a:ext cx="3" cy="4"/></a:xfrm>'
        )
        status, transform = ADAPTER.xfrm(element, "shape", group=False)
        self.assertEqual(status, "exact")
        self.assertEqual(transform, {"x": 1, "y": 2, "width": 3, "height": 4})

    def test_rotation_abstains_from_geometry(self) -> None:
        element = ElementTree.fromstring(
            '<a:xfrm xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" rot="1">'
            '<a:off x="1" y="2"/><a:ext cx="3" cy="4"/></a:xfrm>'
        )
        status, transform = ADAPTER.xfrm(element, "shape", group=False)
        self.assertEqual(status, "unsupportedTransform")
        self.assertIsNone(transform)

    def test_group_child_coordinates_are_scaled_and_translated(self) -> None:
        matrix = ADAPTER.child_matrix(
            ADAPTER.Matrix(),
            {
                "x": 100,
                "y": 200,
                "width": 1_000,
                "height": 400,
                "childX": 10,
                "childY": 20,
                "childWidth": 500,
                "childHeight": 200,
            },
        )
        self.assertEqual(
            ADAPTER.transform_rect(matrix, 10, 20, 100, 50),
            {"x": 100, "y": 200, "width": 200, "height": 100},
        )


if __name__ == "__main__":
    unittest.main()
