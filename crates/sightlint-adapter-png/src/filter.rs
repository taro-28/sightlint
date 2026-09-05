use std::error::Error;
use std::fmt;

use crate::{InflatedPng, PngAdapterError, PngHeader, PngStructure, inflate_png_scanlines};

const ADAM7_PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// Exact geometry and byte layout for one non-empty PNG scanline pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngPass {
    /// One-based pass index. Non-interlaced images use pass one; Adam7 retains indices one to seven.
    pub index: u8,
    /// First destination x coordinate represented by the pass.
    pub start_x: u32,
    /// First destination y coordinate represented by the pass.
    pub start_y: u32,
    /// Destination x stride between samples represented by the pass.
    pub step_x: u32,
    /// Destination y stride between rows represented by the pass.
    pub step_y: u32,
    /// Number of samples represented by each pass row.
    pub width: u32,
    /// Number of rows represented by the pass.
    pub height: u32,
    /// Number of packed sample bytes in each reconstructed pass row.
    pub row_bytes: usize,
    /// Predictor byte distance required by PNG `Sub`, `Average`, and `Paeth` filters.
    pub filter_bytes_per_pixel: usize,
    /// Byte offset of this pass in [`ReconstructedPng::packed_sample_bytes`].
    pub output_offset: usize,
}

/// PNG data after deterministic filter reconstruction but before sample unpacking or Adam7 scatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedPng {
    /// Exact structural metadata validated before decompression and reconstruction.
    pub structure: PngStructure,
    /// Concatenated reconstructed packed rows with every leading filter byte removed.
    pub packed_sample_bytes: Vec<u8>,
    /// Exact geometry for each non-empty scanline pass in file order.
    pub passes: Vec<PngPass>,
}

/// Failure while reconstructing PNG scanline filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngFilterError {
    /// A scanline uses a filter type outside the PNG-defined range zero through four.
    InvalidFilterType {
        /// One-based original PNG pass index.
        pass: u8,
        /// One-based row index within the pass.
        row: u32,
        /// Unsupported filter byte read from the inflated stream.
        filter: u8,
    },
    /// Inflated bytes do not match the scanline layout derived from the validated header.
    ScanlineLayoutMismatch {
        /// Exact byte count required by the derived pass layout.
        expected: usize,
        /// Inflated byte count supplied to the reconstruction stage.
        actual: usize,
    },
    /// Checked arithmetic could not represent an internal pass or output layout.
    LayoutOverflow,
}

impl fmt::Display for PngFilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilterType { pass, row, filter } => write!(
                formatter,
                "PNG pass {pass} row {row} uses invalid filter type {filter}; expected 0 through 4"
            ),
            Self::ScanlineLayoutMismatch { expected, actual } => write!(
                formatter,
                "PNG filter reconstruction expected {expected} inflated scanline bytes but received {actual}"
            ),
            Self::LayoutOverflow => {
                formatter.write_str("PNG scanline layout exceeds the supported address space")
            }
        }
    }
}

impl Error for PngFilterError {}

#[derive(Debug, Clone, Copy)]
struct PassSpec {
    index: u8,
    start_x: u32,
    start_y: u32,
    step_x: u32,
    step_y: u32,
    width: u32,
    height: u32,
    row_bytes: usize,
    filter_bytes_per_pixel: usize,
}

/// Inflates and deterministically reconstructs every PNG scanline filter.
///
/// The returned bytes remain packed according to the source bit depth and color type. Palette
/// expansion, transparency, sample scaling, Adam7 scatter, colors, ink bounds, and semantics are
/// deliberately outside this stage.
///
/// # Errors
///
/// Returns existing PNG header, structure, or inflation errors unchanged. Returns a wrapped
/// [`PngFilterError`] when a scanline contains an invalid filter byte or when the inflated bytes do
/// not match the exact pass layout derived from `IHDR`.
pub fn reconstruct_png_scanlines(input: &[u8]) -> Result<ReconstructedPng, PngAdapterError> {
    let inflated = inflate_png_scanlines(input)?;
    reconstruct_inflated(inflated).map_err(PngAdapterError::InvalidFilterData)
}

