//! Source-sample expansion, not display color management or semantic perception.

use std::{error::Error, fmt};

use serde_json::{Value, json};
use sightlint_ir::{ArtifactIr, Identifier, Selector};

use crate::{
    AlphaBounds, AlphaEdgeCount, AlphaGeometry, PNG_EXTENSION_KEY, PngAdapterError, PngHeader,
    PngPass, PngStructure, ReconstructedPng, TransparentInsets, crc32, observe_alpha_geometry,
    reconstruct_png_scanlines,
};

const MAX_RGBA8_BYTES: u64 = 256 * 1024 * 1024;

/// Row-major, unassociated PNG-encoded RGBA8 samples. No color transform is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedRgba8Raster {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// Exact R, G, B, A source bytes, including RGB under zero alpha.
    pub pixels: Vec<u8>,
}

/// Why this stage deliberately does not expose a raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngRasterUnavailable {
    /// Palette expansion is not implemented.
    IndexedColor,
    /// This stage does not unpack or quantize non-eight-bit samples.
    UnsupportedBitDepth {
        /// Source bit depth.
        bit_depth: u8,
        /// Source color type.
        color_type: u8,
    },
    /// The semantics of tRNS are not interpreted in this stage.
    TransparencyChunk,
    /// Frame selection/compositing is not implemented for animated PNGs.
    AnimationChunks,
    /// Expanded samples would exceed the allocation budget.
    BufferTooLarge {
        /// Required RGBA byte count.
        required: u64,
        /// Maximum additional RGBA byte count.
        limit: u64,
    },
}

impl PngRasterUnavailable {
    /// Stable serialized reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::IndexedColor => "indexedColor",
            Self::UnsupportedBitDepth { .. } => "unsupportedBitDepth",
            Self::TransparencyChunk => "transparencyChunk",
            Self::AnimationChunks => "animationChunks",
            Self::BufferTooLarge { .. } => "bufferTooLarge",
        }
    }
}

/// Availability does not assert validation of every unsupported PNG feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngRasterStatus {
    /// Exact source-sample expansion is available.
    Available(EncodedRgba8Raster),
    /// The validated source stages succeeded but raster interpretation is unsupported.
    Unavailable(PngRasterUnavailable),
}

/// Source observations after filter reconstruction and optional raster expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedPngRaster {
    /// Validated source structure.
    pub structure: PngStructure,
    /// Packed bytes before expansion.
    pub reconstructed_packed_sample_bytes: usize,
    /// Number of nonempty source passes.
    pub non_empty_pass_count: usize,
    /// Staged raster availability.
    pub status: PngRasterStatus,
}

/// Structured failure at the raster boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PngRasterError {
    /// Source framing, pass coordinates, or byte ranges violate the validated layout.
    InvalidLayout,
    /// The allocator refused an otherwise budget-compliant raster allocation.
    AllocationFailed,
}

impl fmt::Display for PngRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLayout => "PNG raster layout is inconsistent with validated source data",
            Self::AllocationFailed => "PNG raster allocation failed within the configured budget",
        })
    }
}

impl Error for PngRasterError {}

/// Validates source stages and expands supported samples without color management.
///
/// # Errors
/// Returns source validation/filter errors unchanged, or a structured raster layout/allocation
/// error. Unsupported interpretation returns `Unavailable`, not a fabricated raster.
pub fn observe_png_raster(input: &[u8]) -> Result<ObservedPngRaster, PngAdapterError> {
    let reconstructed = reconstruct_png_scanlines(input)?;
    let status =
        raster_status(input, &reconstructed).map_err(PngAdapterError::InvalidRasterData)?;
    Ok(ObservedPngRaster {
        structure: reconstructed.structure,
        reconstructed_packed_sample_bytes: reconstructed.packed_sample_bytes.len(),
        non_empty_pass_count: reconstructed.passes.len(),
        status,
    })
}

