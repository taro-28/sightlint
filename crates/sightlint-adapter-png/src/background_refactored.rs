use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::{AnalyzedPng, PixelRect, PngAdapterError, analyze_png_alpha};

const MAX_BACKGROUND_CANDIDATES: usize = 8;
const TOP_EDGE_COLORS: usize = 4;

/// Why exact opaque-border background candidates were or were not produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundCandidateApplicability {
    /// Every normalized pixel is fully opaque and candidates were evaluated.
    FullyOpaque,
    /// At least one pixel is transparent or translucent, so encoded RGB is not treated as a
    /// composited background candidate.
    RequiresFullyOpaquePixels,
}

/// One exact source-code-value candidate for an opaque image background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundCandidate {
    /// Exact PNG-encoded RGBA8 bytes.
    pub rgba: [u8; 4],
    /// Packed big-endian RGBA value used as the final deterministic tie-breaker.
    pub packed_rgba: u32,
    /// Occurrences among unique geometric corner positions.
    pub corner_occurrences: u8,
    /// Occurrences among unique outer-edge pixels.
    pub edge_pixel_count: u64,
    /// Occurrences across the complete image.
    pub image_pixel_count: u64,
    /// Bounds of all pixels not exactly equal to this candidate.
    pub non_candidate_bounds: Option<PixelRect>,
}

impl BackgroundCandidate {
    /// Returns the canonical lowercase `#rrggbbaa` representation.
    #[must_use]
    pub fn hex_rgba(self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3]
        )
    }
}

/// Deterministic opaque-border candidate analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundCandidateAnalysis {
    /// Applicability of exact source-code-value candidate generation.
    pub applicability: BackgroundCandidateApplicability,
    /// Number of unique geometric corner positions sampled.
    pub corner_sample_count: u8,
    /// Number of unique outer-edge pixels sampled.
    pub edge_sample_count: u64,
    /// Total number of image pixels.
    pub image_pixel_count: u64,
    /// Ranked candidate list. The first entry is leading, not verified.
    pub candidates: Vec<BackgroundCandidate>,
}

/// PNG pixels, alpha observations, and non-authoritative opaque-border candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundAnalyzedPng {
    /// Normalized pixels and exact alpha observations.
    pub analyzed: AnalyzedPng,
    /// Candidate analysis that never overwrites exact alpha geometry.
    pub background: BackgroundCandidateAnalysis,
}

/// Failure while deriving exact opaque-border color candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngBackgroundError {
    /// The RGBA8 vector is inconsistent with the validated dimensions.
    RgbaLengthMismatch {
        /// Exact byte count required by the dimensions.
        expected: usize,
        /// Actual RGBA8 byte count.
        actual: usize,
    },
    /// A coordinate derived from validated dimensions could not address one complete pixel.
    PixelAddressMismatch,
}

impl fmt::Display for PngBackgroundError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RgbaLengthMismatch { expected, actual } => write!(
                formatter,
                "PNG background candidate analysis expected {expected} RGBA8 bytes, but received {actual}"
            ),
            Self::PixelAddressMismatch => formatter.write_str(
                "PNG background candidate analysis could not address a declared pixel",
            ),
        }
    }
}

impl Error for PngBackgroundError {}

