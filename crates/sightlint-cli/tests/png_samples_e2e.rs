//! Public-binary E2E coverage for deterministic PNG sample normalization.

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

#[derive(Debug, Clone, Copy)]
struct Header {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
}

#[derive(Debug, Clone, Copy, Default)]
struct Chunks<'a> {
    palette: Option<&'a [u8]>,
    transparency: Option<&'a [u8]>,
    transparency_before_palette: bool,
    transparency_after_idat: bool,
    duplicate_transparency: bool,
    include_color_description: bool,
}

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

fn extension(output: &Output) -> &Value {
    assert_eq!(
        output.status.code(),
        Some(EXIT_SUCCESS),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let document: Value =
        serde_json::from_slice(&output.stdout).expect("canonical Artifact IR JSON");
    Box::leak(Box::new(document))["extensions"]
        .get("org.sightlint.adapter.png")
        .expect("PNG extension")
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

fn build_png(header: Header, scanlines: &[u8], chunks: Chunks<'_>) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&header.width.to_be_bytes());
    ihdr.extend_from_slice(&header.height.to_be_bytes());
    ihdr.extend_from_slice(&[
        header.bit_depth,
        header.color_type,
        0,
        0,
        header.interlace,
    ]);
    append_chunk(&mut bytes, *b"IHDR", &ihdr);

    if chunks.include_color_description {
        append_chunk(&mut bytes, *b"gAMA", &45_455_u32.to_be_bytes());
        append_chunk(&mut bytes, *b"cHRM", &[0_u8; 32]);
        append_chunk(&mut bytes, *b"sRGB", &[0]);
        append_chunk(&mut bytes, *b"iCCP", b"profile\0\0compressed");
    }
    if chunks.transparency_before_palette {
        if let Some(transparency) = chunks.transparency {
            append_chunk(&mut bytes, *b"tRNS", transparency);
        }
    }
    if let Some(palette) = chunks.palette {
        append_chunk(&mut bytes, *b"PLTE", palette);
    }
    if !chunks.transparency_before_palette && !chunks.transparency_after_idat {
        if let Some(transparency) = chunks.transparency {
            append_chunk(&mut bytes, *b"tRNS", transparency);
            if chunks.duplicate_transparency {
                append_chunk(&mut bytes, *b"tRNS", transparency);
            }
        }
    }

    append_chunk(&mut bytes, *b"IDAT", &zlib_stored(scanlines));
    if chunks.transparency_after_idat {
        if let Some(transparency) = chunks.transparency {
            append_chunk(&mut bytes, *b"tRNS", transparency);
        }
    }
    append_chunk(&mut bytes, *b"IEND", &[]);
    bytes
}

fn channel_count(color_type: u8) -> usize {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!("test uses a valid PNG color type"),
    }
}

fn pack_samples(samples: &[u16], bit_depth: u8) -> Vec<u8> {
    match bit_depth {
        1 | 2 | 4 => {
            let depth = usize::from(bit_depth);
            let mut bytes = vec![0_u8; (samples.len() * depth).div_ceil(8)];
            let mask = (1_u16 << bit_depth) - 1;
            for (index, &sample) in samples.iter().enumerate() {
                assert!(sample <= mask);
                let bit_offset = index * depth;
                let shift = 8 - depth - (bit_offset % 8);
                bytes[bit_offset / 8] |=
                    u8::try_from(sample).expect("packed sample fits u8") << shift;
            }
            bytes
        }
        8 => samples
            .iter()
            .map(|&sample| u8::try_from(sample).expect("8-bit sample fits u8"))
            .collect(),
        16 => samples
            .iter()
            .flat_map(|sample| sample.to_be_bytes())
            .collect(),
        _ => unreachable!("test uses a valid PNG bit depth"),
    }
}

fn non_interlaced_scanlines(header: Header, rows: &[Vec<u16>]) -> Vec<u8> {
    assert_eq!(
        rows.len(),
        usize::try_from(header.height).expect("height fits usize")
    );
    let expected_samples = usize::try_from(header.width).expect("width fits usize")
        * channel_count(header.color_type);
    let mut scanlines = Vec::new();
    for row in rows {
        assert_eq!(row.len(), expected_samples);
        scanlines.push(0);
        scanlines.extend_from_slice(&pack_samples(row, header.bit_depth));
    }
    scanlines
}

fn quantize(sample: u16, bit_depth: u8) -> u8 {
    match bit_depth {
        1 | 2 | 4 => {
            let maximum = (1_u32 << bit_depth) - 1;
            u8::try_from(u32::from(sample) * 255 / maximum).expect("scaled sample fits u8")
        }
        8 => u8::try_from(sample).expect("sample fits u8"),
        16 => u8::try_from((u32::from(sample) * 255 + 32_767) / 65_535)
            .expect("quantized sample fits u8"),
        _ => unreachable!("valid PNG bit depth"),
    }
}

fn grayscale_rgba(samples: &[u16], bit_depth: u8, transparent: Option<u16>) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&sample| {
            let value = quantize(sample, bit_depth);
            let alpha = u8::from(transparent != Some(sample)) * 255;
            [value, value, value, alpha]
        })
        .collect()
}