fn raster_status(
    input: &[u8],
    reconstructed: &ReconstructedPng,
) -> Result<PngRasterStatus, PngRasterError> {
    let (transparency, animation) = unsupported_chunks(input)?;
    if animation {
        return Ok(PngRasterStatus::Unavailable(
            PngRasterUnavailable::AnimationChunks,
        ));
    }
    if let Some(reason) = classify_raster(reconstructed.structure.header, transparency)? {
        return Ok(PngRasterStatus::Unavailable(reason));
    }
    scatter_rgba8(reconstructed).map(PngRasterStatus::Available)
}

fn classify_raster(
    header: PngHeader,
    transparency: bool,
) -> Result<Option<PngRasterUnavailable>, PngRasterError> {
    if transparency {
        return Ok(Some(PngRasterUnavailable::TransparencyChunk));
    }
    if header.color_type == 3 {
        return Ok(Some(PngRasterUnavailable::IndexedColor));
    }
    if header.bit_depth != 8 {
        return Ok(Some(PngRasterUnavailable::UnsupportedBitDepth {
            bit_depth: header.bit_depth,
            color_type: header.color_type,
        }));
    }
    let required = rgba_length(header)?;
    Ok(
        (required > MAX_RGBA8_BYTES).then_some(PngRasterUnavailable::BufferTooLarge {
            required,
            limit: MAX_RGBA8_BYTES,
        }),
    )
}

fn rgba_length(header: PngHeader) -> Result<u64, PngRasterError> {
    u64::from(header.width)
        .checked_mul(u64::from(header.height))
        .and_then(|count| count.checked_mul(4))
        .ok_or(PngRasterError::InvalidLayout)
}

fn scatter_rgba8(source: &ReconstructedPng) -> Result<EncodedRgba8Raster, PngRasterError> {
    let header = source.structure.header;
    let channels = match header.color_type {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return Err(PngRasterError::InvalidLayout),
    };
    let length = rgba_length(header)?;
    if length > MAX_RGBA8_BYTES {
        return Err(PngRasterError::InvalidLayout);
    }
    let length = usize::try_from(length).map_err(|_| PngRasterError::InvalidLayout)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .map_err(|_| PngRasterError::AllocationFailed)?;
    pixels.resize(length, 0);
    let mut covered = 0_u64;
    for pass in &source.passes {
        scatter_pass(source, pass, channels, &mut pixels)?;
        covered += u64::from(pass.width) * u64::from(pass.height);
    }
    if covered != u64::from(header.width) * u64::from(header.height) {
        return Err(PngRasterError::InvalidLayout);
    }
    Ok(EncodedRgba8Raster {
        width: header.width,
        height: header.height,
        pixels,
    })
}

fn sample_offset(
    pass: &PngPass,
    row: u32,
    column: u32,
    channels: usize,
) -> Result<usize, PngRasterError> {
    let row = usize::try_from(row).map_err(|_| PngRasterError::InvalidLayout)?;
    let column = usize::try_from(column).map_err(|_| PngRasterError::InvalidLayout)?;
    let column_bytes = column
        .checked_mul(channels)
        .ok_or(PngRasterError::InvalidLayout)?;
    row.checked_mul(pass.row_bytes)
        .and_then(|offset| offset.checked_add(pass.output_offset))
        .and_then(|offset| offset.checked_add(column_bytes))
        .ok_or(PngRasterError::InvalidLayout)
}

