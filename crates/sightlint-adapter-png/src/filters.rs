use std::error::Error;
use std::fmt;

use crate::inflate::{ADAM7_PASSES, channels, inflate_png_scanlines, pass_extent};
use crate::{PngAdapterError, PngHeader, PngStructure};

/// Exact counts of filter types reconstructed from non-empty PNG scanlines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PngFilterCounts {
    /// Scanlines encoded with filter type 0 (`None`).
    pub none: u64,
    /// Scanlines encoded with filter type 1 (`Sub`).
    pub sub: u64,
    /// Scanlines encoded with filter type 2 (`Up`).
    pub up: u64,
    /// Scanlines encoded with filter type 3 (`Average`).
    pub average: u64,
    /// Scanlines encoded with filter type 4 (`Paeth`).
    pub paeth: u64,
}

impl PngFilterCounts {
    /// Returns the total number of reconstructed non-empty scanlines.
    pub fn total(self) -> u64 {
        self.none + self.sub + self.up + self.average + self.paeth
    }

    fn record(&mut self, filter: FilterType) {
        match filter {
            FilterType::None => self.none += 1,
            FilterType::Sub => self.sub += 1,
            FilterType::Up => self.up += 1,
            FilterType::Average => self.average += 1,
            FilterType::Paeth => self.paeth += 1,
        }
    }
}

/// Geometry and byte range for one reconstructed PNG transmission pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngPass {
    /// Zero for a non-interlaced image, or one through seven for Adam7.
    pub number: u8,
    /// First full-image x coordinate represented by the pass.
    pub start_x: u32,
    /// First full-image y coordinate represented by the pass.
    pub start_y: u32,
    /// Horizontal full-image step between pass pixels.
    pub step_x: u32,
    /// Vertical full-image step between pass scanlines.
    pub step_y: u32,
    /// Number of pixels in each non-empty pass scanline.
    pub width: u32,
    /// Number of scanlines in the pass geometry.
    pub height: u32,
    /// Number of reconstructed packed bytes in each non-empty pass scanline.
    pub row_bytes: usize,
    /// Offset of this pass in [`ReconstructedPng::reconstructed_bytes`].
    pub data_offset: usize,
    /// Number of reconstructed bytes contributed by this pass.
    pub data_length: usize,
}

impl PngPass {
    /// Returns whether the pass contributes scanlines and reconstructed data.
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Validated PNG data after reversing scanline filters but before interpreting samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconstructedPng {
    /// Exact structure validated before inflation and filter reconstruction.
    pub structure: PngStructure,
    /// Reconstructed packed row bytes in PNG transmission order, without filter prefixes.
    pub reconstructed_bytes: Vec<u8>,
    /// Non-interlaced or Adam7 pass geometry and byte ranges, including empty passes.
    pub passes: Vec<PngPass>,
    /// Exact counts of the five filter types encountered.
    pub filter_counts: PngFilterCounts,
}

/// Failure while reversing PNG filter method 0 scanlines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngFilterError {
    /// A non-empty scanline declares a filter type outside the defined range zero through four.
    InvalidFilterType {
        /// Unsupported filter byte from the decompressed stream.
        filter_type: u8,
        /// Zero for non-interlaced data, or one through seven for Adam7.
        pass: u8,
        /// Zero-based row within the pass.
        row: u32,
    },
    /// Validated metadata and decompressed bytes disagreed about scanline layout.
    InternalScanlineLayout,
}

impl fmt::Display for PngFilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilterType {
                filter_type,
                pass,
                row,
            } => write!(
                formatter,
                "PNG scanline filter type {filter_type} is invalid at pass {pass}, row {row}"
            ),
            Self::InternalScanlineLayout => formatter.write_str(
                "PNG filtered scanline layout does not match the validated IHDR geometry",
            ),
        }
    }
}

impl Error for PngFilterError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterType {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl FilterType {
    fn parse(filter_type: u8, pass: u8, row: u32) -> Result<Self, PngFilterError> {
        match filter_type {
            0 => Ok(Self::None),
            1 => Ok(Self::Sub),
            2 => Ok(Self::Up),
            3 => Ok(Self::Average),
            4 => Ok(Self::Paeth),
            _ => Err(PngFilterError::InvalidFilterType {
                filter_type,
                pass,
                row,
            }),
        }
    }

