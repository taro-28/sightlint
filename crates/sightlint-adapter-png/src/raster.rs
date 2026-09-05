use std::error::Error;
use std::fmt;

use crate::{
    PngAdapterError, PngHeader, PngPass, PngStructure, ReconstructedPng,
    reconstruct_png_scanlines,
};

const MAX_RGBA8_BYTES: u64 = 256 * 1024 * 1024;
const PNG_SIGNATURE_BYTES: usize = 8;
const CHUNK_OVERHEAD_BYTES: usize = 12;

/// Exact row-major PNG sample bytes expanded to four encoded bytes per pixel.
///
/// Values are source PNG sample values. No gamma, ICC, chromaticity, or display-profile
/// transformation has been applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedRgba8Raster {
    /// Raster width in device pixels.
    pub width: u32,
    /// Raster height in device pixels.
    pub height: u32,
    /// Row-major `R, G, B, A` bytes with exactly four bytes per pixel.
    pub pixels: Vec<u8>,
}

/// Stable reason why a valid PNG cannot yet produce the staged encoded RGBA8 raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngRasterUnavailable {
    /// Indexed color requires palette expansion, which belongs to a later slice.
    IndexedColor,
    /// The staged raster supports only eight-bit source samples.
    UnsupportedBitDepth {
        /// Source bit depth declared by `IHDR`.
        bit_depth: u8,
        /// Source color type declared by `IHDR`.
        color_type: u8,
    },
    /// Applying a `tRNS` transparency chunk belongs to a later slice.
    TransparencyChunk,
    /// Expanding the source to four bytes per pixel would exceed the raster budget.
    BufferTooLarge {
        /// Exact number of RGBA bytes required by the source dimensions.
        required: u64,
        /// Maximum number of RGBA bytes allowed by this adapter version.
        limit: u64,
    },
}

impl PngRasterUnavailable {
    /// Returns a stable machine-readable reason code for Artifact IR metadata.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IndexedColor => "indexedColor",
            Self::UnsupportedBitDepth { .. } => "unsupportedBitDepth",
            Self::TransparencyChunk => "transparencyChunk",
            Self::BufferTooLarge { .. } => "bufferTooLarge",
        }
    }
}

/// Availability of the staged encoded RGBA8 raster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngRasterStatus {
    /// Exact encoded RGBA8 pixels are available.
    Available(EncodedRgba8Raster),
    /// The PNG is valid, but this adapter stage deliberately abstains from pixel expansion.
    Unavailable(PngRasterUnavailable),
}

/// Exact PNG observations retained after packed sample reconstruction and optional raster scatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedPngRaster {
    /// Exact structural metadata validated before decompression.
    pub structure: PngStructure,
    /// Number of reconstructed packed sample bytes before any channel expansion.
    pub reconstructed_packed_sample_bytes: usize,
    /// Number of non-empty source passes represented by the reconstructed rows.
    pub non_empty_pass_count: usize,
    /// Encoded RGBA8 availability and bytes when supported.
    pub status: PngRasterStatus,
}

/// Failure while deriving a canonical addressable raster from reconstructed PNG samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngRasterError {
    /// A validated PNG could not be safely re-walked while checking ancillary chunks.
    InternalChunkWalkMismatch,
    /// Reconstructed pass row width does not match its declared pixel width and color channels.
    PassRowLayoutMismatch {
        /// One-based original PNG pass index.
        pass: u8,
        /// Exact packed bytes expected per row.
        expected: usize,
        /// Packed bytes recorded by filter reconstruction.
        actual: usize,
    },
    /// A reconstructed pass references bytes outside the packed sample buffer.
    PackedSampleRangeMismatch {
        /// One-based original PNG pass index.
        pass: u8,
    },
    /// A pass coordinate falls outside the declared canvas dimensions.
    DestinationOutOfBounds {
        /// One-based original PNG pass index.
        pass: u8,
        /// One-based row within the pass.
        row: u32,
        /// One-based pixel column within the pass row.
        column: u32,
    },
    /// Passes did not account for exactly one sample at every destination coordinate.
    PixelCoverageMismatch {
        /// Pixel count declared by the PNG canvas.
        expected: u64,
        /// Total number of pass samples encountered.
        actual: u64,
    },
    /// Checked integer arithmetic could not represent a derived byte or coordinate layout.
    LayoutOverflow,
}

