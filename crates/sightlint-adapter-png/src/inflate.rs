use std::error::Error;
use std::fmt;

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::inflate_flags::{
    TINFL_FLAG_HAS_MORE_INPUT, TINFL_FLAG_PARSE_ZLIB_HEADER,
    TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate::core::{DecompressorOxide, decompress};

use crate::{PngAdapterError, PngHeader, PngStructure, inspect_png_structure};

const MAX_INFLATED_BYTES: u64 = 256 * 1024 * 1024;
const ADAM7_PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// Validated zlib output for a PNG's still-filtered scanline stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InflatedPng {
    /// Exact structural metadata validated before inflation.
    pub structure: PngStructure,
    /// Decompressed PNG scanline bytes, including each row's leading filter byte.
    pub scanline_bytes: Vec<u8>,
}

/// Failure while validating or inflating PNG `IDAT` payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngInflateError {
    /// The declared raster would require more decoded scanline bytes than the safety budget.
    DecodedDataTooLarge {
        /// Exact byte count required by the declared raster geometry.
        expected: u64,
    },
    /// The zlib/DEFLATE stream, wrapper, or Adler-32 checksum is invalid.
    InvalidZlibStream,
    /// The zlib stream ended before all non-empty `IDAT` payload bytes were consumed.
    TrailingCompressedData,
    /// The stream produced more bytes than the declared PNG raster permits.
    DecodedDataTooLong {
        /// Exact maximum byte count permitted by the declared raster geometry.
        expected: usize,
    },
    /// The stream ended successfully but produced fewer bytes than the declared raster requires.
    DecodedLengthMismatch {
        /// Exact byte count required by the declared raster geometry.
        expected: usize,
        /// Byte count actually produced by the zlib stream.
        actual: usize,
    },
    /// A previously validated chunk stream could not be re-walked to obtain `IDAT` payloads.
    InternalStructureMismatch,
}

impl fmt::Display for PngInflateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecodedDataTooLarge { expected } => write!(
                formatter,
                "PNG requires {expected} decompressed scanline bytes, exceeding the {MAX_INFLATED_BYTES}-byte safety limit"
            ),
            Self::InvalidZlibStream => formatter.write_str(
                "PNG IDAT payload is not a valid zlib stream with a matching Adler-32 checksum",
            ),
            Self::TrailingCompressedData => formatter.write_str(
                "PNG IDAT payload contains bytes after the complete zlib stream",
            ),
            Self::DecodedDataTooLong { expected } => write!(
                formatter,
                "PNG IDAT stream expands beyond the exact {expected}-byte scanline size declared by IHDR"
            ),
            Self::DecodedLengthMismatch { expected, actual } => write!(
                formatter,
                "PNG IDAT stream produced {actual} scanline bytes, but IHDR requires exactly {expected}"
            ),
            Self::InternalStructureMismatch => formatter.write_str(
                "PNG chunk stream changed unexpectedly between structure validation and IDAT inflation",
            ),
        }
    }
}

impl Error for PngInflateError {}

/// Validates and inflates the complete zlib stream carried by consecutive PNG `IDAT` chunks.
///
/// The returned bytes are still PNG-filtered and may contain packed samples. No pixel values,
/// colors, transparency, ink bounds, or semantic observations are derived here.
///
/// # Errors
///
/// Returns existing header/structure errors unchanged, or a wrapped [`PngInflateError`] when the
/// decoded raster exceeds the memory budget, zlib validation fails, compressed bytes remain after
/// the zlib terminator, or the decoded byte count does not exactly match the raster declared by
/// `IHDR`.
pub fn inflate_png_scanlines(input: &[u8]) -> Result<InflatedPng, PngAdapterError> {
    let structure = inspect_png_structure(input)?;
    let expected_u64 = expected_scanline_bytes(structure.header);
    if expected_u64 > MAX_INFLATED_BYTES {
        return Err(PngAdapterError::InvalidImageData(
            PngInflateError::DecodedDataTooLarge {
                expected: expected_u64,
            },
        ));
    }
    let expected = usize::try_from(expected_u64).map_err(|_| {
        PngAdapterError::InvalidImageData(PngInflateError::DecodedDataTooLarge {
            expected: expected_u64,
        })
    })?;
    let payloads = idat_payloads(input).map_err(PngAdapterError::InvalidImageData)?;
    let mut scanline_bytes = vec![0_u8; expected + 1];
    let written = inflate_exact(&payloads, &mut scanline_bytes, expected)
        .map_err(PngAdapterError::InvalidImageData)?;
    if written > expected {
        return Err(PngAdapterError::InvalidImageData(
            PngInflateError::DecodedDataTooLong { expected },
        ));
    }
    if written != expected {
        return Err(PngAdapterError::InvalidImageData(
            PngInflateError::DecodedLengthMismatch {
                expected,
                actual: written,
            },
        ));
    }
    scanline_bytes.truncate(expected);

    Ok(InflatedPng {
        structure,
        scanline_bytes,
    })
}

