//! Conservative, advisory-only observations over source pixels. See ADR 0031.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};

use crate::{
    EncodedRgba8Raster, PngAdapterError, PngRasterError, PngRasterStatus, crc32,
    observe_png_raster,
};

const MAX_INSPECTION_PIXELS: usize = 4_194_304;
const MAX_REGIONS: usize = 1_024;

/// A versioned observation report, deliberately separate from a trusted rule report.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInspection {
    report: Value,
}

impl ImageInspection {
    /// Serializes observations deterministically. This is not the CheckReport schema.
    ///
    /// # Errors
    /// Returns a serialization error if the observation report cannot be encoded.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        sightlint_ir::serialize_canonical(&self.report)
    }

    /// Formats measured gaps with explicit advisory and semantic-uncertainty labels.
    #[must_use]
    pub fn to_human(&self) -> String {
        let mut output = String::from(
            "SightLint image inspection — advisory only, not a UX verdict\n",
        );
        let _ = writeln!(output, "status: {}", self.report["status"]);
        if let Some(reason) = self.report["reason"].as_str() {
            let _ = writeln!(output, "coverage unavailable: {reason}");
        }
        let _ = writeln!(
            output,
            "{} region(s), {} repeated-shape candidate(s)",
            self.report["summary"]["regionCount"], self.report["summary"]["groupCount"]
        );
        if let Some(groups) = self.report["groups"].as_array() {
            for group in groups {
                let label = if group["pattern"] == "unequal" {
                    "ADVISORY unequal gaps"
                } else {
                    "OBSERVED uniform gaps"
                };
                let _ = writeln!(
                    output,
                    "{label}: {} devicePixel ({})",
                    group["gaps"], group["axis"]
                );
            }
        }
        output.push_str(
            "Background and repeated-peer meaning are hypotheses; confirm design intent.\n",
        );
        output
    }
}

/// Inspects a PNG for region and repeated-spacing candidates without a blocking verdict.
///
/// No network access, color correction, OCR, semantic roles, or user-defined policy is used.
/// Unsupported coverage is explicit; even measured unequal gaps do not prove a UX defect.
///
/// # Errors
/// Propagates PNG acquisition errors, or a bounded-buffer allocation/internal-layout error.
pub fn inspect_png(input: &[u8]) -> Result<ImageInspection, PngAdapterError> {
    let observed = observe_png_raster(input)?;
    let mut report = json!({
        "inspectionSchemaVersion": "0.1.0",
        "algorithm": "uniform-perimeter-four-connected-v1",
        "mode": "advisory",
        "blocking": false,
        "status": "unavailable",
        "uxVerdict": "untested",
        "canvas": {
            "id": "canvas", "width": observed.structure.header.width,
            "height": observed.structure.header.height, "unit": "devicePixel",
            "origin": "topLeft"
        },
        "source": {
            "adapter": "sightlint-adapter-png", "version": env!("CARGO_PKG_VERSION"),
            "externalProcessing": false, "byteCrc32": format!("{:08x}", crc32(input)),
            "checksumPurpose": "regression-only-not-cryptographic"
        },
        "limits": { "pixels": MAX_INSPECTION_PIXELS, "regions": MAX_REGIONS },
        "regions": [], "groups": [],
        "summary": { "regionCount": 0, "groupCount": 0, "unequalGapGroupCount": 0 }
    });
    match observed.status {
        PngRasterStatus::Unavailable(reason) => report["reason"] = json!(reason.code()),
        PngRasterStatus::Available(raster) => {
            inspect_raster(&raster, &mut report).map_err(PngAdapterError::InvalidRasterData)?;
        }
    }
    Ok(ImageInspection { report })
}

#[derive(Debug, Clone)]
struct Region {
    seed: usize,
    bounds: [u32; 4],
    count: u32,
    color: [u8; 4],
    uniform_color: bool,
}

impl Region {
    fn id(&self) -> String {
        format!("region:{}", self.seed)
    }

    fn is_solid_rectangle(&self) -> bool {
        self.uniform_color && u64::from(self.count) == self.area()
    }

    fn area(&self) -> u64 {
        u64::from(self.bounds[2]) * u64::from(self.bounds[3])
    }

    fn as_json(&self) -> Value {
        json!({
            "id": self.id(), "bounds": self.bounds, "coordinateSpaceId": "canvas",
            "boundsFormat": "xywh-half-open", "unit": "devicePixel",
            "pixelCount": self.count, "singleColorRectangle": self.is_solid_rectangle(),
            "evidenceId": "raster", "hypothesisId": "border-background"
        })
    }
}

