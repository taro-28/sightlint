//! Versioned, medium-neutral contracts for the `SightLint` Artifact IR.
//!
//! This crate contains data and validation only. Artifact acquisition, perception, geometry
//! queries, policy, and rule verdicts belong outside this boundary.

#![forbid(unsafe_code)]

mod json;
mod model;
mod schema;
mod validation;

pub use json::{serialize_canonical, LoadError};
pub use model::{
    ArtifactDescriptor, ArtifactIr, ArtifactKind, Axis, BoxKind, Canvas,
    CategoricalAlternative, Evidence, EvidenceClass, EvidenceSource, Geometry,
    HorizontalDirection, Identifier, Node, NodeKind, Observed, ObservedRect, Rect, Relation,
    Selector, Size, Uncertainty, Unit, VerticalDirection,
};
pub use schema::artifact_ir_schema_json;
pub use validation::{ValidationCode, ValidationErrors, ValidationIssue};

/// Current serialized Artifact IR schema version.
pub const SCHEMA_VERSION: &str = "0.1.0";
