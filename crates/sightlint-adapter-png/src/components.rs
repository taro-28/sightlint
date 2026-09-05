use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::{
    BackgroundAnalyzedPng, BackgroundCandidateApplicability, PixelRect, PngAdapterError,
    analyze_png_background_candidates,
};

/// Stable identifier for the conservative background-relative segmentation policy.
pub const BACKGROUND_COMPONENT_POLICY_ID: &str = "opaque-border-components-v1";
/// Required percentage of unique outer-edge pixels occupied by the leading candidate.
pub const REQUIRED_EDGE_PERCENT: u64 = 95;
/// Maximum number of foreground row runs retained by production analysis.
pub const MAX_COMPONENT_RUNS: usize = 250_000;
/// Maximum number of final connected components retained by production analysis.
pub const MAX_COMPONENTS: usize = 50_000;

/// Result status for experimental background-relative component extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundComponentStatus {
    /// A candidate qualified and component extraction completed.
    Available,
    /// Encoded pixels contain transparency or translucency.
    RequiresFullyOpaquePixels,
    /// The canvas is smaller than the policy's 3 × 3 applicability floor.
    ImageTooSmall,
    /// The leading border candidate did not satisfy corner and edge support requirements.
    NoQualifiedBackgroundCandidate,
    /// Foreground complexity exceeded the bounded row-run budget.
    RunLimitExceeded,
    /// Final four-connected component count exceeded the bounded component budget.
    ComponentLimitExceeded,
}

/// One exact four-connected component under a stated background hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundRelativeComponent {
    /// Stable index after canonical component sorting.
    pub index: u32,
    /// Device-pixel bounds of all pixels in the component.
    pub bounds: PixelRect,
    /// Exact number of non-candidate pixels in the component.
    pub pixel_count: u64,
    /// Number of maximal horizontal runs forming the component.
    pub run_count: u64,
    /// Whether any component pixel lies on the outer canvas edge.
    pub touches_canvas_edge: bool,
}

/// Bounded component-analysis result and supporting candidate evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundComponentAnalysis {
    /// Explicit outcome or abstention reason.
    pub status: BackgroundComponentStatus,
    /// Leading candidate exact RGBA bytes when one was available.
    pub candidate_rgba: Option<[u8; 4]>,
    /// Leading candidate occurrences among unique corners.
    pub candidate_corner_occurrences: Option<u8>,
    /// Number of unique corner samples used by the policy.
    pub corner_sample_count: u8,
    /// Leading candidate occurrences among unique outer-edge pixels.
    pub candidate_edge_pixel_count: Option<u64>,
    /// Number of unique outer-edge samples used by the policy.
    pub edge_sample_count: u64,
    /// Canonically ordered four-connected foreground components.
    pub components: Vec<BackgroundRelativeComponent>,
}

/// PNG observations through bounded background-relative component extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentAnalyzedPng {
    /// Exact pixels, alpha facts, and opaque-border candidates.
    pub background_analyzed: BackgroundAnalyzedPng,
    /// Experimental component hypotheses under an explicit policy.
    pub component_analysis: BackgroundComponentAnalysis,
}

/// Internal inconsistency while extracting background-relative components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngComponentError {
    /// The normalized RGBA8 vector disagrees with validated image dimensions.
    RgbaLengthMismatch {
        /// Required byte count.
        expected: usize,
        /// Actual normalized byte count.
        actual: usize,
    },
    /// A validated pixel coordinate could not address one RGBA8 value.
    PixelAddressMismatch,
}

impl fmt::Display for PngComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RgbaLengthMismatch { expected, actual } => write!(
                formatter,
                "PNG component analysis expected {expected} RGBA8 bytes, but received {actual}"
            ),
            Self::PixelAddressMismatch => formatter.write_str(
                "PNG component analysis could not address a validated pixel coordinate",
            ),
        }
    }
}

impl Error for PngComponentError {}

#[derive(Debug, Clone, Copy)]
struct SegmentationLimits {
    max_runs: usize,
    max_components: usize,
}

#[derive(Debug, Clone, Copy)]
struct Run {
    y: u32,
    start_x: u32,
    end_x: u32,
}

