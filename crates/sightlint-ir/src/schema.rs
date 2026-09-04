//! JSON Schema generation for Artifact IR documents.

use schemars::schema_for;
use serde_json::Value;

use crate::{ArtifactIr, SCHEMA_VERSION, serialize_canonical};

/// Generates the canonical JSON Schema for the current Artifact IR version.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be represented as JSON.
pub fn artifact_ir_schema_json() -> Result<String, serde_json::Error> {
    let mut schema = serde_json::to_value(schema_for!(ArtifactIr))?;
    if let Value::Object(object) = &mut schema {
        object.insert(
            "$id".to_owned(),
            Value::String(format!("urn:sightlint:schema:artifact-ir:{SCHEMA_VERSION}")),
        );
        object.insert(
            "title".to_owned(),
            Value::String(format!("SightLint Artifact IR {SCHEMA_VERSION}")),
        );
    }
    serialize_canonical(&schema)
}
