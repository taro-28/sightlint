#!/usr/bin/env python3
"""Unit tests for bounded PDF adapter primitives and the reviewed dependency."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

import pypdf
from pypdf import PdfReader
from pypdf.generic import (
    ArrayObject,
    DictionaryObject,
    FloatObject,
    IndirectObject,
    NameObject,
    NumberObject,
)


ADAPTER_PATH = Path(__file__).resolve().parents[1] / "sightlint_pdf.py"
SPEC = importlib.util.spec_from_file_location("sightlint_pdf", ADAPTER_PATH)
assert SPEC is not None and SPEC.loader is not None
ADAPTER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ADAPTER
SPEC.loader.exec_module(ADAPTER)
ROOT = ADAPTER_PATH.parents[2]


class DependencyTests(unittest.TestCase):
    def test_reviewed_pypdf_version_is_loaded(self) -> None:
        self.assertEqual(pypdf.__version__, ADAPTER.PYPDF_VERSION)

    def test_dependency_mismatch_has_stable_failure(self) -> None:
        original = ADAPTER.pypdf.__version__
        try:
            ADAPTER.pypdf.__version__ = "0.0.0"
            with self.assertRaisesRegex(ADAPTER.AdapterError, "requires pypdf") as raised:
                ADAPTER.open_reader(ROOT / "fixtures/pdf/atlas-clean.pdf")
            self.assertEqual(raised.exception.code, "dependency-error")
        finally:
            ADAPTER.pypdf.__version__ = original


class GeometryTests(unittest.TestCase):
    def test_pdf_rect_uses_crop_relative_top_left_coordinates(self) -> None:
        source = {"left": 430, "bottom": 54, "right": 564, "top": 90}
        crop = {"left": 0, "bottom": 0, "right": 612, "top": 792}
        self.assertEqual(
            ADAPTER.normalized_rect(source, crop),
            {"x": 430, "y": 702, "width": 134, "height": 36},
        )

    def test_float_rect_component_is_not_promoted_to_exact(self) -> None:
        value = ArrayObject(
            [NumberObject(0), NumberObject(0), FloatObject(10.0), NumberObject(10)]
        )
        with self.assertRaisesRegex(ADAPTER.AdapterError, "integral PDF number") as raised:
            ADAPTER.source_rect(value, "test Rect")
        self.assertEqual(raised.exception.code, "source-invalid")

    def test_quadpoints_retains_abstention(self) -> None:
        status = ADAPTER.annotation_geometry_status(
            page_exact=True,
            subtype="/Link",
            flags=0,
            action="internalDestination",
            has_quad_points=True,
            has_path=False,
            rectangle={"left": 0, "bottom": 0, "right": 10, "top": 10},
        )
        self.assertEqual(status, "unsupportedQuadPoints")

    def test_unsupported_action_precedes_rectangular_geometry(self) -> None:
        status = ADAPTER.annotation_geometry_status(
            page_exact=True,
            subtype="/Link",
            flags=0,
            action="uri",
            has_quad_points=False,
            has_path=False,
            rectangle={"left": 0, "bottom": 0, "right": 10, "top": 10},
        )
        self.assertEqual(status, "unsupportedAction")

    def test_invalid_rect_remains_explicit(self) -> None:
        status = ADAPTER.annotation_geometry_status(
            page_exact=True,
            subtype="/Link",
            flags=0,
            action="internalDestination",
            has_quad_points=False,
            has_path=False,
            rectangle=None,
            invalid_rectangle=True,
        )
        self.assertEqual(status, "invalidRect")


class ReaderTests(unittest.TestCase):
    def test_clean_fixture_has_bounded_inventory(self) -> None:
        reader = PdfReader(
            ROOT / "fixtures/pdf/atlas-clean.pdf",
            strict=True,
            root_object_recovery_limit=1,
        )
        self.assertEqual(ADAPTER.object_count(reader), 11)
        self.assertEqual(len(reader.pages), 1)

    def test_raw_page_walk_preserves_inherited_rotation_without_inventing_boxes(self) -> None:
        class FakeReader:
            def __init__(self) -> None:
                self.objects: dict[tuple[int, int], DictionaryObject] = {}

            def get_object(self, reference: IndirectObject) -> DictionaryObject:
                return self.objects[(reference.idnum, reference.generation)]

        reader = FakeReader()
        pages_reference = IndirectObject(1, 0, reader)
        page_reference = IndirectObject(2, 0, reader)
        root = DictionaryObject({NameObject("/Pages"): pages_reference})
        reader.objects[(1, 0)] = DictionaryObject(
            {
                NameObject("/Type"): NameObject("/Pages"),
                NameObject("/Rotate"): NumberObject(90),
                NameObject("/Kids"): ArrayObject([page_reference]),
            }
        )
        reader.objects[(2, 0)] = DictionaryObject(
            {NameObject("/Type"): NameObject("/Page")}
        )
        pages = ADAPTER.collect_pages(reader, root, 1)
        self.assertEqual(len(pages), 1)
        page, reference, rotation = pages[0]
        self.assertIs(reference, page_reference)
        self.assertEqual(rotation, 90)
        self.assertNotIn("/MediaBox", page)
        self.assertNotIn("/CropBox", page)

    def test_render_coverage_distinguishes_absent_partial_and_complete(self) -> None:
        self.assertEqual(ADAPTER.rendered_extent_coverage(0, 2), "untested")
        self.assertEqual(ADAPTER.rendered_extent_coverage(1, 2), "partial")
        self.assertEqual(ADAPTER.rendered_extent_coverage(2, 2), "observed")


if __name__ == "__main__":
    unittest.main()
