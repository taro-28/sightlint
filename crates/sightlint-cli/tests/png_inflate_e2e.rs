//! Public-binary E2E coverage for bounded PNG zlib/DEFLATE inflation.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 2;
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

fn assert_error_contains(input: &[u8], expected: &str) {
    let output = run_stdin(&["adapt-image", "-"], input);
    assert_eq!(output.status.code(), Some(EXIT_ERROR));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected {expected:?} in {stderr:?}"
    );
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
    let crc = crc32(&bytes[crc_start..]);
    bytes.extend_from_slice(&crc.to_be_bytes());
}

fn header_prefix(width: u32, height: u32, depth: u8, color_type: u8, interlace: u8) -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[depth, color_type, 0, 0, interlace]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    if color_type == 3 {
        append_chunk(&mut png, *b"PLTE", &[0, 0, 0]);
    }
    png
}

fn channels(color_type: u8) -> u64 {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!("validated test color type"),
    }
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn pass_bytes(width: u32, height: u32, bits_per_pixel: u64) -> u64 {
    if width == 0 || height == 0 {
        return 0;
    }
    let row_bytes = (u64::from(width) * bits_per_pixel).div_ceil(8);
    u64::from(height) * (1 + row_bytes)
}

fn expected_bytes(width: u32, height: u32, depth: u8, color_type: u8, interlace: u8) -> usize {
    let bits_per_pixel = channels(color_type) * u64::from(depth);
    let total = if interlace == 0 {
        pass_bytes(width, height, bits_per_pixel)
    } else {
        ADAM7_PASSES
            .into_iter()
            .map(|(start_x, start_y, step_x, step_y)| {
                pass_bytes(
                    pass_extent(width, start_x, step_x),
                    pass_extent(height, start_y, step_y),
                    bits_per_pixel,
                )
            })
            .sum()
    };
    usize::try_from(total).expect("small test raster fits usize")
}

fn png_with_data(
    width: u32,
    height: u32,
    depth: u8,
    color_type: u8,
    interlace: u8,
    decoded: &[u8],
    split_idat: bool,
) -> Vec<u8> {
    let mut png = header_prefix(width, height, depth, color_type, interlace);
    let compressed = zlib_stored(decoded);
    if split_idat {
        let split = compressed.len() / 2;
        append_chunk(&mut png, *b"IDAT", &compressed[..split]);
        append_chunk(&mut png, *b"IDAT", &compressed[split..]);
    } else {
        append_chunk(&mut png, *b"IDAT", &compressed);
    }
    append_chunk(&mut png, *b"IEND", &[]);
    png
}

// Generated independently with Python's zlib.compress(..., level=6). These vectors exercise
// fixed and dynamic Huffman DEFLATE blocks rather than SightLint's stored-block test encoder.
const PYTHON_ZLIB_FIXED_RGBA_3X2: &[u8] = &[
    0x78, 0x9c, 0x63, 0x60, 0xc0, 0x05, 0x00, 0x00, 0x1a, 0x00, 0x01,
];
const PYTHON_ZLIB_DYNAMIC_RGBA_64X64: &[u8] = &[
    0x78, 0x9c, 0xed, 0xc1, 0x01, 0x0d, 0x00, 0x00, 0x00, 0xc2, 0xa0, 0xf7, 0x4f, 0x6d, 0x0e, 0x37,
    0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x80, 0x77, 0x03, 0x40, 0x40, 0x00, 0x01,
];

#[test]
fn accepts_independent_fixed_and_dynamic_huffman_zlib_streams() {
    for (width, height, compressed) in [
        (3_u32, 2_u32, PYTHON_ZLIB_FIXED_RGBA_3X2),
        (64_u32, 64_u32, PYTHON_ZLIB_DYNAMIC_RGBA_64X64),
    ] {
        let mut png = header_prefix(width, height, 8, 6, 0);
        append_chunk(&mut png, *b"IDAT", compressed);
        append_chunk(&mut png, *b"IEND", &[]);

        let output = run_stdin(&["adapt-image", "-"], &png);
        assert_eq!(
            output.status.code(),
            Some(EXIT_SUCCESS),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let ir: Value = serde_json::from_slice(&output.stdout).expect("canonical IR JSON");
        assert_eq!(
            ir["extensions"]["org.sightlint.adapter.png"]["inflatedScanlineBytes"],
            expected_bytes(width, height, 8, 6, 0)
        );
    }
}

#[test]
fn accepts_exact_zlib_scanline_lengths_and_split_idat() {
    for (width, height, depth, color_type, interlace) in [
        (8, 2, 1, 0, 0),
        (3, 2, 8, 2, 0),
        (5, 3, 4, 3, 0),
        (2, 2, 16, 4, 0),
        (3, 2, 8, 6, 0),
        (1, 1, 8, 6, 1),
        (8, 8, 8, 6, 1),
    ] {
        let expected = expected_bytes(width, height, depth, color_type, interlace);
        let decoded = vec![0_u8; expected];
        let png = png_with_data(width, height, depth, color_type, interlace, &decoded, true);
        let output = run_stdin(&["adapt-image", "-"], &png);
        assert_eq!(
            output.status.code(),
            Some(EXIT_SUCCESS),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let ir: Value = serde_json::from_slice(&output.stdout).expect("canonical IR JSON");
        assert_eq!(
            ir["extensions"]["org.sightlint.adapter.png"]["inflatedScanlineBytes"],
            expected
        );
    }
}

#[test]
fn check_image_remains_byte_deterministic_after_inflation() {
    let expected = expected_bytes(16, 9, 8, 6, 0);
    let png = png_with_data(16, 9, 8, 6, 0, &vec![0_u8; expected], false);
    let first = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_eq!(first.status.code(), Some(EXIT_SUCCESS));
    for _ in 0..10 {
        let repeated = run_stdin(&["check-image", "-", "--format", "json"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert_eq!(first.stdout, repeated.stdout);
    }
}

#[test]
fn rejects_corrupt_zlib_and_adler32() {
    let expected = expected_bytes(3, 2, 8, 6, 0);
    let decoded = vec![0_u8; expected];

    let mut bad_deflate = header_prefix(3, 2, 8, 6, 0);
    append_chunk(&mut bad_deflate, *b"IDAT", &[0x78, 0x01, 0xff]);
    append_chunk(&mut bad_deflate, *b"IEND", &[]);
    assert_error_contains(&bad_deflate, "valid zlib stream");

    let mut compressed = zlib_stored(&decoded);
    let last = compressed.len() - 1;
    compressed[last] ^= 1;
    let mut bad_adler = header_prefix(3, 2, 8, 6, 0);
    append_chunk(&mut bad_adler, *b"IDAT", &compressed);
    append_chunk(&mut bad_adler, *b"IEND", &[]);
    assert_error_contains(&bad_adler, "Adler-32");
}

#[test]
fn rejects_shorter_and_longer_decoded_streams() {
    let expected = expected_bytes(3, 2, 8, 6, 0);
    let short = png_with_data(3, 2, 8, 6, 0, &vec![0_u8; expected - 1], false);
    assert_error_contains(&short, "requires exactly");

    let long = png_with_data(3, 2, 8, 6, 0, &vec![0_u8; expected + 1], false);
    assert_error_contains(&long, "expands beyond");
}

#[test]
fn rejects_oversized_decoded_raster_before_inflating() {
    let mut png = header_prefix(10_000, 7_000, 8, 6, 0);
    append_chunk(&mut png, *b"IDAT", &zlib_stored(&[]));
    append_chunk(&mut png, *b"IEND", &[]);
    assert_error_contains(&png, "decompressed scanline bytes");
}