fn inspect_raster(raster: &EncodedRgba8Raster, report: &mut Value) -> Result<(), PngRasterError> {
    let count = u64::from(raster.width) * u64::from(raster.height);
    if count == 0 || count.checked_mul(4) != Some(raster.pixels.len() as u64) {
        return Err(PngRasterError::InvalidLayout);
    }
    if count > MAX_INSPECTION_PIXELS as u64 {
        report["reason"] = json!("pixelBudgetExceeded");
        return Ok(());
    }
    if raster.pixels.chunks_exact(4).any(|pixel| pixel[3] != 255) {
        report["reason"] = json!("nonOpaqueRaster");
        return Ok(());
    }
    let background = pixel(raster, 0);
    let width = usize::try_from(raster.width).map_err(|_| PngRasterError::InvalidLayout)?;
    let count = usize::try_from(count).map_err(|_| PngRasterError::InvalidLayout)?;
    let varying_border = (0..count).any(|index| {
        let border = index < width || index >= count - width
            || index % width == 0 || index % width == width - 1;
        border && pixel(raster, index) != background
    });
    if varying_border {
        report["reason"] = json!("nonUniformBorder");
        return Ok(());
    }
    let Some(regions) = components(raster, background, count, MAX_REGIONS)? else {
        report["reason"] = json!("regionBudgetExceeded");
        return Ok(());
    };
    let groups = repeated_groups(&regions);
    report["status"] = json!("observed");
    report["uxVerdict"] = json!("cantTell");
    report["evidence"] = json!([{
        "id": "raster", "class": "exactSource", "selector": "IDAT/encoded-rgba8-v1",
        "byteCrc32": format!("{:08x}", crc32(&raster.pixels))
    }]);
    report["backgroundHypothesis"] = json!({
        "id": "border-background", "method": "uniformOpaquePerimeter",
        "encodedRgba8": background, "evidenceIds": ["raster"],
        "semanticConfidence": null, "calibration": "notCalibrated",
        "confirmed": false
    });
    report["summary"] = json!({
        "regionCount": regions.len(), "groupCount": groups.len(),
        "unequalGapGroupCount": groups.iter().filter(|group| group["pattern"] == "unequal").count()
    });
    report["regions"] = json!(regions.iter().map(Region::as_json).collect::<Vec<_>>());
    report["groups"] = json!(groups);
    Ok(())
}

fn pixel(raster: &EncodedRgba8Raster, index: usize) -> [u8; 4] {
    raster.pixels[index * 4..index * 4 + 4]
        .try_into()
        .expect("validated raster index")
}

fn components(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
    count: usize,
    region_limit: usize,
) -> Result<Option<Vec<Region>>, PngRasterError> {
    let mut visited = Vec::new();
    visited.try_reserve_exact(count).map_err(|_| PngRasterError::AllocationFailed)?;
    visited.resize(count, 0_u8);
    let mut stack = Vec::new();
    stack.try_reserve_exact(count).map_err(|_| PngRasterError::AllocationFailed)?;
    let mut regions = Vec::new();
    for seed in 0..count {
        if visited[seed] != 0 || pixel(raster, seed) == background {
            continue;
        }
        if regions.len() == region_limit {
            return Ok(None);
        }
        let region = flood(raster, background, seed, &mut visited, &mut stack);
        regions.push(region);
    }
    Ok(Some(regions))
}

fn enqueue(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
    index: usize,
    visited: &mut [u8],
    stack: &mut Vec<usize>,
) {
    if visited[index] == 0 {
        visited[index] = 1;
        if pixel(raster, index) != background {
            stack.push(index);
        }
    }
}

fn flood(
    raster: &EncodedRgba8Raster,
    background: [u8; 4],
    seed: usize,
    visited: &mut [u8],
    stack: &mut Vec<usize>,
) -> Region {
    let width = usize::try_from(raster.width).expect("bounded raster width");
    let mut minimum_x = raster.width;
    let mut minimum_y = raster.height;
    let mut maximum_x = 0;
    let mut maximum_y = 0;
    let color = pixel(raster, seed);
    let mut uniform_color = true;
    let mut count = 0;
    enqueue(raster, background, seed, visited, stack);
    while let Some(index) = stack.pop() {
        let x = u32::try_from(index % width).expect("bounded raster x");
        let y = u32::try_from(index / width).expect("bounded raster y");
        minimum_x = minimum_x.min(x);
        minimum_y = minimum_y.min(y);
        maximum_x = maximum_x.max(x);
        maximum_y = maximum_y.max(y);
        uniform_color &= pixel(raster, index) == color;
        count += 1;
        if x > 0 {
            enqueue(raster, background, index - 1, visited, stack);
        }
        if x + 1 < raster.width {
            enqueue(raster, background, index + 1, visited, stack);
        }
        if y > 0 {
            enqueue(raster, background, index - width, visited, stack);
        }
        if y + 1 < raster.height {
            enqueue(raster, background, index + width, visited, stack);
        }
    }
    Region {
        seed,
        bounds: [minimum_x, minimum_y, maximum_x - minimum_x + 1, maximum_y - minimum_y + 1],
        count,
        color,
        uniform_color,
    }
}

