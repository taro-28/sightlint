//! Public-binary E2E coverage for staged encoded RGBA8 PNG raster availability.

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
        let block_count = bytes.len().div_ceil(65_535);
        for (index, block) in bytes.chunks(65_535).enumerate() {
            output.push(u8::from(index + 1 == block_count));
            let length = u16::try_from(block.len()).expect("stored block fits u16");
            output.extend_from_slice(&length.to_le_bytes());
            output.extend_from_slice(&(!length).to_le_bytes());
            output.extend_from_slice(block);
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
    let crc = crc32(&bytes[crc_start..]);
    bytes.extend_from_slice(&crc.to_be_bytes());
}

fn build_png(
    width: u32,
    height: u32,
    depth: u8,
    color_type: u8,
    interlace: u8,
    scanlines: &[u8],
    transparency: Option<&[u8]>,
) -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[depth, color_type, 0, 0, interlace]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    if color_type == 3 {
        append_chunk(&mut png, *b"PLTE", &[0, 0, 0, 255, 255, 255]);
    }
    if let Some(data) = transparency {
        append_chunk(&mut png, *b"tRNS", data);
    }
    append_chunk(&mut png, *b"IDAT", &zlib_stored(scanlines));
    append_chunk(&mut png, *b"IEND", &[]);
    png
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

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn adam7_rgba_png() -> Vec<u8> {
    let width = 8_u32;
    let height = 8_u32;
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&[
                u8::try_from(x * 17 + y).expect("red fits"),
                u8::try_from(y * 19 + x).expect("green fits"),
                u8::try_from((x * 31 + y * 13) % 256).expect("blue fits"),
                u8::try_from(255 - x * 7 - y * 5).expect("alpha fits"),
            ]);
        }
    }

    let mut scanlines = Vec::new();
    for (start_x, start_y, step_x, step_y) in ADAM7_PASSES {
        let pass_width = pass_extent(width, start_x, step_x);
        let pass_height = pass_extent(height, start_y, step_y);
        if pass_width == 0 || pass_height == 0 {
            continue;
        }
        for row in 0..pass_height {
            scanlines.push(0);
            let y = start_y + row * step_y;
            for column in 0..pass_width {
                let x = start_x + column * step_x;
                let offset = usize::try_from((y * width + x) * 4).expect("offset fits");
                scanlines.extend_from_slice(&pixels[offset..offset + 4]);
            }
        }
    }
    build_png(width, height, 8, 6, 1, &scanlines, None)
}

#[test]
fn available_raster_metadata_is_exact_and_does_not_serialize_pixels() {
    let scanlines = [
        0_u8, 1, 2, 3, 4, 0, 5, 6, 7, 8,
        0, 9, 10, 11, 12, 0, 13, 14, 15, 16,
    ];
    let png = build_png(2, 2, 8, 6, 0, &scanlines, None);
    let first = run_stdin(&["adapt-image", "-"], &png);
    let ir = parse_ir(&first);
    let metadata = &ir["extensions"]["org.sightlint.adapter.png"];
    assert_eq!(metadata["reconstructedPackedSampleBytes"], 16);
    assert_eq!(metadata["nonEmptyPassCount"], 1);
    assert_eq!(metadata["encodedRgba8Raster"]["status"], "available");
    assert_eq!(metadata["encodedRgba8Raster"]["width"], 2);
    assert_eq!(metadata["encodedRgba8Raster"]["height"], 2);
    assert_eq!(metadata["encodedRgba8Raster"]["byteCount"], 16);
    assert!(metadata["encodedRgba8Raster"].get("pixels").is_none());
    assert!(!String::from_utf8_lossy(&first.stdout).contains("\"pixels\""));

    for _ in 0..10 {
        let repeated = run_stdin(&["adapt-image", "-"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert_eq!(first.stdout, repeated.stdout);
    }
}

#[test]
fn unsupported_raster_formats_remain_successful_with_stable_reasons() {
    let cases = [
        (
            build_png(2, 1, 8, 3, 0, &[0, 0, 1], None),
            "indexedColor",
        ),
        (
            build_png(8, 1, 1, 0, 0, &[0, 0b1010_0101], None),
            "unsupportedBitDepth",
        ),
        (
            build_png(
                1,
                1,
                8,
                2,
                0,
                &[0, 10, 20, 30],
                Some(&[0, 10, 0, 20, 0, 30]),
            ),
            "transparencyChunk",
        ),
    ];

    for (png, reason) in cases {
        let output = run_stdin(&["adapt-image", "-"], &png);
        let ir = parse_ir(&output);
        let raster = &ir["extensions"]["org.sightlint.adapter.png"]["encodedRgba8Raster"];
        assert_eq!(raster["status"], "unavailable");
        assert_eq!(raster["reason"], reason);
        assert!(raster.get("pixels").is_none());
    }
}

#[test]
fn adam7_raster_availability_is_deterministic_through_check_image() {
    let png = adam7_rgba_png();
    let adapted = run_stdin(&["adapt-image", "-"], &png);
    let ir = parse_ir(&adapted);
    let metadata = &ir["extensions"]["org.sightlint.adapter.png"];
    assert_eq!(metadata["nonEmptyPassCount"], 7);
    assert_eq!(metadata["encodedRgba8Raster"]["status"], "available");
    assert_eq!(metadata["encodedRgba8Raster"]["byteCount"], 256);

    let first = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_eq!(first.status.code(), Some(EXIT_SUCCESS));
    for _ in 0..10 {
        let repeated = run_stdin(&["check-image", "-", "--format", "json"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert_eq!(first.stdout, repeated.stdout);
    }
}
