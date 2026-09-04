use std::error::Error;
use std::fmt;

use crate::{PngAdapterError, PngHeader, crc32, inspect_png_header};

pub(crate) const MAX_PNG_INPUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_CHUNKS: usize = 10_000;

/// Exact structural facts from a bounded, CRC-validated PNG chunk stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngStructure {
    /// Validated `IHDR` metadata.
    pub header: PngHeader,
    /// Number of chunks including `IHDR` and `IEND`.
    pub chunk_count: u32,
    /// Number of consecutive `IDAT` chunks.
    pub idat_chunk_count: u32,
    /// Total compressed bytes stored across all `IDAT` chunks.
    pub idat_bytes: u64,
    /// Whether a `PLTE` chunk is present.
    pub has_palette: bool,
}

/// Failure while validating the complete PNG chunk stream after a valid `IHDR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngStructureError {
    /// Adapter input exceeds the library-level binary safety limit.
    InputTooLarge,
    /// A chunk header, payload, or CRC extends beyond the available bytes.
    TruncatedChunk,
    /// More chunks were supplied than the structural safety budget permits.
    TooManyChunks,
    /// A chunk type contains a byte outside ASCII alphabetic characters.
    InvalidChunkType,
    /// The reserved bit in the third chunk-type byte is set.
    InvalidReservedBit,
    /// A critical chunk type is not defined by the PNG specification.
    UnknownCriticalChunk(String),
    /// A second `IHDR` chunk appeared after the required first chunk.
    DuplicateIhdr,
    /// A chunk CRC-32 does not match its type and payload.
    InvalidChunkCrc(String),
    /// More than one `PLTE` chunk is present.
    DuplicatePalette,
    /// `PLTE` appears after image data begins.
    PaletteAfterImageData,
    /// A grayscale color type contains a forbidden `PLTE` chunk.
    PaletteForbidden,
    /// Indexed color is missing its required `PLTE` chunk.
    PaletteRequired,
    /// `PLTE` has an invalid byte length or palette cardinality.
    InvalidPaletteLength,
    /// No `IDAT` chunk is present.
    MissingImageData,
    /// `IDAT` chunks are separated by another chunk.
    NonConsecutiveImageData,
    /// No terminating `IEND` chunk is present.
    MissingIend,
    /// `IEND` contains data instead of being zero length.
    InvalidIendLength,
    /// Bytes or chunks appear after `IEND`.
    TrailingBytes,
}

impl fmt::Display for PngStructureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge => write!(
                formatter,
                "PNG input exceeds the {MAX_PNG_INPUT_BYTES}-byte adapter safety limit"
            ),
            Self::TruncatedChunk => formatter.write_str("PNG chunk framing is truncated"),
            Self::TooManyChunks => write!(
                formatter,
                "PNG contains more than the {MAX_CHUNKS}-chunk safety limit"
            ),
            Self::InvalidChunkType => {
                formatter.write_str("PNG chunk type must contain four ASCII letters")
            }
            Self::InvalidReservedBit => {
                formatter.write_str("PNG chunk type has an invalid reserved bit")
            }
            Self::UnknownCriticalChunk(kind) => {
                write!(formatter, "PNG contains unknown critical chunk {kind}")
            }
            Self::DuplicateIhdr => formatter.write_str("PNG contains more than one IHDR chunk"),
            Self::InvalidChunkCrc(kind) => {
                write!(formatter, "PNG {kind} chunk CRC-32 does not match its payload")
            }
            Self::DuplicatePalette => formatter.write_str("PNG contains more than one PLTE chunk"),
            Self::PaletteAfterImageData => {
                formatter.write_str("PNG PLTE chunk must appear before the first IDAT chunk")
            }
            Self::PaletteForbidden => {
                formatter.write_str("PNG grayscale color types must not contain a PLTE chunk")
            }
            Self::PaletteRequired => {
                formatter.write_str("PNG indexed color type requires a PLTE chunk before IDAT")
            }
            Self::InvalidPaletteLength => formatter.write_str(
                "PNG PLTE length must encode 1..=256 RGB entries and fit the indexed bit depth",
            ),
            Self::MissingImageData => {
                formatter.write_str("PNG must contain at least one IDAT chunk")
            }
            Self::NonConsecutiveImageData => {
                formatter.write_str("PNG IDAT chunks must be consecutive")
            }
            Self::MissingIend => formatter.write_str("PNG is missing the terminating IEND chunk"),
            Self::InvalidIendLength => formatter.write_str("PNG IEND chunk must have zero length"),
            Self::TrailingBytes => formatter.write_str("PNG contains trailing bytes after IEND"),
        }
    }
}

impl Error for PngStructureError {}

