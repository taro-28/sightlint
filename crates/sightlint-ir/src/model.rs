//! Serializable data model for the `SightLint` Artifact IR.

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A stable identifier within one artifact document.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Identifier(String);

impl Identifier {
    /// Creates an identifier without performing validation.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether the identifier is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Identifier {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One versioned, normalized artifact document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactIr {
    /// Version of the serialized Artifact IR schema.
    pub schema_version: String,
    /// Artifact-level identity and medium metadata.
    pub artifact: ArtifactDescriptor,
    /// Coordinate spaces such as pages, slides, screens, or viewports.
    pub canvases: Vec<Canvas>,
    /// Visual or semantic nodes observed in the artifact.
    pub nodes: Vec<Node>,
    /// Source-declared or inferred relations between nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    /// Provenance records referenced by observations and relations.
    pub evidence: Vec<Evidence>,
    /// Namespaced, medium-specific data not understood by the core schema.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// Identity and medium metadata for an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDescriptor {
    /// Artifact identifier, unique within the document.
    pub id: Identifier,
    /// Broad medium represented by the artifact.
    pub kind: ArtifactKind,
    /// Optional human-readable artifact title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional source filename, route, or display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}

/// Broad medium represented by an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    /// A web interface or viewport.
    Web,
    /// A mobile application interface.
    Mobile,
    /// A presentation or slide deck.
    Slide,
    /// A paginated or flowing document.
    Document,
    /// A Portable Document Format artifact.
    Pdf,
    /// A raster or vector image.
    Image,
    /// A medium not represented by another variant.
    Other,
}

/// A root coordinate space such as a page, screen, slide, or viewport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Canvas {
    /// Coordinate-space identifier.
    pub id: Identifier,
    /// Width and height in the declared unit.
    pub size: Size,
    /// Unit used by geometry in this coordinate space.
    pub unit: Unit,
    /// Direction in which horizontal coordinates increase.
    pub horizontal_direction: HorizontalDirection,
    /// Direction in which vertical coordinates increase.
    pub vertical_direction: VerticalDirection,
    /// Evidence supporting the canvas dimensions and coordinate system.
    pub evidence_id: Identifier,
}

/// A two-dimensional size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Size {
    /// Width in the enclosing coordinate space's unit.
    pub width: f64,
    /// Height in the enclosing coordinate space's unit.
    pub height: f64,
}

/// Unit used by one coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Unit {
    /// Cascading Style Sheets logical pixel.
    CssPixel,
    /// Device or bitmap pixel.
    DevicePixel,
    /// Android density-independent pixel.
    Dp,
    /// Typographic or platform point.
    Point,
    /// English Metric Unit used by Office formats.
    Emu,
    /// PDF point.
    PdfPoint,
    /// Coordinate normalized to the closed interval from zero to one.
    Normalized,
}

/// Direction in which horizontal coordinates increase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HorizontalDirection {
    /// Coordinates increase from left to right.
    Right,
    /// Coordinates increase from right to left.
    Left,
}

/// Direction in which vertical coordinates increase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VerticalDirection {
    /// Coordinates increase from top to bottom.
    Down,
    /// Coordinates increase from bottom to top.
    Up,
}

/// A visual or semantic node in an artifact hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    /// Node identifier, unique within the artifact document.
    pub id: Identifier,
    /// Broad primitive represented by the node.
    pub kind: Observed<NodeKind>,
    /// Coordinate space that contains this node.
    pub coordinate_space_id: Identifier,
    /// Optional parent node identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Identifier>,
    /// Optional semantic role, such as `heading`, `button`, or `caption`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Observed<String>>,
    /// Optional accessible or visible name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Observed<String>>,
    /// Distinct source, rendered, ink, and interaction geometry.
    #[serde(default)]
    pub geometry: Geometry,
    /// Namespaced, node-specific extension data.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// Broad primitive represented by a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    /// Text content.
    Text,
    /// Raster or vector image content.
    Image,
    /// Filled or stroked shape.
    Shape,
    /// Line or connector.
    Line,
    /// Interactive control.
    Control,
    /// Table or grid.
    Table,
    /// Chart or data visualization.
    Chart,
    /// Container or grouping node.
    Container,
    /// Primitive not represented by another variant.
    Other,
}

/// A value linked to the evidence that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observed<T> {
    /// Observed value.
    pub value: T,
    /// Evidence record supporting the value.
    pub evidence_id: Identifier,
}

/// Distinct forms of geometry associated with a node.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Geometry {
    /// Space allocated by a source layout system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_box: Option<ObservedRect>,
    /// Render bounds under the adapter's documented effects policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_box: Option<ObservedRect>,
    /// Visible non-transparent ink bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ink_box: Option<ObservedRect>,
    /// Region that can receive pointer or touch interaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_box: Option<ObservedRect>,
}

/// A rectangle linked to a coordinate space and evidence record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedRect {
    /// Rectangle coordinates in the referenced coordinate space.
    pub rect: Rect,
    /// Coordinate space in which the rectangle is expressed.
    pub coordinate_space_id: Identifier,
    /// Evidence record supporting the rectangle.
    pub evidence_id: Identifier,
}

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rect {
    /// Horizontal coordinate in the referenced coordinate space.
    pub x: f64,
    /// Vertical coordinate in the referenced coordinate space.
    pub y: f64,
    /// Non-negative rectangle width.
    pub width: f64,
    /// Non-negative rectangle height.
    pub height: f64,
}