    fn reconstruct(self, filtered: u8, left: u8, above: u8, upper_left: u8) -> u8 {
        let predictor = match self {
            Self::None => 0,
            Self::Sub => left,
            Self::Up => above,
            Self::Average => {
                let average = (u16::from(left) + u16::from(above)) / 2;
                u8::try_from(average).expect("average of two bytes fits u8")
            }
            Self::Paeth => paeth_predictor(left, above, upper_left),
        };
        filtered.wrapping_add(predictor)
    }
}

#[derive(Debug, Clone, Copy)]
struct PassLayout {
    number: u8,
    start_x: u32,
    start_y: u32,
    step_x: u32,
    step_y: u32,
    width: u32,
    height: u32,
    row_bytes: usize,
    data_length: usize,
}

struct ReconstructionState {
    bytes: Vec<u8>,
    read_cursor: usize,
    write_cursor: usize,
    bytes_per_pixel: usize,
    filter_counts: PngFilterCounts,
}

impl ReconstructionState {
    fn new(bytes: Vec<u8>, bytes_per_pixel: usize) -> Self {
        Self {
            bytes,
            read_cursor: 0,
            write_cursor: 0,
            bytes_per_pixel,
            filter_counts: PngFilterCounts::default(),
        }
    }

    fn reconstruct_pass(&mut self, layout: PassLayout) -> Result<PngPass, PngFilterError> {
        let data_offset = self.write_cursor;
        if layout.width != 0 && layout.height != 0 {
            for row in 0..layout.height {
                self.reconstruct_row(layout, row)?;
            }
        }
        let data_length = self
            .write_cursor
            .checked_sub(data_offset)
            .ok_or(PngFilterError::InternalScanlineLayout)?;
        if data_length != layout.data_length {
            return Err(PngFilterError::InternalScanlineLayout);
        }

        Ok(PngPass {
            number: layout.number,
            start_x: layout.start_x,
            start_y: layout.start_y,
            step_x: layout.step_x,
            step_y: layout.step_y,
            width: layout.width,
            height: layout.height,
            row_bytes: layout.row_bytes,
            data_offset,
            data_length,
        })
    }

    fn reconstruct_row(
        &mut self,
        layout: PassLayout,
        row: u32,
    ) -> Result<(), PngFilterError> {
        let filter_byte = self
            .bytes
            .get(self.read_cursor)
            .copied()
            .ok_or(PngFilterError::InternalScanlineLayout)?;
        self.read_cursor = self
            .read_cursor
            .checked_add(1)
            .ok_or(PngFilterError::InternalScanlineLayout)?;
        let filter = FilterType::parse(filter_byte, layout.number, row)?;
        self.filter_counts.record(filter);

        let source_start = self.read_cursor;
        let source_end = source_start
            .checked_add(layout.row_bytes)
            .ok_or(PngFilterError::InternalScanlineLayout)?;
        if source_end > self.bytes.len() {
            return Err(PngFilterError::InternalScanlineLayout);
        }
        let previous_row_start = if row == 0 {
            None
        } else {
            Some(
                self.write_cursor
                    .checked_sub(layout.row_bytes)
                    .ok_or(PngFilterError::InternalScanlineLayout)?,
            )
        };

        for column in 0..layout.row_bytes {
            let source_index = source_start
                .checked_add(column)
                .ok_or(PngFilterError::InternalScanlineLayout)?;
            let destination_index = self
                .write_cursor
                .checked_add(column)
                .ok_or(PngFilterError::InternalScanlineLayout)?;
            let filtered = self
                .bytes
                .get(source_index)
                .copied()
                .ok_or(PngFilterError::InternalScanlineLayout)?;
            let left = if column >= self.bytes_per_pixel {
                self.bytes
                    .get(destination_index - self.bytes_per_pixel)
                    .copied()
                    .ok_or(PngFilterError::InternalScanlineLayout)?
            } else {
                0
            };
            let above = if let Some(previous_start) = previous_row_start {
                self.bytes
                    .get(previous_start + column)
                    .copied()
                    .ok_or(PngFilterError::InternalScanlineLayout)?
            } else {
                0
            };
            let upper_left = if let Some(previous_start) = previous_row_start {
                if column >= self.bytes_per_pixel {
                    self.bytes
                        .get(previous_start + column - self.bytes_per_pixel)
                        .copied()
                        .ok_or(PngFilterError::InternalScanlineLayout)?
                } else {
                    0
                }
            } else {
                0
            };
            let reconstructed = filter.reconstruct(filtered, left, above, upper_left);
            let destination = self
                .bytes
                .get_mut(destination_index)
                .ok_or(PngFilterError::InternalScanlineLayout)?;
            *destination = reconstructed;
        }

        self.read_cursor = source_end;
        self.write_cursor = self
            .write_cursor
            .checked_add(layout.row_bytes)
            .ok_or(PngFilterError::InternalScanlineLayout)?;
        Ok(())
    }