#[derive(Debug, Clone, Copy)]
struct ImageDimensions {
    width: u32,
    height: u32,
    image_pixel_count: u64,
    edge_sample_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct CandidateAccumulator {
    color: u32,
    corner_occurrences: u8,
    edge_pixel_count: u64,
    image_pixel_count: u64,
    non_candidate_bounds: BoundsAccumulator,
}

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

/// Runs exact opaque-border background candidate analysis after PNG normalization.
///
/// # Errors
///
/// Returns existing parser, decoder, normalization, and alpha errors unchanged. Returns a wrapped
/// [`PngBackgroundError`] only when deterministic stages disagree about the normalized RGBA layout.
pub fn analyze_png_background_candidates(
    input: &[u8],
) -> Result<BackgroundAnalyzedPng, PngAdapterError> {
    let analyzed = analyze_png_alpha(input)?;
    let background =
        analyze_opaque_background(&analyzed).map_err(PngAdapterError::InvalidBackgroundData)?;
    Ok(BackgroundAnalyzedPng {
        analyzed,
        background,
    })
}

fn analyze_opaque_background(
    analyzed: &AnalyzedPng,
) -> Result<BackgroundCandidateAnalysis, PngBackgroundError> {
    let dimensions = validate_dimensions(analyzed)?;
    let corners = corner_positions(dimensions.width, dimensions.height);
    let corner_sample_count = u8::try_from(corners.len()).expect("at most four unique corners");

    if analyzed.alpha.opaque_pixel_count != dimensions.image_pixel_count {
        return Ok(inapplicable_analysis(
            dimensions,
            corner_sample_count,
        ));
    }

    let pixels = &analyzed.normalized.rgba8;
    let corner_counts = count_corner_colors(pixels, dimensions.width, &corners)?;
    let edge_counts = count_edge_colors(pixels, dimensions)?;
    let candidate_colors = seed_candidate_colors(&corner_counts, &edge_counts);
    let mut accumulators = initialize_candidates(candidate_colors, &corner_counts, &edge_counts);
    accumulate_image_pixels(pixels, dimensions, &mut accumulators)?;

    let mut candidates: Vec<BackgroundCandidate> = accumulators
        .into_iter()
        .map(CandidateAccumulator::finish)
        .collect();
    candidates.sort_by(candidate_order);

    Ok(BackgroundCandidateAnalysis {
        applicability: BackgroundCandidateApplicability::FullyOpaque,
        corner_sample_count,
        edge_sample_count: dimensions.edge_sample_count,
        image_pixel_count: dimensions.image_pixel_count,
        candidates,
    })
}

fn validate_dimensions(analyzed: &AnalyzedPng) -> Result<ImageDimensions, PngBackgroundError> {
    let header = analyzed.normalized.reconstructed.structure.header;
    let image_pixel_count = u64::from(header.width) * u64::from(header.height);
    let expected_u64 = image_pixel_count * 4;
    let expected = usize::try_from(expected_u64).map_err(|_| {
        PngBackgroundError::RgbaLengthMismatch {
            expected: usize::MAX,
            actual: analyzed.normalized.rgba8.len(),
        }
    })?;
    if analyzed.normalized.rgba8.len() != expected {
        return Err(PngBackgroundError::RgbaLengthMismatch {
            expected,
            actual: analyzed.normalized.rgba8.len(),
        });
    }
    Ok(ImageDimensions {
        width: header.width,
        height: header.height,
        image_pixel_count,
        edge_sample_count: edge_sample_count(header.width, header.height),
    })
}

fn inapplicable_analysis(
    dimensions: ImageDimensions,
    corner_sample_count: u8,
) -> BackgroundCandidateAnalysis {
    BackgroundCandidateAnalysis {
        applicability: BackgroundCandidateApplicability::RequiresFullyOpaquePixels,
        corner_sample_count,
        edge_sample_count: dimensions.edge_sample_count,
        image_pixel_count: dimensions.image_pixel_count,
        candidates: Vec::new(),
    }
}

fn count_corner_colors(
    pixels: &[u8],
    width: u32,
    corners: &[(u32, u32)],
) -> Result<BTreeMap<u32, u8>, PngBackgroundError> {
    let mut counts = BTreeMap::new();
    for &(x, y) in corners {
        let color = color_at(pixels, width, x, y)?;
        *counts.entry(color).or_default() += 1;
    }
    Ok(counts)
}

fn count_edge_colors(
    pixels: &[u8],
    dimensions: ImageDimensions,
) -> Result<BTreeMap<u32, u64>, PngBackgroundError> {
    let mut counts = BTreeMap::new();
    for y in 0..dimensions.height {
        for x in 0..dimensions.width {
            if is_outer_edge(x, y, dimensions.width, dimensions.height) {
                let color = color_at(pixels, dimensions.width, x, y)?;
                *counts.entry(color).or_default() += 1;
            }
        }
    }
    debug_assert_eq!(counts.values().sum::<u64>(), dimensions.edge_sample_count);
    Ok(counts)
}

fn seed_candidate_colors(
    corner_counts: &BTreeMap<u32, u8>,
    edge_counts: &BTreeMap<u32, u64>,
) -> BTreeSet<u32> {
    let mut ranked_edges: Vec<(u32, u64)> = edge_counts
        .iter()
        .map(|(&color, &count)| (color, count))
        .collect();
    ranked_edges.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut colors = BTreeSet::new();
    colors.extend(corner_counts.keys().copied());
    colors.extend(
        ranked_edges
            .iter()
            .take(TOP_EDGE_COLORS)
            .map(|&(color, _)| color),
    );
    debug_assert!(colors.len() <= MAX_BACKGROUND_CANDIDATES);
    colors
}

fn initialize_candidates(
    colors: BTreeSet<u32>,
    corner_counts: &BTreeMap<u32, u8>,
    edge_counts: &BTreeMap<u32, u64>,
) -> Vec<CandidateAccumulator> {
    colors
        .into_iter()
        .map(|color| CandidateAccumulator {
            color,
            corner_occurrences: corner_counts.get(&color).copied().unwrap_or(0),
            edge_pixel_count: edge_counts.get(&color).copied().unwrap_or(0),
            image_pixel_count: 0,
            non_candidate_bounds: BoundsAccumulator::default(),
        })
        .collect()
}

fn accumulate_image_pixels(
    pixels: &[u8],
    dimensions: ImageDimensions,
    candidates: &mut [CandidateAccumulator],
) -> Result<(), PngBackgroundError> {
    for y in 0..dimensions.height {
        for x in 0..dimensions.width {
            let color = color_at(pixels, dimensions.width, x, y)?;
            for candidate in &mut *candidates {
                if candidate.color == color {
                    candidate.image_pixel_count += 1;
                } else {
                    candidate.non_candidate_bounds.include(x, y);
                }
            }
        }
    }
    Ok(())
}

fn candidate_order(left: &BackgroundCandidate, right: &BackgroundCandidate) -> Ordering {
    right
        .corner_occurrences
        .cmp(&left.corner_occurrences)
        .then_with(|| right.edge_pixel_count.cmp(&left.edge_pixel_count))
        .then_with(|| right.image_pixel_count.cmp(&left.image_pixel_count))
        .then_with(|| left.packed_rgba.cmp(&right.packed_rgba))
}

fn corner_positions(width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut positions = BTreeSet::new();
    positions.insert((0, 0));
    positions.insert((width - 1, 0));
    positions.insert((0, height - 1));
    positions.insert((width - 1, height - 1));
    positions.into_iter().collect()
}

fn edge_sample_count(width: u32, height: u32) -> u64 {
    if width == 1 || height == 1 {
        u64::from(width) * u64::from(height)
    } else {
        2 * u64::from(width) + 2 * u64::from(height) - 4
    }
}

fn is_outer_edge(x: u32, y: u32, width: u32, height: u32) -> bool {
    x == 0 || y == 0 || x + 1 == width || y + 1 == height
}

fn color_at(
    pixels: &[u8],
    width: u32,
    x: u32,
    y: u32,
) -> Result<u32, PngBackgroundError> {
    let offset_u64 = (u64::from(y) * u64::from(width) + u64::from(x)) * 4;
    let offset =
        usize::try_from(offset_u64).map_err(|_| PngBackgroundError::PixelAddressMismatch)?;
    let rgba = pixels
        .get(offset..offset + 4)
        .ok_or(PngBackgroundError::PixelAddressMismatch)?;
    Ok(u32::from_be_bytes([rgba[0], rgba[1], rgba[2], rgba[3]]))
}

impl CandidateAccumulator {
    fn finish(self) -> BackgroundCandidate {
        BackgroundCandidate {
            rgba: self.color.to_be_bytes(),
            packed_rgba: self.color,
            corner_occurrences: self.corner_occurrences,
            edge_pixel_count: self.edge_pixel_count,
            image_pixel_count: self.image_pixel_count,
            non_candidate_bounds: self.non_candidate_bounds.finish(),
        }
    }
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
        if self.populated {
            Some(PixelRect {
                x: self.min_x,
                y: self.min_y,
                width: self.max_x - self.min_x + 1,
                height: self.max_y - self.min_y + 1,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackgroundCandidate, candidate_order, corner_positions, edge_sample_count};

    fn candidate(color: u32, corners: u8, edge: u64, image: u64) -> BackgroundCandidate {
        BackgroundCandidate {
            rgba: color.to_be_bytes(),
            packed_rgba: color,
            corner_occurrences: corners,
            edge_pixel_count: edge,
            image_pixel_count: image,
            non_candidate_bounds: None,
        }
    }

    #[test]
    fn degenerate_corner_and_edge_samples_are_unique() {
        assert_eq!(corner_positions(1, 1), vec![(0, 0)]);
        assert_eq!(corner_positions(4, 1), vec![(0, 0), (3, 0)]);
        assert_eq!(corner_positions(1, 3), vec![(0, 0), (0, 2)]);
        assert_eq!(edge_sample_count(1, 1), 1);
        assert_eq!(edge_sample_count(4, 1), 4);
        assert_eq!(edge_sample_count(4, 3), 10);
    }

    #[test]
    fn ordering_uses_documented_stable_tuple() {
        let mut values = vec![
            candidate(3, 2, 9, 30),
            candidate(1, 4, 8, 20),
            candidate(2, 4, 8, 20),
            candidate(4, 4, 7, 100),
        ];
        values.sort_by(candidate_order);
        assert_eq!(
            values
                .iter()
                .map(|candidate| candidate.packed_rgba)
                .collect::<Vec<_>>(),
            vec![1, 2, 4, 3]
        );
    }
}
