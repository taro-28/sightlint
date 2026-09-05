//! Public-binary E2E coverage for zlib stream-consumption boundaries across PNG `IDAT` chunks.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 2;

fn run_stdin(input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sightlint"))
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
    let length = u16::try_from(bytes.len()).expect("small stored test block fits u16");
    let mut output = vec![0x78, 0x01, 0x01];
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(&(!length).to_le_bytes());
    output.extend_from_slice(bytes);
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

fn png_prefix() -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    png
}

fn assert_error_contains(input: &[u8], expected: &str) {
    let output = run_stdin(input);
    assert_eq!(output.status.code(), Some(EXIT_ERROR));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected {expected:?} in {stderr:?}"
    );
}

#[test]
fn accepts_adler32_split_into_a_following_idat() {
    let compressed = zlib_stored(&[0_u8; 5]);
    let split = compressed.len() - 4;
    let mut png = png_prefix();
    append_chunk(&mut png, *b"IDAT", &compressed[..split]);
    append_chunk(&mut png, *b"IDAT", &compressed[split..]);
    append_chunk(&mut png, *b"IEND", &[]);

    let output = run_stdin(&png);
    assert_eq!(
        output.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn accepts_empty_idat_after_complete_zlib_stream() {
    let compressed = zlib_stored(&[0_u8; 5]);
    let mut png = png_prefix();
    append_chunk(&mut png, *b"IDAT", &compressed);
    append_chunk(&mut png, *b"IDAT", &[]);
    append_chunk(&mut png, *b"IEND", &[]);

    let output = run_stdin(&png);
    assert_eq!(output.status.code(), Some(EXIT_SUCCESS));
}

#[test]
fn rejects_bytes_after_zlib_terminator_in_same_idat() {
    let mut compressed = zlib_stored(&[0_u8; 5]);
    compressed.push(0xaa);
    let mut png = png_prefix();
    append_chunk(&mut png, *b"IDAT", &compressed);
    append_chunk(&mut png, *b"IEND", &[]);
    assert_error_contains(&png, "bytes after the complete zlib stream");
}

#[test]
fn rejects_nonempty_idat_after_complete_zlib_stream() {
    let compressed = zlib_stored(&[0_u8; 5]);
    let mut png = png_prefix();
    append_chunk(&mut png, *b"IDAT", &compressed);
    append_chunk(&mut png, *b"IDAT", &[0xaa]);
    append_chunk(&mut png, *b"IEND", &[]);
    assert_error_contains(&png, "bytes after the complete zlib stream");
}