/// Validates the complete PNG chunk stream without decoding compressed image samples.
///
/// # Errors
///
/// Returns the existing header-level [`PngAdapterError`] unchanged when `IHDR` is invalid, and a
/// wrapped structural error for unsafe sizes, malformed later chunk framing or types, CRC
/// mismatch, invalid critical-chunk ordering, palette contract violations, missing image data,
/// or invalid termination.
pub fn inspect_png_structure(input: &[u8]) -> Result<PngStructure, PngAdapterError> {
    if input.len() > MAX_PNG_INPUT_BYTES {
        return Err(PngAdapterError::InvalidStructure(
            PngStructureError::InputTooLarge,
        ));
    }

    let header = inspect_png_header(input)?;
    inspect_png_structure_after_header(input, header).map_err(PngAdapterError::InvalidStructure)
}

fn inspect_png_structure_after_header(
    input: &[u8],
    header: PngHeader,
) -> Result<PngStructure, PngStructureError> {
    let mut offset = 8_usize;
    let mut chunk_count = 0_usize;
    let mut idat_chunk_count = 0_u32;
    let mut idat_bytes = 0_u64;
    let mut saw_palette = false;
    let mut saw_idat = false;
    let mut idat_closed = false;

    while offset < input.len() {
        if chunk_count >= MAX_CHUNKS {
            return Err(PngStructureError::TooManyChunks);
        }
        if input.len() - offset < 12 {
            return Err(PngStructureError::TruncatedChunk);
        }

        let length = u32::from_be_bytes(
            input[offset..offset + 4]
                .try_into()
                .expect("four-byte slice"),
        );
        let data_length = usize::try_from(length).expect("u32 fits usize on supported platforms");
        let chunk_end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(data_length))
            .ok_or(PngStructureError::TruncatedChunk)?;
        if chunk_end > input.len() {
            return Err(PngStructureError::TruncatedChunk);
        }

        let kind_bytes: [u8; 4] = input[offset + 4..offset + 8]
            .try_into()
            .expect("four-byte slice");
        if !kind_bytes.iter().all(u8::is_ascii_alphabetic) {
            return Err(PngStructureError::InvalidChunkType);
        }
        if kind_bytes[2].is_ascii_lowercase() {
            return Err(PngStructureError::InvalidReservedBit);
        }
        let kind = std::str::from_utf8(&kind_bytes).expect("ASCII chunk type");
        let data_start = offset + 8;
        let data_end = data_start + data_length;
        let expected_crc = u32::from_be_bytes(
            input[data_end..data_end + 4]
                .try_into()
                .expect("four-byte slice"),
        );
        if crc32(&input[offset + 4..data_end]) != expected_crc {
            return Err(PngStructureError::InvalidChunkCrc(kind.to_owned()));
        }

        chunk_count += 1;
        match kind {
            "IHDR" => {
                if chunk_count != 1 {
                    return Err(PngStructureError::DuplicateIhdr);
                }
            }
            "PLTE" => {
                if saw_palette {
                    return Err(PngStructureError::DuplicatePalette);
                }
                if saw_idat {
                    return Err(PngStructureError::PaletteAfterImageData);
                }
                if matches!(header.color_type, 0 | 4) {
                    return Err(PngStructureError::PaletteForbidden);
                }
                validate_palette_length(data_length, header)?;
                saw_palette = true;
            }
            "IDAT" => {
                if idat_closed {
                    return Err(PngStructureError::NonConsecutiveImageData);
                }
                if header.color_type == 3 && !saw_palette {
                    return Err(PngStructureError::PaletteRequired);
                }
                saw_idat = true;
                idat_chunk_count = idat_chunk_count.saturating_add(1);
                idat_bytes = idat_bytes.saturating_add(u64::from(length));
            }
            "IEND" => {
                if length != 0 {
                    return Err(PngStructureError::InvalidIendLength);
                }
                if !saw_idat {
                    return Err(PngStructureError::MissingImageData);
                }
                if chunk_end != input.len() {
                    return Err(PngStructureError::TrailingBytes);
                }
                return Ok(PngStructure {
                    header,
                    chunk_count: u32::try_from(chunk_count).expect("chunk budget fits u32"),
                    idat_chunk_count,
                    idat_bytes,
                    has_palette: saw_palette,
                });
            }
            _ => {
                if kind_bytes[0].is_ascii_uppercase() {
                    return Err(PngStructureError::UnknownCriticalChunk(kind.to_owned()));
                }
            }
        }

        if saw_idat && kind != "IDAT" {
            idat_closed = true;
        }
        offset = chunk_end;
    }

    if !saw_idat {
        Err(PngStructureError::MissingImageData)
    } else {
        Err(PngStructureError::MissingIend)
    }
}

fn validate_palette_length(data_length: usize, header: PngHeader) -> Result<(), PngStructureError> {
    if data_length == 0 || data_length > 256 * 3 || data_length % 3 != 0 {
        return Err(PngStructureError::InvalidPaletteLength);
    }
    if header.color_type == 3 {
        let entries = data_length / 3;
        let maximum = 1_usize << header.bit_depth;
        if entries > maximum {
            return Err(PngStructureError::InvalidPaletteLength);
        }
    }
    Ok(())
}
