//! Public-binary E2E coverage for deterministic opaque-border background candidates.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;

type Pixel = [u8; 4];

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn run_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = binary()
        .args(args)
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

fn rgba_png(width: u32, height: u32, pixels: &[Pixel], color_description: bool) -> Vec<u8> {
    assert_eq!(
        pixels.len(),
        usize::try_from(width * height).expect("small raster fits usize")
    );
    let mut scanlines = Vec::new();
    let width_usize = usize::try_from(width).expect("width fits usize");
    for row in pixels.chunks_exact(width_usize) {
        scanlines.push(0);
        for pixel in row {
            scanlines.extend_from_slice(pixel);
        }
    }

    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    append_chunk(&mut bytes, *b"IHDR", &ihdr);
    if color_description {
        append_chunk(&mut bytes, *b"gAMA", &45_455_u32.to_be_bytes());
    }
    append_chunk(&mut bytes, *b"IDAT", &zlib_stored(&scanlines));
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

fn filled(width: u32, height: u32, color: Pixel) -> Vec<Pixel> {
    vec![color; usize::try_from(width * height).expect("small raster fits usize")]
}

fn set_pixel(pixels: &mut [Pixel], width: u32, x: u32, y: u32, color: Pixel) {
    let index = usize::try_from(y * width + x).expect("small pixel index fits usize");
    pixels[index] = color;
}

fn background_analysis(ir: &Value) -> &Value {
    &ir["extensions"]["org.sightlint.adapter.png"]["backgroundCandidateAnalysis"]
}

#[test]
fn ranks_flat_border_background_and_bounds_contrasting_content() {
    let width = 10;
    let height = 8;
    let white = [255, 255, 255, 255];
    let content = [20, 80, 200, 255];
    let mut pixels = filled(width, height, white);
    for y in 2..6 {
        for x in 2..8 {
            set_pixel(&mut pixels, width, x, y, content);
        }
    }

    let input = rgba_png(width, height, &pixels, false);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let extension = &ir["extensions"]["org.sightlint.adapter.png"];
    let analysis = background_analysis(&ir);
    let leading = &analysis["candidates"][0];

    assert_eq!(extension["version"], "0.3.0");
    assert_eq!(analysis["applicability"], "fullyOpaque");
    assert_eq!(analysis["cornerSampleCount"], 4);
    assert_eq!(analysis["edgeSampleCount"], 32);
    assert_eq!(analysis["imagePixelCount"], 80);
    assert_eq!(analysis["candidateLimit"], 8);
    assert_eq!(analysis["leadingCandidateIndex"], 0);
    assert_eq!(leading["rgba"], "#ffffffff");
    assert_eq!(leading["cornerOccurrences"], 4);
    assert_eq!(leading["edgePixelCount"], 32);
    assert_eq!(leading["imagePixelCount"], 56);
    assert_eq!(leading["nonCandidateBounds"]["x"], 2);
    assert_eq!(leading["nonCandidateBounds"]["y"], 2);
    assert_eq!(leading["nonCandidateBounds"]["width"], 6);
    assert_eq!(leading["nonCandidateBounds"]["height"], 4);

    // The hypothesis does not replace exact alpha-visible geometry.
    let ink = &ir["nodes"][0]["geometry"]["inkBox"]["rect"];
    assert_eq!(ink["x"], 0.0);
    assert_eq!(ink["y"], 0.0);
    assert_eq!(ink["width"], 10.0);
    assert_eq!(ink["height"], 8.0);
}

#[test]
fn all_one_color_has_no_non_candidate_bounds() {
    let color = [12, 34, 56, 255];
    let input = rgba_png(3, 2, &filled(3, 2, color), false);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let analysis = background_analysis(&ir);
    assert_eq!(analysis["candidates"].as_array().map(Vec::len), Some(1));
    let leading = &analysis["candidates"][0];
    assert_eq!(leading["rgba"], "#0c2238ff");
    assert_eq!(leading["cornerOccurrences"], 4);
    assert_eq!(leading["edgePixelCount"], 6);
    assert_eq!(leading["imagePixelCount"], 6);
    assert!(leading["nonCandidateBounds"].is_null());
}

#[test]
fn retains_unique_corners_beyond_top_edge_frequency_colors() {
    let width = 14;
    let height = 4;
    let interior = [9, 9, 9, 255];
    let mut pixels = filled(width, height, interior);
    let edge_colors = [
        [10, 0, 0, 255],
        [20, 0, 0, 255],
        [30, 0, 0, 255],
        [40, 0, 0, 255],
        [50, 0, 0, 255],
    ];
    for x in 1..(width - 1) {
        let color = edge_colors[usize::try_from((x - 1) % 5).expect("index fits usize")];
        set_pixel(&mut pixels, width, x, 0, color);
        set_pixel(&mut pixels, width, x, height - 1, color);
    }
    for y in 1..(height - 1) {
        set_pixel(&mut pixels, width, 0, y, edge_colors[0]);
        set_pixel(&mut pixels, width, width - 1, y, edge_colors[1]);
    }
    let corners = [
        ([201, 1, 1, 255], 0, 0),
        ([202, 1, 1, 255], width - 1, 0),
        ([203, 1, 1, 255], 0, height - 1),
        ([204, 1, 1, 255], width - 1, height - 1),
    ];
    for &(color, x, y) in &corners {
        set_pixel(&mut pixels, width, x, y, color);
    }

    let input = rgba_png(width, height, &pixels, false);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let colors: Vec<&str> = background_analysis(&ir)["candidates"]
        .as_array()
        .expect("candidate array")
        .iter()
        .filter_map(|candidate| candidate["rgba"].as_str())
        .collect();
    assert_eq!(colors.len(), 8);
    for (corner, _, _) in corners {
        let expected = format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            corner[0], corner[1], corner[2], corner[3]
        );
        assert!(colors.contains(&expected.as_str()));
    }
}

