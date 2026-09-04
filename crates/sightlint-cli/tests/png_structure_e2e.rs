//! Public-binary E2E coverage for complete PNG chunk-stream validation.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 2;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
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

fn assert_error_contains(input: &[u8], expected: &str) {
    let output = run_stdin(input);
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

fn append_chunk(bytes: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let length = u32::try_from(data.len()).expect("test chunk length fits u32");
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(data);
    let crc_start = bytes.len() - data.len() - 4;
    let crc = crc32(&bytes[crc_start..]);
    bytes.extend_from_slice(&crc.to_be_bytes());
}

fn png_prefix(bit_depth: u8, color_type: u8) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&8_u32.to_be_bytes());
    ihdr.extend_from_slice(&6_u32.to_be_bytes());
    ihdr.extend_from_slice(&[bit_depth, color_type, 0, 0, 0]);
    append_chunk(&mut bytes, *b"IHDR", &ihdr);
    bytes
}

fn rgba_png() -> Vec<u8> {
    let mut bytes = png_prefix(8, 6);
    append_chunk(&mut bytes, *b"IDAT", &[1, 2, 3]);
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

#[test]
fn complete_stream_metadata_is_exposed_deterministically() {
    let mut png = png_prefix(8, 3);
    append_chunk(&mut png, *b"PLTE", &[0, 0, 0, 255, 255, 255]);
    append_chunk(&mut png, *b"IDAT", &[1, 2]);
    append_chunk(&mut png, *b"IDAT", &[3, 4, 5]);
    append_chunk(&mut png, *b"IEND", &[]);

    let output = run_stdin(&png);
    assert_eq!(output.status.code(), Some(EXIT_SUCCESS));
    assert!(output.stderr.is_empty());
    let ir: Value = serde_json::from_slice(&output.stdout).expect("canonical IR JSON");
    let metadata = &ir["extensions"]["org.sightlint.adapter.png"];
    assert_eq!(metadata["chunkCount"], 5);
    assert_eq!(metadata["idatChunkCount"], 2);
    assert_eq!(metadata["idatBytes"], 5);
    assert_eq!(metadata["hasPalette"], true);

    let repeated = run_stdin(&png);
    assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
    assert_eq!(output.stdout, repeated.stdout);
}

#[test]
fn rejects_missing_or_invalid_termination() {
    let mut missing_iend = png_prefix(8, 6);
    append_chunk(&mut missing_iend, *b"IDAT", &[]);
    assert_error_contains(&missing_iend, "missing the terminating IEND");

    let mut trailing = rgba_png();
    trailing.push(0);
    assert_error_contains(&trailing, "trailing bytes after IEND");

    let mut nonempty_iend = png_prefix(8, 6);
    append_chunk(&mut nonempty_iend, *b"IDAT", &[]);
    append_chunk(&mut nonempty_iend, *b"IEND", &[0]);
    assert_error_contains(&nonempty_iend, "IEND chunk must have zero length");
}

#[test]
fn rejects_missing_or_nonconsecutive_image_data() {
    let mut no_idat = png_prefix(8, 6);
    append_chunk(&mut no_idat, *b"IEND", &[]);
    assert_error_contains(&no_idat, "at least one IDAT");

    let mut split_idat = png_prefix(8, 6);
    append_chunk(&mut split_idat, *b"IDAT", &[]);
    append_chunk(&mut split_idat, *b"tEXt", &[]);
    append_chunk(&mut split_idat, *b"IDAT", &[]);
    append_chunk(&mut split_idat, *b"IEND", &[]);
    assert_error_contains(&split_idat, "IDAT chunks must be consecutive");
}

#[test]
fn rejects_palette_contract_violations() {
    let mut indexed_without_palette = png_prefix(8, 3);
    append_chunk(&mut indexed_without_palette, *b"IDAT", &[]);
    append_chunk(&mut indexed_without_palette, *b"IEND", &[]);
    assert_error_contains(&indexed_without_palette, "requires a PLTE chunk");

    let mut grayscale_with_palette = png_prefix(8, 0);
    append_chunk(&mut grayscale_with_palette, *b"PLTE", &[0, 0, 0]);
    append_chunk(&mut grayscale_with_palette, *b"IDAT", &[]);
    append_chunk(&mut grayscale_with_palette, *b"IEND", &[]);
    assert_error_contains(&grayscale_with_palette, "must not contain a PLTE");

    let mut bad_palette_length = png_prefix(1, 3);
    append_chunk(&mut bad_palette_length, *b"PLTE", &[0, 0, 0, 1, 1]);
    append_chunk(&mut bad_palette_length, *b"IDAT", &[]);
    append_chunk(&mut bad_palette_length, *b"IEND", &[]);
    assert_error_contains(&bad_palette_length, "PLTE length");
}

#[test]
fn rejects_unknown_critical_chunks_and_later_crc_damage() {
    let mut unknown = png_prefix(8, 6);
    append_chunk(&mut unknown, *b"ABCD", &[]);
    append_chunk(&mut unknown, *b"IDAT", &[]);
    append_chunk(&mut unknown, *b"IEND", &[]);
    assert_error_contains(&unknown, "unknown critical chunk ABCD");

    let mut bad_crc = rgba_png();
    // IHDR is 33 bytes total. The following IDAT is length + type + 3 data + CRC;
    // flip the final CRC byte without touching IHDR so the later-chunk path is exercised.
    let idat_crc_last = 33 + 4 + 4 + 3 + 3;
    bad_crc[idat_crc_last] ^= 1;
    assert_error_contains(&bad_crc, "IDAT chunk CRC-32");
}

#[test]
fn exact_chunk_budget_boundary_is_enforced() {
    let mut at_limit = png_prefix(8, 6);
    for _ in 0..9_997 {
        append_chunk(&mut at_limit, *b"tEXt", &[]);
    }
    append_chunk(&mut at_limit, *b"IDAT", &[]);
    append_chunk(&mut at_limit, *b"IEND", &[]);
    let accepted = run_stdin(&at_limit);
    assert_eq!(accepted.status.code(), Some(EXIT_SUCCESS));

    let mut above_limit = png_prefix(8, 6);
    for _ in 0..9_998 {
        append_chunk(&mut above_limit, *b"tEXt", &[]);
    }
    append_chunk(&mut above_limit, *b"IDAT", &[]);
    append_chunk(&mut above_limit, *b"IEND", &[]);
    assert_error_contains(&above_limit, "10000-chunk safety limit");
}
