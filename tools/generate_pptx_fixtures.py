#!/usr/bin/env python3
"""Generate deterministic repository-owned PPTX fixtures for ADR 0043."""

from __future__ import annotations

import argparse
import hashlib
import json
import tempfile
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURE_DIR = ROOT / "fixtures" / "pptx"
REQUEST_DIR = ROOT / "evaluation" / "pptx" / "requests"
RENDER_DIR = ROOT / "evaluation" / "pptx" / "renders"
SLIDE_WIDTH = 9_144_000
SLIDE_HEIGHT = 5_143_500
EMU_PER_PIXEL = 9_525
FIXED_TIME = (1980, 1, 1, 0, 0, 0)


def sha256(path: Path) -> str:
    return f"sha256:{hashlib.sha256(path.read_bytes()).hexdigest()}"


def xml_header(body: str) -> bytes:
    return f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n{body}\n'.encode()


def shape(identifier: int, name: str, text: str, x: int, y: int, width: int, height: int, color: str) -> str:
    text_body = ""
    if text:
        text_body = f"""
      <p:txBody>
        <a:bodyPr/><a:lstStyle/>
        <a:p><a:r><a:rPr lang="en-US" sz="1800"/><a:t>{text}</a:t></a:r><a:endParaRPr lang="en-US"/></a:p>
      </p:txBody>"""
    return f"""
    <p:sp>
      <p:nvSpPr><p:cNvPr id="{identifier}" name="{name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{width}" cy="{height}"/></a:xfrm>
        <a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>
        <a:solidFill><a:srgbClr val="{color}"/></a:solidFill>
        <a:ln><a:solidFill><a:srgbClr val="D8DEE9"/></a:solidFill></a:ln>
      </p:spPr>{text_body}
    </p:sp>"""


def title_shape() -> str:
    return shape(2, "Title", "Quarterly operations", 685_800, 457_200, 7_772_400, 685_800, "E8EEF8")


def group(cards: list[tuple[int, str, int, int, int, int, str]]) -> str:
    children = "".join(shape(identifier, f"Card {identifier}", text, x, y, width, height, color) for identifier, text, x, y, width, height, color in cards)
    return f"""
    <p:grpSp>
      <p:nvGrpSpPr><p:cNvPr id="3" name="Metrics group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="685800" y="1600200"/><a:ext cx="7467600" cy="1828800"/><a:chOff x="0" y="0"/><a:chExt cx="7467600" cy="1828800"/></a:xfrm></p:grpSpPr>
      {children}
    </p:grpSp>"""


def slide_xml(case: str) -> bytes:
    if case == "clean":
        cards = [
            (4, "Response time", 0, 0, 2_286_000, 1_828_800, "FFFFFF"),
            (5, "Resolution", 2_590_800, 0, 2_286_000, 1_828_800, "FFFFFF"),
            (6, "Satisfaction", 5_181_600, 0, 2_286_000, 1_828_800, "FFFFFF"),
        ]
    elif case == "off-slide-mutant":
        cards = [
            (4, "Response time", 0, 0, 2_286_000, 1_828_800, "FFFFFF"),
            (5, "Resolution", 2_590_800, 0, 2_286_000, 1_828_800, "FFFFFF"),
            (6, "Satisfaction", 8_229_600, 0, 2_286_000, 1_828_800, "FDE8E8"),
        ]
    elif case == "asymmetric-hard-negative":
        cards = [
            (4, "Primary narrative", 0, 0, 3_352_800, 1_828_800, "FFFFFF"),
            (5, "Supporting note", 3_657_600, 0, 1_371_600, 1_828_800, "F5F7FB"),
            (6, "Next action", 5_638_800, 0, 1_828_800, 1_828_800, "EAF4FF"),
        ]
    else:
        raise ValueError(case)
    body = f"""<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld name="Atlas {case}">
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      {title_shape()}
      {group(cards)}
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"""
    return xml_header(body)


