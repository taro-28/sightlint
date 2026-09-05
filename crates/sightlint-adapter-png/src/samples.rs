use std::error::Error;
use std::fmt;

use crate::{
    PngAdapterError, PngHeader, ReconstructedPng, crc32, reconstruct_png_filters,
};

const MAX_RGBA8_BYTES: u64 = 128 * 1024 * 1024;

/// Presence of PNG chunks that can affect color-managed display interpretation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PngColorDescription {
    /// Whether an `sRGB` rendering-intent chunk is present.
    pub has_srgb: bool,
    /// Whether a `gAMA` transfer chunk is present.
    pub has_gamma: bool,
    /// Whether a `cHRM` chromaticity chunk is present.
    pub has_chromaticities: bool,
    /// Whether an `iCCP` embedded profile chunk is present.
    pub has_icc_profile: bool,
}

impl PngColorDescription {
    /// Returns whether source color-description data remains unapplied.
    #[must_use]
    pub const fn is_present(self) -> bool {
        self.has_srgb || self.has_gamma || self.has_chromaticities || self.has_icc_profile
    }
}

/// Deterministically normalized PNG pixels before color management or compositing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPng {
    /// Exact reconstructed packed samples and pass metadata.
    pub reconstructed: ReconstructedPng,
    /// PNG-encoded RGBA8 bytes in final destination row-major order.
    pub rgba8: Vec<u8>,
    /// CRC-32 of [`Self::rgba8`] for independent differential verification.
    pub rgba8_crc32: u32,
    /// Number of pixels whose normalized alpha is 255.
    pub opaque_pixel_count: u64,
    /// Number of pixels whose normalized alpha is zero.
    pub transparent_pixel_count: u64,
    /// Number of pixels whose normalized alpha is between zero and 255.
    pub translucent_pixel_count: u64,
    /// Number of RGB entries in `PLTE`, or zero when no palette exists.
    pub palette_entry_count: u16,
    /// Number of indexed alpha entries, or one for a grayscale/truecolor transparency key.
    pub transparency_entry_count: u16,
    /// Presence of source color-description chunks that this stage deliberately does not apply.
    pub color_description: PngColorDescription,
}

/// Failure while expanding reconstructed PNG samples into PNG-encoded RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngSampleError {
    /// The RGBA8 destination would exceed the normalization memory budget.
    OutputTooLarge {
        /// Exact number of destination bytes required by the declared dimensions.
        expected: u64,
    },
    /// More than one `tRNS` chunk is present.
    DuplicateTransparency,
    /// `tRNS` appears after image data begins.
    TransparencyAfterImageData,
    /// Indexed `tRNS` appears before the required palette.
    TransparencyBeforePalette,
    /// The source color type already includes alpha and therefore forbids `tRNS`.
    TransparencyForbidden {
        /// PNG color type declared by `IHDR`.
        color_type: u8,
    },
    /// `tRNS` has a byte length that does not match its color-type contract.
    InvalidTransparencyLength {
        /// PNG color type declared by `IHDR`.
        color_type: u8,
        /// Actual `tRNS` payload length.
        length: usize,
    },
    /// A grayscale or truecolor transparency key exceeds the declared sample depth.
    TransparencySampleOutOfRange {
        /// Invalid original sample value.
        sample: u16,
        /// PNG sample bit depth declared by `IHDR`.
        bit_depth: u8,
    },
    /// An indexed pixel references a palette entry that does not exist.
    PaletteIndexOutOfRange {
        /// Invalid palette index.
        index: u8,
        /// Final destination pixel column.
        x: u32,
        /// Final destination pixel row.
        y: u32,
    },
    /// Reconstructed packed bytes do not match their declared pass layout.
    PackedSampleLayoutMismatch,
    /// Adam7 or non-interlaced pass coverage did not write every destination pixel exactly once.
    PixelCoverageMismatch {
        /// Number of destination pixels declared by `IHDR`.
        expected: u64,
        /// Number of pixels produced by the pass layout.
        actual: u64,
    },
    /// A structurally validated chunk stream could not be re-walked for palette metadata.
    InternalChunkMismatch,
}

