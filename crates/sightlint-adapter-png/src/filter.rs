use std::error::Error;
use std::fmt;

use crate::{InflatedPng, PngAdapterError, PngHeader, PngStructure, crc32, inflate_png_scanlines};

const ADAM7_PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// Exact layout of one non-empty reconstructed PNG pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngPassLayout {
    /// Zero for a non-interlaced image, or the one-based Adam7 pass number.
    pub pass: u8,
    /// First destination pixel column represented by the pass.
    pub start_x: u32,
    /// First destination pixel row represented by the pass.
    pub start_y: u32,
    /// Destination-pixel column stride.
    pub step_x: u32,
    /// Destination-pixel row stride.
    pub step_y: u32,
    /// Number of pixels encoded by each pass row.
    pub width: u32,
    /// Number of rows encoded by the pass.
    pub height: u32,
    /// Packed sample bytes in each reconstructed pass row.
    pub row_bytes: u64,
    /// Filter predictor byte width for this color type and bit depth.
    pub bytes_per_pixel: u8,
    /// Byte offset into [`ReconstructedPng::packed_sample_bytes`].
    pub sample_offset: u64,
    /// Byte length within [`ReconstructedPng::packed_sample_bytes`].
    pub sample_length: u64,
}

/// PNG data after deterministic filter reconstruction but before sample unpacking or color expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedPng {
    /// Exact structural metadata validated before inflation and reconstruction.
    pub structure: PngStructure,
    /// Packed, unfiltered sample bytes concatenated in pass and row order.
    pub packed_sample_bytes: Vec<u8>,
    /// Layout descriptors for the one non-interlaced pass or each non-empty Adam7 pass.
    pub passes: Vec<PngPassLayout>,
    /// Number of scanlines that used filters None, Sub, Up, Average, and Paeth respectively.
    pub filter_counts: [u64; 5],
    /// Deterministic CRC-32 of `packed_sample_bytes` for differential verification.
    pub packed_sample_crc32: u32,
}

/// Failure while reconstructing PNG scanline filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngFilterError {
    /// A scanline selected a filter outside the five methods defined by PNG.
    InvalidFilterType {
        /// Invalid selector byte.
        filter: u8,
        /// Zero for non-interlaced data, or the one-based Adam7 pass number.
        pass: u8,
        /// Zero-based row within the pass.
        row: u32,
    },
    /// Inflated bytes do not match the layout calculated from the validated header.
    ScanlineLayoutMismatch {
        /// Byte count required by the calculated pass layout.
        expected: usize,
        /// Byte count supplied by the inflation stage.
        actual: usize,
    },
}

impl fmt::Display for PngFilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilterType { filter, pass, row } => write!(
                formatter,
                "PNG filter type {filter} is invalid at pass {pass}, row {row}"
            ),
            Self::ScanlineLayoutMismatch { expected, actual } => write!(
                formatter,
                "PNG inflated scanline layout requires {expected} bytes, but received {actual}"
            ),
        }
    }
}

impl Error for PngFilterError {}

#[derive(Debug, Clone, Copy)]
struct PassPlan {
    pass: u8,
    start_x: u32,
    start_y: u32,
    step_x: u32,
    step_y: u32,
    width: u32,
    height: u32,
    row_bytes: usize,
    bytes_per_pixel: usize,
}

/// Validates, inflates, and reconstructs PNG scanline filters.
///
/// Returned samples remain packed according to the PNG bit depth and color type. Palette lookup,
/// transparency, channel expansion, color management, and visual observations are deliberately
/// deferred to later adapter stages.
///
/// # Errors
///
/// Returns existing header, structure, and inflation errors unchanged. Returns a wrapped
/// [`PngFilterError`] when a scanline selects an invalid filter or the inflation and pass-layout
/// stages disagree about byte counts.
pub fn reconstruct_png_filters(input: &[u8]) -> Result<ReconstructedPng, PngAdapterError> {
    let inflated = inflate_png_scanlines(input)?;
    reconstruct_inflated(inflated).map_err(PngAdapterError::InvalidFilterData)
}

