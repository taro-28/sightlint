//! Exact source-alpha observations over supported encoded RGBA8 samples.

use sightlint_ir::{Identifier, ObservedRect, Rect};

use crate::{EncodedRgba8Raster, PngRasterError};

/// Half-open device-pixel bounds represented as x, y, width, and height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaBounds {
    /// Left edge in device pixels.
    pub x: u32,
    /// Top edge in device pixels.
    pub y: u32,
    /// Half-open width in device pixels.
    pub width: u32,
    /// Half-open height in device pixels.
    pub height: u32,
}

impl AlphaBounds {
    pub(crate) const fn as_array(self) -> [u32; 4] {
        [self.x, self.y, self.width, self.height]
    }

    pub(crate) fn as_observed_rect(self) -> ObservedRect {
        ObservedRect {
            rect: Rect {
                x: f64::from(self.x),
                y: f64::from(self.y),
                width: f64::from(self.width),
                height: f64::from(self.height),
            },
            coordinate_space_id: Identifier::from("canvas"),
            evidence_id: Identifier::from("evidence:png-alpha"),
        }
    }
}

/// Transparent margins outside the visible source-alpha bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransparentInsets {
    /// Transparent rows above the visible bounds.
    pub top: u32,
    /// Transparent columns to the right of the visible bounds.
    pub right: u32,
    /// Transparent rows below the visible bounds.
    pub bottom: u32,
    /// Transparent columns to the left of the visible bounds.
    pub left: u32,
}

/// Exact sample counts classified by encoded alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaPixelCounts {
    /// Total number of samples.
    pub total: u64,
    /// Samples whose alpha is greater than zero.
    pub visible: u64,
    /// Samples whose alpha is exactly 255.
    pub opaque: u64,
    /// Samples whose alpha is between one and 254 inclusive.
    pub translucent: u64,
    /// Samples whose alpha is zero.
    pub transparent: u64,
}

/// Visible sample count on one canvas edge and the number of samples on that edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaEdgeCount {
    /// Number of source-visible samples on the edge.
    pub count: u32,
    /// Total number of samples on the edge.
    pub denominator: u32,
}

/// Exact source-visible occupancy of the four canvas edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaEdgePixels {
    /// Top edge occupancy.
    pub top: AlphaEdgeCount,
    /// Right edge occupancy.
    pub right: AlphaEdgeCount,
    /// Bottom edge occupancy.
    pub bottom: AlphaEdgeCount,
    /// Left edge occupancy.
    pub left: AlphaEdgeCount,
}

/// Exact geometry and counts derived from unassociated PNG-encoded alpha8 samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaGeometry {
    /// Enclosing bounds of samples whose alpha is greater than zero.
    pub visible_bounds: Option<AlphaBounds>,
    /// Enclosing bounds of samples whose alpha is exactly 255.
    pub opaque_bounds: Option<AlphaBounds>,
    /// Transparent margins outside the visible bounds, absent when no sample is visible.
    pub transparent_insets: Option<TransparentInsets>,
    /// Exact alpha-class sample counts.
    pub pixel_counts: AlphaPixelCounts,
    /// Exact source-visible edge occupancy.
    pub visible_edge_pixels: AlphaEdgePixels,
    /// Whether every encoded alpha sample is zero.
    pub entirely_transparent: bool,
    /// Whether every encoded alpha sample is greater than zero.
    pub all_pixels_visible: bool,
}

