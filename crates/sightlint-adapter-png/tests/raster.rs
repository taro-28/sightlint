//! Integration tests for staged encoded RGBA8 PNG raster availability.

use sightlint_adapter_png::{
    EncodedRgba8Raster, PngRasterStatus, PngRasterUnavailable, observe_png_raster,
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

fn non_interlaced_scanlines(source: &[u8], row_bytes: usize, height: u32) -> Vec<u8> {
    let expected = row_bytes * usize::try_from(height).expect("height fits usize");
    assert_eq!(source.len(), expected);
    let mut scanlines = Vec::with_capacity(expected + usize::try_from(height).expect("height"));
    for row in source.chunks_exact(row_bytes) {
        scanlines.push(0);
        scanlines.extend_from_slice(row);
    }
    scanlines
}

fn available(status: PngRasterStatus) -> EncodedRgba8Raster {
    match status {
        PngRasterStatus::Available(raster) => raster,
        PngRasterStatus::Unavailable(reason) => {
            panic!("expected available raster, got {reason:?}")
        }
    }
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

#[test]
fn expands_each_supported_color_type_to_exact_rgba_bytes() {
    let cases = [
        (
            0_u8,
            vec![10, 20, 30, 40],
            vec![
                10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255, 40, 40, 40, 255,
            ],
        ),
        (
            2,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255],
        ),
        (
            4,
            vec![9, 0, 17, 64, 25, 128, 33, 255],
            vec![9, 9, 9, 0, 17, 17, 17, 64, 25, 25, 25, 128, 33, 33, 33, 255],
        ),
        (
            6,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ],
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ],
        ),
    ];

    for (color_type, source, expected) in cases {
        let channels = match color_type {
            0 => 1,
            2 => 3,
            4 => 2,
            6 => 4,
            _ => unreachable!("test color type"),
        };
        let scanlines = non_interlaced_scanlines(&source, 2 * channels, 2);
        let png = build_png(2, 2, 8, color_type, 0, &scanlines, None);
        let observed = observe_png_raster(&png).expect("valid supported PNG");
        let raster = available(observed.status);
        assert_eq!(raster.width, 2);
        assert_eq!(raster.height, 2);
        assert_eq!(raster.pixels, expected, "color type {color_type}");
        assert_eq!(observed.non_empty_pass_count, 1);
        assert_eq!(observed.reconstructed_packed_sample_bytes, source.len());
    }
}

#[test]
fn scatters_every_adam7_sample_to_its_unique_canvas_coordinate() {
    let width = 8_u32;
    let height = 8_u32;
    let mut expected = Vec::with_capacity(usize::try_from(width * height * 4).expect("small raster"));
    for y in 0..height {
        for x in 0..width {
            expected.extend_from_slice(&[
                u8::try_from(x * 17 + y).expect("red fits"),
                u8::try_from(y * 19 + x).expect("green fits"),
                u8::try_from((x * 31 + y * 13) % 256).expect("blue fits"),
                u8::try_from(255 - (x * 7 + y * 5)).expect("alpha fits"),
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
                let offset = usize::try_from((y * width + x) * 4).expect("offset fits usize");
                scanlines.extend_from_slice(&expected[offset..offset + 4]);
            }
        }
    }

    let png = build_png(width, height, 8, 6, 1, &scanlines, None);
    let observed = observe_png_raster(&png).expect("valid Adam7 PNG");
    let raster = available(observed.status);
    assert_eq!(raster.pixels, expected);
    assert_eq!(observed.non_empty_pass_count, 7);
    assert_eq!(raster.pixels.len(), usize::try_from(width * height * 4).expect("length"));
}

#[test]
fn valid_but_unhandled_formats_abstain_without_becoming_input_errors() {
    let indexed = build_png(2, 1, 8, 3, 0, &[0, 0, 1], None);
    let indexed_observation = observe_png_raster(&indexed).expect("valid indexed PNG");
    assert_eq!(
        indexed_observation.status,
        PngRasterStatus::Unavailable(PngRasterUnavailable::IndexedColor)
    );

    let packed = build_png(8, 1, 1, 0, 0, &[0, 0b1010_0101], None);
    let packed_observation = observe_png_raster(&packed).expect("valid packed PNG");
    assert_eq!(
        packed_observation.status,
        PngRasterStatus::Unavailable(PngRasterUnavailable::UnsupportedBitDepth {
            bit_depth: 1,
            color_type: 0,
        })
    );

    let sixteen_bit = build_png(1, 1, 16, 6, 0, &[0, 0, 1, 0, 2, 0, 3, 0, 4], None);
    let sixteen_bit_observation = observe_png_raster(&sixteen_bit).expect("valid 16-bit PNG");
    assert_eq!(
        sixteen_bit_observation.status,
        PngRasterStatus::Unavailable(PngRasterUnavailable::UnsupportedBitDepth {
            bit_depth: 16,
            color_type: 6,
        })
    );

    let with_transparency = build_png(
        1,
        1,
        8,
        2,
        0,
        &[0, 10, 20, 30],
        Some(&[0, 10, 0, 20, 0, 30]),
    );
    let transparency_observation =
        observe_png_raster(&with_transparency).expect("valid PNG with tRNS");
    assert_eq!(
        transparency_observation.status,
        PngRasterStatus::Unavailable(PngRasterUnavailable::TransparencyChunk)
    );
}
