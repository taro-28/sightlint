//! Evaluation-only comparison of exact-color background and segmentation hypotheses.
//!
//! This module does not produce rule results and does not change the strict [`crate::inspection`]
//! policy. See ADR 0039.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::inspection::{MAX_INSPECTION_PIXELS, MAX_REGIONS, Region, components, pixel};
use crate::{
    EncodedRgba8Raster, PngAdapterError, PngRasterError, PngRasterStatus, crc32, observe_png_raster,
};

const MAX_ROW_RUNS: usize = 250_000;
const STRICT_POLICY: &str = "strict-uniform-perimeter-flood-v1";
const RANKED_POLICY: &str = "ranked-exact-border-flood-v1";
const QUALIFIED_POLICY: &str = "qualified-corner-95-row-runs-v1";

/// A versioned evaluation report comparing three nonblocking segmentation policies.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageSegmentationBenchmark {
    report: Value,
}

impl ImageSegmentationBenchmark {
    /// Serializes the benchmark report with deterministic object-key ordering.
    ///
    /// # Errors
    /// Returns a serialization error if the report cannot be encoded.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        sightlint_ir::serialize_canonical(&self.report)
    }
}

/// Compares the ADR 0039 policies over one locally decoded PNG raster.
///
/// The result is acquisition evidence only. Background candidates remain unconfirmed, semantic
/// applicability is `cantTell`, and the rule outcome is `untested`.
///
/// # Errors
/// Propagates PNG validation/decoding errors and bounded allocation failures.
pub fn benchmark_png_segmentation(
    input: &[u8],
) -> Result<ImageSegmentationBenchmark, PngAdapterError> {
    let observed = observe_png_raster(input)?;
    let width = observed.structure.header.width;
    let height = observed.structure.header.height;
    let input_pixels = u64::from(width) * u64::from(height);
    let source_crc = format!("{:08x}", crc32(input));
    let mut report = json!({
        "benchmarkSchemaVersion": "0.1.0",
        "mode": "evaluationOnly",
        "blocking": false,
        "ruleOutcome": "untested",
        "canvas": {
            "id": "canvas", "width": width, "height": height,
            "unit": "devicePixel", "origin": "topLeft"
        },
        "source": {
            "adapter": "sightlint-adapter-png", "version": env!("CARGO_PKG_VERSION"),
            "externalProcessing": false, "byteCrc32": source_crc,
            "checksumPurpose": "regression-only-not-cryptographic"
        },
        "limits": {
            "pixels": MAX_INSPECTION_PIXELS, "regions": MAX_REGIONS, "rowRuns": MAX_ROW_RUNS
        },
        "policies": [],
        "limitations": [
            "Background candidates are unconfirmed exact-color hypotheses.",
            "Regions are pixel components, not semantic UI objects or peer groups.",
            "Encoded PNG channels are not display-color or compositing evidence.",
            "No benchmark observation is a passed, failed, or blocking rule result."
        ]
    });

    let policies = match observed.status {
        PngRasterStatus::Unavailable(reason) => {
            report["evidence"] = json!([source_evidence(input)]);
            unavailable_policies(reason.code(), input_pixels)
        }
        PngRasterStatus::Available(raster) => {
            let expected_bytes = input_pixels.checked_mul(4);
            if input_pixels == 0 || expected_bytes != Some(raster.pixels.len() as u64) {
                return Err(PngAdapterError::InvalidRasterData(
                    PngRasterError::InvalidLayout,
                ));
            }
            report["evidence"] = json!([raster_evidence(&raster)]);
            if input_pixels > MAX_INSPECTION_PIXELS as u64 {
                unavailable_policies("pixelBudgetExceeded", input_pixels)
            } else if raster.pixels.chunks_exact(4).any(|sample| sample[3] != 255) {
                unavailable_policies("nonOpaqueRaster", input_pixels)
            } else {
                evaluate_policies(&raster)?
            }
        }
    };
    report["policies"] = json!(policies);
    Ok(ImageSegmentationBenchmark { report })
}

fn source_evidence(input: &[u8]) -> Value {
    json!({
        "id": "source", "class": "exactSource", "selector": "PNG/source",
        "byteCrc32": format!("{:08x}", crc32(input)),
        "interpretation": "Source structure only; encoded RGBA raster unavailable"
    })
}

