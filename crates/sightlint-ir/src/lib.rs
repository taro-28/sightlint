//! Versioned, medium-neutral contracts for the `SightLint` Artifact IR.
//!
//! This crate contains data and validation only. Artifact acquisition, perception, geometry
//! queries, policy, and rule verdicts belong outside this boundary.

#![forbid(unsafe_code)]

mod interaction;
mod json;
mod model;
mod schema;
mod validation;
mod visual;

pub use interaction::{
    EffectLatency, EffectResolution, INTERACTION_EXTENSION_KEY, INTERACTION_EXTENSION_VERSION,
    InteractionAction, InteractionExtension, InteractionExtensionErrors, InteractionTrace,
    InteractionValidationCode, InteractionValidationIssue, RecoveryContract, RecoveryKind,
    TraceConsistency, TraceEvent, TraceEventDetail, TraceExecution, VisibleState,
    interaction_extension_schema_json,
};
pub use json::{LoadError, serialize_canonical};
pub use model::{
    ArtifactDescriptor, ArtifactIr, ArtifactKind, Axis, BoxKind, Canvas, CategoricalAlternative,
    Evidence, EvidenceClass, EvidenceSource, Geometry, HorizontalDirection, Identifier, Node,
    NodeKind, Observed, ObservedRect, Rect, Relation, Selector, Size, Uncertainty, Unit,
    VerticalDirection,
};
pub use schema::artifact_ir_schema_json;
pub use validation::{ValidationCode, ValidationErrors, ValidationIssue};
pub use visual::{
    AlignmentAnchor, ExtentDimension, Length, VISUAL_EXTENSION_KEY, VISUAL_EXTENSION_VERSION,
    VisualContract, VisualExtension, VisualExtensionErrors, VisualStyle, VisualValidationCode,
    VisualValidationIssue, visual_extension_schema_json,
};

/// Current serialized Artifact IR core schema version.
pub const SCHEMA_VERSION: &str = "0.1.0";