fn assert_rgba_metadata(
    metadata: &Value,
    expected_rgba: &[u8],
    opaque: u64,
    transparent: u64,
    translucent: u64,
) {
    assert_eq!(metadata["pixelEncoding"], "pngEncodedRgba8");
    assert_eq!(metadata["colorManagementApplied"], false);
    assert_eq!(metadata["rgba8Bytes"], expected_rgba.len());
    assert_eq!(
        metadata["rgba8Crc32"],
        format!("{:08x}", crc32(expected_rgba))
    );
    assert_eq!(metadata["opaquePixelCount"], opaque);
    assert_eq!(metadata["transparentPixelCount"], transparent);
    assert_eq!(metadata["translucentPixelCount"], translucent);
}

#[test]
fn normalizes_every_grayscale_depth_with_documented_scaling() {
    for bit_depth in [1, 2, 4, 8, 16] {
        let maximum = if bit_depth == 16 {
            u16::MAX
        } else {
            (1_u16 << bit_depth) - 1
        };
        let samples = [0, maximum / 3, maximum / 2, maximum];
        let header = Header {
            width: 4,
            height: 1,
            bit_depth,
            color_type: 0,
            interlace: 0,
        };
        let scanlines = non_interlaced_scanlines(header, &[samples.to_vec()]);
        let input = build_png(header, &scanlines, Chunks::default());
        let output = run_stdin(&["adapt-image", "-"], &input);
        let metadata = extension(&output);
        assert_rgba_metadata(
            metadata,
            &grayscale_rgba(&samples, bit_depth, None),
            4,
            0,
            0,
        );
    }
}

#[test]
fn applies_grayscale_and_truecolor_keys_before_quantization() {
    let grayscale_samples = [0x1234, 0x1260];
    assert_eq!(quantize(grayscale_samples[0], 16), quantize(grayscale_samples[1], 16));
    let grayscale_header = Header {
        width: 2,
        height: 1,
        bit_depth: 16,
        color_type: 0,
        interlace: 0,
    };
    let grayscale_scanlines =
        non_interlaced_scanlines(grayscale_header, &[grayscale_samples.to_vec()]);
    let grayscale_key = grayscale_samples[1].to_be_bytes();
    let grayscale = build_png(
        grayscale_header,
        &grayscale_scanlines,
        Chunks {
            transparency: Some(&grayscale_key),
            ..Chunks::default()
        },
    );
    let grayscale_output = run_stdin(&["adapt-image", "-"], &grayscale);
    assert_rgba_metadata(
        extension(&grayscale_output),
        &grayscale_rgba(&grayscale_samples, 16, Some(grayscale_samples[1])),
        1,
        1,
        0,
    );

    let truecolor_header = Header {
        width: 2,
        height: 1,
        bit_depth: 8,
        color_type: 2,
        interlace: 0,
    };
    let truecolor_samples = vec![10, 20, 30, 10, 20, 31];
    let truecolor_scanlines =
        non_interlaced_scanlines(truecolor_header, &[truecolor_samples]);
    let key = [0, 10, 0, 20, 0, 31];
    let truecolor = build_png(
        truecolor_header,
        &truecolor_scanlines,
        Chunks {
            transparency: Some(&key),
            ..Chunks::default()
        },
    );
    let expected = [10, 20, 30, 255, 10, 20, 31, 0];
    let truecolor_output = run_stdin(&["adapt-image", "-"], &truecolor);
    assert_rgba_metadata(extension(&truecolor_output), &expected, 1, 1, 0);
}