#[derive(Debug)]
struct UnionFind {
    parents: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
struct ComponentAccumulator {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    pixel_count: u64,
    run_count: u64,
    touches_canvas_edge: bool,
}

#[derive(Debug)]
enum RunCollection {
    Complete { runs: Vec<Run>, unions: UnionFind },
    LimitExceeded,
}

#[derive(Debug)]
enum ComponentCollection {
    Complete(Vec<BackgroundRelativeComponent>),
    LimitExceeded,
}

/// Derives bounded four-connected components under the conservative leading-background policy.
///
/// # Errors
///
/// Returns existing PNG parser and pixel-analysis errors unchanged. Returns a wrapped
/// [`PngComponentError`] only when already validated dimensions disagree with normalized RGBA8
/// storage. Complexity limits are represented as non-fatal analysis statuses.
pub fn segment_png_background_components(
    input: &[u8],
) -> Result<ComponentAnalyzedPng, PngAdapterError> {
    let background_analyzed = analyze_png_background_candidates(input)?;
    let component_analysis = segment_background(
        &background_analyzed,
        SegmentationLimits {
            max_runs: MAX_COMPONENT_RUNS,
            max_components: MAX_COMPONENTS,
        },
    )
    .map_err(PngAdapterError::InvalidComponentData)?;
    Ok(ComponentAnalyzedPng {
        background_analyzed,
        component_analysis,
    })
}

fn segment_background(
    artifact: &BackgroundAnalyzedPng,
    limits: SegmentationLimits,
) -> Result<BackgroundComponentAnalysis, PngComponentError> {
    let background = &artifact.background;
    let header = artifact
        .analyzed
        .normalized
        .reconstructed
        .structure
        .header;
    validate_rgba_length(
        &artifact.analyzed.normalized.rgba8,
        header.width,
        header.height,
    )?;

    if background.applicability != BackgroundCandidateApplicability::FullyOpaque {
        return Ok(empty_analysis(
            BackgroundComponentStatus::RequiresFullyOpaquePixels,
            background,
            None,
        ));
    }
    let leading = background.candidates.first();
    if header.width < 3 || header.height < 3 {
        return Ok(empty_analysis(
            BackgroundComponentStatus::ImageTooSmall,
            background,
            leading,
        ));
    }
    let Some(candidate) = leading else {
        return Ok(empty_analysis(
            BackgroundComponentStatus::NoQualifiedBackgroundCandidate,
            background,
            None,
        ));
    };
    if !candidate_qualifies(candidate, background) {
        return Ok(empty_analysis(
            BackgroundComponentStatus::NoQualifiedBackgroundCandidate,
            background,
            Some(candidate),
        ));
    }

    let pixels = &artifact.analyzed.normalized.rgba8;
    let components = match collect_runs(
        pixels,
        header.width,
        header.height,
        candidate.packed_rgba,
        limits.max_runs,
    )? {
        RunCollection::LimitExceeded => {
            return Ok(empty_analysis(
                BackgroundComponentStatus::RunLimitExceeded,
                background,
                Some(candidate),
            ));
        }
        RunCollection::Complete { runs, mut unions } => {
            match aggregate_components(
                &runs,
                &mut unions,
                header.width,
                header.height,
                limits.max_components,
            ) {
                ComponentCollection::Complete(components) => components,
                ComponentCollection::LimitExceeded => {
                    return Ok(empty_analysis(
                        BackgroundComponentStatus::ComponentLimitExceeded,
                        background,
                        Some(candidate),
                    ));
                }
            }
        }
    };

    Ok(BackgroundComponentAnalysis {
        status: BackgroundComponentStatus::Available,
        candidate_rgba: Some(candidate.rgba),
        candidate_corner_occurrences: Some(candidate.corner_occurrences),
        corner_sample_count: background.corner_sample_count,
        candidate_edge_pixel_count: Some(candidate.edge_pixel_count),
        edge_sample_count: background.edge_sample_count,
        components,
    })
}

fn validate_rgba_length(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(), PngComponentError> {
    let expected_u64 = u64::from(width) * u64::from(height) * 4;
    let expected = usize::try_from(expected_u64).map_err(|_| {
        PngComponentError::RgbaLengthMismatch {
            expected: usize::MAX,
            actual: pixels.len(),
        }
    })?;
    if pixels.len() != expected {
        return Err(PngComponentError::RgbaLengthMismatch {
            expected,
            actual: pixels.len(),
        });
    }
    Ok(())
}

fn candidate_qualifies(
    candidate: &crate::BackgroundCandidate,
    analysis: &crate::BackgroundCandidateAnalysis,
) -> bool {
    candidate.corner_occurrences == analysis.corner_sample_count
        && candidate.edge_pixel_count * 100 >= analysis.edge_sample_count * REQUIRED_EDGE_PERCENT
}

fn empty_analysis(
    status: BackgroundComponentStatus,
    background: &crate::BackgroundCandidateAnalysis,
    candidate: Option<&crate::BackgroundCandidate>,
) -> BackgroundComponentAnalysis {
    BackgroundComponentAnalysis {
        status,
        candidate_rgba: candidate.map(|candidate| candidate.rgba),
        candidate_corner_occurrences: candidate.map(|candidate| candidate.corner_occurrences),
        corner_sample_count: background.corner_sample_count,
        candidate_edge_pixel_count: candidate.map(|candidate| candidate.edge_pixel_count),
        edge_sample_count: background.edge_sample_count,
        components: Vec::new(),
    }
}

fn collect_runs(
    pixels: &[u8],
    width: u32,
    height: u32,
    background: u32,
    max_runs: usize,
) -> Result<RunCollection, PngComponentError> {
    let mut runs = Vec::new();
    let mut unions = UnionFind::new();
    let mut previous_row = Vec::new();

    for y in 0..height {
        let current_row = collect_row_runs(
            pixels,
            width,
            y,
            background,
            max_runs,
            &mut runs,
            &mut unions,
        )?;
        let Some(current_row) = current_row else {
            return Ok(RunCollection::LimitExceeded);
        };
        connect_adjacent_rows(&runs, &previous_row, &current_row, &mut unions);
        previous_row = current_row;
    }

    Ok(RunCollection::Complete { runs, unions })
}

fn collect_row_runs(
    pixels: &[u8],
    width: u32,
    y: u32,
    background: u32,
    max_runs: usize,
    runs: &mut Vec<Run>,
    unions: &mut UnionFind,
) -> Result<Option<Vec<usize>>, PngComponentError> {
    let mut indexes = Vec::new();
    let mut x = 0_u32;
    while x < width {
        while x < width && color_at(pixels, width, x, y)? == background {
            x += 1;
        }
        if x == width {
            break;
        }
        let start_x = x;
        while x < width && color_at(pixels, width, x, y)? != background {
            x += 1;
        }
        if runs.len() == max_runs {
            return Ok(None);
        }
        let index = runs.len();
        runs.push(Run {
            y,
            start_x,
            end_x: x,
        });
        unions.push();
        indexes.push(index);
    }
    Ok(Some(indexes))
}

fn connect_adjacent_rows(
    runs: &[Run],
    previous: &[usize],
    current: &[usize],
    unions: &mut UnionFind,
) {
    let mut previous_start = 0_usize;
    for &current_index in current {
        let current_run = runs[current_index];
        while previous_start < previous.len()
            && runs[previous[previous_start]].end_x <= current_run.start_x
        {
            previous_start += 1;
        }
        let mut previous_index = previous_start;
        while previous_index < previous.len()
            && runs[previous[previous_index]].start_x < current_run.end_x
        {
            let other_index = previous[previous_index];
            let other = runs[other_index];
            if other.end_x > current_run.start_x {
                unions.union(current_index, other_index);
            }
            previous_index += 1;
        }
    }
}

fn aggregate_components(
    runs: &[Run],
    unions: &mut UnionFind,
    width: u32,
    height: u32,
    max_components: usize,
) -> ComponentCollection {
    let mut accumulators = BTreeMap::<usize, ComponentAccumulator>::new();
    for (index, &run) in runs.iter().enumerate() {
        let root = unions.find(index);
        if !accumulators.contains_key(&root) && accumulators.len() == max_components {
            return ComponentCollection::LimitExceeded;
        }
        accumulators
            .entry(root)
            .and_modify(|component| component.include(run, width, height))
            .or_insert_with(|| ComponentAccumulator::from_run(run, width, height));
    }

    let mut components: Vec<BackgroundRelativeComponent> = accumulators
        .into_values()
        .map(ComponentAccumulator::finish)
        .collect();
    components.sort_by(|left, right| {
        left.bounds
            .y
            .cmp(&right.bounds.y)
            .then_with(|| left.bounds.x.cmp(&right.bounds.x))
            .then_with(|| left.bounds.height.cmp(&right.bounds.height))
            .then_with(|| left.bounds.width.cmp(&right.bounds.width))
            .then_with(|| left.pixel_count.cmp(&right.pixel_count))
            .then_with(|| left.run_count.cmp(&right.run_count))
    });
    for (index, component) in components.iter_mut().enumerate() {
        component.index = u32::try_from(index).expect("component limit fits u32");
    }
    ComponentCollection::Complete(components)
}

fn color_at(
    pixels: &[u8],
    width: u32,
    x: u32,
    y: u32,
) -> Result<u32, PngComponentError> {
    let offset_u64 = (u64::from(y) * u64::from(width) + u64::from(x)) * 4;
    let offset = usize::try_from(offset_u64).map_err(|_| PngComponentError::PixelAddressMismatch)?;
    let rgba = pixels
        .get(offset..offset + 4)
        .ok_or(PngComponentError::PixelAddressMismatch)?;
    Ok(u32::from_be_bytes([rgba[0], rgba[1], rgba[2], rgba[3]]))
}

impl UnionFind {
    fn new() -> Self {
        Self { parents: Vec::new() }
    }

    fn push(&mut self) {
        self.parents.push(self.parents.len());
    }

    fn find(&mut self, index: usize) -> usize {
        let mut root = index;
        while self.parents[root] != root {
            root = self.parents[root];
        }
        let mut current = index;
        while self.parents[current] != current {
            let parent = self.parents[current];
            self.parents[current] = root;
            current = parent;
        }
        root
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        let (minimum, maximum) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        self.parents[maximum] = minimum;
    }
}

impl ComponentAccumulator {
    fn from_run(run: Run, width: u32, height: u32) -> Self {
        Self {
            min_x: run.start_x,
            min_y: run.y,
            max_x: run.end_x - 1,
            max_y: run.y,
            pixel_count: u64::from(run.end_x - run.start_x),
            run_count: 1,
            touches_canvas_edge: run.start_x == 0
                || run.end_x == width
                || run.y == 0
                || run.y + 1 == height,
        }
    }

    fn include(&mut self, run: Run, width: u32, height: u32) {
        self.min_x = self.min_x.min(run.start_x);
        self.min_y = self.min_y.min(run.y);
        self.max_x = self.max_x.max(run.end_x - 1);
        self.max_y = self.max_y.max(run.y);
        self.pixel_count += u64::from(run.end_x - run.start_x);
        self.run_count += 1;
        self.touches_canvas_edge |= run.start_x == 0
            || run.end_x == width
            || run.y == 0
            || run.y + 1 == height;
    }

    fn finish(self) -> BackgroundRelativeComponent {
        BackgroundRelativeComponent {
            index: 0,
            bounds: PixelRect {
                x: self.min_x,
                y: self.min_y,
                width: self.max_x - self.min_x + 1,
                height: self.max_y - self.min_y + 1,
            },
            pixel_count: self.pixel_count,
            run_count: self.run_count,
            touches_canvas_edge: self.touches_canvas_edge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BackgroundComponentStatus, ComponentCollection, RunCollection, SegmentationLimits,
        aggregate_components, collect_runs,
    };

    const BACKGROUND: [u8; 4] = [255, 255, 255, 255];
    const FOREGROUND: [u8; 4] = [0, 0, 0, 255];

    fn pixels(width: u32, height: u32, foreground: &[(u32, u32)]) -> Vec<u8> {
        let mut values = vec![
            BACKGROUND;
            usize::try_from(width * height).expect("test raster fits usize")
        ];
        for &(x, y) in foreground {
            values[usize::try_from(y * width + x).expect("test index fits usize")] = FOREGROUND;
        }
        values.into_iter().flatten().collect()
    }

    fn components(
        width: u32,
        height: u32,
        foreground: &[(u32, u32)],
        limits: SegmentationLimits,
    ) -> Result<ComponentCollection, BackgroundComponentStatus> {
        let values = pixels(width, height, foreground);
        let background = u32::from_be_bytes(BACKGROUND);
        match collect_runs(&values, width, height, background, limits.max_runs)
            .expect("valid test pixels")
        {
            RunCollection::LimitExceeded => Err(BackgroundComponentStatus::RunLimitExceeded),
            RunCollection::Complete { runs, mut unions } => Ok(aggregate_components(
                &runs,
                &mut unions,
                width,
                height,
                limits.max_components,
            )),
        }
    }

    #[test]
    fn diagonal_pixels_are_not_four_connected() {
        let result = components(
            3,
            3,
            &[(0, 0), (1, 1), (2, 2)],
            SegmentationLimits {
                max_runs: 10,
                max_components: 10,
            },
        )
        .expect("within run limit");
        let ComponentCollection::Complete(values) = result else {
            panic!("within component limit");
        };
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn overlapping_adjacent_runs_form_one_component() {
        let result = components(
            6,
            4,
            &[(1, 1), (2, 1), (2, 2), (3, 2), (4, 2)],
            SegmentationLimits {
                max_runs: 10,
                max_components: 10,
            },
        )
        .expect("within run limit");
        let ComponentCollection::Complete(values) = result else {
            panic!("within component limit");
        };
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].bounds.x, 1);
        assert_eq!(values[0].bounds.y, 1);
        assert_eq!(values[0].bounds.width, 4);
        assert_eq!(values[0].bounds.height, 2);
        assert_eq!(values[0].pixel_count, 5);
        assert_eq!(values[0].run_count, 2);
    }

    #[test]
    fn reduced_limits_return_explicit_outcomes() {
        let run_limited = components(
            6,
            1,
            &[(0, 0), (2, 0), (4, 0)],
            SegmentationLimits {
                max_runs: 2,
                max_components: 10,
            },
        );
        assert_eq!(run_limited, Err(BackgroundComponentStatus::RunLimitExceeded));

        let component_limited = components(
            6,
            1,
            &[(0, 0), (2, 0), (4, 0)],
            SegmentationLimits {
                max_runs: 10,
                max_components: 2,
            },
        )
        .expect("within run limit");
        assert!(matches!(component_limited, ComponentCollection::LimitExceeded));
    }
}
