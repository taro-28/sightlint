#!/usr/bin/env python3
"""Generate deterministic, reviewable Artifact IR fixtures for CLI end-to-end tests."""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "fixtures" / "e2e"
VISUAL_EXTENSION_KEY = "org.sightlint.visual"
VISUAL_EXTENSION_VERSION = "0.1.0"


def evidence(identifier: str, evidence_class: str, adapter: str) -> dict[str, Any]:
    return {
        "id": identifier,
        "class": evidence_class,
        "source": {
            "adapter": adapter,
            "adapterVersion": "0.1.0",
            "externalProcessing": False,
        },
    }


def observed(value: Any, evidence_id: str) -> dict[str, Any]:
    return {"value": value, "evidenceId": evidence_id}


def observed_rect(
    x: float,
    y: float,
    width: float,
    height: float,
    *,
    canvas: str = "canvas-main",
) -> dict[str, Any]:
    return {
        "rect": {"x": x, "y": y, "width": width, "height": height},
        "coordinateSpaceId": canvas,
        "evidenceId": "e-render",
    }


def node(
    identifier: str,
    x: float | None,
    y: float | None,
    *,
    width: float = 100.0,
    height: float = 30.0,
    canvas: str = "canvas-main",
    name: str,
    kind: str = "control",
    role: str = "button",
    parent_id: str | None = None,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "id": identifier,
        "kind": observed(kind, "e-source"),
        "coordinateSpaceId": canvas,
        "role": observed(role, "e-source"),
        "name": observed(name, "e-source"),
        "geometry": {},
    }
    if parent_id is not None:
        result["parentId"] = parent_id
    if x is not None and y is not None:
        result["geometry"]["renderBox"] = observed_rect(
            x, y, width, height, canvas=canvas
        )
    return result


def unit_for_kind(kind: str) -> str:
    return {
        "web": "cssPixel",
        "mobile": "dp",
        "slide": "point",
        "document": "point",
        "pdf": "pdfPoint",
        "image": "devicePixel",
        "other": "devicePixel",
    }[kind]


def base_artifact(
    kind: str,
    artifact_id: str,
    *,
    include_relations: bool = True,
) -> dict[str, Any]:
    nodes = [
        node("node-a", 20.0, 20.0, name="First"),
        node("node-b", 20.0, 70.0, name="Second"),
        node("node-c", 20.0, 120.0, name="Third"),
    ]
    relations: list[dict[str, Any]] = []
    if include_relations:
        relations = [
            {
                "type": "nonOverlapping",
                "id": "relation-non-overlap",
                "nodeIds": ["node-a", "node-b", "node-c"],
                "boxKind": "render",
                "tolerance": 0.0,
                "evidenceId": "e-contract",
            },
            {
                "type": "peerSequence",
                "id": "relation-spacing",
                "nodeIds": ["node-a", "node-b", "node-c"],
                "axis": "vertical",
                "boxKind": "render",
                "expectedGap": 20.0,
                "tolerance": 0.0,
                "evidenceId": "e-contract",
            },
        ]

    return {
        "schemaVersion": "0.1.0",
        "artifact": {
            "id": artifact_id,
            "kind": kind,
            "title": "Fixture artifact",
            "sourceName": f"{artifact_id}.json",
        },
        "canvases": [
            {
                "id": "canvas-main",
                "size": {"width": 400.0, "height": 300.0},
                "unit": unit_for_kind(kind),
                "horizontalDirection": "right",
                "verticalDirection": "down",
                "evidenceId": "e-render",
            }
        ],
        "nodes": nodes,
        "relations": relations,
        "evidence": [
            evidence("e-source", "exactSource", "fixture-native"),
            evidence("e-render", "exactRender", "fixture-renderer"),
            evidence("e-contract", "declaredContract", "fixture-contract"),
        ],
    }


def font_style(value: float, unit: str, evidence_id: str = "e-render") -> dict[str, Any]:
    return {
        "fontSize": {
            "value": {"value": value, "unit": unit},
            "evidenceId": evidence_id,
        }
    }


def alignment_contract(
    *,
    node_ids: list[str] | None = None,
    axis: str = "horizontal",
    anchor: str = "start",
    box_kind: str = "render",
    tolerance: float = 0.0,
    evidence_id: str = "e-contract",
) -> dict[str, Any]:
    return {
        "type": "peerAlignment",
        "nodeIds": node_ids or ["node-a", "node-b", "node-c"],
        "axis": axis,
        "anchor": anchor,
        "boxKind": box_kind,
        "tolerance": tolerance,
        "evidenceId": evidence_id,
    }