#[test]
fn degenerate_images_use_unique_corner_and_edge_positions() {
    let single = rgba_png(1, 1, &[[1, 2, 3, 255]], false);
    let single_ir = parse_ir(&run_stdin(&["adapt-image", "-"], &single));
    let single_analysis = background_analysis(&single_ir);
    assert_eq!(single_analysis["cornerSampleCount"], 1);
    assert_eq!(single_analysis["edgeSampleCount"], 1);
    assert_eq!(single_analysis["candidates"][0]["cornerOccurrences"], 1);

    let row_pixels = [
        [1, 1, 1, 255],
        [2, 2, 2, 255],
        [2, 2, 2, 255],
        [3, 3, 3, 255],
    ];
    let row = rgba_png(4, 1, &row_pixels, false);
    let row_ir = parse_ir(&run_stdin(&["adapt-image", "-"], &row));
    let row_analysis = background_analysis(&row_ir);
    assert_eq!(row_analysis["cornerSampleCount"], 2);
    assert_eq!(row_analysis["edgeSampleCount"], 4);
    assert_eq!(row_analysis["imagePixelCount"], 4);
}

#[test]
fn alpha_images_are_explicitly_inapplicable() {
    for alpha in [0, 128] {
        let mut pixels = filled(2, 2, [255, 255, 255, 255]);
        set_pixel(&mut pixels, 2, 1, 1, [20, 30, 40, alpha]);
        let input = rgba_png(2, 2, &pixels, false);
        let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
        let analysis = background_analysis(&ir);
        assert_eq!(analysis["applicability"], "requiresFullyOpaquePixels");
        assert_eq!(analysis["candidates"].as_array().map(Vec::len), Some(0));
        assert!(analysis["leadingCandidateIndex"].is_null());
    }
}

#[test]
fn unresolved_color_description_is_preserved_without_changing_candidate_space() {
    let pixels = filled(4, 3, [120, 130, 140, 255]);
    let input = rgba_png(4, 3, &pixels, true);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let extension = &ir["extensions"]["org.sightlint.adapter.png"];
    let analysis = background_analysis(&ir);
    assert_eq!(extension["unappliedColorDescriptionPresent"], true);
    assert_eq!(extension["colorManagementApplied"], false);
    assert_eq!(analysis["colorSpace"], "pngEncodedRgba8");
    assert_eq!(analysis["candidates"][0]["rgba"], "#78828cff");
}

#[test]
fn candidate_output_and_reports_are_byte_deterministic() {
    let width = 9;
    let height = 7;
    let mut pixels = filled(width, height, [250, 250, 250, 255]);
    for y in 1..6 {
        for x in 2..7 {
            let value = u8::try_from((x * 31 + y * 19) % 255).expect("value fits u8");
            set_pixel(&mut pixels, width, x, y, [value, 80, 160, 255]);
        }
    }
    let input = rgba_png(width, height, &pixels, false);
    let first_ir = run_stdin(&["adapt-image", "-"], &input);
    let first_report = run_stdin(&["check-image", "-", "--format", "json"], &input);
    assert_eq!(first_ir.status.code(), Some(EXIT_SUCCESS));
    assert_eq!(first_report.status.code(), Some(EXIT_SUCCESS));
    for _ in 0..10 {
        let repeated_ir = run_stdin(&["adapt-image", "-"], &input);
        let repeated_report = run_stdin(&["check-image", "-", "--format", "json"], &input);
        assert_eq!(first_ir.stdout, repeated_ir.stdout);
        assert_eq!(first_report.stdout, repeated_report.stdout);
    }
}