fn scatter_pass(
    source: &ReconstructedPng,
    pass: &PngPass,
    channels: usize,
    pixels: &mut [u8],
) -> Result<(), PngRasterError> {
    let header = source.structure.header;
    let width = usize::try_from(pass.width).map_err(|_| PngRasterError::InvalidLayout)?;
    if width.checked_mul(channels) != Some(pass.row_bytes) {
        return Err(PngRasterError::InvalidLayout);
    }
    // Descriptors originate only from validated IHDR and the fixed, disjoint Adam7 plan.
    // This private function does not accept arbitrary externally supplied pass descriptors.
    for row in 0..pass.height {
        for column in 0..pass.width {
            let offset = sample_offset(pass, row, column, channels)?;
            let end = offset
                .checked_add(channels)
                .ok_or(PngRasterError::InvalidLayout)?;
            let sample = source
                .packed_sample_bytes
                .get(offset..end)
                .ok_or(PngRasterError::InvalidLayout)?;
            let x = column
                .checked_mul(pass.step_x)
                .and_then(|value| value.checked_add(pass.start_x))
                .ok_or(PngRasterError::InvalidLayout)?;
            let y = row
                .checked_mul(pass.step_y)
                .and_then(|value| value.checked_add(pass.start_y))
                .ok_or(PngRasterError::InvalidLayout)?;
            if x >= header.width || y >= header.height {
                return Err(PngRasterError::InvalidLayout);
            }
            let position =
                usize::try_from((u64::from(y) * u64::from(header.width) + u64::from(x)) * 4)
                    .map_err(|_| PngRasterError::InvalidLayout)?;
            let rgba = match sample {
                [gray] => [*gray, *gray, *gray, 255],
                [gray, alpha] => [*gray, *gray, *gray, *alpha],
                [red, green, blue] => [*red, *green, *blue, 255],
                [red, green, blue, alpha] => [*red, *green, *blue, *alpha],
                _ => return Err(PngRasterError::InvalidLayout),
            };
            pixels
                .get_mut(position..position + 4)
                .ok_or(PngRasterError::InvalidLayout)?
                .copy_from_slice(&rgba);
        }
    }
    Ok(())
}

fn unsupported_chunks(input: &[u8]) -> Result<(bool, bool), PngRasterError> {
    let mut offset = 8_usize;
    let mut transparency = false;
    let mut animation = false;
    loop {
        let framing = input
            .get(offset..offset + 8)
            .ok_or(PngRasterError::InvalidLayout)?;
        let length = u32::from_be_bytes(
            framing[..4]
                .try_into()
                .map_err(|_| PngRasterError::InvalidLayout)?,
        );
        let kind = &framing[4..];
        transparency |= kind == b"tRNS";
        animation |= matches!(kind, b"acTL" | b"fcTL" | b"fdAT");
        let next = usize::try_from(length)
            .ok()
            .and_then(|length| offset.checked_add(length))
            .and_then(|end| end.checked_add(12))
            .ok_or(PngRasterError::InvalidLayout)?;
        if next > input.len() {
            return Err(PngRasterError::InvalidLayout);
        }
        if kind == b"IEND" {
            return Ok((transparency, animation));
        }
        offset = next;
    }
}

pub(crate) fn attach_raster(
    document: &mut ArtifactIr,
    input: &[u8],
    reconstructed: &ReconstructedPng,
) -> Result<(), PngAdapterError> {
    let status = raster_status(input, reconstructed).map_err(PngAdapterError::InvalidRasterData)?;
    let mut evidence = document.evidence[0].clone();
    evidence.id = Identifier::from("evidence:png-raster");
    evidence.selector = Some(Selector::NativeId {
        native_id: "IDAT/encoded-rgba8-v1".to_owned(),
    });
    document.evidence.push(evidence);
    document
        .extensions
        .get_mut(PNG_EXTENSION_KEY)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            PngAdapterError::InvalidArtifactIr("missing PNG metadata object".to_owned())
        })?
        .insert("encodedRgba8Raster".to_owned(), raster_metadata(&status));
    attach_alpha_geometry(document, &status)?;
    Ok(())
}

fn attach_alpha_geometry(
    document: &mut ArtifactIr,
    status: &PngRasterStatus,
) -> Result<(), PngAdapterError> {
    let metadata = match status {
        PngRasterStatus::Available(raster) => {
            let observed =
                observe_alpha_geometry(raster).map_err(PngAdapterError::InvalidRasterData)?;
            let mut evidence = document.evidence[0].clone();
            evidence.id = Identifier::from("evidence:png-alpha");
            evidence.selector = Some(Selector::NativeId {
                native_id: "IDAT/encoded-rgba8-v1/alpha8".to_owned(),
            });
            document.evidence.push(evidence);
            document.nodes[0].geometry.ink_box =
                observed.visible_bounds.map(AlphaBounds::as_observed_rect);
            alpha_available_metadata(observed)
        }
        PngRasterStatus::Unavailable(reason) => json!({
            "version": "0.1.0",
            "status": "unavailable",
            "reason": reason.code()
        }),
    };
    document
        .extensions
        .get_mut(PNG_EXTENSION_KEY)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            PngAdapterError::InvalidArtifactIr("missing PNG metadata object".to_owned())
        })?
        .insert("alphaGeometry".to_owned(), metadata);
    Ok(())
}