def package_parts(case: str) -> dict[str, bytes]:
    content_types = xml_header("""<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>""")
    root_rels = xml_header("""<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>""")
    presentation = xml_header(f"""<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="{SLIDE_WIDTH}" cy="{SLIDE_HEIGHT}" type="screen16x9"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>""")
    presentation_rels = xml_header("""<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>""")
    slide_rels = xml_header("""<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>""")
    layout = xml_header("""<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
  <p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>""")
    layout_rels = xml_header("""<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>""")
    master = xml_header("""<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld name="Master"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" folHlink="folHlink" hlink="hlink" tx1="dk1" tx2="dk2"/>
  <p:sldLayoutIdLst><p:sldLayoutId id="1" r:id="rId1"/></p:sldLayoutIdLst>
  <p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles>
</p:sldMaster>""")
    master_rels = xml_header("""<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>""")
    theme = xml_header("""<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="SightLint fixture">
  <a:themeElements>
    <a:clrScheme name="SightLint"><a:dk1><a:srgbClr val="172033"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="334155"/></a:dk2><a:lt2><a:srgbClr val="F8FAFC"/></a:lt2><a:accent1><a:srgbClr val="2563EB"/></a:accent1><a:accent2><a:srgbClr val="0F766E"/></a:accent2><a:accent3><a:srgbClr val="B45309"/></a:accent3><a:accent4><a:srgbClr val="7C3AED"/></a:accent4><a:accent5><a:srgbClr val="BE123C"/></a:accent5><a:accent6><a:srgbClr val="0369A1"/></a:accent6><a:hlink><a:srgbClr val="0000FF"/></a:hlink><a:folHlink><a:srgbClr val="800080"/></a:folHlink></a:clrScheme>
    <a:fontScheme name="SightLint"><a:majorFont><a:latin typeface="Liberation Sans"/></a:majorFont><a:minorFont><a:latin typeface="Liberation Sans"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="SightLint"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme>
  </a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/>
</a:theme>""")
    core = xml_header("""<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>SightLint PPTX fixture</dc:title><dc:creator>SightLint project</dc:creator><cp:lastModifiedBy>SightLint project</cp:lastModifiedBy></cp:coreProperties>""")
    app = xml_header("""<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><Application>SightLint fixture generator</Application><PresentationFormat>On-screen Show (16:9)</PresentationFormat><Slides>1</Slides></Properties>""")
    return {
        "[Content_Types].xml": content_types,
        "_rels/.rels": root_rels,
        "docProps/app.xml": app,
        "docProps/core.xml": core,
        "ppt/_rels/presentation.xml.rels": presentation_rels,
        "ppt/presentation.xml": presentation,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels": layout_rels,
        "ppt/slideLayouts/slideLayout1.xml": layout,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels": master_rels,
        "ppt/slideMasters/slideMaster1.xml": master,
        "ppt/slides/_rels/slide1.xml.rels": slide_rels,
        "ppt/slides/slide1.xml": slide_xml(case),
        "ppt/theme/theme1.xml": theme,
    }


def build_pptx(case: str, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED, allowZip64=False) as archive:
        for name, content in sorted(package_parts(case).items()):
            info = zipfile.ZipInfo(name, FIXED_TIME)
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)


def request(case: str, pptx_path: Path, pptx_reference: Path, render_path: Path) -> dict[str, object]:
    return {
        "protocolVersion": "0.1.0",
        "requestId": f"pptx-{case}",
        "artifact": {"id": f"pptx-{case}", "title": f"Atlas presentation {case}"},
        "input": {"reference": str(pptx_reference.relative_to(ROOT)), "sha256": sha256(pptx_path)},
        "renders": [{"slideIndex": 1, "reference": str(render_path.relative_to(ROOT)), "sha256": sha256(render_path), "emuPerPixel": EMU_PER_PIXEL}],
        "privacy": {"externalProcessing": False, "retention": "none", "textPolicy": "digestOnly"},
        "execution": {
            "maxArchiveBytes": 1_048_576,
            "maxRenderBytes": 1_048_576,
            "maxEntries": 64,
            "maxExpandedBytes": 2_097_152,
            "maxXmlBytes": 262_144,
            "maxCompressionRatio": 20,
            "maxSlides": 4,
            "maxNodes": 64,
            "maxGroupDepth": 8,
            "maxOutputBytes": 1_048_576,
        },
    }


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()


def expected_files() -> dict[Path, bytes]:
    files: dict[Path, bytes] = {}
    with tempfile.TemporaryDirectory() as temporary:
        temp = Path(temporary)
        for case in ("clean", "off-slide-mutant", "asymmetric-hard-negative"):
            pptx = temp / f"atlas-{case}.pptx"
            build_pptx(case, pptx)
            final_pptx = FIXTURE_DIR / pptx.name
            files[final_pptx] = pptx.read_bytes()
            render = RENDER_DIR / f"atlas-{case}.png"
            if not render.is_file():
                raise SystemExit(f"missing reviewed render: {render.relative_to(ROOT)}")
            request_value = request(case, pptx, final_pptx, render)
            files[REQUEST_DIR / f"atlas-{case}.json"] = canonical_json(request_value)
    return files


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = expected_files()
    drift = []
    for path, content in files.items():
        if args.check:
            if not path.is_file() or path.read_bytes() != content:
                drift.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
    if drift:
        raise SystemExit("PPTX fixture drift: " + ", ".join(drift))
    print(f"PPTX fixtures: {len(files)} files {'verified' if args.check else 'generated'}")


if __name__ == "__main__":
    main()