fn reconstruct_inflated(mut inflated: InflatedPng) -> Result<ReconstructedPng, PngFilterError> {
    let plans = pass_plans(inflated.structure.header);
    let expected = expected_filtered_bytes(&plans);
    if inflated.scanline_bytes.len() != expected {
        return Err(PngFilterError::ScanlineLayoutMismatch {
            expected,
            actual: inflated.scanline_bytes.len(),
        });
    }

    let mut source_cursor = 0_usize;
    let mut destination_cursor = 0_usize;
    let mut layouts = Vec::with_capacity(plans.len());
    let mut filter_counts = [0_u64; 5];

    for plan in plans {
        let sample_offset = destination_cursor;
        reconstruct_pass(
            &mut inflated.scanline_bytes,
            plan,
            &mut source_cursor,
            &mut destination_cursor,
            &mut filter_counts,
        )?;
        layouts.push(public_layout(
            plan,
            sample_offset,
            destination_cursor - sample_offset,
        ));
    }

    if source_cursor != expected {
        return Err(PngFilterError::ScanlineLayoutMismatch {
            expected,
            actual: source_cursor,
        });
    }
    inflated.scanline_bytes.truncate(destination_cursor);
    let packed_sample_crc32 = crc32(&inflated.scanline_bytes);

    Ok(ReconstructedPng {
        structure: inflated.structure,
        packed_sample_bytes: inflated.scanline_bytes,
        passes: layouts,
        filter_counts,
        packed_sample_crc32,
    })
}

fn reconstruct_pass(
    bytes: &mut [u8],
    plan: PassPlan,
    source_cursor: &mut usize,
    destination_cursor: &mut usize,
    filter_counts: &mut [u64; 5],
) -> Result<(), PngFilterError> {
    for row in 0..plan.height {
        let filter = bytes[*source_cursor];
        *source_cursor += 1;
        if filter > 4 {
            return Err(PngFilterError::InvalidFilterType {
                filter,
                pass: plan.pass,
                row,
            });
        }
        filter_counts[usize::from(filter)] += 1;

        reconstruct_row(
            bytes,
            *source_cursor,
            *destination_cursor,
            plan.row_bytes,
            plan.bytes_per_pixel,
            row > 0,
            filter,
        );
        *source_cursor += plan.row_bytes;
        *destination_cursor += plan.row_bytes;
    }
    Ok(())
}

fn reconstruct_row(
    bytes: &mut [u8],
    source_start: usize,
    destination_start: usize,
    row_bytes: usize,
    bytes_per_pixel: usize,
    has_previous_row: bool,
    filter: u8,
) {
    let mut column = 0_usize;
    while column < row_bytes {
        let filtered = bytes[source_start + column];
        let left = if column >= bytes_per_pixel {
            bytes[destination_start + column - bytes_per_pixel]
        } else {
            0
        };
        let above = if has_previous_row {
            bytes[destination_start + column - row_bytes]
        } else {
            0
        };
        let upper_left = if has_previous_row && column >= bytes_per_pixel {
            bytes[destination_start + column - row_bytes - bytes_per_pixel]
        } else {
            0
        };
        let predictor = predictor(filter, left, above, upper_left);
        bytes[destination_start + column] = filtered.wrapping_add(predictor);
        column += 1;
    }
}

fn predictor(filter: u8, left: u8, above: u8, upper_left: u8) -> u8 {
    match filter {
        0 => 0,
        1 => left,
        2 => above,
        3 => {
            let sum = u16::from(left) + u16::from(above);
            u8::try_from(sum / 2).expect("average of two bytes fits u8")
        }
        4 => paeth(left, above, upper_left),
        _ => unreachable!("filter selector validated before reconstruction"),
    }
}