/// Provenance for one or more observations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    /// Evidence identifier, unique within the artifact document.
    pub id: Identifier,
    /// Authority and acquisition class of the evidence.
    pub class: EvidenceClass,
    /// Adapter, model, digest, and transmission metadata.
    pub source: EvidenceSource,
    /// Optional selector locating the observation in its source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<Selector>,
    /// Optional calibrated confidence for probabilistic evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Optional bounded or categorical uncertainty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Uncertainty>,
}

/// Authority and acquisition class of an evidence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceClass {
    /// Exact declaration from a source format or user contract.
    ExactSource,
    /// Exact measurement from deterministic rendered output.
    ExactRender,
    /// Semantics supplied by an accessibility or platform API.
    PlatformSemantics,
    /// Deterministic or bounded measurement from pixels.
    VisionMeasured,
    /// Probabilistic inference from OCR, detection, or a model.
    VisionInferred,
    /// Reproducible event, focus, state, or network trace.
    InteractionTrace,
    /// Explicit project, design-system, API, or effect contract.
    DeclaredContract,
    /// Evidence whose authority is not known.
    Unknown,
}

/// Origin of an evidence record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSource {
    /// Adapter or sensor name.
    pub adapter: String,
    /// Adapter or sensor version.
    pub adapter_version: String,
    /// Optional model identifier for probabilistic perception.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional digest of the exact input used to create the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,
    /// Whether artifact content left the local execution boundary.
    #[serde(default)]
    pub external_processing: bool,
}

/// Source selector identifying where evidence originated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Selector {
    /// JSON Pointer into a structured source document.
    JsonPointer {
        /// RFC 6901 pointer, including the empty root pointer.
        pointer: String,
    },
    /// Native source identifier such as a DOM, slide, or platform node ID.
    NativeId {
        /// Adapter-specific native identifier.
        native_id: String,
    },
    /// Rectangular source region.
    Region {
        /// Coordinate space containing the region.
        coordinate_space_id: Identifier,
        /// Region bounds.
        rect: Rect,
    },
    /// Half-open range within a text source.
    TextRange {
        /// Inclusive UTF-8 byte offset.
        start: usize,
        /// Exclusive UTF-8 byte offset.
        end: usize,
    },
}

/// Explicit representation of uncertainty in an evidence record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Uncertainty {
    /// Closed numeric interval.
    ScalarRange {
        /// Lower interval bound.
        lower: f64,
        /// Upper interval bound.
        upper: f64,
    },
    /// Per-component tolerance around a rectangle measurement.
    RectTolerance {
        /// Horizontal-coordinate tolerance.
        x: f64,
        /// Vertical-coordinate tolerance.
        y: f64,
        /// Width tolerance.
        width: f64,
        /// Height tolerance.
        height: f64,
    },
    /// Alternative semantic values with calibrated confidence.
    CategoricalAlternatives {
        /// Alternative candidates.
        alternatives: Vec<CategoricalAlternative>,
    },
}

/// One categorical alternative for an inferred observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CategoricalAlternative {
    /// Alternative value represented as a stable string.
    pub value: String,
    /// Calibrated confidence in the closed interval from zero to one.
    pub confidence: f64,
}

/// Source-declared or inferred relationship between nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Relation {
    /// Nodes expected not to overlap under the selected geometry kind.
    NonOverlapping {
        /// Relation identifier.
        id: Identifier,
        /// Member node identifiers.
        node_ids: Vec<Identifier>,
        /// Geometry kind used by the constraint.
        box_kind: BoxKind,
        /// Maximum tolerated overlap on either axis.
        tolerance: f64,
        /// Evidence supporting the relation.
        evidence_id: Identifier,
    },
    /// Ordered peers whose adjacent gaps should be consistent.
    PeerSequence {
        /// Relation identifier.
        id: Identifier,
        /// Ordered member node identifiers.
        node_ids: Vec<Identifier>,
        /// Sequence axis.
        axis: Axis,
        /// Geometry kind used to measure gaps.
        box_kind: BoxKind,
        /// Optional declared expected gap.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_gap: Option<f64>,
        /// Maximum permitted absolute gap difference.
        tolerance: f64,
        /// Evidence supporting the sequence and any declared expectation.
        evidence_id: Identifier,
    },
}

impl Relation {
    /// Returns the relation identifier.
    pub fn id(&self) -> &Identifier {
        match self {
            Self::NonOverlapping { id, .. } | Self::PeerSequence { id, .. } => id,
        }
    }

    /// Returns the evidence supporting the relation.
    pub fn evidence_id(&self) -> &Identifier {
        match self {
            Self::NonOverlapping { evidence_id, .. } | Self::PeerSequence { evidence_id, .. } => {
                evidence_id
            }
        }
    }

    /// Returns relation member identifiers.
    pub fn node_ids(&self) -> &[Identifier] {
        match self {
            Self::NonOverlapping { node_ids, .. } | Self::PeerSequence { node_ids, .. } => node_ids,
        }
    }
}

/// Geometry kind selected by a rule or relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BoxKind {
    /// Source layout bounds.
    Layout,
    /// Render bounds under an effects policy.
    Render,
    /// Visible ink bounds.
    Ink,
    /// Interactive hit bounds.
    Hit,
}

impl BoxKind {
    /// Returns the stable serialized aspect name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Layout => "layoutBox",
            Self::Render => "renderBox",
            Self::Ink => "inkBox",
            Self::Hit => "hitBox",
        }
    }
}

/// Axis along which an ordered relation is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Axis {
    /// Horizontal axis.
    Horizontal,
    /// Vertical axis.
    Vertical,
}