fn raster_evidence(raster: &EncodedRgba8Raster) -> Value {
    json!({
        "id": "raster", "class": "exactSource", "selector": "IDAT/encoded-rgba8-v1",
        "byteCrc32": format!("{:08x}", crc32(&raster.pixels)),
        "interpretation": "PNG encoded channels; no display-color, compositing, or semantic claim"
    })
}

fn unavailable_policies(reason: &'static str, input_pixels: u64) -> Vec<Value> {
    [
        (STRICT_POLICY, "fourConnectedFloodFill"),
        (RANKED_POLICY, "fourConnectedFloodFill"),
        (QUALIFIED_POLICY, "horizontalRowRunsUnionFind"),
    ]
    .into_iter()
    .map(|(policy, method)| {
        policy_report(
            policy,
            reason,
            &[],
            None,
            Segmentation::not_run(input_pixels, method),
        )
    })
    .collect()
}

#[derive(Debug, Clone)]
struct Candidate {
    color: [u8; 4],
    corner_count: usize,
    perimeter_count: usize,
    image_count: usize,
    corner_denominator: usize,
    perimeter_denominator: usize,
    image_denominator: usize,
    eligible: bool,
    rejection_reason: Option<&'static str>,
}

impl Candidate {
    fn id(&self) -> String {
        format!(
            "candidate:{:02x}{:02x}{:02x}{:02x}",
            self.color[0], self.color[1], self.color[2], self.color[3]
        )
    }

    fn as_json(&self) -> Value {
        json!({
            "candidateId": self.id(),
            "encodedRgba8": self.color,
            "cornerPixels": {
                "count": self.corner_count, "denominator": self.corner_denominator
            },
            "perimeterPixels": {
                "count": self.perimeter_count, "denominator": self.perimeter_denominator
            },
            "imagePixels": {
                "count": self.image_count, "denominator": self.image_denominator
            },
            "selectionEligibility": if self.eligible { "eligible" } else { "rejected" },
            "rejectionReason": self.rejection_reason
        })
    }
}

struct RasterCounts {
    corners: BTreeMap<[u8; 4], usize>,
    perimeter: BTreeMap<[u8; 4], usize>,
    corner_denominator: usize,
    perimeter_denominator: usize,
    image_denominator: usize,
}

fn evaluate_policies(raster: &EncodedRgba8Raster) -> Result<Vec<Value>, PngAdapterError> {
    let counts = raster_counts(raster);
    Ok(vec![
        strict_policy(raster, &counts)?,
        ranked_policy(raster, &counts)?,
        qualified_policy(raster, &counts)?,
    ])
}

fn strict_policy(
    raster: &EncodedRgba8Raster,
    counts: &RasterCounts,
) -> Result<Value, PngAdapterError> {
    let color = pixel(raster, 0);
    let uniform = counts.perimeter.get(&color).copied() == Some(counts.perimeter_denominator);
    let candidate = candidate(
        raster,
        counts,
        color,
        uniform,
        (!uniform).then_some("nonUniformBorder"),
    );
    if !uniform {
        let candidates = vec![candidate];
        return Ok(policy_report(
            STRICT_POLICY,
            "nonUniformBorder",
            &candidates,
            None,
            Segmentation::not_run(counts.image_denominator as u64, "fourConnectedFloodFill"),
        ));
    }
    let selected = candidate.clone();
    let segmentation = flood_segments(raster, color)?;
    Ok(policy_report_from_segments(
        STRICT_POLICY,
        &[candidate],
        &selected,
        segmentation,
        "fourConnectedFloodFill",
        counts.image_denominator as u64,
    ))
}

fn ranked_policy(
    raster: &EncodedRgba8Raster,
    counts: &RasterCounts,
) -> Result<Value, PngAdapterError> {
    let mut colors = counts.corners.keys().copied().collect::<BTreeSet<_>>();
    let mut edge = counts
        .perimeter
        .iter()
        .map(|(color, count)| (*color, *count))
        .collect::<Vec<_>>();
    edge.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    colors.extend(edge.into_iter().take(4).map(|(color, _)| color));
    let mut candidates = colors
        .into_iter()
        .map(|color| candidate(raster, counts, color, true, None))
        .collect::<Vec<_>>();
    sort_candidates(&mut candidates);
    candidates.truncate(8);
    let selected = candidates
        .first()
        .cloned()
        .expect("nonempty raster candidate");
    let segmentation = flood_segments(raster, selected.color)?;
    Ok(policy_report_from_segments(
        RANKED_POLICY,
        &candidates,
        &selected,
        segmentation,
        "fourConnectedFloodFill",
        counts.image_denominator as u64,
    ))
}