fn paeth(left: u8, above: u8, upper_left: u8) -> u8 {
    let left = i32::from(left);
    let above = i32::from(above);
    let upper_left = i32::from(upper_left);
    let estimate = left + above - upper_left;
    let left_distance = (estimate - left).abs();
    let above_distance = (estimate - above).abs();
    let upper_left_distance = (estimate - upper_left).abs();

    if left_distance <= above_distance && left_distance <= upper_left_distance {
        u8::try_from(left).expect("source byte fits u8")
    } else if above_distance <= upper_left_distance {
        u8::try_from(above).expect("source byte fits u8")
    } else {
        u8::try_from(upper_left).expect("source byte fits u8")
    }
}

fn pass_plans(header: PngHeader) -> Vec<PassPlan> {
    let bits_per_pixel = u64::from(channel_count(header.color_type)) * u64::from(header.bit_depth);
    let bytes_per_pixel = usize::try_from(bits_per_pixel.div_ceil(8).max(1))
        .expect("validated PNG pixel width fits usize");

    if header.interlace_method == 0 {
        return vec![make_plan(
            0,
            (0, 0, 1, 1),
            header.width,
            header.height,
            bits_per_pixel,
            bytes_per_pixel,
        )];
    }

    ADAM7_PASSES
        .into_iter()
        .enumerate()
        .filter_map(|(index, geometry)| {
            let width = pass_extent(header.width, geometry.0, geometry.2);
            let height = pass_extent(header.height, geometry.1, geometry.3);
            if width == 0 || height == 0 {
                None
            } else {
                Some(make_plan(
                    u8::try_from(index + 1).expect("Adam7 pass index fits u8"),
                    geometry,
                    width,
                    height,
                    bits_per_pixel,
                    bytes_per_pixel,
                ))
            }
        })
        .collect()
}

fn make_plan(
    pass: u8,
    geometry: (u32, u32, u32, u32),
    width: u32,
    height: u32,
    bits_per_pixel: u64,
    bytes_per_pixel: usize,
) -> PassPlan {
    PassPlan {
        pass,
        start_x: geometry.0,
        start_y: geometry.1,
        step_x: geometry.2,
        step_y: geometry.3,
        width,
        height,
        row_bytes: usize::try_from(packed_row_bytes(width, bits_per_pixel))
            .expect("bounded decoded row fits usize"),
        bytes_per_pixel,
    }
}

fn public_layout(plan: PassPlan, sample_offset: usize, sample_length: usize) -> PngPassLayout {
    PngPassLayout {
        pass: plan.pass,
        start_x: plan.start_x,
        start_y: plan.start_y,
        step_x: plan.step_x,
        step_y: plan.step_y,
        width: plan.width,
        height: plan.height,
        row_bytes: u64::try_from(plan.row_bytes).expect("bounded row width fits u64"),
        bytes_per_pixel: u8::try_from(plan.bytes_per_pixel)
            .expect("PNG filter byte width fits u8"),
        sample_offset: u64::try_from(sample_offset).expect("bounded sample offset fits u64"),
        sample_length: u64::try_from(sample_length).expect("bounded sample length fits u64"),
    }
}

fn expected_filtered_bytes(plans: &[PassPlan]) -> usize {
    plans
        .iter()
        .map(|plan| {
            usize::try_from(plan.height).expect("bounded height fits usize") * (1 + plan.row_bytes)
        })
        .sum()
}

fn packed_row_bytes(width: u32, bits_per_pixel: u64) -> u64 {
    (u64::from(width) * bits_per_pixel).div_ceil(8)
}

fn pass_extent(size: u32, start: u32, step: u32) -> u32 {
    if size <= start {
        0
    } else {
        1 + (size - start - 1) / step
    }
}

