//! Public-binary E2E coverage for deterministic PNG filter reconstruction.

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

fn png_with_scanlines(
    width: u32,
    height: u32,
    depth: u8,
    color_type: u8,
    interlace: u8,
    scanlines: &[u8],
    split_idat: bool,
) -> Vec<u8> {
    let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[depth, color_type, 0, 0, interlace]);
    append_chunk(&mut png, *b"IHDR", &ihdr);
    if color_type == 3 {
        append_chunk(&mut png, *b"PLTE", &[0, 0, 0]);
    }

    let compressed = zlib_stored(scanlines);
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

fn reference_average(left: u8, up: u8) -> u8 {
    // Independent widened arithmetic: the sum is at most 510.
    let sum: u16 = [left, up].map(u16::from).iter().sum();
    u8::try_from(sum / 2).expect("byte average")
}

fn reference_paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(upper_left);
    let candidates = [left, up, upper_left];
    let distances = candidates.map(|candidate| (estimate - i16::from(candidate)).abs());
    let mut selected = 0_usize;
    if distances[1] < distances[selected] {
        selected = 1;
    }
    if distances[2] < distances[selected] {
        selected = 2;
    }
    candidates[selected]
}

fn encode_row(filter: u8, current: &[u8], previous: Option<&[u8]>, bpp: usize) -> Vec<u8> {
    current
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            let left = if index >= bpp {
                current[index - bpp]
            } else {
                0
            };
            let up = previous.map_or(0, |row| row[index]);
            let upper_left =
                previous.map_or(0, |row| if index >= bpp { row[index - bpp] } else { 0 });
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => reference_average(left, up),
                4 => reference_paeth(left, up, upper_left),
                _ => unreachable!("test filter range"),
            };
            value.wrapping_sub(predictor)
        })
        .collect()
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn all_filters_png() -> Vec<u8> {
    let rows: Vec<Vec<u8>> = (0..5)
        .map(|row| {
            (0..9)
                .map(|byte| u8::try_from((row * 61 + byte * 29 + 241) % 256).expect("test byte"))
                .collect()
        })
        .collect();
    let mut scanlines = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let filter = u8::try_from(index).expect("filter fits u8");
        let previous = index
            .checked_sub(1)
            .map(|previous| rows[previous].as_slice());
        scanlines.push(filter);
        scanlines.extend_from_slice(&encode_row(filter, row, previous, 3));
    }
    png_with_scanlines(3, 5, 8, 2, 0, &scanlines, true)
}

fn adam7_png() -> Vec<u8> {
    let mut scanlines = Vec::new();
    for (index, (start_x, start_y, step_x, step_y)) in ADAM7_PASSES.into_iter().enumerate() {
        let width = pass_extent(8, start_x, step_x);
        let height = pass_extent(8, start_y, step_y);
        if width == 0 || height == 0 {
            continue;
        }
        let row_bytes = usize::try_from(width * 4).expect("small row fits usize");
        let mut previous: Option<Vec<u8>> = None;
        for row_index in 0..height {
            let row: Vec<u8> = (0..row_bytes)
                .map(|byte| {
                    let seed = (index + 1) * 43
                        + usize::try_from(row_index).expect("row fits usize") * 23
                        + byte * 7;
                    u8::try_from(seed % 256).expect("test byte")
                })
                .collect();
            let filter = if row_index == 0 { 2 } else { 4 };
            scanlines.push(filter);
            scanlines.extend_from_slice(&encode_row(filter, &row, previous.as_deref(), 4));
            previous = Some(row);
        }
    }
    png_with_scanlines(8, 8, 8, 6, 1, &scanlines, true)
}

#[test]
fn all_five_filters_flow_through_the_public_adapter_and_metadata() {
    let png = all_filters_png();
    let first = run_stdin(&["adapt-image", "-"], &png);
    assert_eq!(
        first.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    let ir: Value = serde_json::from_slice(&first.stdout).expect("canonical IR JSON");
    let metadata = &ir["extensions"]["org.sightlint.adapter.png"];
    assert_eq!(metadata["inflatedScanlineBytes"], 50);
    assert_eq!(metadata["reconstructedPackedSampleBytes"], 45);
    assert_eq!(metadata["nonEmptyPassCount"], 1);
    assert!(ir["nodes"][0]["geometry"].get("inkBox").is_none());
    assert!(ir["nodes"][0].get("role").is_none());

    for _ in 0..10 {
        let repeated = run_stdin(&["adapt-image", "-"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert!(repeated.stderr.is_empty());
        assert_eq!(first.stdout, repeated.stdout);
    }
}

#[test]
fn invalid_filter_is_a_stable_public_input_error() {
    let scanlines = [0_u8, 1, 2, 5, 3, 4];
    let png = png_with_scanlines(2, 2, 8, 0, 0, &scanlines, false);
    for command in ["adapt-image", "check-image"] {
        let output = run_stdin(&[command, "-"], &png);
        assert_eq!(output.status.code(), Some(EXIT_ERROR));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("pass 1 row 2"), "{stderr}");
        assert!(stderr.contains("invalid filter type 5"), "{stderr}");
    }
}

#[test]
fn adam7_pass_reconstruction_remains_deterministic_through_check_image() {
    let png = adam7_png();
    let first = run_stdin(&["check-image", "-", "--format", "json"], &png);
    assert_eq!(
        first.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    let report: Value = serde_json::from_slice(&first.stdout).expect("JSON report");
    // CheckReport has always exposed `results`; `findings` was never its wire contract.
    assert_eq!(report["reportSchemaVersion"], "0.3.0");
    assert_eq!(report["artifactKind"], "image");
    assert_eq!(report["summary"]["failed"], 0);
    assert!(
        !report["results"]
            .as_array()
            .expect("rule results")
            .is_empty()
    );

    let adapted = run_stdin(&["adapt-image", "-"], &png);
    assert_eq!(adapted.status.code(), Some(EXIT_SUCCESS));
    assert!(adapted.stderr.is_empty());
    let ir: Value = serde_json::from_slice(&adapted.stdout).expect("canonical IR JSON");
    assert_eq!(
        ir["extensions"]["org.sightlint.adapter.png"]["nonEmptyPassCount"],
        7
    );
    let via_ir = run_stdin(&["check", "-", "--format", "json"], &adapted.stdout);
    assert_eq!(via_ir.status.code(), Some(EXIT_SUCCESS));
    assert!(via_ir.stderr.is_empty());
    assert_eq!(first.stdout, via_ir.stdout);

    for _ in 0..10 {
        let repeated = run_stdin(&["check-image", "-", "--format", "json"], &png);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS));
        assert!(repeated.stderr.is_empty());
        assert_eq!(first.stdout, repeated.stdout);
    }
}