/// Returns the exact decompressed scanline byte count required by a validated PNG header.
pub(crate) fn expected_scanline_bytes(header: PngHeader) -> u64 {
    let bits_per_pixel = u64::from(channels(header.color_type)) * u64::from(header.bit_depth);
    if header.interlace_method == 0 {
        return pass_bytes(header.width, header.height, bits_per_pixel);
    }

    ADAM7_PASSES
        .into_iter()
        .map(|(start_x, start_y, step_x, step_y)| {
            let width = pass_extent(header.width, start_x, step_x);
            let height = pass_extent(header.height, start_y, step_y);
            pass_bytes(width, height, bits_per_pixel)
        })
        .sum()
}

fn inflate_exact(
    payloads: &[&[u8]],
    output: &mut [u8],
    expected: usize,
) -> Result<usize, PngInflateError> {
    let mut decompressor = DecompressorOxide::new();
    let mut output_position = 0_usize;

    for (index, payload) in payloads.iter().enumerate() {
        let more_non_empty_input = payloads[index + 1..]
            .iter()
            .any(|remaining| !remaining.is_empty());
        let mut flags = TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF | TINFL_FLAG_PARSE_ZLIB_HEADER;
        if more_non_empty_input {
            flags |= TINFL_FLAG_HAS_MORE_INPUT;
        }
        let (status, consumed, produced) =
            decompress(&mut decompressor, payload, output, output_position, flags);
        output_position += produced;

        match status {
            TINFLStatus::NeedsMoreInput => {
                if consumed != payload.len() {
                    return Err(PngInflateError::InvalidZlibStream);
                }
            }
            TINFLStatus::Done => {
                if consumed != payload.len() || more_non_empty_input {
                    return Err(PngInflateError::TrailingCompressedData);
                }
                return Ok(output_position);
            }
            TINFLStatus::HasMoreOutput => {
                return Err(PngInflateError::DecodedDataTooLong { expected });
            }
            _ => return Err(PngInflateError::InvalidZlibStream),
        }
    }

    Err(PngInflateError::InvalidZlibStream)
}

fn channels(color_type: u8) -> u8 {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!("validated PNG color type"),
    }
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn pass_bytes(width: u32, height: u32, bits_per_pixel: u64) -> u64 {
    if width == 0 || height == 0 {
        return 0;
    }
    let row_bits = u64::from(width) * bits_per_pixel;
    let row_bytes = row_bits.div_ceil(8);
    u64::from(height) * (1 + row_bytes)
}

fn idat_payloads(input: &[u8]) -> Result<Vec<&[u8]>, PngInflateError> {
    let mut offset = 8_usize;
    let mut payloads = Vec::new();
    while offset < input.len() {
        let length_bytes = input
            .get(offset..offset + 4)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        let length = u32::from_be_bytes(
            length_bytes
                .try_into()
                .map_err(|_| PngInflateError::InternalStructureMismatch)?,
        );
        let data_length =
            usize::try_from(length).map_err(|_| PngInflateError::InternalStructureMismatch)?;
        let kind = input
            .get(offset + 4..offset + 8)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        let data_start = offset
            .checked_add(8)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        let data_end = data_start
            .checked_add(data_length)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        let next = data_end
            .checked_add(4)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        let data = input
            .get(data_start..data_end)
            .ok_or(PngInflateError::InternalStructureMismatch)?;
        if kind == b"IDAT" {
            payloads.push(data);
        }
        if kind == b"IEND" {
            return Ok(payloads);
        }
        if next > input.len() {
            return Err(PngInflateError::InternalStructureMismatch);
        }
        offset = next;
    }
    Err(PngInflateError::InternalStructureMismatch)
}

#[cfg(test)]
mod tests {
    use super::{expected_scanline_bytes, pass_extent};
    use crate::PngHeader;

    fn header(width: u32, height: u32, depth: u8, color_type: u8, interlace: u8) -> PngHeader {
        PngHeader {
            width,
            height,
            bit_depth: depth,
            color_type,
            compression_method: 0,
            filter_method: 0,
            interlace_method: interlace,
        }
    }

    #[test]
    fn calculates_packed_non_interlaced_scanline_sizes() {
        assert_eq!(expected_scanline_bytes(header(8, 2, 1, 0, 0)), 4);
        assert_eq!(expected_scanline_bytes(header(3, 2, 8, 2, 0)), 20);
        assert_eq!(expected_scanline_bytes(header(3, 2, 16, 6, 0)), 50);
    }

    #[test]
    fn calculates_adam7_passes_without_counting_empty_rows() {
        assert_eq!(pass_extent(1, 4, 8), 0);
        assert_eq!(pass_extent(9, 0, 8), 2);
        assert_eq!(expected_scanline_bytes(header(1, 1, 8, 6, 1)), 5);
        assert_eq!(expected_scanline_bytes(header(8, 8, 8, 6, 1)), 271);
    }
}