impl fmt::Display for PngSampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputTooLarge { expected } => write!(
                formatter,
                "PNG requires {expected} RGBA8 bytes, exceeding the {MAX_RGBA8_BYTES}-byte normalization limit"
            ),
            Self::DuplicateTransparency => {
                formatter.write_str("PNG contains more than one tRNS chunk")
            }
            Self::TransparencyAfterImageData => {
                formatter.write_str("PNG tRNS chunk must appear before the first IDAT chunk")
            }
            Self::TransparencyBeforePalette => {
                formatter.write_str("PNG indexed tRNS chunk must appear after PLTE")
            }
            Self::TransparencyForbidden { color_type } => write!(
                formatter,
                "PNG color type {color_type} includes alpha and must not contain tRNS"
            ),
            Self::InvalidTransparencyLength { color_type, length } => write!(
                formatter,
                "PNG tRNS length {length} is invalid for color type {color_type}"
            ),
            Self::TransparencySampleOutOfRange { sample, bit_depth } => write!(
                formatter,
                "PNG tRNS sample {sample} exceeds the {bit_depth}-bit source range"
            ),
            Self::PaletteIndexOutOfRange { index, x, y } => write!(
                formatter,
                "PNG palette index {index} at pixel ({x}, {y}) has no PLTE entry"
            ),
            Self::PackedSampleLayoutMismatch => formatter.write_str(
                "PNG reconstructed packed samples do not match the declared pass layout",
            ),
            Self::PixelCoverageMismatch { expected, actual } => write!(
                formatter,
                "PNG pass layout produced {actual} pixels, but IHDR declares {expected}"
            ),
            Self::InternalChunkMismatch => formatter.write_str(
                "PNG chunk stream changed unexpectedly while reading palette or transparency data",
            ),
        }
    }
}

impl Error for PngSampleError {}

#[derive(Debug, Clone, Copy)]
enum Transparency<'a> {
    None,
    Grayscale(u16),
    Truecolor([u16; 3]),
    Indexed(&'a [u8]),
}

#[derive(Debug, Clone, Copy)]
struct SourceTables<'a> {
    palette: Option<&'a [u8]>,
    transparency: Transparency<'a>,
    transparency_entry_count: u16,
    color_description: PngColorDescription,
}

#[derive(Debug, Clone, Copy, Default)]
struct AlphaCounts {
    opaque: u64,
    transparent: u64,
    translucent: u64,
}

/// Validates, reconstructs, and expands a PNG into deterministic PNG-encoded RGBA8 bytes.
///
/// The output is not color managed or composited. It represents a documented transformation of
/// PNG source code values and remains distinct from final display appearance.
///
/// # Errors
///
/// Returns existing PNG structure, inflation, and filter errors unchanged. Returns a wrapped
/// [`PngSampleError`] for invalid palette/transparency contracts, unsafe destination size,
/// impossible palette indices, or disagreement between reconstructed pass metadata and samples.
pub fn normalize_png_rgba8(input: &[u8]) -> Result<NormalizedPng, PngAdapterError> {
    let reconstructed = reconstruct_png_filters(input)?;
    let header = reconstructed.structure.header;
    let tables = source_tables(input, header).map_err(PngAdapterError::InvalidSampleData)?;

    let rgba8_length = rgba8_length(header).map_err(PngAdapterError::InvalidSampleData)?;
    let mut rgba8 = vec![0_u8; rgba8_length];
    let counts = scatter_pixels(&reconstructed, tables, &mut rgba8)
        .map_err(PngAdapterError::InvalidSampleData)?;
    let rgba8_crc32 = crc32(&rgba8);
    let palette_entry_count = tables.palette.map_or(0, |palette| {
        u16::try_from(palette.len() / 3).expect("validated PNG palette fits u16")
    });

    Ok(NormalizedPng {
        reconstructed,
        rgba8,
        rgba8_crc32,
        opaque_pixel_count: counts.opaque,
        transparent_pixel_count: counts.transparent,
        translucent_pixel_count: counts.translucent,
        palette_entry_count,
        transparency_entry_count: tables.transparency_entry_count,
        color_description: tables.color_description,
    })
}

