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
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "id": identifier,
        "kind": observed("control", "e-source"),
        "coordinateSpaceId": canvas,
        "role": observed("button", "e-source"),
        "name": observed(name, "e-source"),
        "geometry": {},
    }
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


def build_fixtures() -> dict[str, str]:
    documents: dict[str, dict[str, Any]] = {}

    for kind in ("web", "mobile", "slide", "document", "pdf", "image", "other"):
        documents[f"pass-{kind}.json"] = base_artifact(
            kind, f"artifact-pass-{kind}"
        )

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

    overlap = base_artifact(
        "web", "artifact-fail-overlap", include_relations=False
    )
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
        node(
            "node-outside",
            390.0,
            20.0,
            width=30.0,
            name="Outside",
        )
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
        node(
            "node-b",
            20.0,
            70.0,
            canvas="canvas-second",
            name="Second",
        ),
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

    inapplicable = base_artifact(
        "image", "artifact-inapplicable", include_relations=False
    )
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
    invalid_geometry["nodes"][0]["geometry"]["renderBox"]["rect"][
        "width"
    ] = -1.0
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

    rendered = {
        name: json.dumps(document, indent=2, ensure_ascii=False) + "\n"
        for name, document in documents.items()
    }
    rendered["invalid-json.json"] = '{"schemaVersion": "0.1.0",\n'
    rendered["README.md"] = """# End-to-end fixture corpus

These synthetic artifacts are generated by `tools/generate_e2e_fixtures.py` and committed for
human review. They exercise the public CLI from bytes through parsing, semantic validation,
geometry queries, rules, reports, and exit codes.

- `pass-*` proves the same core contract across every current artifact kind.
- `fail-*` is a mutation fixture for one initial rule.
- `cant-tell-*` proves conservative abstention when evidence is missing or incomparable.
- `invalid-*` proves malformed or semantically invalid input returns exit code 2.
- `inapplicable.json` proves lack of an applicable target is explicit rather than guessed.

Do not hand-edit generated JSON. Change the generator, regenerate, inspect the diff, and run the
binary E2E suite.
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