fn alpha_available_metadata(observed: AlphaGeometry) -> Value {
    json!({
        "version": "0.1.0",
        "status": "available",
        "sourceAlphaEncoding": "unassociatedPngEncodedAlpha8",
        "visiblePredicate": "alphaGreaterThanZero",
        "opaquePredicate": "alphaEquals255",
        "coordinateSpaceId": "canvas",
        "unit": "devicePixel",
        "boundsFormat": "xywh-half-open",
        "evidenceId": "evidence:png-alpha",
        "visibleBounds": observed.visible_bounds.map(AlphaBounds::as_array),
        "opaqueBounds": observed.opaque_bounds.map(AlphaBounds::as_array),
        "transparentInsets": observed.transparent_insets.map(insets_metadata),
        "pixelCounts": {
            "total": observed.pixel_counts.total,
            "visible": observed.pixel_counts.visible,
            "opaque": observed.pixel_counts.opaque,
            "translucent": observed.pixel_counts.translucent,
            "transparent": observed.pixel_counts.transparent
        },
        "visibleEdgePixels": {
            "top": edge_metadata(observed.visible_edge_pixels.top),
            "right": edge_metadata(observed.visible_edge_pixels.right),
            "bottom": edge_metadata(observed.visible_edge_pixels.bottom),
            "left": edge_metadata(observed.visible_edge_pixels.left)
        },
        "entirelyTransparent": observed.entirely_transparent,
        "allPixelsVisible": observed.all_pixels_visible
    })
}

fn insets_metadata(insets: TransparentInsets) -> Value {
    json!({
        "top": insets.top,
        "right": insets.right,
        "bottom": insets.bottom,
        "left": insets.left
    })
}

fn edge_metadata(edge: AlphaEdgeCount) -> Value {
    json!({
        "count": edge.count,
        "denominator": edge.denominator
    })
}

fn raster_metadata(status: &PngRasterStatus) -> Value {
    let mut metadata = json!({
        "version": "0.1.0",
        "encoding": "pngEncodedRgba8",
        "colorManagementApplied": false,
        "evidenceId": "evidence:png-raster"
    });
    match status {
        PngRasterStatus::Available(raster) => {
            metadata["status"] = json!("available");
            metadata["width"] = json!(raster.width);
            metadata["height"] = json!(raster.height);
            metadata["byteCount"] = json!(raster.pixels.len());
            metadata["byteCrc32"] = json!(format!("{:08x}", crc32(&raster.pixels)));
        }
        PngRasterStatus::Unavailable(reason) => {
            metadata["status"] = json!("unavailable");
            metadata["reason"] = json!(reason.code());
            if let PngRasterUnavailable::BufferTooLarge { required, limit } = reason {
                metadata["requiredBytes"] = json!(required);
                metadata["limitBytes"] = json!(limit);
            }
        }
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::{MAX_RGBA8_BYTES, PngRasterUnavailable, classify_raster};
    use crate::PngHeader;

    #[test]
    fn allocation_boundary_is_classified_before_allocation() {
        let mut header = PngHeader {
            width: 8_192,
            height: 8_192,
            bit_depth: 8,
            color_type: 0,
            compression_method: 0,
            filter_method: 0,
            interlace_method: 0,
        };
        assert_eq!(classify_raster(header, false), Ok(None));
        header.width += 1;
        assert_eq!(
            classify_raster(header, false),
            Ok(Some(PngRasterUnavailable::BufferTooLarge {
                required: 8_193 * 8_192 * 4,
                limit: MAX_RGBA8_BYTES,
            }))
        );
    }
}
