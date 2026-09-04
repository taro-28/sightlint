//! Deterministic PNG metadata adapter for `SightLint`.
//!
//! This crate deliberately inspects only the PNG signature and `IHDR` chunk. It does not decode
//! pixel samples and therefore does not infer ink bounds, text, components, or semantic roles.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

use serde_json::json;
use sightlint_ir::{
    ArtifactDescriptor, ArtifactIr, ArtifactKind, Canvas, Evidence, EvidenceClass, EvidenceSource,
    Geometry, HorizontalDirection, Identifier, Node, NodeKind, Observed, ObservedRect, Rect,
    Selector, Size, Unit, VerticalDirection,
};

const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const IHDR_DATA_LENGTH: u32 = 13;
const MAX_DIMENSION: u32 = 100_000;
const MAX_PIXELS: u64 = 100_000_000;
const ADAPTER_NAME: &str = "sightlint-adapter-png";
const PNG_EXTENSION_KEY: &str = "org.sightlint.adapter.png";

/// Exact metadata available from a validated PNG `IHDR` chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngHeader {
    /// Image width in device pixels.
    pub width: u32,
    /// Image height in device pixels.
    pub height: u32,
    /// PNG sample bit depth.
    pub bit_depth: u8,
    /// PNG color type.
    pub color_type: u8,
    /// PNG compression method. The current PNG specification defines only zero.
    pub compression_method: u8,
    /// PNG filter method. The current PNG specification defines only zero.
    pub filter_method: u8,
    /// PNG interlace method: zero for none or one for Adam7.
    pub interlace_method: u8,
}

/// Failure while validating PNG header metadata or producing Artifact IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PngAdapterError {
    /// Input is too short to contain the required PNG header structure.
    Truncated,
    /// Input does not begin with the standard PNG signature.
    InvalidSignature,
    /// The first PNG chunk is not `IHDR`.
    MissingIhdr,
    /// The `IHDR` chunk does not contain exactly 13 data bytes.
    InvalidIhdrLength(u32),
    /// The stored `IHDR` CRC does not match the chunk type and data.
    InvalidIhdrCrc,
    /// Width or height is zero.
    ZeroDimension,
    /// A dimension exceeds the adapter safety cap.
    DimensionTooLarge { width: u32, height: u32 },
    /// Total pixel count exceeds the adapter safety cap.
    PixelCountTooLarge { width: u32, height: u32 },
    /// PNG color type is not one of the defined values.
    InvalidColorType(u8),
    /// Bit depth is not legal for the declared PNG color type.
    InvalidBitDepth { bit_depth: u8, color_type: u8 },
    /// PNG compression method is unsupported or invalid.
    InvalidCompressionMethod(u8),
    /// PNG filter method is unsupported or invalid.
    InvalidFilterMethod(u8),
    /// PNG interlace method is unsupported or invalid.
    InvalidInterlaceMethod(u8),
    /// The adapter produced IR that violates the current core contract.
    InvalidArtifactIr(String),
}

impl fmt::Display for PngAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("PNG input is truncated before a complete IHDR chunk"),
            Self::InvalidSignature => formatter.write_str("input does not have the PNG signature"),
            Self::MissingIhdr => formatter.write_str("PNG first chunk must be IHDR"),
            Self::InvalidIhdrLength(length) => {
                write!(formatter, "PNG IHDR length must be 13 bytes, got {length}")
            }
            Self::InvalidIhdrCrc => formatter.write_str("PNG IHDR CRC-32 does not match the header data"),
            Self::ZeroDimension => formatter.write_str("PNG width and height must both be non-zero"),
            Self::DimensionTooLarge { width, height } => write!(
                formatter,
                "PNG dimensions {width}x{height} exceed the {MAX_DIMENSION}-pixel per-axis safety limit"
            ),
            Self::PixelCountTooLarge { width, height } => write!(
                formatter,
                "PNG dimensions {width}x{height} exceed the {MAX_PIXELS}-pixel safety limit"
            ),
            Self::InvalidColorType(color_type) => {
                write!(formatter, "PNG color type {color_type} is invalid")
            }
            Self::InvalidBitDepth {
                bit_depth,
                color_type,
            } => write!(
                formatter,
                "PNG bit depth {bit_depth} is invalid for color type {color_type}"
            ),
            Self::InvalidCompressionMethod(method) => {
                write!(formatter, "PNG compression method {method} is invalid")
            }
            Self::InvalidFilterMethod(method) => {
                write!(formatter, "PNG filter method {method} is invalid")
            }
            Self::InvalidInterlaceMethod(method) => {
                write!(formatter, "PNG interlace method {method} is invalid")
            }
            Self::InvalidArtifactIr(message) => {
                write!(formatter, "PNG adapter produced invalid Artifact IR: {message}")
            }
        }
    }
}

impl Error for PngAdapterError {}