def extent_contract(
    *,
    node_ids: list[str] | None = None,
    dimension: str = "width",
    box_kind: str = "render",
    tolerance: float = 0.0,
    evidence_id: str = "e-contract",
) -> dict[str, Any]:
    return {
        "type": "peerExtent",
        "nodeIds": node_ids or ["node-a", "node-b", "node-c"],
        "dimension": dimension,
        "boxKind": box_kind,
        "tolerance": tolerance,
        "evidenceId": evidence_id,
    }


def peer_font_contract(
    *,
    node_ids: list[str] | None = None,
    tolerance: float = 0.0,
    evidence_id: str = "e-contract",
) -> dict[str, Any]:
    return {
        "type": "peerFontSize",
        "nodeIds": node_ids or ["node-a", "node-b", "node-c"],
        "tolerance": tolerance,
        "evidenceId": evidence_id,
    }


def minimum_font_contract(
    unit: str,
    *,
    node_ids: list[str] | None = None,
    minimum: float = 14.0,
    evidence_id: str = "e-contract",
) -> dict[str, Any]:
    return {
        "type": "minimumFontSize",
        "nodeIds": node_ids or ["node-a", "node-b", "node-c"],
        "minimum": {"value": minimum, "unit": unit},
        "evidenceId": evidence_id,
    }


def add_visual_extension(
    document: dict[str, Any],
    *,
    node_styles: dict[str, Any] | None = None,
    contracts: dict[str, Any] | None = None,
    extension_version: str = VISUAL_EXTENSION_VERSION,
) -> dict[str, Any]:
    document.setdefault("extensions", {})[VISUAL_EXTENSION_KEY] = {
        "extensionVersion": extension_version,
        "nodeStyles": node_styles or {},
        "contracts": contracts or {},
    }
    return document


def m2_artifact(kind: str, artifact_id: str) -> dict[str, Any]:
    document = base_artifact(kind, artifact_id)
    unit = unit_for_kind(kind)
    return add_visual_extension(
        document,
        node_styles={
            node_id: font_style(16.0, unit)
            for node_id in ("node-a", "node-b", "node-c")
        },
        contracts={
            "contract-alignment": alignment_contract(),
            "contract-extent": extent_contract(),
            "contract-minimum-font": minimum_font_contract(unit),
            "contract-peer-font": peer_font_contract(),
        },
    )


def add_layout_box(
    document: dict[str, Any],
    node_id: str,
    *,
    x: float,
    y: float,
    width: float = 100.0,
    height: float = 30.0,
    canvas: str = "canvas-main",
) -> None:
    target = next(node_value for node_value in document["nodes"] if node_value["id"] == node_id)
    target["geometry"]["layoutBox"] = observed_rect(
        x, y, width, height, canvas=canvas
    )