#[test]
fn expands_palette_alpha_and_rejects_missing_entries() {
    let header = Header {
        width: 4,
        height: 1,
        bit_depth: 2,
        color_type: 3,
        interlace: 0,
    };
    let palette = [
        255, 0, 0, // red
        0, 255, 0, // green
        0, 0, 255, // blue
    ];
    let alpha = [0, 128];
    let scanlines = non_interlaced_scanlines(header, &[vec![0, 1, 2, 1]]);
    let input = build_png(
        header,
        &scanlines,
        Chunks {
            palette: Some(&palette),
            transparency: Some(&alpha),
            ..Chunks::default()
        },
    );
    let expected = [
        255, 0, 0, 0, 0, 255, 0, 128, 0, 0, 255, 255, 0, 255, 0, 128,
    ];
    let output = run_stdin(&["adapt-image", "-"], &input);
    let metadata = extension(&output);
    assert_rgba_metadata(metadata, &expected, 1, 1, 2);
    assert_eq!(metadata["paletteEntryCount"], 3);
    assert_eq!(metadata["transparencyEntryCount"], 2);

    let bad_scanlines = non_interlaced_scanlines(header, &[vec![3, 0, 0, 0]]);
    let bad = build_png(
        header,
        &bad_scanlines,
        Chunks {
            palette: Some(&palette),
            ..Chunks::default()
        },
    );
    assert_error_contains(&bad, "palette index 3 at pixel (0, 0)");
}

#[test]
fn normalizes_grayscale_alpha_and_rgba_sixteen_bit() {
    let gray_alpha_header = Header {
        width: 2,
        height: 1,
        bit_depth: 8,
        color_type: 4,
        interlace: 0,
    };
    let gray_alpha_scanlines =
        non_interlaced_scanlines(gray_alpha_header, &[vec![10, 0, 200, 127]]);
    let gray_alpha = build_png(gray_alpha_header, &gray_alpha_scanlines, Chunks::default());
    let gray_alpha_expected = [10, 10, 10, 0, 200, 200, 200, 127];
    let gray_alpha_output = run_stdin(&["adapt-image", "-"], &gray_alpha);
    assert_rgba_metadata(
        extension(&gray_alpha_output),
        &gray_alpha_expected,
        0,
        1,
        1,
    );

    let rgba_header = Header {
        width: 2,
        height: 1,
        bit_depth: 16,
        color_type: 6,
        interlace: 0,
    };
    let rgba_samples = vec![0, u16::MAX, 32_768, u16::MAX, u16::MAX, 0, 257, 32_768];
    let rgba_scanlines = non_interlaced_scanlines(rgba_header, &[rgba_samples]);
    let rgba = build_png(rgba_header, &rgba_scanlines, Chunks::default());
    let rgba_expected = [0, 255, 128, 255, 255, 0, 1, 128];
    let rgba_output = run_stdin(&["adapt-image", "-"], &rgba);
    assert_rgba_metadata(extension(&rgba_output), &rgba_expected, 1, 0, 1);
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn adam7_grayscale_scanlines(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
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
                let index = usize::try_from(y * width + x).expect("small raster index fits usize");
                scanlines.push(pixels[index]);
            }
        }
    }
    scanlines
}

#[test]
fn scatters_all_adam7_passes_to_final_row_major_rgba() {
    let header = Header {
        width: 8,
        height: 8,
        bit_depth: 8,
        color_type: 0,
        interlace: 1,
    };
    let pixels: Vec<u8> = (0_u8..64).map(|value| value.wrapping_mul(3)).collect();
    let scanlines = adam7_grayscale_scanlines(header.width, header.height, &pixels);
    let input = build_png(header, &scanlines, Chunks::default());
    let expected: Vec<u8> = pixels
        .iter()
        .flat_map(|&value| [value, value, value, 255])
        .collect();
    let output = run_stdin(&["adapt-image", "-"], &input);
    let metadata = extension(&output);
    assert_rgba_metadata(metadata, &expected, 64, 0, 0);
    assert_eq!(metadata["activePassCount"], 7);
}