/// Parses and validates the deterministic metadata in a PNG signature and `IHDR` chunk.
///
/// # Errors
///
/// Returns a stable [`PngAdapterError`] when the signature, chunk framing, CRC, dimensions, or
/// PNG header fields are invalid or outside the adapter safety limits.
pub fn inspect_png_header(input: &[u8]) -> Result<PngHeader, PngAdapterError> {
    if input.len() < PNG_SIGNATURE.len() {
        return Err(PngAdapterError::Truncated);
    }
    if input[..PNG_SIGNATURE.len()] != PNG_SIGNATURE {
        return Err(PngAdapterError::InvalidSignature);
    }

    // Signature + length + type + 13 data bytes + CRC.
    const REQUIRED_HEADER_BYTES: usize = 8 + 4 + 4 + 13 + 4;
    if input.len() < REQUIRED_HEADER_BYTES {
        return Err(PngAdapterError::Truncated);
    }

    let length = u32::from_be_bytes(input[8..12].try_into().expect("four-byte slice"));
    if length != IHDR_DATA_LENGTH {
        return Err(PngAdapterError::InvalidIhdrLength(length));
    }
    if &input[12..16] != b"IHDR" {
        return Err(PngAdapterError::MissingIhdr);
    }

    let data = &input[16..29];
    let expected_crc = u32::from_be_bytes(input[29..33].try_into().expect("four-byte slice"));
    let actual_crc = crc32(&input[12..29]);
    if actual_crc != expected_crc {
        return Err(PngAdapterError::InvalidIhdrCrc);
    }

    let width = u32::from_be_bytes(data[0..4].try_into().expect("four-byte slice"));
    let height = u32::from_be_bytes(data[4..8].try_into().expect("four-byte slice"));
    let header = PngHeader {
        width,
        height,
        bit_depth: data[8],
        color_type: data[9],
        compression_method: data[10],
        filter_method: data[11],
        interlace_method: data[12],
    };
    validate_header(header)?;
    Ok(header)
}

/// Converts validated PNG header metadata into evidence-backed Artifact IR.
///
/// `source_name` should identify the local source path or display name when available. The
/// adapter never transmits the input externally.
///
/// # Errors
///
/// Returns [`PngAdapterError`] for invalid PNG metadata or if the constructed document fails the
/// current Artifact IR validation contract.
pub fn adapt_png(input: &[u8], source_name: Option<String>) -> Result<ArtifactIr, PngAdapterError> {
    let header = inspect_png_header(input)?;
    let evidence_id = Identifier::from("evidence:png-header");
    let canvas_id = Identifier::from("canvas");

    let evidence = Evidence {
        id: evidence_id.clone(),
        class: EvidenceClass::ExactSource,
        source: EvidenceSource {
            adapter: ADAPTER_NAME.to_owned(),
            adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
            model: None,
            input_digest: None,
            external_processing: false,
        },
        selector: Some(Selector::NativeId {
            native_id: "IHDR".to_owned(),
        }),
        confidence: None,
        uncertainty: None,
    };

    let full_rect = ObservedRect {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: f64::from(header.width),
            height: f64::from(header.height),
        },
        coordinate_space_id: canvas_id.clone(),
        evidence_id: evidence_id.clone(),
    };

    let mut document = ArtifactIr {
        schema_version: sightlint_ir::SCHEMA_VERSION.to_owned(),
        artifact: ArtifactDescriptor {
            id: Identifier::from("artifact"),
            kind: ArtifactKind::Image,
            title: None,
            source_name,
        },
        canvases: vec![Canvas {
            id: canvas_id.clone(),
            size: Size {
                width: f64::from(header.width),
                height: f64::from(header.height),
            },
            unit: Unit::DevicePixel,
            horizontal_direction: HorizontalDirection::Right,
            vertical_direction: VerticalDirection::Down,
            evidence_id: evidence_id.clone(),
        }],
        nodes: vec![Node {
            id: Identifier::from("image"),
            kind: Observed {
                value: NodeKind::Image,
                evidence_id: evidence_id.clone(),
            },
            coordinate_space_id: canvas_id,
            parent_id: None,
            role: None,
            name: None,
            geometry: Geometry {
                layout_box: None,
                render_box: Some(full_rect),
                ink_box: None,
                hit_box: None,
            },
            extensions: Default::default(),
        }],
        relations: Vec::new(),
        evidence: vec![evidence],
        extensions: Default::default(),
    };

    document.extensions.insert(
        PNG_EXTENSION_KEY.to_owned(),
        json!({
            "version": "0.1.0",
            "bitDepth": header.bit_depth,
            "colorType": header.color_type,
            "compressionMethod": header.compression_method,
            "filterMethod": header.filter_method,
            "interlaceMethod": header.interlace_method
        }),
    );

    document
        .validate()
        .map_err(|error| PngAdapterError::InvalidArtifactIr(error.to_string()))?;
    Ok(document)
}