impl fmt::Display for PngRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalChunkWalkMismatch => formatter.write_str(
                "PNG chunk stream changed unexpectedly before raster availability classification",
            ),
            Self::PassRowLayoutMismatch {
                pass,
                expected,
                actual,
            } => write!(
                formatter,
                "PNG pass {pass} requires {expected} packed bytes per row but reconstruction supplied {actual}"
            ),
            Self::PackedSampleRangeMismatch { pass } => write!(
                formatter,
                "PNG pass {pass} references bytes outside the reconstructed sample buffer"
            ),
            Self::DestinationOutOfBounds { pass, row, column } => write!(
                formatter,
                "PNG pass {pass} row {row} column {column} maps outside the declared canvas"
            ),
            Self::PixelCoverageMismatch { expected, actual } => write!(
                formatter,
                "PNG passes contain {actual} samples but the declared canvas requires {expected} pixels"
            ),
            Self::LayoutOverflow => {
                formatter.write_str("PNG raster layout exceeds the supported address space")
            }
        }
    }
}

impl Error for PngRasterError {}

/// Reconstructs PNG samples and produces an encoded RGBA8 raster when the staged format contract
/// supports the source exactly.
///
/// Legal but not-yet-supported PNG variants return [`PngRasterStatus::Unavailable`] rather than a
/// malformed-input error. Raw pixel bytes remain inside the adapter boundary and are not
/// serialized into Artifact IR.
///
/// # Errors
///
/// Returns existing PNG validation, inflation, or filter errors unchanged. Returns a wrapped
/// [`PngRasterError`] only when validated internal layouts cannot be reconciled safely.
pub fn observe_png_raster(input: &[u8]) -> Result<ObservedPngRaster, PngAdapterError> {
    let reconstructed = reconstruct_png_scanlines(input)?;
    let reconstructed_packed_sample_bytes = reconstructed.packed_sample_bytes.len();
    let non_empty_pass_count = reconstructed.passes.len();
    let structure = reconstructed.structure;
    let has_transparency = contains_chunk(input, *b"tRNS")
        .map_err(PngAdapterError::InvalidRasterData)?;

    let status = match classify_raster(structure.header, has_transparency)
        .map_err(PngAdapterError::InvalidRasterData)?
    {
        Some(reason) => PngRasterStatus::Unavailable(reason),
        None => PngRasterStatus::Available(
            scatter_rgba8(&reconstructed).map_err(PngAdapterError::InvalidRasterData)?,
        ),
    };

    Ok(ObservedPngRaster {
        structure,
        reconstructed_packed_sample_bytes,
        non_empty_pass_count,
        status,
    })
}