fn reconstruct_inflated(inflated: InflatedPng) -> Result<ReconstructedPng, PngFilterError> {
    let pass_specs = pass_specs(inflated.structure.header)?;
    let expected_input = filtered_input_length(&pass_specs)?;
    if inflated.scanline_bytes.len() != expected_input {
        return Err(PngFilterError::ScanlineLayoutMismatch {
            expected: expected_input,
            actual: inflated.scanline_bytes.len(),
        });
    }

    let output_capacity = reconstructed_output_length(&pass_specs)?;
    let mut packed_sample_bytes = Vec::with_capacity(output_capacity);
    let mut passes = Vec::with_capacity(pass_specs.len());
    let mut input_offset = 0_usize;

    for spec in pass_specs {
        let output_offset = packed_sample_bytes.len();
        let mut previous_row_start = None;

        for row_index in 0..spec.height {
            let filter = *inflated
                .scanline_bytes
                .get(input_offset)
                .ok_or(PngFilterError::ScanlineLayoutMismatch {
                    expected: expected_input,
                    actual: input_offset,
                })?;
            input_offset = input_offset
                .checked_add(1)
                .ok_or(PngFilterError::LayoutOverflow)?;
            let row_end = input_offset
                .checked_add(spec.row_bytes)
                .ok_or(PngFilterError::LayoutOverflow)?;
            let filtered_row = inflated.scanline_bytes.get(input_offset..row_end).ok_or(
                PngFilterError::ScanlineLayoutMismatch {
                    expected: expected_input,
                    actual: row_end,
                },
            )?;
            let previous_row = previous_row_start.map(|start| {
                &packed_sample_bytes[start..start + spec.row_bytes]
            });
            let reconstructed_row = reconstruct_row(
                filter,
                filtered_row,
                previous_row,
                spec.filter_bytes_per_pixel,
                spec.index,
                row_index + 1,
            )?;
            previous_row_start = Some(packed_sample_bytes.len());
            packed_sample_bytes.extend_from_slice(&reconstructed_row);
            input_offset = row_end;
        }

        passes.push(PngPass {
            index: spec.index,
            start_x: spec.start_x,
            start_y: spec.start_y,
            step_x: spec.step_x,
            step_y: spec.step_y,
            width: spec.width,
            height: spec.height,
            row_bytes: spec.row_bytes,
            filter_bytes_per_pixel: spec.filter_bytes_per_pixel,
            output_offset,
        });
    }

    if input_offset != inflated.scanline_bytes.len()
        || packed_sample_bytes.len() != output_capacity
    {
        return Err(PngFilterError::ScanlineLayoutMismatch {
            expected: expected_input,
            actual: inflated.scanline_bytes.len(),
        });
    }

    Ok(ReconstructedPng {
        structure: inflated.structure,
        packed_sample_bytes,
        passes,
    })
}

fn reconstruct_row(
    filter: u8,
    filtered: &[u8],
    previous: Option<&[u8]>,
    bytes_per_pixel: usize,
    pass: u8,
    row: u32,
) -> Result<Vec<u8>, PngFilterError> {
    if filter > 4 {
        return Err(PngFilterError::InvalidFilterType { pass, row, filter });
    }
    if previous.is_some_and(|bytes| bytes.len() != filtered.len()) {
        return Err(PngFilterError::ScanlineLayoutMismatch {
            expected: filtered.len(),
            actual: previous.map_or(0, <[u8]>::len),
        });
    }

    let mut reconstructed = vec![0_u8; filtered.len()];
    for (index, &byte) in filtered.iter().enumerate() {
        let left = index
            .checked_sub(bytes_per_pixel)
            .map_or(0, |left_index| reconstructed[left_index]);
        let up = previous.map_or(0, |previous_row| previous_row[index]);
        let upper_left = previous.map_or(0, |previous_row| {
            index
                .checked_sub(bytes_per_pixel)
                .map_or(0, |left_index| previous_row[left_index])
        });
        let predictor = match filter {
            0 => 0,
            1 => left,
            2 => up,
            3 => average(left, up),
            4 => paeth(left, up, upper_left),
            _ => unreachable!("filter range checked before reconstruction"),
        };
        reconstructed[index] = byte.wrapping_add(predictor);
    }
    Ok(reconstructed)
}

fn average(left: u8, up: u8) -> u8 {
    let value = (u16::from(left) + u16::from(up)) / 2;
    u8::try_from(value).expect("average of two bytes fits u8")
}

fn paeth(left: u8, up: u8, upper_left: u8) -> u8 {
    let left = i32::from(left);
    let up = i32::from(up);
    let upper_left = i32::from(upper_left);
    let estimate = left + up - upper_left;
    let left_distance = (estimate - left).abs();
    let up_distance = (estimate - up).abs();
    let upper_left_distance = (estimate - upper_left).abs();

    if left_distance <= up_distance && left_distance <= upper_left_distance {
        u8::try_from(left).expect("source byte fits u8")
    } else if up_distance <= upper_left_distance {
        u8::try_from(up).expect("source byte fits u8")
    } else {
        u8::try_from(upper_left).expect("source byte fits u8")
    }
}