#[test]
fn rejects_invalid_transparency_contracts() {
    let gray_header = Header {
        width: 1,
        height: 1,
        bit_depth: 4,
        color_type: 0,
        interlace: 0,
    };
    let gray_scanlines = non_interlaced_scanlines(gray_header, &[vec![0]]);

    let duplicate = build_png(
        gray_header,
        &gray_scanlines,
        Chunks {
            transparency: Some(&[0, 0]),
            duplicate_transparency: true,
            ..Chunks::default()
        },
    );
    assert_error_contains(&duplicate, "more than one tRNS");

    let after_idat = build_png(
        gray_header,
        &gray_scanlines,
        Chunks {
            transparency: Some(&[0, 0]),
            transparency_after_idat: true,
            ..Chunks::default()
        },
    );
    assert_error_contains(&after_idat, "tRNS chunk must appear before");

    let malformed = build_png(
        gray_header,
        &gray_scanlines,
        Chunks {
            transparency: Some(&[0]),
            ..Chunks::default()
        },
    );
    assert_error_contains(&malformed, "tRNS length 1 is invalid");

    let out_of_range = build_png(
        gray_header,
        &gray_scanlines,
        Chunks {
            transparency: Some(&[0, 16]),
            ..Chunks::default()
        },
    );
    assert_error_contains(&out_of_range, "exceeds the 4-bit source range");

    let indexed_header = Header {
        width: 1,
        height: 1,
        bit_depth: 1,
        color_type: 3,
        interlace: 0,
    };
    let indexed_scanlines = non_interlaced_scanlines(indexed_header, &[vec![0]]);
    let before_palette = build_png(
        indexed_header,
        &indexed_scanlines,
        Chunks {
            palette: Some(&[0, 0, 0]),
            transparency: Some(&[0]),
            transparency_before_palette: true,
            ..Chunks::default()
        },
    );
    assert_error_contains(&before_palette, "must appear after PLTE");

    let rgba_header = Header {
        width: 1,
        height: 1,
        bit_depth: 8,
        color_type: 6,
        interlace: 0,
    };
    let rgba_scanlines = non_interlaced_scanlines(rgba_header, &[vec![0, 0, 0, 255]]);
    let forbidden = build_png(
        rgba_header,
        &rgba_scanlines,
        Chunks {
            transparency: Some(&[0, 0]),
            ..Chunks::default()
        },
    );
    assert_error_contains(&forbidden, "includes alpha and must not contain tRNS");
}

#[test]
fn reports_unapplied_color_description_and_remains_deterministic() {
    let header = Header {
        width: 3,
        height: 2,
        bit_depth: 8,
        color_type: 2,
        interlace: 0,
    };
    let rows = vec![
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
        vec![10, 11, 12, 13, 14, 15, 16, 17, 18],
    ];
    let scanlines = non_interlaced_scanlines(header, &rows);
    let input = build_png(
        header,
        &scanlines,
        Chunks {
            include_color_description: true,
            ..Chunks::default()
        },
    );
    let first_ir = run_stdin(&["adapt-image", "-"], &input);
    let first_report = run_stdin(&["check-image", "-", "--format", "json"], &input);
    let metadata = extension(&first_ir);
    assert_eq!(metadata["unappliedColorDescriptionPresent"], true);
    assert_eq!(metadata["colorDescription"]["srgb"], true);
    assert_eq!(metadata["colorDescription"]["gamma"], true);
    assert_eq!(metadata["colorDescription"]["chromaticities"], true);
    assert_eq!(metadata["colorDescription"]["iccProfile"], true);

    assert_eq!(first_report.status.code(), Some(EXIT_SUCCESS));
    for _ in 0..10 {
        let repeated_ir = run_stdin(&["adapt-image", "-"], &input);
        let repeated_report = run_stdin(&["check-image", "-", "--format", "json"], &input);
        assert_eq!(first_ir.stdout, repeated_ir.stdout);
        assert_eq!(first_report.stdout, repeated_report.stdout);
    }
}

#[test]
fn rejects_rgba8_output_above_the_normalization_budget() {
    let header = Header {
        width: 8_192,
        height: 4_097,
        bit_depth: 1,
        color_type: 0,
        interlace: 0,
    };
    let row_bytes = usize::try_from(header.width / 8).expect("row width fits usize");
    let mut scanlines = Vec::with_capacity(
        usize::try_from(header.height).expect("height fits usize") * (1 + row_bytes),
    );
    for _ in 0..header.height {
        scanlines.push(0);
        scanlines.resize(scanlines.len() + row_bytes, 0);
    }
    let input = build_png(header, &scanlines, Chunks::default());
    assert_error_contains(&input, "RGBA8 bytes");
}
