use sightlint_adapter_png::{PngAdapterError, PngStructureError, inspect_png_structure};

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

fn ihdr_data(bit_depth: u8, color_type: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(13);
    data.extend_from_slice(&4_u32.to_be_bytes());
    data.extend_from_slice(&3_u32.to_be_bytes());
    data.extend_from_slice(&[bit_depth, color_type, 0, 0, 0]);
    data
}

fn png_prefix(bit_depth: u8, color_type: u8) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    append_chunk(&mut bytes, *b"IHDR", &ihdr_data(bit_depth, color_type));
    bytes
}

fn structural_error(input: &[u8]) -> PngStructureError {
    match inspect_png_structure(input) {
        Err(PngAdapterError::InvalidStructure(error)) => error,
        other => panic!("expected structural error, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_ihdr_and_palette_chunks() {
    let mut duplicate_ihdr = png_prefix(8, 6);
    append_chunk(&mut duplicate_ihdr, *b"IHDR", &ihdr_data(8, 6));
    append_chunk(&mut duplicate_ihdr, *b"IDAT", &[]);
    append_chunk(&mut duplicate_ihdr, *b"IEND", &[]);
    assert_eq!(
        structural_error(&duplicate_ihdr),
        PngStructureError::DuplicateIhdr
    );

    let mut duplicate_palette = png_prefix(8, 2);
    append_chunk(&mut duplicate_palette, *b"PLTE", &[0, 0, 0]);
    append_chunk(&mut duplicate_palette, *b"PLTE", &[1, 1, 1]);
    append_chunk(&mut duplicate_palette, *b"IDAT", &[]);
    append_chunk(&mut duplicate_palette, *b"IEND", &[]);
    assert_eq!(
        structural_error(&duplicate_palette),
        PngStructureError::DuplicatePalette
    );
}

#[test]
fn rejects_palette_after_image_data() {
    let mut png = png_prefix(8, 2);
    append_chunk(&mut png, *b"IDAT", &[]);
    append_chunk(&mut png, *b"PLTE", &[0, 0, 0]);
    append_chunk(&mut png, *b"IEND", &[]);
    assert_eq!(
        structural_error(&png),
        PngStructureError::PaletteAfterImageData
    );
}

#[test]
fn rejects_invalid_chunk_type_and_reserved_bit() {
    let mut invalid_type = png_prefix(8, 6);
    append_chunk(&mut invalid_type, *b"tE1t", &[]);
    append_chunk(&mut invalid_type, *b"IDAT", &[]);
    append_chunk(&mut invalid_type, *b"IEND", &[]);
    assert_eq!(
        structural_error(&invalid_type),
        PngStructureError::InvalidChunkType
    );

    let mut reserved_bit = png_prefix(8, 6);
    append_chunk(&mut reserved_bit, *b"texT", &[]);
    append_chunk(&mut reserved_bit, *b"IDAT", &[]);
    append_chunk(&mut reserved_bit, *b"IEND", &[]);
    assert_eq!(
        structural_error(&reserved_bit),
        PngStructureError::InvalidReservedBit
    );
}

#[test]
fn rejects_truncated_and_oversized_chunk_framing() {
    let mut truncated = png_prefix(8, 6);
    truncated.extend_from_slice(&[0, 0, 0]);
    assert_eq!(
        structural_error(&truncated),
        PngStructureError::TruncatedChunk
    );

    let mut oversized = png_prefix(8, 6);
    oversized.extend_from_slice(&u32::MAX.to_be_bytes());
    oversized.extend_from_slice(b"tEXt");
    oversized.extend_from_slice(&0_u32.to_be_bytes());
    assert_eq!(
        structural_error(&oversized),
        PngStructureError::TruncatedChunk
    );
}

#[test]
fn library_enforces_binary_input_limit_before_chunk_walk() {
    const MAX_PNG_INPUT_BYTES: usize = 64 * 1024 * 1024;
    let mut bytes = png_prefix(8, 6);
    bytes.resize(MAX_PNG_INPUT_BYTES + 1, 0);
    assert_eq!(structural_error(&bytes), PngStructureError::InputTooLarge);
}