fn qualified_policy(
    raster: &EncodedRgba8Raster,
    counts: &RasterCounts,
) -> Result<Value, PngAdapterError> {
    let too_small = raster.width < 3 || raster.height < 3;
    let mut candidates = counts
        .corners
        .keys()
        .copied()
        .map(|color| {
            let edge_count = counts.perimeter.get(&color).copied().unwrap_or(0);
            let qualifies = !too_small
                && edge_count.saturating_mul(100)
                    >= counts.perimeter_denominator.saturating_mul(95);
            let reason = if too_small {
                Some("canvasTooSmall")
            } else if qualifies {
                None
            } else {
                Some("edgeSupportBelow95Percent")
            };
            candidate(raster, counts, color, qualifies, reason)
        })
        .collect::<Vec<_>>();
    sort_candidates(&mut candidates);
    let selected = candidates.iter().find(|item| item.eligible).cloned();
    let Some(selected) = selected else {
        let reason = if too_small {
            "canvasTooSmall"
        } else {
            "noQualifiedBackgroundCandidate"
        };
        return Ok(policy_report(
            QUALIFIED_POLICY,
            reason,
            &candidates,
            None,
            Segmentation::not_run(
                counts.image_denominator as u64,
                "horizontalRowRunsUnionFind",
            ),
        ));
    };
    let segmentation = row_run_segments(raster, selected.color)?;
    Ok(policy_report_from_segments(
        QUALIFIED_POLICY,
        &candidates,
        &selected,
        segmentation,
        "horizontalRowRunsUnionFind",
        counts.image_denominator as u64,
    ))
}

fn candidate(
    raster: &EncodedRgba8Raster,
    counts: &RasterCounts,
    color: [u8; 4],
    eligible: bool,
    rejection_reason: Option<&'static str>,
) -> Candidate {
    Candidate {
        color,
        corner_count: counts.corners.get(&color).copied().unwrap_or(0),
        perimeter_count: counts.perimeter.get(&color).copied().unwrap_or(0),
        image_count: raster
            .pixels
            .chunks_exact(4)
            .filter(|sample| *sample == color)
            .count(),
        corner_denominator: counts.corner_denominator,
        perimeter_denominator: counts.perimeter_denominator,
        image_denominator: counts.image_denominator,
        eligible,
        rejection_reason,
    }
}

fn sort_candidates(candidates: &mut [Candidate]) {
    candidates.sort_by(|left, right| {
        right
            .corner_count
            .cmp(&left.corner_count)
            .then_with(|| right.perimeter_count.cmp(&left.perimeter_count))
            .then_with(|| right.image_count.cmp(&left.image_count))
            .then_with(|| left.color.cmp(&right.color))
    });
}

fn raster_counts(raster: &EncodedRgba8Raster) -> RasterCounts {
    let width = usize::try_from(raster.width).expect("bounded raster width");
    let height = usize::try_from(raster.height).expect("bounded raster height");
    let image_denominator = width * height;
    let corner_indices = [0, width - 1, (height - 1) * width, height * width - 1]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut corners = BTreeMap::new();
    let mut perimeter = BTreeMap::new();
    let mut perimeter_denominator = 0;
    for index in 0..image_denominator {
        let color = pixel(raster, index);
        if corner_indices.contains(&index) {
            *corners.entry(color).or_insert(0) += 1;
        }
        let x = index % width;
        let y = index / width;
        if x == 0 || x + 1 == width || y == 0 || y + 1 == height {
            *perimeter.entry(color).or_insert(0) += 1;
            perimeter_denominator += 1;
        }
    }
    RasterCounts {
        corners,
        perimeter,
        corner_denominator: corner_indices.len(),
        perimeter_denominator,
        image_denominator,
    }
}

enum SegmentResult {
    Observed {
        regions: Vec<Region>,
        row_run_count: Option<usize>,
    },
    Unavailable {
        reason: &'static str,
        row_run_count: Option<usize>,
    },
}