#[derive(Debug, Clone, Copy)]
struct BoundsAccumulator {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl BoundsAccumulator {
    const fn new(x: u32, y: u32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    const fn finish(self) -> AlphaBounds {
        AlphaBounds {
            x: self.min_x,
            y: self.min_y,
            width: self.max_x - self.min_x + 1,
            height: self.max_y - self.min_y + 1,
        }
    }
}

/// Observes exact source-alpha geometry in one pass without allocating a second image buffer.
///
/// # Errors
///
/// Returns [`PngRasterError::InvalidLayout`] when dimensions and RGBA bytes disagree.
pub fn observe_alpha_geometry(
    raster: &EncodedRgba8Raster,
) -> Result<AlphaGeometry, PngRasterError> {
    let total = u64::from(raster.width)
        .checked_mul(u64::from(raster.height))
        .ok_or(PngRasterError::InvalidLayout)?;
    let expected_bytes = total
        .checked_mul(4)
        .and_then(|length| usize::try_from(length).ok())
        .ok_or(PngRasterError::InvalidLayout)?;
    if raster.width == 0 || raster.height == 0 || raster.pixels.len() != expected_bytes {
        return Err(PngRasterError::InvalidLayout);
    }

    let mut visible_bounds = None;
    let mut opaque_bounds = None;
    let mut visible = 0_u64;
    let mut opaque = 0_u64;
    let mut translucent = 0_u64;
    let mut top = 0_u32;
    let mut right = 0_u32;
    let mut bottom = 0_u32;
    let mut left = 0_u32;

    for (index, sample) in raster.pixels.chunks_exact(4).enumerate() {
        let index = u64::try_from(index).map_err(|_| PngRasterError::InvalidLayout)?;
        let x = u32::try_from(index % u64::from(raster.width))
            .map_err(|_| PngRasterError::InvalidLayout)?;
        let y = u32::try_from(index / u64::from(raster.width))
            .map_err(|_| PngRasterError::InvalidLayout)?;
        let alpha = sample[3];
        if alpha > 0 {
            visible += 1;
            include(&mut visible_bounds, x, y);
            top += u32::from(y == 0);
            right += u32::from(x == raster.width - 1);
            bottom += u32::from(y == raster.height - 1);
            left += u32::from(x == 0);
        }
        if alpha == 255 {
            opaque += 1;
            include(&mut opaque_bounds, x, y);
        } else if alpha > 0 {
            translucent += 1;
        }
    }

    let visible_bounds = visible_bounds.map(BoundsAccumulator::finish);
    let opaque_bounds = opaque_bounds.map(BoundsAccumulator::finish);
    let transparent = total - visible;
    let transparent_insets = visible_bounds.map(|bounds| TransparentInsets {
        top: bounds.y,
        right: raster.width - (bounds.x + bounds.width),
        bottom: raster.height - (bounds.y + bounds.height),
        left: bounds.x,
    });

    Ok(AlphaGeometry {
        visible_bounds,
        opaque_bounds,
        transparent_insets,
        pixel_counts: AlphaPixelCounts {
            total,
            visible,
            opaque,
            translucent,
            transparent,
        },
        visible_edge_pixels: AlphaEdgePixels {
            top: AlphaEdgeCount {
                count: top,
                denominator: raster.width,
            },
            right: AlphaEdgeCount {
                count: right,
                denominator: raster.height,
            },
            bottom: AlphaEdgeCount {
                count: bottom,
                denominator: raster.width,
            },
            left: AlphaEdgeCount {
                count: left,
                denominator: raster.height,
            },
        },
        entirely_transparent: visible == 0,
        all_pixels_visible: visible == total,
    })
}

fn include(bounds: &mut Option<BoundsAccumulator>, x: u32, y: u32) {
    match bounds {
        Some(bounds) => bounds.include(x, y),
        None => *bounds = Some(BoundsAccumulator::new(x, y)),
    }
}

#[cfg(test)]
mod tests {
    use super::{AlphaBounds, AlphaEdgeCount, observe_alpha_geometry};
    use crate::{EncodedRgba8Raster, PngRasterError};

    fn raster(width: u32, height: u32, alphas: &[u8]) -> EncodedRgba8Raster {
        let pixels = alphas
            .iter()
            .flat_map(|alpha| [17, 29, 41, *alpha])
            .collect();
        EncodedRgba8Raster {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn distinguishes_visible_opaque_and_translucent_bounds() {
        let observed =
            observe_alpha_geometry(&raster(4, 3, &[0, 0, 0, 0, 0, 1, 255, 0, 0, 0, 128, 0]))
                .expect("valid raster");
        assert_eq!(
            observed.visible_bounds,
            Some(AlphaBounds {
                x: 1,
                y: 1,
                width: 2,
                height: 2
            })
        );
        assert_eq!(
            observed.opaque_bounds,
            Some(AlphaBounds {
                x: 2,
                y: 1,
                width: 1,
                height: 1
            })
        );
        assert_eq!(observed.pixel_counts.visible, 3);
        assert_eq!(observed.pixel_counts.opaque, 1);
        assert_eq!(observed.pixel_counts.translucent, 2);
        assert_eq!(observed.pixel_counts.transparent, 9);
        assert_eq!(observed.transparent_insets.expect("insets").right, 1);
    }

    #[test]
    fn counts_corner_samples_on_each_incident_edge() {
        let observed =
            observe_alpha_geometry(&raster(2, 2, &[255, 0, 0, 255])).expect("valid raster");
        let one_of_two = AlphaEdgeCount {
            count: 1,
            denominator: 2,
        };
        assert_eq!(observed.visible_edge_pixels.top, one_of_two);
        assert_eq!(observed.visible_edge_pixels.right, one_of_two);
        assert_eq!(observed.visible_edge_pixels.bottom, one_of_two);
        assert_eq!(observed.visible_edge_pixels.left, one_of_two);
        assert!(!observed.all_pixels_visible);
    }

    #[test]
    fn preserves_absence_for_entirely_transparent_raster() {
        let observed = observe_alpha_geometry(&raster(2, 1, &[0, 0])).expect("valid raster");
        assert!(observed.visible_bounds.is_none());
        assert!(observed.opaque_bounds.is_none());
        assert!(observed.transparent_insets.is_none());
        assert!(observed.entirely_transparent);
        assert!(!observed.all_pixels_visible);
    }

    #[test]
    fn fully_opaque_degenerate_dimensions_have_full_bounds() {
        let observed =
            observe_alpha_geometry(&raster(1, 3, &[255, 255, 255])).expect("valid raster");
        assert_eq!(
            observed.visible_bounds,
            Some(AlphaBounds {
                x: 0,
                y: 0,
                width: 1,
                height: 3
            })
        );
        assert!(observed.all_pixels_visible);
        assert_eq!(observed.visible_edge_pixels.left.count, 3);
        assert_eq!(observed.visible_edge_pixels.right.count, 3);
    }

    #[test]
    fn rejects_inconsistent_layout() {
        let invalid = EncodedRgba8Raster {
            width: 2,
            height: 1,
            pixels: vec![0; 4],
        };
        assert_eq!(
            observe_alpha_geometry(&invalid),
            Err(PngRasterError::InvalidLayout)
        );
    }
}
