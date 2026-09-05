//! Public-binary E2E coverage for deterministic PNG filter reconstruction.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 2;

// Derived independently with a Python reference implementation from five RGBA rows using
// None, Sub, Up, Average, and Paeth respectively. The expected reconstructed CRC-32 is 36ed567a.
const ALL_FILTERS_RGBA_2X5: &[u8] = &[
    0x00, 0x0a, 0x14, 0x1e, 0x28, 0x0f, 0x19, 0x23, 0x2d, 0x01, 0x0c, 0x16, 0x20, 0x2a,
    0x06, 0x06, 0x06, 0x06, 0x02, 0x58, 0x44, 0x30, 0x1c, 0x5c, 0x48, 0x34, 0x20, 0x03,
    0xd3, 0xdd, 0xe7, 0xf1, 0xe0, 0xe7, 0xef, 0xf6, 0x04, 0xc3, 0x8c, 0x55, 0x1e, 0x0a,
    0x0a, 0x0a, 0x0a,
];

// Independently derived Adam7 transmission stream for an 8x8, 8-bit grayscale image. Every
// non-empty pass uses Up, so the first row of each pass detects failure to reset the prior row.
// The reconstructed pass-order data CRC-32 is 690a7c66.
const ADAM7_UP_GRAYSCALE_8X8: &[u8] = &[
    0x02, 0x1d, 0x02, 0x3a, 0x02, 0x57, 0x5a, 0x02, 0x74, 0x77, 0x02, 0x07, 0x07, 0x02,
    0x91, 0x94, 0x97, 0x9a, 0x02, 0x07, 0x07, 0x07, 0x07, 0x02, 0xae, 0xb1, 0xb4, 0xb7,
    0x02, 0x07, 0x07, 0x07, 0x07, 0x02, 0x07, 0x07, 0x07, 0x07, 0x02, 0x07, 0x07, 0x07,
    0x07, 0x02, 0xcb, 0xce, 0xd1, 0xd4, 0xd7, 0xda, 0xdd, 0xe0, 0x02, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x02, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x02, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
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
    assert_eq!(
        output.status.code(),
        Some(EXIT_ERROR),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

fn png_with_filtered_data(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
    filtered_data: &[u8],
    split_idat: bool,
) -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[bit_depth, color_type, 0, 0, interlace]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    if color_type == 3 {
        append_chunk(&mut png, *b"PLTE", &[0, 0, 0, 255, 255, 255]);
    }

    let compressed = zlib_stored(filtered_data);
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

fn metadata(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let ir: Value = serde_json::from_slice(&output.stdout).expect("canonical IR JSON");
    ir["extensions"]["org.sightlint.adapter.png"].clone()
}

#[test]
fn reconstructs_all_five_filters_and_exposes_exact_metadata() {
    let png = png_with_filtered_data(2, 5, 8, 6, 0, ALL_FILTERS_RGBA_2X5, true);
    let output = run_stdin(&["adapt-image", "-"], &png);
    let png_metadata = metadata(&output);

    assert_eq!(png_metadata["version"], "0.2.0");
    assert_eq!(png_metadata["inflatedScanlineBytes"], 45);
    assert_eq!(png_metadata["reconstructedDataBytes"], 40);
    assert_eq!(png_metadata["reconstructedDataCrc32"], "36ed567a");
    assert_eq!(png_metadata["scanlineCount"], 5);
    assert_eq!(png_metadata["nonEmptyPassCount"], 1);
    for filter in ["none", "sub", "up", "average", "paeth"] {
        assert_eq!(png_metadata["filterCounts"][filter], 1);
    }

    let repeated = run_stdin(&["adapt-image", "-"], &png);
    assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
    assert_eq!(output.stdout, repeated.stdout);
    assert_eq!(output.stderr, repeated.stderr);
}

#[test]
fn packed_sub_byte_samples_use_one_filter_byte_per_pixel_group() {
    let png = png_with_filtered_data(16, 1, 1, 0, 0, &[1, 0xaa, 0x22], false);
    let output = run_stdin(&["adapt-image", "-"], &png);
    let png_metadata = metadata(&output);

    assert_eq!(png_metadata["reconstructedDataBytes"], 2);
    assert_eq!(png_metadata["reconstructedDataCrc32"], "87e3c807");
    assert_eq!(png_metadata["filterCounts"]["sub"], 1);
}

#[test]
fn adam7_resets_prior_rows_between_passes_and_skips_empty_passes() {
    let png = png_with_filtered_data(8, 8, 8, 0, 1, ADAM7_UP_GRAYSCALE_8X8, true);
    let output = run_stdin(&["adapt-image", "-"], &png);
    let png_metadata = metadata(&output);

    assert_eq!(png_metadata["reconstructedDataBytes"], 64);
    assert_eq!(png_metadata["reconstructedDataCrc32"], "690a7c66");
    assert_eq!(png_metadata["scanlineCount"], 15);
    assert_eq!(png_metadata["nonEmptyPassCount"], 7);
    assert_eq!(png_metadata["filterCounts"]["up"], 15);

    let one_pixel = png_with_filtered_data(1, 1, 8, 6, 1, &[0, 1, 2, 3, 4], false);
    let one_pixel_output = run_stdin(&["adapt-image", "-"], &one_pixel);
    let one_pixel_metadata = metadata(&one_pixel_output);
    assert_eq!(one_pixel_metadata["reconstructedDataBytes"], 4);
    assert_eq!(one_pixel_metadata["reconstructedDataCrc32"], "b63cfbcd");
    assert_eq!(one_pixel_metadata["scanlineCount"], 1);
    assert_eq!(one_pixel_metadata["nonEmptyPassCount"], 1);
}

#[test]
fn invalid_filter_byte_is_rejected_after_successful_inflation() {
    let non_interlaced = png_with_filtered_data(1, 1, 8, 6, 0, &[5, 0, 0, 0, 0], false);
    assert_error_contains(&non_interlaced, "filter type 5 is invalid at pass 0, row 0");

    let adam7 = png_with_filtered_data(1, 1, 8, 6, 1, &[5, 0, 0, 0, 0], false);
    assert_error_contains(&adam7, "filter type 5 is invalid at pass 1, row 0");
}

#[test]
fn check_image_report_remains_byte_deterministic_after_reconstruction() {
    let png = png_with_filtered_data(2, 5, 8, 6, 0, ALL_FILTERS_RGBA_2X5, true);
    let first = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_eq!(first.status.code(), Some(EXIT_SUCCESS));
    assert!(first.stderr.is_empty());

    for _ in 0..10 {
        let repeated = run_stdin(&["check-image", "-", "--format", "json"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert_eq!(first.stdout, repeated.stdout);
        assert_eq!(first.stderr, repeated.stderr);
    }
}