fn validate_header(header: PngHeader) -> Result<(), PngAdapterError> {
    if header.width == 0 || header.height == 0 {
        return Err(PngAdapterError::ZeroDimension);
    }
    if header.width > MAX_DIMENSION || header.height > MAX_DIMENSION {
        return Err(PngAdapterError::DimensionTooLarge {
            width: header.width,
            height: header.height,
        });
    }
    if u64::from(header.width) * u64::from(header.height) > MAX_PIXELS {
        return Err(PngAdapterError::PixelCountTooLarge {
            width: header.width,
            height: header.height,
        });
    }

    let valid_depth = match header.color_type {
        0 => matches!(header.bit_depth, 1 | 2 | 4 | 8 | 16),
        2 => matches!(header.bit_depth, 8 | 16),
        3 => matches!(header.bit_depth, 1 | 2 | 4 | 8),
        4 => matches!(header.bit_depth, 8 | 16),
        6 => matches!(header.bit_depth, 8 | 16),
        other => return Err(PngAdapterError::InvalidColorType(other)),
    };
    if !valid_depth {
        return Err(PngAdapterError::InvalidBitDepth {
            bit_depth: header.bit_depth,
            color_type: header.color_type,
        });
    }
    if header.compression_method != 0 {
        return Err(PngAdapterError::InvalidCompressionMethod(
            header.compression_method,
        ));
    }
    if header.filter_method != 0 {
        return Err(PngAdapterError::InvalidFilterMethod(header.filter_method));
    }
    if !matches!(header.interlace_method, 0 | 1) {
        return Err(PngAdapterError::InvalidInterlaceMethod(
            header.interlace_method,
        ));
    }
    Ok(())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::{PngAdapterError, adapt_png, crc32, inspect_png_header};
    use sightlint_ir::{ArtifactKind, EvidenceClass, NodeKind, Unit};

    fn png_header(
        width: u32,
        height: u32,
        bit_depth: u8,
        color_type: u8,
        compression: u8,
        filter: u8,
        interlace: u8,
    ) -> Vec<u8> {
        let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[bit_depth, color_type, compression, filter, interlace]);
        let crc = crc32(&bytes[12..29]);
        bytes.extend_from_slice(&crc.to_be_bytes());
        bytes
    }

    #[test]
    fn accepts_valid_header_matrix_representatives() {
        for (depth, color_type) in [(1, 0), (8, 2), (4, 3), (16, 4), (8, 6)] {
            let bytes = png_header(64, 32, depth, color_type, 0, 0, 0);
            let header = inspect_png_header(&bytes).expect("valid PNG header");
            assert_eq!(header.width, 64);
            assert_eq!(header.height, 32);
            assert_eq!(header.bit_depth, depth);
            assert_eq!(header.color_type, color_type);
        }
    }

    #[test]
    fn accepts_adam7_interlace() {
        let bytes = png_header(1, 1, 8, 6, 0, 0, 1);
        assert_eq!(
            inspect_png_header(&bytes)
                .expect("Adam7 is valid")
                .interlace_method,
            1
        );
    }

    #[test]
    fn rejects_corrupt_crc() {
        let mut bytes = png_header(1, 1, 8, 6, 0, 0, 0);
        bytes[20] ^= 1;
        assert_eq!(
            inspect_png_header(&bytes),
            Err(PngAdapterError::InvalidIhdrCrc)
        );
    }

    #[test]
    fn rejects_invalid_bit_depth_for_color_type() {
        let bytes = png_header(1, 1, 4, 6, 0, 0, 0);
        assert_eq!(
            inspect_png_header(&bytes),
            Err(PngAdapterError::InvalidBitDepth {
                bit_depth: 4,
                color_type: 6,
            })
        );
    }

    #[test]
    fn rejects_zero_and_oversized_dimensions() {
        let zero = png_header(0, 1, 8, 6, 0, 0, 0);
        assert_eq!(inspect_png_header(&zero), Err(PngAdapterError::ZeroDimension));

        let huge = png_header(100_001, 1, 8, 6, 0, 0, 0);
        assert!(matches!(
            inspect_png_header(&huge),
            Err(PngAdapterError::DimensionTooLarge { .. })
        ));

        let too_many_pixels = png_header(20_000, 20_000, 8, 6, 0, 0, 0);
        assert!(matches!(
            inspect_png_header(&too_many_pixels),
            Err(PngAdapterError::PixelCountTooLarge { .. })
        ));
    }

    #[test]
    fn emits_exact_source_ir_without_invented_ink_or_semantics() {
        let bytes = png_header(320, 200, 8, 6, 0, 0, 0);
        let document = adapt_png(&bytes, Some("sample.png".to_owned())).expect("valid IR");
        assert_eq!(document.artifact.kind, ArtifactKind::Image);
        assert_eq!(document.artifact.source_name.as_deref(), Some("sample.png"));
        assert_eq!(document.canvases[0].unit, Unit::DevicePixel);
        assert_eq!(document.nodes[0].kind.value, NodeKind::Image);
        assert!(document.nodes[0].geometry.render_box.is_some());
        assert!(document.nodes[0].geometry.ink_box.is_none());
        assert!(document.nodes[0].role.is_none());
        assert!(document.nodes[0].name.is_none());
        assert_eq!(document.evidence[0].class, EvidenceClass::ExactSource);
        assert!(!document.evidence[0].source.external_processing);
    }
}
