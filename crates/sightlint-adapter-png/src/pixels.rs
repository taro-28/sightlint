use std::error::Error;
use std::fmt;

use crate::{NormalizedPng, PngAdapterError, normalize_png_rgba8};

/// Integer device-pixel rectangle using pixel-edge coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    /// Left pixel-edge coordinate.
    pub x: u32,
    /// Top pixel-edge coordinate.
    pub y: u32,
    /// Rectangle width in device pixels.
    pub width: u32,
    /// Rectangle height in device pixels.
    pub height: u32,
}

/// Transparent canvas insets outside alpha-visible content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelInsets {
    /// Fully transparent rows above the visible bounds.
    pub top: u32,
    /// Fully transparent columns to the right of the visible bounds.
    pub right: u32,
    /// Fully transparent rows below the visible bounds.
    pub bottom: u32,
    /// Fully transparent columns to the left of the visible bounds.
    pub left: u32,
}

/// Counts of alpha-visible pixels on each outer canvas edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeVisibleCounts {
    /// Visible pixels in the first canvas row.
    pub top: u64,
    /// Visible pixels in the final canvas column.
    pub right: u64,
    /// Visible pixels in the final canvas row.
    pub bottom: u64,
    /// Visible pixels in the first canvas column.
    pub left: u64,
}

/// Exact geometry and occupancy derived only from the normalized alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaPixelAnalysis {
    /// Bounds of pixels whose alpha is greater than zero.
    pub visible_bounds: Option<PixelRect>,
    /// Bounds of pixels whose alpha is exactly 255.
    pub opaque_bounds: Option<PixelRect>,
    /// Transparent insets outside visible bounds, or `None` for an entirely transparent image.
    pub transparent_insets: Option<PixelInsets>,
    /// Number of pixels whose alpha is greater than zero.
    pub visible_pixel_count: u64,
    /// Number of pixels whose alpha is exactly 255.
    pub opaque_pixel_count: u64,
    /// Number of pixels whose alpha is exactly zero.
    pub transparent_pixel_count: u64,
    /// Number of pixels whose alpha is between zero and 255.
    pub translucent_pixel_count: u64,
    /// Visible-pixel counts on the four outer canvas edges.
    pub edge_visible_pixels: EdgeVisibleCounts,
    /// Whether every source pixel is fully transparent.
    pub all_transparent: bool,
    /// Whether every source pixel has non-zero alpha.
    pub all_visible: bool,
}

/// Normalized PNG pixels together with exact alpha-derived observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzedPng {
    /// Deterministically normalized PNG source pixels.
    pub normalized: NormalizedPng,
    /// Exact alpha-channel analysis.
    pub alpha: AlphaPixelAnalysis,
}

/// Failure while deriving exact alpha-visible geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngPixelError {
    /// The RGBA8 vector does not contain exactly four bytes for every declared pixel.
    RgbaLengthMismatch {
        /// Exact byte count required by the declared dimensions.
        expected: usize,
        /// Actual normalized buffer length.
        actual: usize,
    },
    /// A second deterministic scan disagreed with normalization alpha counts.
    AlphaCountMismatch,
}

impl fmt::Display for PngPixelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RgbaLengthMismatch { expected, actual } => write!(
                formatter,
                "PNG alpha analysis expected {expected} RGBA8 bytes, but received {actual}"
            ),
            Self::AlphaCountMismatch => formatter.write_str(
                "PNG alpha analysis disagrees with the normalized alpha classification",
            ),
        }
    }
}

impl Error for PngPixelError {}

#[derive(Debug, Clone, Copy)]
struct BoundsAccumulator {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    populated: bool,
}

impl Default for BoundsAccumulator {
    fn default() -> Self {
        Self {
            min_x: u32::MAX,
            min_y: u32::MAX,
            max_x: 0,
            max_y: 0,
            populated: false,
        }
    }
}

/// Normalizes a PNG and derives exact geometry from non-zero and fully opaque alpha values.
///
/// # Errors
///
/// Returns existing PNG parser and normalization errors unchanged. Returns a wrapped
/// [`PngPixelError`] only if deterministic adapter stages disagree about the RGBA8 layout or
/// alpha classification.
pub fn analyze_png_alpha(input: &[u8]) -> Result<AnalyzedPng, PngAdapterError> {
    let normalized = normalize_png_rgba8(input)?;
    let alpha = analyze_normalized(&normalized).map_err(PngAdapterError::InvalidPixelData)?;
    Ok(AnalyzedPng { normalized, alpha })
}

