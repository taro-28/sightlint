//! Public-binary E2E coverage for exact alpha-visible PNG observations.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const ADAM7_PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

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

fn build_rgba_png(width: u32, height: u32, interlace: u8, scanlines: &[u8]) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, interlace]);
    append_chunk(&mut bytes, *b"IHDR", &ihdr);
    append_chunk(&mut bytes, *b"IDAT", &zlib_stored(scanlines));
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

fn non_interlaced_scanlines(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(
        pixels.len(),
        usize::try_from(width * height).expect("small pixel count fits usize")
    );
    let width = usize::try_from(width).expect("width fits usize");
    let mut scanlines = Vec::new();
    for row in pixels.chunks_exact(width) {
        scanlines.push(0);
        for pixel in row {
            scanlines.extend_from_slice(pixel);
        }
    }
    scanlines
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn adam7_scanlines(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(
        pixels.len(),
        usize::try_from(width * height).expect("small pixel count fits usize")
    );
    let mut scanlines = Vec::new();
    for (start_x, start_y, step_x, step_y) in ADAM7_PASSES {
        let pass_width = pass_extent(width, start_x, step_x);
        let pass_height = pass_extent(height, start_y, step_y);
        if pass_width == 0 || pass_height == 0 {
            continue;
        }
        for pass_row in 0..pass_height {
            scanlines.push(0);
            let y = start_y + pass_row * step_y;
            for pass_column in 0..pass_width {
                let x = start_x + pass_column * step_x;
                let index = usize::try_from(y * width + x).expect("small index fits usize");
                scanlines.extend_from_slice(&pixels[index]);
            }
        }
    }
    scanlines
}

fn transparent_pixels(width: u32, height: u32) -> Vec<[u8; 4]> {
    vec![[0, 0, 0, 0]; usize::try_from(width * height).expect("small raster fits usize")]
}

fn set_pixel(pixels: &mut [[u8; 4]], width: u32, x: u32, y: u32, alpha: u8) {
    let index = usize::try_from(y * width + x).expect("small index fits usize");
    pixels[index] = [17, 34, 51, alpha];
}

#[test]
fn emits_exact_visible_bounds_insets_counts_and_evidence() {
    let width = 6;
    let height = 5;
    let mut pixels = transparent_pixels(width, height);
    set_pixel(&mut pixels, width, 1, 1, 255);
    set_pixel(&mut pixels, width, 4, 1, 255);
    set_pixel(&mut pixels, width, 2, 2, 127);
    set_pixel(&mut pixels, width, 1, 3, 255);
    set_pixel(&mut pixels, width, 4, 3, 255);

    let scanlines = non_interlaced_scanlines(width, height, &pixels);
    let input = build_rgba_png(width, height, 0, &scanlines);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let metadata = &ir["extensions"]["org.sightlint.adapter.png"];
    let alpha = &metadata["alphaAnalysis"];

    assert_eq!(metadata["version"], "0.2.0");
    assert_eq!(alpha["visibleBounds"]["x"], 1);
    assert_eq!(alpha["visibleBounds"]["y"], 1);
    assert_eq!(alpha["visibleBounds"]["width"], 4);
    assert_eq!(alpha["visibleBounds"]["height"], 3);
    assert_eq!(alpha["opaqueBounds"]["x"], 1);
    assert_eq!(alpha["opaqueBounds"]["y"], 1);
    assert_eq!(alpha["opaqueBounds"]["width"], 4);
    assert_eq!(alpha["opaqueBounds"]["height"], 3);
    assert_eq!(alpha["transparentInsets"]["top"], 1);
    assert_eq!(alpha["transparentInsets"]["right"], 1);
    assert_eq!(alpha["transparentInsets"]["bottom"], 1);
    assert_eq!(alpha["transparentInsets"]["left"], 1);
    assert_eq!(alpha["visiblePixelCount"], 5);
    assert_eq!(alpha["opaquePixelCount"], 4);
    assert_eq!(alpha["transparentPixelCount"], 25);
    assert_eq!(alpha["translucentPixelCount"], 1);
    assert_eq!(alpha["edgeVisiblePixels"]["top"], 0);
    assert_eq!(alpha["edgeVisiblePixels"]["right"], 0);
    assert_eq!(alpha["edgeVisiblePixels"]["bottom"], 0);
    assert_eq!(alpha["edgeVisiblePixels"]["left"], 0);
    assert_eq!(alpha["allTransparent"], false);
    assert_eq!(alpha["allVisible"], false);

    let ink = &ir["nodes"][0]["geometry"]["inkBox"];
    assert_eq!(ink["rect"]["x"], 1.0);
    assert_eq!(ink["rect"]["y"], 1.0);
    assert_eq!(ink["rect"]["width"], 4.0);
    assert_eq!(ink["rect"]["height"], 3.0);
    assert_eq!(ink["evidenceId"], "evidence:png-alpha");
    assert!(ir["evidence"].as_array().is_some_and(|items| {
        items.iter().any(|item| item["id"] == "evidence:png-alpha")
    }));
}

#[test]
fn edge_counts_include_corner_pixels_on_both_edges() {
    let width = 4;
    let height = 3;
    let mut pixels = transparent_pixels(width, height);
    set_pixel(&mut pixels, width, 0, 0, 1);
    set_pixel(&mut pixels, width, 3, 0, 255);
    set_pixel(&mut pixels, width, 0, 1, 255);
    set_pixel(&mut pixels, width, 3, 2, 128);
    let input = build_rgba_png(
        width,
        height,
        0,
        &non_interlaced_scanlines(width, height, &pixels),
    );
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let edges = &ir["extensions"]["org.sightlint.adapter.png"]["alphaAnalysis"]
        ["edgeVisiblePixels"];
    assert_eq!(edges["top"], 2);
    assert_eq!(edges["right"], 2);
    assert_eq!(edges["bottom"], 1);
    assert_eq!(edges["left"], 2);
}

#[test]
fn distinguishes_fully_opaque_and_fully_transparent_images() {
    let width = 3;
    let height = 2;
    let opaque = vec![[1, 2, 3, 255]; 6];
    let opaque_input = build_rgba_png(
        width,
        height,
        0,
        &non_interlaced_scanlines(width, height, &opaque),
    );
    let opaque_ir = parse_ir(&run_stdin(&["adapt-image", "-"], &opaque_input));
    let opaque_alpha = &opaque_ir["extensions"]["org.sightlint.adapter.png"]["alphaAnalysis"];
    assert_eq!(opaque_alpha["visibleBounds"]["width"], 3);
    assert_eq!(opaque_alpha["visibleBounds"]["height"], 2);
    assert_eq!(opaque_alpha["allTransparent"], false);
    assert_eq!(opaque_alpha["allVisible"], true);
    assert_eq!(opaque_alpha["transparentPixelCount"], 0);
    assert_eq!(
        opaque_ir["nodes"][0]["geometry"]["inkBox"]["rect"]["width"],
        3.0
    );

    let transparent = transparent_pixels(width, height);
    let transparent_input = build_rgba_png(
        width,
        height,
        0,
        &non_interlaced_scanlines(width, height, &transparent),
    );
    let transparent_ir = parse_ir(&run_stdin(&["adapt-image", "-"], &transparent_input));
    let transparent_alpha =
        &transparent_ir["extensions"]["org.sightlint.adapter.png"]["alphaAnalysis"];
    assert!(transparent_alpha["visibleBounds"].is_null());
    assert!(transparent_alpha["opaqueBounds"].is_null());
    assert!(transparent_alpha["transparentInsets"].is_null());
    assert_eq!(transparent_alpha["allTransparent"], true);
    assert_eq!(transparent_alpha["allVisible"], false);
    assert_eq!(transparent_alpha["visiblePixelCount"], 0);
    assert!(transparent_ir["nodes"][0]["geometry"].get("inkBox").is_none());
}

#[test]
fn alpha_geometry_is_preserved_after_adam7_scattering() {
    let width = 8;
    let height = 8;
    let mut pixels = transparent_pixels(width, height);
    for y in 2..=6 {
        for x in 3..=5 {
            if (x + y) % 3 != 0 {
                set_pixel(&mut pixels, width, x, y, 200);
            }
        }
    }
    set_pixel(&mut pixels, width, 3, 2, 255);
    set_pixel(&mut pixels, width, 5, 6, 255);
    let scanlines = adam7_scanlines(width, height, &pixels);
    let input = build_rgba_png(width, height, 1, &scanlines);
    let ir = parse_ir(&run_stdin(&["adapt-image", "-"], &input));
    let alpha = &ir["extensions"]["org.sightlint.adapter.png"]["alphaAnalysis"];
    assert_eq!(alpha["visibleBounds"]["x"], 3);
    assert_eq!(alpha["visibleBounds"]["y"], 2);
    assert_eq!(alpha["visibleBounds"]["width"], 3);
    assert_eq!(alpha["visibleBounds"]["height"], 5);
    assert_eq!(alpha["transparentInsets"]["top"], 2);
    assert_eq!(alpha["transparentInsets"]["right"], 2);
    assert_eq!(alpha["transparentInsets"]["bottom"], 1);
    assert_eq!(alpha["transparentInsets"]["left"], 3);
}

#[test]
fn alpha_analysis_ir_and_reports_are_byte_deterministic() {
    let width = 7;
    let height = 6;
    let mut pixels = transparent_pixels(width, height);
    for y in 1..5 {
        for x in 2..6 {
            set_pixel(
                &mut pixels,
                width,
                x,
                y,
                u8::try_from(40 + x * 20 + y * 7).expect("test alpha fits u8"),
            );
        }
    }
    let input = build_rgba_png(
        width,
        height,
        0,
        &non_interlaced_scanlines(width, height, &pixels),
    );
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