    fn finish(mut self) -> Result<(Vec<u8>, PngFilterCounts), PngFilterError> {
        if self.read_cursor != self.bytes.len() {
            return Err(PngFilterError::InternalScanlineLayout);
        }
        self.bytes.truncate(self.write_cursor);
        Ok((self.bytes, self.filter_counts))
    }
}

/// Validates, inflates, and reverses PNG filter method 0 scanlines.
///
/// Reconstructed bytes remain packed, padded, and ordered by PNG transmission pass. No color,
/// alpha, palette, pixel, ink, component, text, or semantic interpretation occurs here.
///
/// # Errors
///
/// Returns existing header, chunk, or inflation errors unchanged. Returns a wrapped
/// [`PngFilterError`] when a scanline declares an unsupported filter type or the validated
/// scanline layout cannot be reconstructed exactly.
pub fn reconstruct_png_scanlines(input: &[u8]) -> Result<ReconstructedPng, PngAdapterError> {
    let inflated = inflate_png_scanlines(input)?;
    reconstruct_inflated(inflated).map_err(PngAdapterError::InvalidFilterData)
}

fn reconstruct_inflated(
    inflated: crate::InflatedPng,
) -> Result<ReconstructedPng, PngFilterError> {
    let structure = inflated.structure;
    let layouts = pass_layouts(structure.header)?;
    let bytes_per_pixel = filter_bytes_per_pixel(structure.header)?;
    let mut state = ReconstructionState::new(inflated.scanline_bytes, bytes_per_pixel);
    let mut passes = Vec::with_capacity(layouts.len());
    for layout in layouts {
        passes.push(state.reconstruct_pass(layout)?);
    }
    let (reconstructed_bytes, filter_counts) = state.finish()?;

    Ok(ReconstructedPng {
        structure,
        reconstructed_bytes,
        passes,
        filter_counts,
    })
}

fn pass_layouts(header: PngHeader) -> Result<Vec<PassLayout>, PngFilterError> {
    let bits_per_pixel = bits_per_pixel(header);
    if header.interlace_method == 0 {
        return Ok(vec![make_pass_layout(
            0,
            0,
            0,
            1,
            1,
            header.width,
            header.height,
            bits_per_pixel,
        )?]);
    }

    ADAM7_PASSES
        .into_iter()
        .enumerate()
        .map(|(index, (start_x, start_y, step_x, step_y))| {
            let number = u8::try_from(index + 1).expect("Adam7 has seven passes");
            make_pass_layout(
                number,
                start_x,
                start_y,
                step_x,
                step_y,
                pass_extent(header.width, start_x, step_x),
                pass_extent(header.height, start_y, step_y),
                bits_per_pixel,
            )
        })
        .collect()
}

fn make_pass_layout(
    number: u8,
    start_x: u32,
    start_y: u32,
    step_x: u32,
    step_y: u32,
    width: u32,
    height: u32,
    bits_per_pixel: u64,
) -> Result<PassLayout, PngFilterError> {
    let row_bytes = packed_row_bytes(width, bits_per_pixel)?;
    let row_count = usize::try_from(height).map_err(|_| PngFilterError::InternalScanlineLayout)?;
    let data_length = if width == 0 || height == 0 {
        0
    } else {
        row_bytes
            .checked_mul(row_count)
            .ok_or(PngFilterError::InternalScanlineLayout)?
    };
    Ok(PassLayout {
        number,
        start_x,
        start_y,
        step_x,
        step_y,
        width,
        height,
        row_bytes,
        data_length,
    })
}

fn bits_per_pixel(header: PngHeader) -> u64 {
    u64::from(channels(header.color_type)) * u64::from(header.bit_depth)
}

fn packed_row_bytes(width: u32, bits_per_pixel: u64) -> Result<usize, PngFilterError> {
    let bytes = (u64::from(width) * bits_per_pixel).div_ceil(8);
    usize::try_from(bytes).map_err(|_| PngFilterError::InternalScanlineLayout)
}

fn filter_bytes_per_pixel(header: PngHeader) -> Result<usize, PngFilterError> {
    let bytes = bits_per_pixel(header).div_ceil(8).max(1);
    usize::try_from(bytes).map_err(|_| PngFilterError::InternalScanlineLayout)
}

fn paeth_predictor(left: u8, above: u8, upper_left: u8) -> u8 {
    let left = i32::from(left);
    let above = i32::from(above);
    let upper_left = i32::from(upper_left);
    let estimate = left + above - upper_left;
    let left_distance = (estimate - left).abs();
    let above_distance = (estimate - above).abs();
    let upper_left_distance = (estimate - upper_left).abs();

    if left_distance <= above_distance && left_distance <= upper_left_distance {
        u8::try_from(left).expect("predictor input is a byte")
    } else if above_distance <= upper_left_distance {
        u8::try_from(above).expect("predictor input is a byte")
    } else {
        u8::try_from(upper_left).expect("predictor input is a byte")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PngFilterCounts, PngFilterError, reconstruct_inflated, paeth_predictor,
    };
    use crate::{InflatedPng, PngHeader, PngStructure};

    fn inflated(
        width: u32,
        height: u32,
        bit_depth: u8,
        color_type: u8,
        interlace_method: u8,
        scanline_bytes: Vec<u8>,
    ) -> InflatedPng {
        InflatedPng {
            structure: PngStructure {
                header: PngHeader {
                    width,
                    height,
                    bit_depth,
                    color_type,
                    compression_method: 0,
                    filter_method: 0,
                    interlace_method,
                },
                chunk_count: 3,
                idat_chunk_count: 1,
                idat_bytes: 0,
                has_palette: color_type == 3,
            },
            scanline_bytes,
        }
    }

    #[test]
    fn reconstructs_all_filter_types_from_independent_reference_bytes() {
        let filtered = vec![
            0x00, 0x0a, 0x14, 0x1e, 0x28, 0x0f, 0x19, 0x23, 0x2d, 0x01, 0x0c, 0x16,
            0x20, 0x2a, 0x06, 0x06, 0x06, 0x06, 0x02, 0x58, 0x44, 0x30, 0x1c, 0x5c,
            0x48, 0x34, 0x20, 0x03, 0xd3, 0xdd, 0xe7, 0xf1, 0xe0, 0xe7, 0xef, 0xf6,
            0x04, 0xc3, 0x8c, 0x55, 0x1e, 0x0a, 0x0a, 0x0a, 0x0a,
        ];
        let expected = vec![
            10, 20, 30, 40, 15, 25, 35, 45, 12, 22, 32, 42, 18, 28, 38, 48, 100,
            90, 80, 70, 110, 100, 90, 80, 5, 10, 15, 20, 25, 30, 35, 40, 200, 150,
            100, 50, 210, 160, 110, 60,
        ];

        let reconstructed = reconstruct_inflated(inflated(2, 5, 8, 6, 0, filtered))
            .expect("reference filter stream must reconstruct");
        assert_eq!(reconstructed.reconstructed_bytes, expected);
        assert_eq!(
            reconstructed.filter_counts,
            PngFilterCounts {
                none: 1,
                sub: 1,
                up: 1,
                average: 1,
                paeth: 1,
            }
        );
        assert_eq!(reconstructed.passes.len(), 1);
        assert_eq!(reconstructed.passes[0].row_bytes, 8);
        assert_eq!(reconstructed.passes[0].data_length, 40);
    }

    #[test]
    fn packed_samples_round_filter_bytes_per_pixel_up_to_one() {
        let reconstructed = reconstruct_inflated(inflated(
            16,
            1,
            1,
            0,
            0,
            vec![1, 0xaa, 0x22],
        ))
        .expect("packed Sub row must reconstruct");
        assert_eq!(reconstructed.reconstructed_bytes, [0xaa, 0xcc]);
    }

    #[test]
    fn rejects_filter_types_outside_method_zero() {
        let error = reconstruct_inflated(inflated(1, 1, 8, 6, 0, vec![5, 0, 0, 0, 0]))
            .expect_err("filter type five must be rejected");
        assert_eq!(
            error,
            PngFilterError::InvalidFilterType {
                filter_type: 5,
                pass: 0,
                row: 0,
            }
        );
    }

    #[test]
    fn paeth_preserves_specification_tie_order() {
        assert_eq!(paeth_predictor(10, 10, 0), 10);
        assert_eq!(paeth_predictor(10, 20, 10), 20);
        assert_eq!(paeth_predictor(10, 20, 30), 10);
    }
}