fn pass_specs(header: PngHeader) -> Result<Vec<PassSpec>, PngFilterError> {
    let bits_per_pixel = u64::from(channels(header.color_type)) * u64::from(header.bit_depth);
    let filter_bytes_per_pixel = usize::try_from(bits_per_pixel.div_ceil(8))
        .map_err(|_| PngFilterError::LayoutOverflow)?
        .max(1);

    if header.interlace_method == 0 {
        return Ok(vec![pass_spec(
            1,
            0,
            0,
            1,
            1,
            header.width,
            header.height,
            bits_per_pixel,
            filter_bytes_per_pixel,
        )?]);
    }

    ADAM7_PASSES
        .into_iter()
        .enumerate()
        .filter_map(|(index, (start_x, start_y, step_x, step_y))| {
            let width = pass_extent(header.width, start_x, step_x);
            let height = pass_extent(header.height, start_y, step_y);
            if width == 0 || height == 0 {
                None
            } else {
                Some(pass_spec(
                    u8::try_from(index + 1).expect("Adam7 index fits u8"),
                    start_x,
                    start_y,
                    step_x,
                    step_y,
                    width,
                    height,
                    bits_per_pixel,
                    filter_bytes_per_pixel,
                ))
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn pass_spec(
    index: u8,
    start_x: u32,
    start_y: u32,
    step_x: u32,
    step_y: u32,
    width: u32,
    height: u32,
    bits_per_pixel: u64,
    filter_bytes_per_pixel: usize,
) -> Result<PassSpec, PngFilterError> {
    let row_bytes_u64 = (u64::from(width) * bits_per_pixel).div_ceil(8);
    let row_bytes =
        usize::try_from(row_bytes_u64).map_err(|_| PngFilterError::LayoutOverflow)?;
    Ok(PassSpec {
        index,
        start_x,
        start_y,
        step_x,
        step_y,
        width,
        height,
        row_bytes,
        filter_bytes_per_pixel,
    })
}

fn filtered_input_length(passes: &[PassSpec]) -> Result<usize, PngFilterError> {
    passes.iter().try_fold(0_usize, |total, pass| {
        let row_with_filter = pass
            .row_bytes
            .checked_add(1)
            .ok_or(PngFilterError::LayoutOverflow)?;
        let pass_length = row_with_filter
            .checked_mul(
                usize::try_from(pass.height).map_err(|_| PngFilterError::LayoutOverflow)?,
            )
            .ok_or(PngFilterError::LayoutOverflow)?;
        total
            .checked_add(pass_length)
            .ok_or(PngFilterError::LayoutOverflow)
    })
}

fn reconstructed_output_length(passes: &[PassSpec]) -> Result<usize, PngFilterError> {
    passes.iter().try_fold(0_usize, |total, pass| {
        let pass_length = pass
            .row_bytes
            .checked_mul(
                usize::try_from(pass.height).map_err(|_| PngFilterError::LayoutOverflow)?,
            )
            .ok_or(PngFilterError::LayoutOverflow)?;
        total
            .checked_add(pass_length)
            .ok_or(PngFilterError::LayoutOverflow)
    })
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

#[cfg(test)]
mod tests {
    use super::{PngFilterError, average, paeth, reconstruct_row};

    #[test]
    fn reconstructs_all_five_filters_from_known_rows() {
        assert_eq!(
            reconstruct_row(0, &[10, 20, 30], None, 1, 1, 1).expect("None filter"),
            [10, 20, 30]
        );
        assert_eq!(
            reconstruct_row(1, &[10, 10, 10], None, 1, 1, 1).expect("Sub filter"),
            [10, 20, 30]
        );
        assert_eq!(
            reconstruct_row(2, &[9, 18, 27], Some(&[1, 2, 3]), 1, 1, 2)
                .expect("Up filter"),
            [10, 20, 30]
        );
        assert_eq!(
            reconstruct_row(3, &[9, 13, 17], Some(&[2, 4, 6]), 1, 1, 2)
                .expect("Average filter"),
            [10, 20, 30]
        );
        assert_eq!(
            reconstruct_row(4, &[7, 10, 10], Some(&[3, 9, 12]), 1, 1, 2)
                .expect("Paeth filter"),
            [10, 20, 30]
        );
    }

    #[test]
    fn reconstruction_wraps_modulo_256() {
        assert_eq!(
            reconstruct_row(1, &[250, 10], None, 1, 1, 1).expect("Sub filter"),
            [250, 4]
        );
    }

    #[test]
    fn predictor_helpers_follow_png_integer_rules() {
        assert_eq!(average(1, 2), 1);
        assert_eq!(average(255, 255), 255);
        assert_eq!(paeth(10, 20, 30), 10);
        assert_eq!(paeth(10, 20, 15), 15);
        assert_eq!(paeth(100, 10, 10), 100);
    }

    #[test]
    fn invalid_filter_reports_original_pass_and_row() {
        assert_eq!(
            reconstruct_row(5, &[0], None, 1, 7, 3),
            Err(PngFilterError::InvalidFilterType {
                pass: 7,
                row: 3,
                filter: 5,
            })
        );
    }
}