fn analyze_normalized(normalized: &NormalizedPng) -> Result<AlphaPixelAnalysis, PngPixelError> {
    let header = normalized.reconstructed.structure.header;
    let expected_u64 = u64::from(header.width) * u64::from(header.height) * 4;
    let expected = usize::try_from(expected_u64).map_err(|_| PngPixelError::RgbaLengthMismatch {
        expected: usize::MAX,
        actual: normalized.rgba8.len(),
    })?;
    if normalized.rgba8.len() != expected {
        return Err(PngPixelError::RgbaLengthMismatch {
            expected,
            actual: normalized.rgba8.len(),
        });
    }

    let width = usize::try_from(header.width).expect("validated PNG width fits usize");
    let mut visible_bounds = BoundsAccumulator::default();
    let mut opaque_bounds = BoundsAccumulator::default();
    let mut visible_pixel_count = 0_u64;
    let mut opaque_pixel_count = 0_u64;
    let mut transparent_pixel_count = 0_u64;
    let mut translucent_pixel_count = 0_u64;
    let mut edge_visible_pixels = EdgeVisibleCounts::default();

    for (index, pixel) in normalized.rgba8.chunks_exact(4).enumerate() {
        let x = u32::try_from(index % width).expect("pixel column fits u32");
        let y = u32::try_from(index / width).expect("pixel row fits u32");
        let alpha = pixel[3];
        match alpha {
            0 => transparent_pixel_count += 1,
            255 => {
                visible_pixel_count += 1;
                opaque_pixel_count += 1;
                visible_bounds.include(x, y);
                opaque_bounds.include(x, y);
                edge_visible_pixels.record(x, y, header.width, header.height);
            }
            _ => {
                visible_pixel_count += 1;
                translucent_pixel_count += 1;
                visible_bounds.include(x, y);
                edge_visible_pixels.record(x, y, header.width, header.height);
            }
        }
    }

    if opaque_pixel_count != normalized.opaque_pixel_count
        || transparent_pixel_count != normalized.transparent_pixel_count
        || translucent_pixel_count != normalized.translucent_pixel_count
    {
        return Err(PngPixelError::AlphaCountMismatch);
    }

    let visible_bounds = visible_bounds.finish();
    let opaque_bounds = opaque_bounds.finish();
    let transparent_insets = visible_bounds.map(|bounds| PixelInsets {
        top: bounds.y,
        right: header.width - bounds.x - bounds.width,
        bottom: header.height - bounds.y - bounds.height,
        left: bounds.x,
    });

    Ok(AlphaPixelAnalysis {
        visible_bounds,
        opaque_bounds,
        transparent_insets,
        visible_pixel_count,
        opaque_pixel_count,
        transparent_pixel_count,
        translucent_pixel_count,
        edge_visible_pixels,
        all_transparent: visible_pixel_count == 0,
        all_visible: transparent_pixel_count == 0,
    })
}

impl BoundsAccumulator {
    fn include(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
        self.populated = true;
    }

    fn finish(self) -> Option<PixelRect> {
        self.populated.then_some(PixelRect {
            x: self.min_x,
            y: self.min_y,
            width: self.max_x - self.min_x + 1,
            height: self.max_y - self.min_y + 1,
        })
    }
}

impl EdgeVisibleCounts {
    fn record(&mut self, x: u32, y: u32, width: u32, height: u32) {
        if y == 0 {
            self.top += 1;
        }
        if x + 1 == width {
            self.right += 1;
        }
        if y + 1 == height {
            self.bottom += 1;
        }
        if x == 0 {
            self.left += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoundsAccumulator, EdgeVisibleCounts, PixelRect};

    #[test]
    fn bounds_use_inclusive_pixels_and_edge_coordinates() {
        let mut bounds = BoundsAccumulator::default();
        bounds.include(4, 7);
        bounds.include(9, 10);
        assert_eq!(
            bounds.finish(),
            Some(PixelRect {
                x: 4,
                y: 7,
                width: 6,
                height: 4,
            })
        );
    }

    #[test]
    fn corner_pixels_contribute_to_both_touching_edges() {
        let mut edges = EdgeVisibleCounts::default();
        edges.record(0, 0, 3, 2);
        edges.record(2, 1, 3, 2);
        assert_eq!(edges.top, 1);
        assert_eq!(edges.right, 1);
        assert_eq!(edges.bottom, 1);
        assert_eq!(edges.left, 1);
    }
}