fn classify_raster(
    header: PngHeader,
    has_transparency: bool,
) -> Result<Option<PngRasterUnavailable>, PngRasterError> {
    if has_transparency {
        return Ok(Some(PngRasterUnavailable::TransparencyChunk));
    }
    if header.color_type == 3 {
        return Ok(Some(PngRasterUnavailable::IndexedColor));
    }
    if header.bit_depth != 8 {
        return Ok(Some(PngRasterUnavailable::UnsupportedBitDepth {
            bit_depth: header.bit_depth,
            color_type: header.color_type,
        }));
    }

    let required = u64::from(header.width)
        .checked_mul(u64::from(header.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(PngRasterError::LayoutOverflow)?;
    if required > MAX_RGBA8_BYTES {
        return Ok(Some(PngRasterUnavailable::BufferTooLarge {
            required,
            limit: MAX_RGBA8_BYTES,
        }));
    }
    Ok(None)
}

fn scatter_rgba8(reconstructed: &ReconstructedPng) -> Result<EncodedRgba8Raster, PngRasterError> {
    let header = reconstructed.structure.header;
    let source_channels = usize::from(source_channels(header.color_type));
    let pixel_count = u64::from(header.width)
        .checked_mul(u64::from(header.height))
        .ok_or(PngRasterError::LayoutOverflow)?;
    let output_length_u64 = pixel_count
        .checked_mul(4)
        .ok_or(PngRasterError::LayoutOverflow)?;
    let output_length =
        usize::try_from(output_length_u64).map_err(|_| PngRasterError::LayoutOverflow)?;
    let mut pixels = vec![0_u8; output_length];
    let mut covered_pixels = 0_u64;

    for pass in &reconstructed.passes {
        validate_pass_row_layout(*pass, source_channels)?;
        validate_pass_sample_range(pass, &reconstructed.packed_sample_bytes)?;
        scatter_pass(
            pass,
            header,
            source_channels,
            &reconstructed.packed_sample_bytes,
            &mut pixels,
        )?;
        covered_pixels = covered_pixels
            .checked_add(u64::from(pass.width) * u64::from(pass.height))
            .ok_or(PngRasterError::LayoutOverflow)?;
    }

    if covered_pixels != pixel_count {
        return Err(PngRasterError::PixelCoverageMismatch {
            expected: pixel_count,
            actual: covered_pixels,
        });
    }

    Ok(EncodedRgba8Raster {
        width: header.width,
        height: header.height,
        pixels,
    })
}

fn validate_pass_row_layout(
    pass: PngPass,
    source_channels: usize,
) -> Result<(), PngRasterError> {
    let width = usize::try_from(pass.width).map_err(|_| PngRasterError::LayoutOverflow)?;
    let expected = width
        .checked_mul(source_channels)
        .ok_or(PngRasterError::LayoutOverflow)?;
    if pass.row_bytes != expected {
        return Err(PngRasterError::PassRowLayoutMismatch {
            pass: pass.index,
            expected,
            actual: pass.row_bytes,
        });
    }
    Ok(())
}

fn validate_pass_sample_range(pass: &PngPass, packed: &[u8]) -> Result<(), PngRasterError> {
    let height = usize::try_from(pass.height).map_err(|_| PngRasterError::LayoutOverflow)?;
    let length = pass
        .row_bytes
        .checked_mul(height)
        .ok_or(PngRasterError::LayoutOverflow)?;
    let end = pass
        .output_offset
        .checked_add(length)
        .ok_or(PngRasterError::LayoutOverflow)?;
    if end > packed.len() {
        return Err(PngRasterError::PackedSampleRangeMismatch { pass: pass.index });
    }
    Ok(())
}

fn scatter_pass(
    pass: &PngPass,
    header: PngHeader,
    source_channels: usize,
    packed: &[u8],
    output: &mut [u8],
) -> Result<(), PngRasterError> {
    for row in 0..pass.height {
        let row_offset = usize::try_from(row)
            .map_err(|_| PngRasterError::LayoutOverflow)?
            .checked_mul(pass.row_bytes)
            .and_then(|offset| pass.output_offset.checked_add(offset))
            .ok_or(PngRasterError::LayoutOverflow)?;
        for column in 0..pass.width {
            let source_offset = usize::try_from(column)
                .map_err(|_| PngRasterError::LayoutOverflow)?
                .checked_mul(source_channels)
                .and_then(|offset| row_offset.checked_add(offset))
                .ok_or(PngRasterError::LayoutOverflow)?;
            let source_end = source_offset
                .checked_add(source_channels)
                .ok_or(PngRasterError::LayoutOverflow)?;
            let source = packed
                .get(source_offset..source_end)
                .ok_or(PngRasterError::PackedSampleRangeMismatch { pass: pass.index })?;

            let x = pass
                .start_x
                .checked_add(
                    column
                        .checked_mul(pass.step_x)
                        .ok_or(PngRasterError::LayoutOverflow)?,
                )
                .ok_or(PngRasterError::LayoutOverflow)?;
            let y = pass
                .start_y
                .checked_add(
                    row.checked_mul(pass.step_y)
                        .ok_or(PngRasterError::LayoutOverflow)?,
                )
                .ok_or(PngRasterError::LayoutOverflow)?;
            if x >= header.width || y >= header.height {
                return Err(PngRasterError::DestinationOutOfBounds {
                    pass: pass.index,
                    row: row + 1,
                    column: column + 1,
                });
            }

            let destination = u64::from(y)
                .checked_mul(u64::from(header.width))
                .and_then(|offset| offset.checked_add(u64::from(x)))
                .and_then(|pixel| pixel.checked_mul(4))
                .ok_or(PngRasterError::LayoutOverflow)?;
            let destination =
                usize::try_from(destination).map_err(|_| PngRasterError::LayoutOverflow)?;
            let destination_end = destination
                .checked_add(4)
                .ok_or(PngRasterError::LayoutOverflow)?;
            let rgba = expand_pixel(source, header.color_type);
            output
                .get_mut(destination..destination_end)
                .ok_or(PngRasterError::DestinationOutOfBounds {
                    pass: pass.index,
                    row: row + 1,
                    column: column + 1,
                })?
                .copy_from_slice(&rgba);
        }
    }
    Ok(())
}

fn expand_pixel(source: &[u8], color_type: u8) -> [u8; 4] {
    match color_type {
        0 => [source[0], source[0], source[0], 255],
        2 => [source[0], source[1], source[2], 255],
        4 => [source[0], source[0], source[0], source[1]],
        6 => [source[0], source[1], source[2], source[3]],
        _ => unreachable!("staged raster classifier rejects unsupported color types"),
    }
}

fn source_channels(color_type: u8) -> u8 {
    match color_type {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!("staged raster classifier rejects unsupported color types"),
    }
}

fn contains_chunk(input: &[u8], target: [u8; 4]) -> Result<bool, PngRasterError> {
    let mut offset = PNG_SIGNATURE_BYTES;
    loop {
        let length_bytes = input
            .get(offset..offset + 4)
            .ok_or(PngRasterError::InternalChunkWalkMismatch)?;
        let length = u32::from_be_bytes(
            length_bytes
                .try_into()
                .map_err(|_| PngRasterError::InternalChunkWalkMismatch)?,
        );
        let data_length =
            usize::try_from(length).map_err(|_| PngRasterError::InternalChunkWalkMismatch)?;
        let kind: [u8; 4] = input
            .get(offset + 4..offset + 8)
            .ok_or(PngRasterError::InternalChunkWalkMismatch)?
            .try_into()
            .map_err(|_| PngRasterError::InternalChunkWalkMismatch)?;
        if kind == target {
            return Ok(true);
        }
        let next = offset
            .checked_add(CHUNK_OVERHEAD_BYTES)
            .and_then(|value| value.checked_add(data_length))
            .ok_or(PngRasterError::InternalChunkWalkMismatch)?;
        if kind == *b"IEND" {
            return Ok(false);
        }
        if next > input.len() {
            return Err(PngRasterError::InternalChunkWalkMismatch);
        }
        offset = next;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_RGBA8_BYTES, PngRasterUnavailable, classify_raster, expand_pixel, source_channels,
    };
    use crate::PngHeader;

    fn header(width: u32, height: u32, bit_depth: u8, color_type: u8) -> PngHeader {
        PngHeader {
            width,
            height,
            bit_depth,
            color_type,
            compression_method: 0,
            filter_method: 0,
            interlace_method: 0,
        }
    }

    #[test]
    fn expands_supported_color_types_without_color_management() {
        assert_eq!(expand_pixel(&[7], 0), [7, 7, 7, 255]);
        assert_eq!(expand_pixel(&[1, 2, 3], 2), [1, 2, 3, 255]);
        assert_eq!(expand_pixel(&[9, 17], 4), [9, 9, 9, 17]);
        assert_eq!(expand_pixel(&[4, 5, 6, 7], 6), [4, 5, 6, 7]);
        assert_eq!(source_channels(0), 1);
        assert_eq!(source_channels(2), 3);
        assert_eq!(source_channels(4), 2);
        assert_eq!(source_channels(6), 4);
    }

    #[test]
    fn classifies_deliberate_format_abstentions() {
        assert_eq!(
            classify_raster(header(1, 1, 8, 3), false),
            Ok(Some(PngRasterUnavailable::IndexedColor))
        );
        assert_eq!(
            classify_raster(header(1, 1, 16, 6), false),
            Ok(Some(PngRasterUnavailable::UnsupportedBitDepth {
                bit_depth: 16,
                color_type: 6,
            }))
        );
        assert_eq!(
            classify_raster(header(1, 1, 8, 2), true),
            Ok(Some(PngRasterUnavailable::TransparencyChunk))
        );
    }

    #[test]
    fn classifies_rgba_memory_boundary_without_allocating_it() {
        assert_eq!(classify_raster(header(8_192, 8_192, 8, 0), false), Ok(None));
        let required = 8_193_u64 * 8_192 * 4;
        assert_eq!(
            classify_raster(header(8_193, 8_192, 8, 0), false),
            Ok(Some(PngRasterUnavailable::BufferTooLarge {
                required,
                limit: MAX_RGBA8_BYTES,
            }))
        );
    }
}