fn rgba8_length(header: PngHeader) -> Result<usize, PngSampleError> {
    let expected = u64::from(header.width)
        .checked_mul(u64::from(header.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(PngSampleError::OutputTooLarge { expected: u64::MAX })?;
    if expected > MAX_RGBA8_BYTES {
        return Err(PngSampleError::OutputTooLarge { expected });
    }
    usize::try_from(expected).map_err(|_| PngSampleError::OutputTooLarge { expected })
}

fn source_tables(input: &[u8], header: PngHeader) -> Result<SourceTables<'_>, PngSampleError> {
    let mut offset = 8_usize;
    let mut palette = None;
    let mut transparency_data = None;
    let mut saw_idat = false;
    let mut color_description = PngColorDescription::default();

    while offset < input.len() {
        let length_slice = input
            .get(offset..offset + 4)
            .ok_or(PngSampleError::InternalChunkMismatch)?;
        let length = u32::from_be_bytes(
            length_slice
                .try_into()
                .map_err(|_| PngSampleError::InternalChunkMismatch)?,
        );
        let data_length =
            usize::try_from(length).map_err(|_| PngSampleError::InternalChunkMismatch)?;
        let kind = input
            .get(offset + 4..offset + 8)
            .ok_or(PngSampleError::InternalChunkMismatch)?;
        let data_start = offset
            .checked_add(8)
            .ok_or(PngSampleError::InternalChunkMismatch)?;
        let data_end = data_start
            .checked_add(data_length)
            .ok_or(PngSampleError::InternalChunkMismatch)?;
        let next = data_end
            .checked_add(4)
            .ok_or(PngSampleError::InternalChunkMismatch)?;
        let data = input
            .get(data_start..data_end)
            .ok_or(PngSampleError::InternalChunkMismatch)?;

        match kind {
            b"PLTE" => palette = Some(data),
            b"tRNS" => {
                if transparency_data.is_some() {
                    return Err(PngSampleError::DuplicateTransparency);
                }
                if saw_idat {
                    return Err(PngSampleError::TransparencyAfterImageData);
                }
                if header.color_type == 3 && palette.is_none() {
                    return Err(PngSampleError::TransparencyBeforePalette);
                }
                transparency_data = Some(data);
            }
            b"IDAT" => saw_idat = true,
            b"sRGB" => color_description.has_srgb = true,
            b"gAMA" => color_description.has_gamma = true,
            b"cHRM" => color_description.has_chromaticities = true,
            b"iCCP" => color_description.has_icc_profile = true,
            b"IEND" => break,
            _ => {}
        }
        offset = next;
    }

    let (transparency, transparency_entry_count) =
        validate_transparency(header, palette, transparency_data)?;
    Ok(SourceTables {
        palette,
        transparency,
        transparency_entry_count,
        color_description,
    })
}

fn validate_transparency<'a>(
    header: PngHeader,
    palette: Option<&'a [u8]>,
    data: Option<&'a [u8]>,
) -> Result<(Transparency<'a>, u16), PngSampleError> {
    let Some(data) = data else {
        return Ok((Transparency::None, 0));
    };

    match header.color_type {
        0 => {
            if data.len() != 2 {
                return Err(PngSampleError::InvalidTransparencyLength {
                    color_type: header.color_type,
                    length: data.len(),
                });
            }
            let sample = u16::from_be_bytes([data[0], data[1]]);
            validate_transparency_sample(sample, header.bit_depth)?;
            Ok((Transparency::Grayscale(sample), 1))
        }
        2 => {
            if data.len() != 6 {
                return Err(PngSampleError::InvalidTransparencyLength {
                    color_type: header.color_type,
                    length: data.len(),
                });
            }
            let samples = [
                u16::from_be_bytes([data[0], data[1]]),
                u16::from_be_bytes([data[2], data[3]]),
                u16::from_be_bytes([data[4], data[5]]),
            ];
            for sample in samples {
                validate_transparency_sample(sample, header.bit_depth)?;
            }
            Ok((Transparency::Truecolor(samples), 1))
        }
        3 => {
            let palette = palette.ok_or(PngSampleError::TransparencyBeforePalette)?;
            let palette_entries = palette.len() / 3;
            if data.is_empty() || data.len() > palette_entries {
                return Err(PngSampleError::InvalidTransparencyLength {
                    color_type: header.color_type,
                    length: data.len(),
                });
            }
            let count = u16::try_from(data.len()).expect("validated alpha table fits u16");
            Ok((Transparency::Indexed(data), count))
        }
        4 | 6 => Err(PngSampleError::TransparencyForbidden {
            color_type: header.color_type,
        }),
        _ => unreachable!("validated PNG color type"),
    }
}

fn validate_transparency_sample(sample: u16, bit_depth: u8) -> Result<(), PngSampleError> {
    let maximum = if bit_depth == 16 {
        u16::MAX
    } else {
        (1_u16 << bit_depth) - 1
    };
    if sample > maximum {
        return Err(PngSampleError::TransparencySampleOutOfRange { sample, bit_depth });
    }
    Ok(())
}

