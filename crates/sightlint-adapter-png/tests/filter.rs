//! Integration tests for deterministic PNG scanline filter reconstruction.

use sightlint_adapter_png::{
    PngAdapterError, PngFilterError, reconstruct_png_scanlines,
};

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
struct PredictorCase {
    width: u32,
    depth: u8,
    color_type: u8,
    expected_bpp: usize,
    expected_row_bytes: usize,
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
            let upper_left = previous.map_or(0, |row| {
                if index >= bpp {
                    row[index - bpp]
                } else {
                    0
                }
            });
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => u8::try_from((u16::from(left) + u16::from(up)) / 2)
                    .expect("byte average"),
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

#[test]
fn reconstructs_all_filter_types_from_independently_encoded_rows() {
    let rows = [
        vec![3, 250, 17, 99, 4, 201, 8, 64, 155],
        vec![240, 2, 33, 110, 220, 7, 44, 180, 19],
        vec![8, 15, 200, 77, 6, 250, 91, 13, 128],
        vec![255, 0, 1, 2, 3, 4, 5, 6, 7],
        vec![19, 89, 149, 209, 12, 72, 132, 192, 252],
    ];
    let filters = [0_u8, 1, 2, 3, 4];
    let mut scanlines = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| rows[previous].as_slice());
        scanlines.push(filters[index]);
        scanlines.extend_from_slice(&encode_row(filters[index], row, previous, 3));
    }

    let png = png_with_scanlines(3, 5, 8, 2, 0, &scanlines, true);
    let reconstructed = reconstruct_png_scanlines(&png).expect("valid filtered PNG");
    let expected: Vec<u8> = rows.into_iter().flatten().collect();
    assert_eq!(reconstructed.packed_sample_bytes, expected);
    assert_eq!(reconstructed.passes.len(), 1);
    assert_eq!(reconstructed.passes[0].row_bytes, 9);
    assert_eq!(reconstructed.passes[0].filter_bytes_per_pixel, 3);
}

#[test]
fn derives_predictor_widths_for_every_legal_byte_class() {
    let cases = [
        PredictorCase {
            width: 9,
            depth: 1,
            color_type: 0,
            expected_bpp: 1,
            expected_row_bytes: 2,
        },
        PredictorCase {
            width: 5,
            depth: 2,
            color_type: 0,
            expected_bpp: 1,
            expected_row_bytes: 2,
        },
        PredictorCase {
            width: 3,
            depth: 4,
            color_type: 3,
            expected_bpp: 1,
            expected_row_bytes: 2,
        },
        PredictorCase {
            width: 3,
            depth: 8,
            color_type: 4,
            expected_bpp: 2,
            expected_row_bytes: 6,
        },
        PredictorCase {
            width: 3,
            depth: 8,
            color_type: 2,
            expected_bpp: 3,
            expected_row_bytes: 9,
        },
        PredictorCase {
            width: 3,
            depth: 8,
            color_type: 6,
            expected_bpp: 4,
            expected_row_bytes: 12,
        },
        PredictorCase {
            width: 2,
            depth: 16,
            color_type: 2,
            expected_bpp: 6,
            expected_row_bytes: 12,
        },
        PredictorCase {
            width: 2,
            depth: 16,
            color_type: 6,
            expected_bpp: 8,
            expected_row_bytes: 16,
        },
    ];

    for case in cases {
        let original: Vec<u8> = (0..case.expected_row_bytes)
            .map(|index| u8::try_from((index * 53 + 247) % 256).expect("test byte"))
            .collect();
        let mut scanlines = vec![1_u8];
        scanlines.extend_from_slice(&encode_row(1, &original, None, case.expected_bpp));
        let png = png_with_scanlines(
            case.width,
            1,
            case.depth,
            case.color_type,
            0,
            &scanlines,
            false,
        );
        let reconstructed = reconstruct_png_scanlines(&png).expect("valid predictor case");
        assert_eq!(reconstructed.packed_sample_bytes, original, "{case:?}");
        assert_eq!(
            reconstructed.passes[0].filter_bytes_per_pixel,
            case.expected_bpp,
            "{case:?}"
        );
        assert_eq!(reconstructed.passes[0].row_bytes, case.expected_row_bytes);
    }
}

#[test]
fn resets_previous_row_state_at_every_adam7_pass_boundary() {
    let width = 8_u32;
    let height = 8_u32;
    let mut scanlines = Vec::new();
    let mut expected = Vec::new();
    let mut expected_offset = 0_usize;
    let mut expected_passes = Vec::new();

    for (index, (start_x, start_y, step_x, step_y)) in ADAM7_PASSES.into_iter().enumerate() {
        let pass_width = pass_extent(width, start_x, step_x);
        let pass_height = pass_extent(height, start_y, step_y);
        if pass_width == 0 || pass_height == 0 {
            continue;
        }
        let row_bytes = usize::try_from(pass_width * 4).expect("small row fits usize");
        let mut previous: Option<Vec<u8>> = None;
        for row_index in 0..pass_height {
            let row: Vec<u8> = (0..row_bytes)
                .map(|byte_index| {
                    let seed = (index + 1) * 31
                        + usize::try_from(row_index).expect("row fits usize") * 17
                        + byte_index * 13;
                    u8::try_from(seed % 256).expect("test byte")
                })
                .collect();
            let filter = if row_index == 0 { 2 } else { 4 };
            scanlines.push(filter);
            scanlines.extend_from_slice(&encode_row(filter, &row, previous.as_deref(), 4));
            expected.extend_from_slice(&row);
            previous = Some(row);
        }
        expected_passes.push((
            u8::try_from(index + 1).expect("pass fits u8"),
            start_x,
            start_y,
            step_x,
            step_y,
            pass_width,
            pass_height,
            row_bytes,
            expected_offset,
        ));
        expected_offset += row_bytes * usize::try_from(pass_height).expect("height fits usize");
    }

    let png = png_with_scanlines(width, height, 8, 6, 1, &scanlines, true);
    let reconstructed = reconstruct_png_scanlines(&png).expect("valid Adam7 PNG");
    assert_eq!(reconstructed.packed_sample_bytes, expected);
    assert_eq!(reconstructed.passes.len(), expected_passes.len());
    for (actual, expected_pass) in reconstructed.passes.iter().zip(expected_passes) {
        assert_eq!(
            (
                actual.index,
                actual.start_x,
                actual.start_y,
                actual.step_x,
                actual.step_y,
                actual.width,
                actual.height,
                actual.row_bytes,
                actual.output_offset,
            ),
            expected_pass
        );
        assert_eq!(actual.filter_bytes_per_pixel, 4);
    }
}

#[test]
fn rejects_invalid_filter_with_exact_pass_and_row_location() {
    let scanlines = [0_u8, 1, 2, 5, 3, 4];
    let png = png_with_scanlines(2, 2, 8, 0, 0, &scanlines, false);
    assert_eq!(
        reconstruct_png_scanlines(&png),
        Err(PngAdapterError::InvalidFilterData(
            PngFilterError::InvalidFilterType {
                pass: 1,
                row: 2,
                filter: 5,
            }
        ))
    );
}
