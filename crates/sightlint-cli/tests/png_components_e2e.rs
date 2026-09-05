//! Public-binary E2E coverage for bounded background-relative component hypotheses.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

const EXIT_SUCCESS: i32 = 0;

type Pixel = [u8; 4];

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("CLI crate lives below repository root")
        .to_path_buf()
}

fn corpus_manifest() -> Value {
    let path = repository_root().join("fixtures/evaluation/image/manifest.json");
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_slice(&bytes).expect("image evaluation manifest")
}

fn corpus_case(case_id: &str) -> (Value, PathBuf) {
    let manifest = corpus_manifest();
    let case = manifest["cases"]
        .as_array()
        .expect("cases array")
        .iter()
        .find(|case| case["id"] == case_id)
        .unwrap_or_else(|| panic!("missing corpus case {case_id}"))
        .clone();
    let file = case["file"].as_str().expect("case file");
    let path = repository_root().join("fixtures/evaluation/image").join(file);
    (case, path)
}

fn run_file(args: &[&str], path: &Path) -> Output {
    binary()
        .args(args)
        .arg(path)
        .output()
        .expect("failed to execute sightlint")
}

fn run_stdin(input: &[u8]) -> Output {
    let mut child = binary()
        .args(["adapt-image", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn sightlint");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("failed to write PNG bytes");
    child.wait_with_output().expect("failed to collect output")
}

fn parse_ir(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("canonical Artifact IR JSON")
}

fn component_analysis(document: &Value) -> &Value {
    &document["extensions"]["org.sightlint.adapter.png"]["backgroundRelativeComponents"]
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for &byte in bytes {
        a = (a + u32::from(byte)) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

fn zlib_stored(bytes: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    if bytes.is_empty() {
        output.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let chunk_count = bytes.len().div_ceil(65_535);
        for (index, chunk) in bytes.chunks(65_535).enumerate() {
            output.push(u8::from(index + 1 == chunk_count));
            let length = u16::try_from(chunk.len()).expect("stored block fits u16");
            output.extend_from_slice(&length.to_le_bytes());
            output.extend_from_slice(&(!length).to_le_bytes());
            output.extend_from_slice(chunk);
        }
    }
    output.extend_from_slice(&adler32(bytes).to_be_bytes());
    output
}

fn append_chunk(bytes: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let length = u32::try_from(data.len()).expect("test chunk length fits u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(data);
    let crc_start = bytes.len() - data.len() - 4;
    bytes.extend_from_slice(&crc32(&bytes[crc_start..]).to_be_bytes());
}

fn rgba_png(width: u32, height: u32, pixels: &[Pixel]) -> Vec<u8> {
    assert_eq!(
        pixels.len(),
        usize::try_from(width * height).expect("test raster fits usize")
    );
    let mut scanlines = Vec::new();
    let width_usize = usize::try_from(width).expect("width fits usize");
    for row in pixels.chunks_exact(width_usize) {
        scanlines.push(0);
        for pixel in row {
            scanlines.extend_from_slice(pixel);
        }
    }
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    append_chunk(&mut png, *b"IDAT", &zlib_stored(&scanlines));
    append_chunk(&mut png, *b"IEND", &[]);
    png
}

fn filled(width: u32, height: u32, color: Pixel) -> Vec<Pixel> {
    vec![color; usize::try_from(width * height).expect("test raster fits usize")]
}

fn set_pixel(pixels: &mut [Pixel], width: u32, x: u32, y: u32, color: Pixel) {
    pixels[usize::try_from(y * width + x).expect("test index fits usize")] = color;
}

fn expected_regions(case: &Value) -> BTreeMap<&str, &Value> {
    case["groundTruth"]["regions"]
        .as_array()
        .expect("regions")
        .iter()
        .map(|region| (region["id"].as_str().expect("region id"), &region["rect"]))
        .collect()
}

fn rect(document: &Value, component_index: usize) -> Value {
    component_analysis(document)["components"][component_index]["bounds"].clone()
}

fn horizontal_gap(left: &Value, right: &Value) -> i64 {
    let left_x = left["x"].as_i64().expect("left x");
    let left_width = left["width"].as_i64().expect("left width");
    let right_x = right["x"].as_i64().expect("right x");
    right_x - left_x - left_width
}

#[test]
fn dashboard_components_match_regions_and_clean_mutation_gaps() {
    for (case_id, expected_gaps) in [
        ("opaque-dashboard-clean", vec![12, 12]),
        ("opaque-dashboard-spacing-mutation", vec![12, 19]),
    ] {
        let (case, path) = corpus_case(case_id);
        let document = parse_ir(&run_file(&["adapt-image"], &path));
        let analysis = component_analysis(&document);
        assert_eq!(
            document["extensions"]["org.sightlint.adapter.png"]["version"],
            "0.4.0"
        );
        assert_eq!(analysis["status"], "available");
        assert_eq!(analysis["policy"], "opaque-border-components-v1");
        assert_eq!(analysis["candidateRgba"], "#f6f7f9ff");
        assert_eq!(analysis["requiredEdgePercent"], 95);
        assert_eq!(analysis["components"].as_array().map(Vec::len), Some(4));

        let regions = expected_regions(&case);
        assert_eq!(rect(&document, 0), *regions["top-navigation"]);
        assert_eq!(rect(&document, 1), *regions["card-1"]);
        assert_eq!(rect(&document, 2), *regions["card-2"]);
        assert_eq!(rect(&document, 3), *regions["card-3"]);
        for index in 0..4 {
            assert_eq!(
                analysis["components"][index]["touchesCanvasEdge"],
                false
            );
        }

        let gaps = vec![
            horizontal_gap(&rect(&document, 1), &rect(&document, 2)),
            horizontal_gap(&rect(&document, 2), &rect(&document, 3)),
        ];
        assert_eq!(gaps, expected_gaps, "case {case_id}");
        assert_eq!(
            case["groundTruth"]["peerGroups"][0]["gaps"]["values"],
            json!(expected_gaps)
        );

        // Hypothetical components do not replace exact alpha-visible core geometry.
        assert_eq!(
            document["nodes"][0]["geometry"]["inkBox"]["rect"],
            json!({"x": 0.0, "y": 0.0, "width": 240.0, "height": 160.0})
        );
        assert_eq!(document["nodes"].as_array().map(Vec::len), Some(1));
        assert_eq!(document["relations"].as_array().map(Vec::len), Some(0));
    }
}

#[test]
fn weak_border_and_alpha_cases_abstain_explicitly() {
    let (_, tie_path) = corpus_case("opaque-border-tie");
    let tie = parse_ir(&run_file(&["adapt-image"], &tie_path));
    assert_eq!(
        component_analysis(&tie)["status"],
        "noQualifiedBackgroundCandidate"
    );
    assert_eq!(
        component_analysis(&tie)["components"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    for case_id in ["transparent-symbol-padding", "translucent-overlay"] {
        let (_, path) = corpus_case(case_id);
        let document = parse_ir(&run_file(&["adapt-image"], &path));
        assert_eq!(
            component_analysis(&document)["status"],
            "requiresFullyOpaquePixels",
            "case {case_id}"
        );
    }
}

#[test]
fn unanimous_corners_without_edge_support_do_not_qualify() {
    let width = 10;
    let height = 10;
    let white = [255, 255, 255, 255];
    let black = [0, 0, 0, 255];
    let mut pixels = filled(width, height, black);
    for &(x, y) in &[(0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)] {
        set_pixel(&mut pixels, width, x, y, white);
    }
    let document = parse_ir(&run_stdin(&rgba_png(width, height, &pixels)));
    let analysis = component_analysis(&document);
    assert_eq!(analysis["status"], "noQualifiedBackgroundCandidate");
    assert_eq!(analysis["candidateRgba"], "#ffffffff");
    assert_eq!(analysis["candidateCornerOccurrences"], 4);
    assert_eq!(analysis["candidateEdgePixelCount"], 4);
    assert_eq!(analysis["edgeSampleCount"], 36);
}

#[test]
fn tiny_images_are_explicitly_outside_policy() {
    for (width, height) in [(1, 1), (2, 3), (3, 2)] {
        let pixels = filled(width, height, [255, 255, 255, 255]);
        let document = parse_ir(&run_stdin(&rgba_png(width, height, &pixels)));
        assert_eq!(
            component_analysis(&document)["status"],
            "imageTooSmall",
            "{width}x{height}"
        );
    }
}

#[test]
fn component_output_and_reports_are_byte_deterministic() {
    let (_, path) = corpus_case("opaque-dashboard-spacing-mutation");
    let first_ir = run_file(&["adapt-image"], &path);
    let first_report = run_file(&["check-image", "--format", "json"], &path);
    assert_eq!(first_ir.status.code(), Some(EXIT_SUCCESS));
    assert_eq!(first_report.status.code(), Some(EXIT_SUCCESS));
    for _ in 0..10 {
        let repeated_ir = run_file(&["adapt-image"], &path);
        let repeated_report = run_file(&["check-image", "--format", "json"], &path);
        assert_eq!(first_ir.stdout, repeated_ir.stdout);
        assert_eq!(first_report.stdout, repeated_report.stdout);
    }
}