type GroupKey = (u32, u32, u32, [u8; 4]);

fn repeated_groups(regions: &[Region]) -> Vec<Value> {
    let mut groups = Vec::new();
    for axis in [0_usize, 1] {
        let mut candidates: BTreeMap<GroupKey, Vec<usize>> = BTreeMap::new();
        for (index, region) in regions.iter().enumerate() {
            if region.is_solid_rectangle() {
                let bounds = region.bounds;
                let key = (bounds[1 - axis], bounds[2], bounds[3], region.color);
                candidates.entry(key).or_default().push(index);
            }
        }
        for mut indices in candidates.into_values() {
            if indices.len() < 3 {
                continue;
            }
            indices.sort_by_key(|index| regions[*index].bounds[axis]);
            if let Some(group) = describe_group(regions, &indices, axis) {
                groups.push(group);
            }
        }
    }
    groups
}

fn intersects(left: [u32; 4], right: [u32; 4]) -> bool {
    left[0] < right[0] + right[2] && right[0] < left[0] + left[2]
        && left[1] < right[1] + right[3] && right[1] < left[1] + left[3]
}

fn describe_group(regions: &[Region], indices: &[usize], axis: usize) -> Option<Value> {
    let first = &regions[*indices.first()?];
    let last = &regions[*indices.last()?];
    let mut strip = first.bounds;
    strip[axis + 2] = last.bounds[axis] + last.bounds[axis + 2] - strip[axis];
    if regions.iter().enumerate().any(|(index, region)| {
        !indices.contains(&index) && intersects(strip, region.bounds)
    }) {
        return None;
    }
    let gaps = indices.windows(2).map(|pair| {
        let left = regions[pair[0]].bounds;
        let right = regions[pair[1]].bounds;
        right[axis].checked_sub(left[axis] + left[axis + 2])
    }).collect::<Option<Vec<u32>>>()?;
    let minimum = *gaps.iter().min()?;
    let maximum = *gaps.iter().max()?;
    if minimum == 0 {
        return None;
    }
    let direction = if axis == 0 { "horizontal" } else { "vertical" };
    let unequal = minimum != maximum;
    Some(json!({
        "id": format!("{direction}:{}", first.id()), "axis": direction,
        "regionIds": indices.iter().map(|index| regions[*index].id()).collect::<Vec<_>>(),
        "gaps": gaps, "minimumGap": minimum, "maximumGap": maximum,
        "gapSpread": maximum - minimum, "unit": "devicePixel", "measurementTolerance": 0,
        "pattern": if unequal { "unequal" } else { "uniform" },
        "uxVerdict": "cantTell", "blocking": false,
        "semanticConfidence": null, "calibration": "notCalibrated",
        "groupingMethod": "same-size-single-color-rectangles-in-one-line",
        "evidenceIds": ["raster"], "hypothesisId": "border-background",
        "advice": if unequal {
            "Measured gaps differ. Confirm these shapes are peers and whether spacing should be uniform."
        } else {
            "Measured gaps are uniform. This does not establish semantic peers or overall UX quality."
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::{MAX_INSPECTION_PIXELS, MAX_REGIONS, components, inspect_raster};
    use crate::{EncodedRgba8Raster, PngRasterError};
    use serde_json::json;

    #[test]
    fn invalid_raster_shape_is_an_error_not_partial_success() {
        let raster = EncodedRgba8Raster { width: 2, height: 1, pixels: vec![0; 4] };
        assert_eq!(inspect_raster(&raster, &mut json!({})), Err(PngRasterError::InvalidLayout));
    }

    #[test]
    fn component_budget_is_exact_and_overflow_discards_partial_observations() {
        let raster = EncodedRgba8Raster {
            width: 5, height: 1,
            pixels: [0_u8, 255, 0, 255, 0].into_iter().flat_map(|v| [v, v, v, 255]).collect(),
        };
        assert_eq!(components(&raster, [255; 4], 5, 3).unwrap().unwrap().len(), 3);
        assert!(components(&raster, [255; 4], 5, 2).unwrap().is_none());
        assert_eq!(MAX_REGIONS, 1_024);
        assert_eq!(MAX_INSPECTION_PIXELS, 4_194_304);
    }

    #[test]
    fn diagonal_pixels_remain_separate_under_four_connectivity() {
        let raster = EncodedRgba8Raster {
            width: 2, height: 2,
            pixels: [0_u8, 255, 255, 0].into_iter().flat_map(|v| [v, v, v, 255]).collect(),
        };
        let regions = components(&raster, [255; 4], 4, 2).unwrap().unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].bounds, [0, 0, 1, 1]);
        assert_eq!(regions[1].bounds, [1, 1, 1, 1]);
    }
}
