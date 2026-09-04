//! End-to-end tests for deterministic PNG adaptation through the public `sightlint` binary.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_FINDINGS: i32 = 1;
const EXIT_ERROR: i32 = 2;

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
        .expect("failed to write binary test input");
    child.wait_with_output().expect("failed to collect output")
}

fn assert_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn parse_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
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

fn png_header(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    bytes.extend_from_slice(&13_u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[bit_depth, color_type, compression, filter, interlace]);
    let crc = crc32(&bytes[12..29]);
    bytes.extend_from_slice(&crc.to_be_bytes());
    if color_type == 3 {
        append_chunk(&mut bytes, *b"PLTE", &[0, 0, 0]);
    }
    append_chunk(&mut bytes, *b"IDAT", &[]);
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

#[test]
fn adapt_image_emits_exact_canonical_ir_without_invented_semantics() {
    let png = png_header(320, 200, 8, 6, 0, 0, 0);
    let output = run_stdin(&["adapt-image", "-"], &png);
    assert_code(&output, EXIT_SUCCESS);
    assert!(output.stderr.is_empty());

    let ir = parse_stdout(&output);
    assert_eq!(ir["artifact"]["kind"], "image");
    assert_eq!(ir["canvases"][0]["size"]["width"], 320.0);
    assert_eq!(ir["canvases"][0]["size"]["height"], 200.0);
    assert_eq!(ir["canvases"][0]["unit"], "devicePixel");
    assert_eq!(ir["nodes"][0]["kind"]["value"], "image");
    assert_eq!(
        ir["nodes"][0]["geometry"]["renderBox"]["rect"]["width"],
        320.0
    );
    assert!(ir["nodes"][0]["geometry"].get("inkBox").is_none());
    assert!(ir["nodes"][0].get("role").is_none());
    assert!(ir["nodes"][0].get("name").is_none());
    assert_eq!(ir["evidence"][0]["class"], "exactSource");
    assert_eq!(
        ir["evidence"][0]["source"]["adapter"],
        "sightlint-adapter-png"
    );
    assert_eq!(ir["evidence"][0]["source"]["externalProcessing"], false);
    assert_eq!(ir["extensions"]["org.sightlint.adapter.png"]["bitDepth"], 8);
    assert_eq!(
        ir["extensions"]["org.sightlint.adapter.png"]["colorType"],
        6
    );
}

#[test]
fn check_image_runs_adapter_ir_engine_and_json_report_in_one_command() {
    let png = png_header(64, 32, 8, 2, 0, 0, 0);
    let output = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_code(&output, EXIT_SUCCESS);
    let report = parse_stdout(&output);
    assert_eq!(report["artifactId"], "artifact");
    assert_eq!(report["artifactKind"], "image");
    assert_eq!(report["summary"]["failed"], 0);
    assert_eq!(report["summary"]["cantTell"], 0);
}

#[test]
fn strict_check_image_only_fails_when_engine_reports_cant_tell() {
    let png = png_header(64, 32, 8, 2, 0, 0, 0);
    let advisory = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_code(&advisory, EXIT_SUCCESS);

    let strict = run_stdin(
        &["check-image", "-", "--format", "json", "--deny-cant-tell"],
        &png,
    );
    // Header-only image IR has exact full-raster geometry and no ambiguous declared relation.
    assert_code(&strict, EXIT_SUCCESS);
    assert_eq!(advisory.stdout, strict.stdout);
    assert_ne!(EXIT_FINDINGS, EXIT_ERROR);
}

#[test]
fn adapter_and_report_outputs_are_byte_identical_across_repeated_runs() {
    let png = png_header(111, 73, 8, 6, 0, 0, 1);
    let expected_ir = run_stdin(&["adapt-image", "-"], &png);
    assert_code(&expected_ir, EXIT_SUCCESS);
    let expected_report = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_code(&expected_report, EXIT_SUCCESS);

    for iteration in 0..20 {
        let ir = run_stdin(&["adapt-image", "-"], &png);
        assert_code(&ir, EXIT_SUCCESS);
        assert_eq!(expected_ir.stdout, ir.stdout, "IR run {iteration} differed");

        let report = run_stdin(&["check-image", "-", "--format", "json"], &png);
        assert_code(&report, EXIT_SUCCESS);
        assert_eq!(
            expected_report.stdout, report.stdout,
            "report run {iteration} differed"
        );
    }
}

#[test]
fn valid_png_header_variants_are_accepted_by_the_public_binary() {
    for (bit_depth, color_type, interlace) in [
        (1, 0, 0),
        (8, 0, 0),
        (8, 2, 0),
        (4, 3, 0),
        (16, 4, 0),
        (8, 6, 0),
        (8, 6, 1),
    ] {
        let png = png_header(2, 3, bit_depth, color_type, 0, 0, interlace);
        let output = run_stdin(&["adapt-image", "-"], &png);
        assert_code(&output, EXIT_SUCCESS);
        let ir = parse_stdout(&output);
        assert_eq!(
            ir["extensions"]["org.sightlint.adapter.png"]["bitDepth"],
            bit_depth
        );
        assert_eq!(
            ir["extensions"]["org.sightlint.adapter.png"]["colorType"],
            color_type
        );
        assert_eq!(
            ir["extensions"]["org.sightlint.adapter.png"]["interlaceMethod"],
            interlace
        );
    }
}

#[test]
fn malformed_png_headers_fail_before_ir_or_rules_are_emitted() {
    let mut wrong_signature = png_header(1, 1, 8, 6, 0, 0, 0);
    wrong_signature[0] = 0;

    let truncated = png_header(1, 1, 8, 6, 0, 0, 0)[..20].to_vec();

    let mut wrong_chunk = png_header(1, 1, 8, 6, 0, 0, 0);
    wrong_chunk[12..16].copy_from_slice(b"PLTE");
    let crc = crc32(&wrong_chunk[12..29]);
    wrong_chunk[29..33].copy_from_slice(&crc.to_be_bytes());

    let mut wrong_length = png_header(1, 1, 8, 6, 0, 0, 0);
    wrong_length[8..12].copy_from_slice(&12_u32.to_be_bytes());

    let mut wrong_crc = png_header(1, 1, 8, 6, 0, 0, 0);
    wrong_crc[32] ^= 1;

    for (input, expected) in [
        (wrong_signature, "PNG signature"),
        (truncated, "truncated"),
        (wrong_chunk, "first chunk must be IHDR"),
        (wrong_length, "IHDR length must be 13"),
        (wrong_crc, "CRC-32"),
    ] {
        let output = run_stdin(&["adapt-image", "-"], &input);
        assert_code(&output, EXIT_ERROR);
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "expected {expected}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn invalid_png_field_matrix_and_resource_limits_are_rejected() {
    for (input, expected) in [
        (png_header(0, 1, 8, 6, 0, 0, 0), "non-zero"),
        (
            png_header(100_001, 1, 8, 6, 0, 0, 0),
            "per-axis safety limit",
        ),
        (
            png_header(20_000, 20_000, 8, 6, 0, 0, 0),
            "pixel safety limit",
        ),
        (png_header(1, 1, 8, 1, 0, 0, 0), "color type 1"),
        (
            png_header(1, 1, 4, 6, 0, 0, 0),
            "bit depth 4 is invalid for color type 6",
        ),
        (png_header(1, 1, 8, 6, 1, 0, 0), "compression method 1"),
        (png_header(1, 1, 8, 6, 0, 1, 0), "filter method 1"),
        (png_header(1, 1, 8, 6, 0, 0, 2), "interlace method 2"),
    ] {
        let output = run_stdin(&["check-image", "-", "--format", "json"], &input);
        assert_code(&output, EXIT_ERROR);
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "expected {expected}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