def build_m1_fixtures(documents: dict[str, dict[str, Any]]) -> None:
    for kind in ("web", "mobile", "slide", "document", "pdf", "image", "other"):
        documents[f"pass-{kind}.json"] = base_artifact(kind, f"artifact-pass-{kind}")

    shuffled = copy.deepcopy(documents["pass-web.json"])
    shuffled["nodes"].reverse()
    shuffled["relations"].reverse()
    shuffled["evidence"] = [
        shuffled["evidence"][2],
        shuffled["evidence"][0],
        shuffled["evidence"][1],
    ]
    documents["pass-web-shuffled.json"] = shuffled

    spacing = base_artifact("web", "artifact-fail-spacing")
    spacing["nodes"][2]["geometry"]["renderBox"]["rect"]["y"] = 130.0
    documents["fail-spacing.json"] = spacing

    overlap = base_artifact("web", "artifact-fail-overlap", include_relations=False)
    overlap["nodes"] = [
        node("node-a", 20.0, 20.0, name="First"),
        node("node-b", 20.0, 40.0, name="Second"),
    ]
    overlap["relations"] = [
        {
            "type": "nonOverlapping",
            "id": "relation-non-overlap",
            "nodeIds": ["node-a", "node-b"],
            "boxKind": "render",
            "tolerance": 0.0,
            "evidenceId": "e-contract",
        }
    ]
    documents["fail-overlap.json"] = overlap

    bounds = base_artifact("web", "artifact-fail-bounds", include_relations=False)
    bounds["nodes"] = [
        node("node-outside", 390.0, 20.0, width=30.0, name="Outside")
    ]
    documents["fail-bounds.json"] = bounds

    missing_box = base_artifact(
        "web", "artifact-cant-tell-missing-box", include_relations=False
    )
    missing_box["nodes"] = [
        node("node-a", 20.0, 20.0, name="First"),
        node("node-b", None, None, name="Missing"),
    ]
    missing_box["relations"] = [
        {
            "type": "nonOverlapping",
            "id": "relation-non-overlap",
            "nodeIds": ["node-a", "node-b"],
            "boxKind": "render",
            "tolerance": 0.0,
            "evidenceId": "e-contract",
        }
    ]
    documents["cant-tell-missing-box.json"] = missing_box

    cross_canvas = base_artifact(
        "web", "artifact-cant-tell-cross-canvas", include_relations=False
    )
    cross_canvas["canvases"].append(
        {
            "id": "canvas-second",
            "size": {"width": 400.0, "height": 300.0},
            "unit": "cssPixel",
            "horizontalDirection": "right",
            "verticalDirection": "down",
            "evidenceId": "e-render",
        }
    )
    cross_canvas["nodes"] = [
        node("node-a", 20.0, 20.0, name="First"),
        node("node-b", 20.0, 70.0, canvas="canvas-second", name="Second"),
    ]
    cross_canvas["relations"] = [
        {
            "type": "peerSequence",
            "id": "relation-spacing",
            "nodeIds": ["node-a", "node-b"],
            "axis": "vertical",
            "boxKind": "render",
            "expectedGap": 20.0,
            "tolerance": 0.0,
            "evidenceId": "e-contract",
        }
    ]
    documents["cant-tell-cross-canvas.json"] = cross_canvas

    inapplicable = base_artifact("image", "artifact-inapplicable", include_relations=False)
    inapplicable["nodes"] = []
    documents["inapplicable.json"] = inapplicable

    invalid_schema = copy.deepcopy(documents["pass-web.json"])
    invalid_schema["schemaVersion"] = "9.9.9"
    documents["invalid-schema-version.json"] = invalid_schema

    invalid_reference = copy.deepcopy(documents["pass-web.json"])
    invalid_reference["relations"][0]["nodeIds"][1] = "missing-node"
    documents["invalid-reference.json"] = invalid_reference

    invalid_cycle = copy.deepcopy(documents["pass-web.json"])
    invalid_cycle["nodes"][0]["parentId"] = "node-b"
    invalid_cycle["nodes"][1]["parentId"] = "node-a"
    documents["invalid-cycle.json"] = invalid_cycle

    invalid_confidence = copy.deepcopy(documents["pass-web.json"])
    invalid_confidence["evidence"][1]["confidence"] = 1.25
    documents["invalid-confidence.json"] = invalid_confidence

    invalid_uncertainty = copy.deepcopy(documents["pass-web.json"])
    invalid_uncertainty["evidence"][1]["uncertainty"] = {
        "type": "scalarRange",
        "lower": 5.0,
        "upper": 2.0,
    }
    documents["invalid-uncertainty.json"] = invalid_uncertainty

    invalid_geometry = copy.deepcopy(documents["pass-web.json"])
    invalid_geometry["nodes"][0]["geometry"]["renderBox"]["rect"]["width"] = -1.0
    documents["invalid-negative-geometry.json"] = invalid_geometry

    missing_confidence = copy.deepcopy(documents["pass-web.json"])
    missing_confidence["evidence"][1]["class"] = "visionInferred"
    documents["invalid-missing-confidence.json"] = missing_confidence

    empty_identifier = copy.deepcopy(documents["pass-web.json"])
    empty_identifier["artifact"]["id"] = ""
    documents["invalid-empty-identifier.json"] = empty_identifier

    duplicate_identifier = copy.deepcopy(documents["pass-web.json"])
    duplicate_identifier["nodes"][0]["id"] = "e-render"
    documents["invalid-duplicate-identifier.json"] = duplicate_identifier