fn flood_segments(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
) -> Result<SegmentResult, PngAdapterError> {
    let count = usize::try_from(u64::from(raster.width) * u64::from(raster.height))
        .map_err(|_| PngAdapterError::InvalidRasterData(PngRasterError::InvalidLayout))?;
    let Some(regions) = components(raster, background, count, MAX_REGIONS)
        .map_err(PngAdapterError::InvalidRasterData)?
    else {
        return Ok(SegmentResult::Unavailable {
            reason: "regionBudgetExceeded",
            row_run_count: None,
        });
    };
    Ok(SegmentResult::Observed {
        regions,
        row_run_count: None,
    })
}

#[derive(Debug, Clone, Copy)]
struct RowRun {
    y: u32,
    start: u32,
    end: u32,
}

#[derive(Debug)]
struct CollectedRuns {
    runs: Vec<RowRun>,
    row_starts: Vec<usize>,
}

#[derive(Debug)]
struct Aggregate {
    seed: usize,
    minimum_x: u32,
    minimum_y: u32,
    maximum_x: u32,
    maximum_y: u32,
    count: u32,
    color: [u8; 4],
    uniform_color: bool,
}

#[derive(Debug)]
struct UnionFind {
    parents: Vec<usize>,
    ranks: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Result<Self, PngRasterError> {
        let mut parents = Vec::new();
        parents
            .try_reserve_exact(size)
            .map_err(|_| PngRasterError::AllocationFailed)?;
        parents.extend(0..size);
        let mut ranks = Vec::new();
        ranks
            .try_reserve_exact(size)
            .map_err(|_| PngRasterError::AllocationFailed)?;
        ranks.resize(size, 0);
        Ok(Self { parents, ranks })
    }