fn scatter_pixels(
    reconstructed: &ReconstructedPng,
    tables: SourceTables<'_>,
    output: &mut [u8],
) -> Result<AlphaCounts, PngSampleError> {
    let header = reconstructed.structure.header;
    let mut counts = AlphaCounts::default();
    let mut written_pixels = 0_u64;

    for pass in &reconstructed.passes {
        let row_bytes = usize::try_from(pass.row_bytes)
            .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
        let sample_offset = usize::try_from(pass.sample_offset)
            .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
        let sample_length = usize::try_from(pass.sample_length)
            .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
        let expected_length = row_bytes
            .checked_mul(
                usize::try_from(pass.height)
                    .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?,
            )
            .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
        if sample_length != expected_length {
            return Err(PngSampleError::PackedSampleLayoutMismatch);
        }
        let pass_end = sample_offset
            .checked_add(sample_length)
            .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
        let pass_bytes = reconstructed
            .packed_sample_bytes
            .get(sample_offset..pass_end)
            .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;

        for row_index in 0..pass.height {
            let row_start = usize::try_from(row_index)
                .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?
                .checked_mul(row_bytes)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let row = pass_bytes
                .get(row_start..row_start + row_bytes)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let y = pass
                .start_y
                .checked_add(row_index.saturating_mul(pass.step_y))
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;

            for column in 0..pass.width {
                let x = pass
                    .start_x
                    .checked_add(column.saturating_mul(pass.step_x))
                    .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
                let pixel = decode_pixel(row, column, header, tables, x, y)?;
                write_pixel(output, header.width, x, y, pixel)?;
                counts.record(pixel[3]);
                written_pixels += 1;
            }
        }
    }

    let expected_pixels = u64::from(header.width) * u64::from(header.height);
    if written_pixels != expected_pixels {
        return Err(PngSampleError::PixelCoverageMismatch {
            expected: expected_pixels,
            actual: written_pixels,
        });
    }
    Ok(counts)
}

fn decode_pixel(
    row: &[u8],
    column: u32,
    header: PngHeader,
    tables: SourceTables<'_>,
    x: u32,
    y: u32,
) -> Result<[u8; 4], PngSampleError> {
    let column = usize::try_from(column).map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
    match header.color_type {
        0 => {
            let gray = sample_at(row, column, header.bit_depth)?;
            let alpha = match tables.transparency {
                Transparency::Grayscale(key) if gray == key => 0,
                _ => 255,
            };
            let gray = quantize_sample(gray, header.bit_depth);
            Ok([gray, gray, gray, alpha])
        }
        2 => {
            let base = column
                .checked_mul(3)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let samples = [
                sample_at(row, base, header.bit_depth)?,
                sample_at(row, base + 1, header.bit_depth)?,
                sample_at(row, base + 2, header.bit_depth)?,
            ];
            let alpha = match tables.transparency {
                Transparency::Truecolor(key) if samples == key => 0,
                _ => 255,
            };
            Ok([
                quantize_sample(samples[0], header.bit_depth),
                quantize_sample(samples[1], header.bit_depth),
                quantize_sample(samples[2], header.bit_depth),
                alpha,
            ])
        }
        3 => {
            let raw_index = sample_at(row, column, header.bit_depth)?;
            let index = u8::try_from(raw_index)
                .map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
            let palette = tables
                .palette
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let palette_offset = usize::from(index)
                .checked_mul(3)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let color = palette.get(palette_offset..palette_offset + 3).ok_or(
                PngSampleError::PaletteIndexOutOfRange { index, x, y },
            )?;
            let alpha = match tables.transparency {
                Transparency::Indexed(values) => {
                    values.get(usize::from(index)).copied().unwrap_or(255)
                }
                _ => 255,
            };
            Ok([color[0], color[1], color[2], alpha])
        }
        4 => {
            let base = column
                .checked_mul(2)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let gray = sample_at(row, base, header.bit_depth)?;
            let alpha = sample_at(row, base + 1, header.bit_depth)?;
            let gray = quantize_sample(gray, header.bit_depth);
            Ok([gray, gray, gray, quantize_sample(alpha, header.bit_depth)])
        }
        6 => {
            let base = column
                .checked_mul(4)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            Ok([
                quantize_sample(sample_at(row, base, header.bit_depth)?, header.bit_depth),
                quantize_sample(sample_at(row, base + 1, header.bit_depth)?, header.bit_depth),
                quantize_sample(sample_at(row, base + 2, header.bit_depth)?, header.bit_depth),
                quantize_sample(sample_at(row, base + 3, header.bit_depth)?, header.bit_depth),
            ])
        }
        _ => unreachable!("validated PNG color type"),
    }
}