fn channel_count(color_type: u8) -> u8 {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!("validated PNG color type"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PngFilterError, channel_count, pass_plans, predictor, reconstruct_inflated,
    };
    use crate::{InflatedPng, PngHeader, PngStructure, expected_scanline_bytes};

    fn header(width: u32, height: u32, bit_depth: u8, color_type: u8, interlace: u8) -> PngHeader {
        PngHeader {
            width,
            height,
            bit_depth,
            color_type,
            compression_method: 0,
            filter_method: 0,
            interlace_method: interlace,
        }
    }

    fn inflated(header: PngHeader, scanline_bytes: Vec<u8>) -> InflatedPng {
        InflatedPng {
            structure: PngStructure {
                header,
                chunk_count: 3,
                idat_chunk_count: 1,
                idat_bytes: 0,
                has_palette: header.color_type == 3,
            },
            scanline_bytes,
        }
    }

    fn encode_row(filter: u8, row: &[u8], previous: Option<&[u8]>, bytes_per_pixel: usize) -> Vec<u8> {
        row.iter()
            .enumerate()
            .map(|(column, &value)| {
                let left = column
                    .checked_sub(bytes_per_pixel)
                    .map_or(0, |index| row[index]);
                let above = previous.map_or(0, |prior| prior[column]);
                let upper_left = previous.map_or(0, |prior| {
                    column
                        .checked_sub(bytes_per_pixel)
                        .map_or(0, |index| prior[index])
                });
                value.wrapping_sub(predictor(filter, left, above, upper_left))
            })
            .collect()
    }

    #[test]
    fn reconstructs_all_five_filters_with_wrapping_arithmetic() {
        let rows = [
            vec![250, 2, 200, 17],
            vec![1, 255, 3, 240],
            vec![70, 8, 250, 4],
            vec![255, 0, 129, 33],
            vec![4, 244, 1, 222],
        ];
        let mut scanlines = Vec::new();
        for (filter, row) in (0_u8..=4).zip(rows.iter()) {
            scanlines.push(filter);
            let previous = usize::from(filter)
                .checked_sub(1)
                .map(|index| rows[index].as_slice());
            scanlines.extend_from_slice(&encode_row(filter, row, previous, 1));
        }

        let reconstructed = reconstruct_inflated(inflated(header(4, 5, 8, 0, 0), scanlines))
            .expect("all PNG filters reconstruct");
        let expected: Vec<u8> = rows.into_iter().flatten().collect();
        assert_eq!(reconstructed.packed_sample_bytes, expected);
        assert_eq!(reconstructed.filter_counts, [1, 1, 1, 1, 1]);
        assert_eq!(reconstructed.passes.len(), 1);
    }

    #[test]
    fn filter_byte_width_covers_packed_and_sixteen_bit_samples() {
        let packed = pass_plans(header(8, 1, 1, 0, 0));
        assert_eq!(packed[0].bytes_per_pixel, 1);
        assert_eq!(packed[0].row_bytes, 1);

        let rgba16 = pass_plans(header(2, 1, 16, 6, 0));
        assert_eq!(rgba16[0].bytes_per_pixel, 8);
        assert_eq!(rgba16[0].row_bytes, 16);
        assert_eq!(channel_count(6), 4);
    }

    #[test]
    fn rejects_unknown_filter_with_pass_and_row_location() {
        let error = reconstruct_inflated(inflated(header(1, 1, 8, 0, 0), vec![5, 0]))
            .expect_err("filter 5 is undefined");
        assert_eq!(
            error,
            PngFilterError::InvalidFilterType {
                filter: 5,
                pass: 0,
                row: 0,
            }
        );
    }

    #[test]
    fn pass_layout_matches_inflater_size_for_legal_header_matrix() {
        for (width, height, depth, color_type, interlace) in [
            (8, 2, 1, 0, 0),
            (7, 3, 4, 3, 0),
            (3, 2, 8, 2, 0),
            (2, 2, 16, 4, 0),
            (3, 2, 16, 6, 0),
            (1, 1, 8, 6, 1),
            (8, 8, 8, 6, 1),
            (17, 11, 2, 0, 1),
        ] {
            let header = header(width, height, depth, color_type, interlace);
            let calculated: usize = pass_plans(header)
                .iter()
                .map(|plan| {
                    usize::try_from(plan.height).expect("test height fits usize")
                        * (1 + plan.row_bytes)
                })
                .sum();
            assert_eq!(
                u64::try_from(calculated).expect("test size fits u64"),
                expected_scanline_bytes(header)
            );
        }
    }
}