def build_m2_fixtures(documents: dict[str, dict[str, Any]]) -> None:
    kinds = ("web", "mobile", "slide", "document", "pdf", "image", "other")
    for kind in kinds:
        documents[f"m2-pass-{kind}.json"] = m2_artifact(
            kind, f"artifact-m2-pass-{kind}"
        )

    shuffled = copy.deepcopy(documents["m2-pass-web.json"])
    shuffled["nodes"].reverse()
    shuffled["relations"].reverse()
    shuffled["evidence"].reverse()
    visual = shuffled["extensions"][VISUAL_EXTENSION_KEY]
    visual["nodeStyles"] = dict(reversed(list(visual["nodeStyles"].items())))
    visual["contracts"] = dict(reversed(list(visual["contracts"].items())))
    for contract in visual["contracts"].values():
        contract["nodeIds"].reverse()
    documents["m2-pass-web-shuffled.json"] = shuffled

    parent_boundary = base_artifact(
        "web", "artifact-m2-pass-parent-boundary", include_relations=False
    )
    parent_boundary["nodes"] = [
        node(
            "parent",
            10.0,
            10.0,
            width=200.0,
            height=160.0,
            name="Parent",
            kind="container",
            role="group",
        ),
        node(
            "child",
            10.0,
            10.0,
            width=200.0,
            height=160.0,
            name="Child",
            parent_id="parent",
        ),
    ]
    add_visual_extension(parent_boundary)
    documents["m2-pass-parent-boundary.json"] = parent_boundary

    parent_fail = copy.deepcopy(parent_boundary)
    parent_fail["artifact"]["id"] = "artifact-m2-fail-parent-containment"
    parent_fail["nodes"][1]["geometry"]["renderBox"]["rect"].update(
        {"x": 195.0, "width": 30.0}
    )
    documents["m2-fail-parent-containment.json"] = parent_fail

    parent_missing = copy.deepcopy(parent_boundary)
    parent_missing["artifact"]["id"] = "artifact-m2-cant-tell-parent-missing-box"
    parent_missing["nodes"][0]["geometry"] = {}
    documents["m2-cant-tell-parent-missing-box.json"] = parent_missing

    alignment_fail = copy.deepcopy(documents["m2-pass-web.json"])
    alignment_fail["artifact"]["id"] = "artifact-m2-fail-alignment"
    alignment_fail["nodes"][2]["geometry"]["renderBox"]["rect"]["x"] = 32.0
    documents["m2-fail-alignment.json"] = alignment_fail

    alignment_tolerance = copy.deepcopy(documents["m2-pass-web.json"])
    alignment_tolerance["artifact"]["id"] = "artifact-m2-pass-alignment-tolerance"
    for node_value, x in zip(alignment_tolerance["nodes"], (20.0, 21.0, 22.0)):
        node_value["geometry"]["renderBox"]["rect"]["x"] = x
    alignment_tolerance["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-alignment"
    ]["tolerance"] = 1.0
    documents["m2-pass-alignment-tolerance.json"] = alignment_tolerance

    rtl = base_artifact("web", "artifact-m2-pass-alignment-rtl")
    rtl["canvases"][0]["horizontalDirection"] = "left"
    for node_value, x, width in zip(
        rtl["nodes"], (20.0, 40.0, 60.0), (100.0, 80.0, 60.0)
    ):
        node_value["geometry"]["renderBox"]["rect"].update(
            {"x": x, "width": width}
        )
    add_visual_extension(
        rtl, contracts={"contract-alignment": alignment_contract()}
    )
    documents["m2-pass-alignment-rtl.json"] = rtl

    vertical_up = base_artifact(
        "web", "artifact-m2-pass-alignment-vertical-up", include_relations=False
    )
    vertical_up["canvases"][0]["verticalDirection"] = "up"
    vertical_up["nodes"] = [
        node("node-a", 20.0, 170.0, width=80.0, height=30.0, name="First"),
        node("node-b", 130.0, 160.0, width=80.0, height=40.0, name="Second"),
        node("node-c", 240.0, 150.0, width=80.0, height=50.0, name="Third"),
    ]
    add_visual_extension(
        vertical_up,
        contracts={
            "contract-alignment": alignment_contract(
                axis="vertical", anchor="start"
            )
        },
    )
    documents["m2-pass-alignment-vertical-up.json"] = vertical_up

    alignment_missing = base_artifact(
        "web", "artifact-m2-cant-tell-alignment-missing-box"
    )
    add_layout_box(alignment_missing, "node-a", x=20.0, y=20.0)
    add_layout_box(alignment_missing, "node-b", x=20.0, y=70.0)
    add_visual_extension(
        alignment_missing,
        contracts={
            "contract-alignment": alignment_contract(box_kind="layout")
        },
    )
    documents["m2-cant-tell-alignment-missing-box.json"] = alignment_missing

    alignment_cross = base_artifact(
        "web", "artifact-m2-cant-tell-alignment-cross-canvas"
    )
    alignment_cross["canvases"].append(
        {
            "id": "canvas-second",
            "size": {"width": 400.0, "height": 300.0},
            "unit": "cssPixel",
            "horizontalDirection": "right",
            "verticalDirection": "down",
            "evidenceId": "e-render",
        }
    )
    add_layout_box(alignment_cross, "node-a", x=20.0, y=20.0)
    add_layout_box(
        alignment_cross,
        "node-b",
        x=20.0,
        y=70.0,
        canvas="canvas-second",
    )
    add_visual_extension(
        alignment_cross,
        contracts={
            "contract-alignment": alignment_contract(
                node_ids=["node-a", "node-b"], box_kind="layout"
            )
        },
    )
    documents["m2-cant-tell-alignment-cross-canvas.json"] = alignment_cross

    extent_fail = copy.deepcopy(documents["m2-pass-web.json"])
    extent_fail["artifact"]["id"] = "artifact-m2-fail-extent"
    extent_fail["nodes"][2]["geometry"]["renderBox"]["rect"]["width"] = 112.0
    documents["m2-fail-extent.json"] = extent_fail

    extent_tolerance = copy.deepcopy(documents["m2-pass-web.json"])
    extent_tolerance["artifact"]["id"] = "artifact-m2-pass-extent-tolerance"
    for node_value, width in zip(extent_tolerance["nodes"], (99.0, 100.0, 101.0)):
        node_value["geometry"]["renderBox"]["rect"]["width"] = width
    extent_tolerance["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-extent"
    ]["tolerance"] = 1.0
    documents["m2-pass-extent-tolerance.json"] = extent_tolerance

    extent_missing = base_artifact("web", "artifact-m2-cant-tell-extent-missing-box")
    add_layout_box(extent_missing, "node-a", x=20.0, y=20.0)
    add_layout_box(extent_missing, "node-b", x=20.0, y=70.0)
    add_visual_extension(
        extent_missing,
        contracts={"contract-extent": extent_contract(box_kind="layout")},
    )
    documents["m2-cant-tell-extent-missing-box.json"] = extent_missing

    extent_cross = base_artifact("web", "artifact-m2-cant-tell-extent-cross-canvas")
    extent_cross["canvases"].append(
        {
            "id": "canvas-second",
            "size": {"width": 400.0, "height": 300.0},
            "unit": "cssPixel",
            "horizontalDirection": "right",
            "verticalDirection": "down",
            "evidenceId": "e-render",
        }
    )
    add_layout_box(extent_cross, "node-a", x=20.0, y=20.0)
    add_layout_box(extent_cross, "node-b", x=20.0, y=70.0, canvas="canvas-second")
    add_visual_extension(
        extent_cross,
        contracts={
            "contract-extent": extent_contract(
                node_ids=["node-a", "node-b"], box_kind="layout"
            )
        },
    )
    documents["m2-cant-tell-extent-cross-canvas.json"] = extent_cross

    peer_font_fail = copy.deepcopy(documents["m2-pass-web.json"])
    peer_font_fail["artifact"]["id"] = "artifact-m2-fail-peer-font-size"
    peer_font_fail["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-c"] = font_style(
        18.0, "cssPixel"
    )
    documents["m2-fail-peer-font-size.json"] = peer_font_fail

    peer_font_tolerance = copy.deepcopy(documents["m2-pass-web.json"])
    peer_font_tolerance["artifact"]["id"] = "artifact-m2-pass-peer-font-size-tolerance"
    styles = peer_font_tolerance["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]
    for node_id, value in zip(("node-a", "node-b", "node-c"), (15.0, 16.0, 17.0)):
        styles[node_id] = font_style(value, "cssPixel")
    peer_font_tolerance["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-peer-font"
    ]["tolerance"] = 1.0
    documents["m2-pass-peer-font-size-tolerance.json"] = peer_font_tolerance

    peer_font_missing = copy.deepcopy(documents["m2-pass-web.json"])
    peer_font_missing["artifact"]["id"] = "artifact-m2-cant-tell-peer-font-size-missing"
    del peer_font_missing["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-c"]
    documents["m2-cant-tell-peer-font-size-missing.json"] = peer_font_missing

    peer_font_units = copy.deepcopy(documents["m2-pass-web.json"])
    peer_font_units["artifact"]["id"] = "artifact-m2-cant-tell-peer-font-size-units"
    peer_font_units["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-c"] = font_style(
        16.0, "point"
    )
    documents["m2-cant-tell-peer-font-size-units.json"] = peer_font_units

    minimum_fail = copy.deepcopy(documents["m2-pass-web.json"])
    minimum_fail["artifact"]["id"] = "artifact-m2-fail-minimum-font-size"
    minimum_fail["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-c"] = font_style(
        12.0, "cssPixel"
    )
    del minimum_fail["extensions"][VISUAL_EXTENSION_KEY]["contracts"]["contract-peer-font"]
    documents["m2-fail-minimum-font-size.json"] = minimum_fail

    minimum_boundary = copy.deepcopy(documents["m2-pass-web.json"])
    minimum_boundary["artifact"]["id"] = "artifact-m2-pass-minimum-font-size-boundary"
    for node_id in ("node-a", "node-b", "node-c"):
        minimum_boundary["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"][node_id] = font_style(
            14.0, "cssPixel"
        )
    documents["m2-pass-minimum-font-size-boundary.json"] = minimum_boundary

    minimum_missing = copy.deepcopy(documents["m2-pass-web.json"])
    minimum_missing["artifact"]["id"] = "artifact-m2-cant-tell-minimum-font-size-missing"
    del minimum_missing["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-c"]
    del minimum_missing["extensions"][VISUAL_EXTENSION_KEY]["contracts"]["contract-peer-font"]
    documents["m2-cant-tell-minimum-font-size-missing.json"] = minimum_missing

    minimum_units = copy.deepcopy(documents["m2-pass-web.json"])
    minimum_units["artifact"]["id"] = "artifact-m2-cant-tell-minimum-font-size-units"
    minimum_units["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-minimum-font"
    ]["minimum"]["unit"] = "point"
    documents["m2-cant-tell-minimum-font-size-units.json"] = minimum_units

    visual_inapplicable = base_artifact("web", "artifact-m2-inapplicable-visual")
    add_visual_extension(visual_inapplicable)
    documents["m2-inapplicable-visual.json"] = visual_inapplicable

    unknown_extension = copy.deepcopy(documents["pass-web.json"])
    unknown_extension["artifact"]["id"] = "artifact-m2-unknown-extension"
    unknown_extension["extensions"] = {
        "com.example.opaque": {
            "z": [3, 2, 1],
            "nested": {"message": "preserve me", "enabled": True},
        }
    }
    documents["m2-unknown-extension.json"] = unknown_extension

    invalid_version = copy.deepcopy(documents["m2-pass-web.json"])
    invalid_version["extensions"][VISUAL_EXTENSION_KEY]["extensionVersion"] = "9.9.9"
    documents["m2-invalid-visual-version.json"] = invalid_version

    invalid_payload = copy.deepcopy(documents["pass-web.json"])
    invalid_payload["extensions"] = {VISUAL_EXTENSION_KEY: "not-an-object"}
    documents["m2-invalid-visual-payload.json"] = invalid_payload

    invalid_node = copy.deepcopy(documents["m2-pass-web.json"])
    invalid_node["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["missing-node"] = font_style(
        16.0, "cssPixel"
    )
    documents["m2-invalid-visual-node-reference.json"] = invalid_node

    invalid_evidence = copy.deepcopy(documents["m2-pass-web.json"])
    invalid_evidence["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-alignment"
    ]["evidenceId"] = "missing-evidence"
    documents["m2-invalid-visual-evidence-reference.json"] = invalid_evidence

    duplicate_member = copy.deepcopy(documents["m2-pass-web.json"])
    duplicate_member["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-alignment"
    ]["nodeIds"] = ["node-a", "node-a"]
    documents["m2-invalid-visual-duplicate-member.json"] = duplicate_member

    negative_tolerance = copy.deepcopy(documents["m2-pass-web.json"])
    negative_tolerance["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-extent"
    ]["tolerance"] = -1.0
    documents["m2-invalid-visual-negative-tolerance.json"] = negative_tolerance

    zero_font = copy.deepcopy(documents["m2-pass-web.json"])
    zero_font["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-a"] = font_style(
        0.0, "cssPixel"
    )
    documents["m2-invalid-visual-zero-font.json"] = zero_font

    normalized_minimum = copy.deepcopy(documents["m2-pass-web.json"])
    normalized_minimum["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-minimum-font"
    ]["minimum"]["unit"] = "normalized"
    documents["m2-invalid-visual-normalized-minimum.json"] = normalized_minimum

    empty_style = copy.deepcopy(documents["m2-pass-web.json"])
    empty_style["extensions"][VISUAL_EXTENSION_KEY]["nodeStyles"]["node-a"] = {}
    documents["m2-invalid-visual-empty-style.json"] = empty_style

    insufficient_members = copy.deepcopy(documents["m2-pass-web.json"])
    insufficient_members["extensions"][VISUAL_EXTENSION_KEY]["contracts"][
        "contract-peer-font"
    ]["nodeIds"] = ["node-a"]
    documents["m2-invalid-visual-insufficient-members.json"] = insufficient_members

    empty_contract_id = copy.deepcopy(documents["m2-pass-web.json"])
    contracts = empty_contract_id["extensions"][VISUAL_EXTENSION_KEY]["contracts"]
    contracts[""] = contracts.pop("contract-alignment")
    documents["m2-invalid-visual-empty-contract-id.json"] = empty_contract_id


def build_fixtures() -> dict[str, str]:
    documents: dict[str, dict[str, Any]] = {}
    build_m1_fixtures(documents)
    build_m2_fixtures(documents)

    rendered = {
        name: json.dumps(document, indent=2, ensure_ascii=False) + "\n"
        for name, document in documents.items()
    }
    rendered["invalid-json.json"] = '{"schemaVersion": "0.1.0",\n'
    rendered["README.md"] = """# End-to-end fixture corpus

These synthetic artifacts are generated by `tools/generate_e2e_fixtures.py` and committed for
human review. They exercise the public CLI from bytes through parsing, semantic validation,
geometry queries, official visual-extension rules, reports, and exit codes.

M1 coverage:

- `pass-*` proves the same core contract across every current artifact kind.
- `fail-*` is a targeted mutation fixture for one initial rule.
- `cant-tell-*` proves conservative abstention when evidence is missing or incomparable.
- `invalid-*` proves malformed or semantically invalid input returns exit code 2.
- `inapplicable.json` proves lack of an applicable target is explicit rather than guessed.

M2 coverage:

- `m2-pass-*` covers every artifact kind, direction-aware boundaries, and explicit tolerances.
- `m2-fail-*` kills parent-containment, peer-alignment, peer-extent, peer-font-size, and
  minimum-font-size rules independently.
- `m2-cant-tell-*` covers absent observations, incompatible units, and cross-canvas geometry.
- `m2-invalid-visual-*` covers official-extension decoding, versioning, references, membership,
  numeric, typography-unit, and empty-style validation.
- `m2-inapplicable-visual.json` proves every M2 rule explicitly abstains without a target contract.
- `m2-unknown-extension.json` proves unknown namespaced data survives canonical normalization.

Do not hand-edit generated JSON. Change the generator, regenerate, inspect the diff, and run the
binary E2E suite. Every new rule requires pass, targeted mutation, `cantTell`, inapplicable,
boundary, and malformed-input coverage where applicable.
"""
    return rendered


def apply(*, check: bool) -> int:
    expected = build_fixtures()
    differences: list[str] = []

    for name, content in expected.items():
        path = OUTPUT / name
        actual = path.read_text(encoding="utf-8") if path.exists() else None
        if actual != content:
            differences.append(name)
            if not check:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8", newline="\n")

    if OUTPUT.exists():
        stale = sorted(
            path.name
            for path in OUTPUT.iterdir()
            if path.is_file() and path.name not in expected
        )
        differences.extend(f"stale:{name}" for name in stale)
        if not check:
            for name in stale:
                (OUTPUT / name).unlink()

    if check and differences:
        print("fixture corpus differs from generator:")
        for name in differences:
            print(f"- {name}")
        return 1

    if not check:
        print(f"generated {len(expected) - 1} JSON fixtures and README.md")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when committed fixtures differ from deterministic generation",
    )
    arguments = parser.parse_args()
    return apply(check=arguments.check)


if __name__ == "__main__":
    raise SystemExit(main())