    fn find(&mut self, item: usize) -> usize {
        let parent = self.parents[item];
        if parent == item {
            item
        } else {
            let root = self.find(parent);
            self.parents[item] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left_root = self.find(left);
        let mut right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        match self.ranks[left_root].cmp(&self.ranks[right_root]) {
            Ordering::Less => std::mem::swap(&mut left_root, &mut right_root),
            Ordering::Equal => self.ranks[left_root] += 1,
            Ordering::Greater => {}
        }
        self.parents[right_root] = left_root;
    }
}

fn row_run_segments(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
) -> Result<SegmentResult, PngAdapterError> {
    let width = usize::try_from(raster.width)
        .map_err(|_| PngAdapterError::InvalidRasterData(PngRasterError::InvalidLayout))?;
    let height = usize::try_from(raster.height)
        .map_err(|_| PngAdapterError::InvalidRasterData(PngRasterError::InvalidLayout))?;
    let Some(collected) = collect_row_runs(raster, background, width, height)? else {
        return Ok(SegmentResult::Unavailable {
            reason: "runBudgetExceeded",
            row_run_count: Some(MAX_ROW_RUNS + 1),
        });
    };
    let mut union_find = connect_row_runs(&collected, height)?;
    let Some(regions) = aggregate_row_runs(raster, width, &collected.runs, &mut union_find) else {
        return Ok(SegmentResult::Unavailable {
            reason: "regionBudgetExceeded",
            row_run_count: Some(collected.runs.len()),
        });
    };
    Ok(SegmentResult::Observed {
        regions,
        row_run_count: Some(collected.runs.len()),
    })
}

fn collect_row_runs(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
    width: usize,
    height: usize,
) -> Result<Option<CollectedRuns>, PngAdapterError> {
    let mut runs = Vec::new();
    runs.try_reserve_exact(MAX_ROW_RUNS.min(width.saturating_mul(height)))
        .map_err(|_| PngAdapterError::InvalidRasterData(PngRasterError::AllocationFailed))?;
    let mut row_starts = Vec::new();
    row_starts
        .try_reserve_exact(height + 1)
        .map_err(|_| PngAdapterError::InvalidRasterData(PngRasterError::AllocationFailed))?;
    for y in 0..height {
        row_starts.push(runs.len());
        let mut x = 0;
        while x < width {
            while x < width && pixel(raster, y * width + x) == background {
                x += 1;
            }
            if x == width {
                break;
            }
            let start = x;
            while x < width && pixel(raster, y * width + x) != background {
                x += 1;
            }
            if runs.len() == MAX_ROW_RUNS {
                return Ok(None);
            }
            runs.push(RowRun {
                y: u32::try_from(y).expect("bounded row"),
                start: u32::try_from(start).expect("bounded run start"),
                end: u32::try_from(x - 1).expect("bounded run end"),
            });
        }
    }
    row_starts.push(runs.len());
    Ok(Some(CollectedRuns { runs, row_starts }))
}

fn connect_row_runs(
    collected: &CollectedRuns,
    height: usize,
) -> Result<UnionFind, PngAdapterError> {
    let runs = &collected.runs;
    let row_starts = &collected.row_starts;
    let mut union_find = UnionFind::new(runs.len()).map_err(PngAdapterError::InvalidRasterData)?;
    for y in 1..height {
        let previous = row_starts[y - 1]..row_starts[y];
        let current = row_starts[y]..row_starts[y + 1];
        let mut previous_index = previous.start;
        for current_index in current {
            while previous_index < previous.end
                && runs[previous_index].end < runs[current_index].start
            {
                previous_index += 1;
            }
            let mut overlap = previous_index;
            while overlap < previous.end && runs[overlap].start <= runs[current_index].end {
                union_find.union(current_index, overlap);
                overlap += 1;
            }
        }
    }
    Ok(union_find)
}

fn aggregate_row_runs(
    raster: &EncodedRgba8Raster,
    width: usize,
    runs: &[RowRun],
    union_find: &mut UnionFind,
) -> Option<Vec<Region>> {
    let mut roots = BTreeSet::new();
    for index in 0..runs.len() {
        roots.insert(union_find.find(index));
        if roots.len() > MAX_REGIONS {
            return None;
        }
    }

    let mut aggregates: BTreeMap<usize, Aggregate> = BTreeMap::new();
    for (index, run) in runs.iter().enumerate() {
        let root = union_find.find(index);
        let seed = usize::try_from(run.y).expect("bounded row") * width
            + usize::try_from(run.start).expect("bounded start");
        let first_color = pixel(raster, seed);
        let aggregate = aggregates.entry(root).or_insert(Aggregate {
            seed,
            minimum_x: run.start,
            minimum_y: run.y,
            maximum_x: run.end,
            maximum_y: run.y,
            count: 0,
            color: first_color,
            uniform_color: true,
        });
        aggregate.seed = aggregate.seed.min(seed);
        aggregate.minimum_x = aggregate.minimum_x.min(run.start);
        aggregate.minimum_y = aggregate.minimum_y.min(run.y);
        aggregate.maximum_x = aggregate.maximum_x.max(run.end);
        aggregate.maximum_y = aggregate.maximum_y.max(run.y);
        for x in run.start..=run.end {
            let sample_index = usize::try_from(run.y).expect("bounded row") * width
                + usize::try_from(x).expect("bounded column");
            aggregate.uniform_color &= pixel(raster, sample_index) == aggregate.color;
            aggregate.count += 1;
        }
    }
    let mut regions = aggregates
        .into_values()
        .map(|item| Region {
            seed: item.seed,
            bounds: [
                item.minimum_x,
                item.minimum_y,
                item.maximum_x - item.minimum_x + 1,
                item.maximum_y - item.minimum_y + 1,
            ],
            count: item.count,
            color: item.color,
            uniform_color: item.uniform_color,
        })
        .collect::<Vec<_>>();
    regions.sort_by_key(|item| item.seed);
    Some(regions)
}

struct Segmentation {
    status: &'static str,
    reason: Option<&'static str>,
    method: &'static str,
    input_pixel_count: u64,
    foreground_pixel_count: Option<u64>,
    row_run_count: Option<usize>,
    regions: Vec<Region>,
}

impl Segmentation {
    fn not_run(input_pixel_count: u64, _planned_method: &'static str) -> Self {
        Self {
            status: "unavailable",
            reason: None,
            method: "notRun",
            input_pixel_count,
            foreground_pixel_count: None,
            row_run_count: None,
            regions: Vec::new(),
        }
    }
}

fn policy_report_from_segments(
    policy_id: &'static str,
    candidates: &[Candidate],
    selected: &Candidate,
    result: SegmentResult,
    method: &'static str,
    input_pixel_count: u64,
) -> Value {
    let segmentation = match result {
        SegmentResult::Observed {
            regions,
            row_run_count,
        } => Segmentation {
            status: "observed",
            reason: None,
            method,
            input_pixel_count,
            foreground_pixel_count: Some(regions.iter().map(|item| u64::from(item.count)).sum()),
            row_run_count,
            regions,
        },
        SegmentResult::Unavailable {
            reason,
            row_run_count,
        } => Segmentation {
            status: "unavailable",
            reason: Some(reason),
            method,
            input_pixel_count,
            foreground_pixel_count: None,
            row_run_count,
            regions: Vec::new(),
        },
    };
    policy_report(
        policy_id,
        segmentation.reason.unwrap_or(""),
        candidates,
        Some(selected),
        segmentation,
    )
}

fn policy_report(
    policy_id: &'static str,
    reason: &'static str,
    candidates: &[Candidate],
    selected: Option<&Candidate>,
    mut segmentation: Segmentation,
) -> Value {
    if segmentation.reason.is_none() && segmentation.status == "unavailable" && !reason.is_empty() {
        segmentation.reason = Some(reason);
    }
    let selected_id = selected.map(Candidate::id);
    let hypothesis_id = selected_id.as_deref().unwrap_or("candidate:00000000");
    let region_json = segmentation
        .regions
        .iter()
        .map(|region| region.as_benchmark_json(hypothesis_id))
        .collect::<Vec<_>>();
    json!({
        "policyId": policy_id,
        "status": segmentation.status,
        "reason": segmentation.reason,
        "semanticApplicability": "cantTell",
        "ruleOutcome": "untested",
        "backgroundSelection": {
            "status": if selected_id.is_some() { "selected" } else { "unavailable" },
            "selectedCandidateId": selected_id,
            "confirmed": false,
            "semanticConfidence": null,
            "calibration": "notCalibrated",
            "candidates": candidates.iter().map(Candidate::as_json).collect::<Vec<_>>()
        },
        "segmentation": {
            "method": segmentation.method,
            "connectivity": "four",
            "inputPixelCount": segmentation.input_pixel_count,
            "foregroundPixelCount": segmentation.foreground_pixel_count,
            "rowRunCount": segmentation.row_run_count
        },
        "regions": region_json,
        "summary": {
            "candidateCount": candidates.len(),
            "regionCount": segmentation.regions.len()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_ROW_RUNS, benchmark_png_segmentation, row_run_segments};
    use crate::{EncodedRgba8Raster, segmentation::SegmentResult};

    #[test]
    fn row_runs_preserve_four_connectivity_and_do_not_merge_diagonals() {
        let raster = EncodedRgba8Raster {
            width: 3,
            height: 3,
            pixels: [255_u8, 0, 255, 0, 255, 0, 255, 0, 255]
                .into_iter()
                .flat_map(|value| [value, value, value, 255])
                .collect(),
        };
        let SegmentResult::Observed {
            regions,
            row_run_count,
        } = row_run_segments(&raster, [255; 4]).unwrap()
        else {
            panic!("bounded checkerboard should be observed");
        };
        assert_eq!(row_run_count, Some(4));
        assert_eq!(regions.len(), 4);
        assert!(regions.iter().all(|region| region.bounds[2..] == [1, 1]));
    }

    #[test]
    fn row_run_limit_accepts_the_boundary_and_discards_overflow() {
        assert_eq!(MAX_ROW_RUNS, 250_000);
        for height in [500_u32, 502] {
            let pixels = (0..height)
                .flat_map(|_| 0..1_001_u32)
                .flat_map(|column| {
                    let value = if column % 2 == 0 { 255 } else { 0 };
                    [value, value, value, 255]
                })
                .collect();
            let raster = EncodedRgba8Raster {
                width: 1_001,
                height,
                pixels,
            };
            match row_run_segments(&raster, [255; 4]).unwrap() {
                SegmentResult::Observed {
                    regions,
                    row_run_count,
                } => {
                    assert_eq!(height, 500);
                    assert_eq!(row_run_count, Some(MAX_ROW_RUNS));
                    assert_eq!(regions.len(), 500);
                }
                SegmentResult::Unavailable {
                    reason,
                    row_run_count,
                } => {
                    assert_eq!(height, 502);
                    assert_eq!(reason, "runBudgetExceeded");
                    assert_eq!(row_run_count, Some(MAX_ROW_RUNS + 1));
                }
            }
        }
    }

    #[test]
    fn malformed_png_remains_an_error() {
        assert!(benchmark_png_segmentation(b"not a PNG").is_err());
    }
}