fn sample_at(row: &[u8], sample_index: usize, bit_depth: u8) -> Result<u16, PngSampleError> {
    match bit_depth {
        1 | 2 | 4 => {
            let depth = usize::from(bit_depth);
            let bit_offset = sample_index
                .checked_mul(depth)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let byte = row
                .get(bit_offset / 8)
                .copied()
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let within_byte = bit_offset % 8;
            let shift = 8_usize
                .checked_sub(depth + within_byte)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let mask = u8::try_from((1_u16 << bit_depth) - 1)
                .expect("sub-byte PNG sample mask fits u8");
            Ok(u16::from((byte >> shift) & mask))
        }
        8 => row
            .get(sample_index)
            .copied()
            .map(u16::from)
            .ok_or(PngSampleError::PackedSampleLayoutMismatch),
        16 => {
            let offset = sample_index
                .checked_mul(2)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            let bytes = row
                .get(offset..offset + 2)
                .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
            Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
        }
        _ => unreachable!("validated PNG bit depth"),
    }
}

fn quantize_sample(sample: u16, bit_depth: u8) -> u8 {
    match bit_depth {
        1 | 2 | 4 => {
            let maximum = (1_u32 << bit_depth) - 1;
            let scaled = u32::from(sample) * 255 / maximum;
            u8::try_from(scaled).expect("scaled sub-byte sample fits u8")
        }
        8 => u8::try_from(sample).expect("8-bit sample fits u8"),
        16 => {
            let scaled = (u32::from(sample) * 255 + 32_767) / 65_535;
            u8::try_from(scaled).expect("quantized 16-bit sample fits u8")
        }
        _ => unreachable!("validated PNG bit depth"),
    }
}

fn write_pixel(
    output: &mut [u8],
    width: u32,
    x: u32,
    y: u32,
    pixel: [u8; 4],
) -> Result<(), PngSampleError> {
    let pixel_index = u64::from(y)
        .checked_mul(u64::from(width))
        .and_then(|row| row.checked_add(u64::from(x)))
        .and_then(|index| index.checked_mul(4))
        .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
    let offset =
        usize::try_from(pixel_index).map_err(|_| PngSampleError::PackedSampleLayoutMismatch)?;
    let destination = output
        .get_mut(offset..offset + 4)
        .ok_or(PngSampleError::PackedSampleLayoutMismatch)?;
    destination.copy_from_slice(&pixel);
    Ok(())
}

impl AlphaCounts {
    fn record(&mut self, alpha: u8) {
        match alpha {
            0 => self.transparent += 1,
            255 => self.opaque += 1,
            _ => self.translucent += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{quantize_sample, sample_at, validate_transparency_sample};

    #[test]
    fn unpacks_most_significant_sub_byte_samples() {
        let row = [0b_10_01_11_00];
        assert_eq!(sample_at(&row, 0, 2).expect("sample"), 2);
        assert_eq!(sample_at(&row, 1, 2).expect("sample"), 1);
        assert_eq!(sample_at(&row, 2, 2).expect("sample"), 3);
        assert_eq!(sample_at(&row, 3, 2).expect("sample"), 0);
    }

    #[test]
    fn quantization_has_documented_endpoints_and_rounding() {
        assert_eq!(quantize_sample(0, 1), 0);
        assert_eq!(quantize_sample(1, 1), 255);
        assert_eq!(quantize_sample(1, 2), 85);
        assert_eq!(quantize_sample(15, 4), 255);
        assert_eq!(quantize_sample(128, 8), 128);
        assert_eq!(quantize_sample(0, 16), 0);
        assert_eq!(quantize_sample(32_768, 16), 128);
        assert_eq!(quantize_sample(u16::MAX, 16), 255);
    }

    #[test]
    fn transparency_keys_are_checked_at_source_depth() {
        assert!(validate_transparency_sample(15, 4).is_ok());
        assert!(validate_transparency_sample(16, 4).is_err());
        assert!(validate_transparency_sample(u16::MAX, 16).is_ok());
    }
}
