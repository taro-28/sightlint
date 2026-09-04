//! Deterministic JSON loading and serialization helpers.

use std::error::Error;
use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::{ArtifactIr, Relation, ValidationErrors};

/// Failure while decoding or validating an Artifact IR JSON document.
#[derive(Debug)]
pub enum LoadError {
    /// Input is not valid JSON for the current schema.
    Json(serde_json::Error),
    /// Input decoded successfully but violates Artifact IR invariants.
    Validation(ValidationErrors),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "failed to decode Artifact IR JSON: {error}"),
            Self::Validation(errors) => errors.fmt(formatter),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Validation(errors) => Some(errors),
        }
    }
}

impl ArtifactIr {
    /// Parses and validates a JSON Artifact IR document.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::Json`] for decoding failures and
    /// [`LoadError::Validation`] for structural or provenance failures.
    pub fn from_json_str(input: &str) -> Result<Self, LoadError> {
        let document: Self = serde_json::from_str(input).map_err(LoadError::Json)?;
        document.validate().map_err(LoadError::Validation)?;
        Ok(document)
    }

    /// Returns a clone with all unordered top-level collections in canonical order.
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical
            .canvases
            .sort_by(|left, right| left.id.cmp(&right.id));
        canonical
            .nodes
            .sort_by(|left, right| left.id.cmp(&right.id));
        canonical
            .relations
            .sort_by(|left, right| left.id().cmp(right.id()));
        canonical
            .evidence
            .sort_by(|left, right| left.id.cmp(&right.id));

        for relation in &mut canonical.relations {
            if let Relation::NonOverlapping { node_ids, .. } = relation {
                node_ids.sort();
            }
        }

        canonical
    }

    /// Serializes the document as canonical, pretty-printed JSON with a final newline.
    ///
    /// # Errors
    ///
    /// Returns an error when a value cannot be represented as JSON.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serialize_canonical(&self.canonicalized())
    }
}

/// Serializes any value as recursively key-sorted, pretty-printed JSON with a final newline.
///
/// Collection ordering inside the value remains semantically significant. Callers must sort
/// unordered arrays before invoking this function.
///
/// # Errors
///
/// Returns an error when the value cannot be represented as JSON.
pub fn serialize_canonical<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(value)?;
    sort_json_keys(&mut value);
    let mut output = serde_json::to_string_pretty(&value)?;
    output.push('\n');
    Ok(output)
}

fn sort_json_keys(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let previous = std::mem::take(object);
            let mut entries: Vec<_> = previous.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            for (key, mut child) in entries {
                sort_json_keys(&mut child);
                object.insert(key, child);
            }
        }
        Value::Array(array) => {
            for child in array {
                sort_json_keys(child);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}
