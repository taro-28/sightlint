//! Semantic validation for Artifact IR documents.

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::error::Error;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactIr, Evidence, EvidenceClass, Identifier, ObservedRect, Rect, Relation, SCHEMA_VERSION,
    Selector, Uncertainty,
};

/// Stable category for one Artifact IR validation problem.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ValidationCode {
    /// The document uses an unsupported schema version.
    UnsupportedSchemaVersion,
    /// An identifier is empty.
    EmptyIdentifier,
    /// An identifier is reused within the same artifact document.
    DuplicateIdentifier,
    /// A referenced canvas, node, relation, or evidence record does not exist.
    InvalidReference,
    /// A numeric value is `NaN` or infinite.
    NonFiniteNumber,
    /// A dimension or tolerance is negative.
    NegativeDimension,
    /// A canvas has zero width or height.
    EmptyCanvas,
    /// Parent relationships contain a cycle.
    HierarchyCycle,
    /// A calibrated confidence is outside the closed interval from zero to one.
    InvalidConfidence,
    /// Probabilistic evidence omits calibrated confidence.
    MissingConfidence,
    /// An uncertainty declaration is malformed.
    InvalidUncertainty,
    /// A source selector is malformed.
    InvalidSelector,
    /// A relation has too few or duplicate members.
    InvalidRelation,
    /// Required adapter provenance is missing.
    InvalidEvidenceSource,
}

/// One deterministic semantic validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationIssue {
    /// Stable machine-readable issue category.
    pub code: ValidationCode,
    /// JSON Pointer-like path to the offending value.
    pub path: String,
    /// Human-readable explanation without unstable runtime data.
    pub message: String,
}

/// Ordered collection of semantic validation issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    issues: Vec<ValidationIssue>,
}

impl ValidationErrors {
    /// Creates an ordered validation error collection.
    pub fn new(mut issues: Vec<ValidationIssue>) -> Self {
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
        issues.dedup();
        Self { issues }
    }

    /// Returns the ordered issues.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Returns the number of issues.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Returns whether no issues are present.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "Artifact IR validation failed with {} issue(s):",
            self.issues.len()
        )?;
        for issue in &self.issues {
            writeln!(
                formatter,
                "- {:?} at {}: {}",
                issue.code, issue.path, issue.message
            )?;
        }
        Ok(())
    }
}

impl Error for ValidationErrors {}

impl ArtifactIr {
    /// Validates all core Artifact IR invariants.
    ///
    /// # Errors
    ///
    /// Returns every deterministically observable semantic issue in stable order.
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut validator = Validator::default();

        if self.schema_version != SCHEMA_VERSION {
            validator.issue(
                ValidationCode::UnsupportedSchemaVersion,
                "/schemaVersion",
                format!(
                    "expected schema version {SCHEMA_VERSION}, found {}",
                    self.schema_version
                ),
            );
        }

        validator.register_id(&self.artifact.id, "/artifact/id");

        for (index, canvas) in self.canvases.iter().enumerate() {
            let base = format!("/canvases/{index}");
            validator.register_id(&canvas.id, &format!("{base}/id"));
            validate_positive_finite(
                canvas.size.width,
                &format!("{base}/size/width"),
                &mut validator,
            );
            validate_positive_finite(
                canvas.size.height,
                &format!("{base}/size/height"),
                &mut validator,
            );
        }

        for (index, node) in self.nodes.iter().enumerate() {
            validator.register_id(&node.id, &format!("/nodes/{index}/id"));
        }

        for (index, relation) in self.relations.iter().enumerate() {
            validator.register_id(relation.id(), &format!("/relations/{index}/id"));
        }

        for (index, evidence) in self.evidence.iter().enumerate() {
            validator.register_id(&evidence.id, &format!("/evidence/{index}/id"));
        }

        let canvas_ids = self
            .canvases
            .iter()
            .map(|canvas| canvas.id.clone())
            .collect::<BTreeSet<_>>();
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<BTreeSet<_>>();
        let evidence_ids = self
            .evidence
            .iter()
            .map(|evidence| evidence.id.clone())
            .collect::<BTreeSet<_>>();

        for (index, canvas) in self.canvases.iter().enumerate() {
            validator.require_reference(
                &canvas.evidence_id,
                &evidence_ids,
                &format!("/canvases/{index}/evidenceId"),
                "evidence",
            );
        }

        for (index, node) in self.nodes.iter().enumerate() {
            let base = format!("/nodes/{index}");
            validator.require_reference(
                &node.coordinate_space_id,
                &canvas_ids,
                &format!("{base}/coordinateSpaceId"),
                "canvas",
            );
            if let Some(parent_id) = &node.parent_id {
                validator.require_reference(
                    parent_id,
                    &node_ids,
                    &format!("{base}/parentId"),
                    "node",
                );
                if parent_id == &node.id {
                    validator.issue(
                        ValidationCode::HierarchyCycle,
                        format!("{base}/parentId"),
                        "a node cannot be its own parent",
                    );
                }
            }

            validator.require_reference(
                &node.kind.evidence_id,
                &evidence_ids,
                &format!("{base}/kind/evidenceId"),
                "evidence",
            );
            if let Some(role) = &node.role {
                validator.require_reference(
                    &role.evidence_id,
                    &evidence_ids,
                    &format!("{base}/role/evidenceId"),
                    "evidence",
                );
            }
            if let Some(name) = &node.name {
                validator.require_reference(
                    &name.evidence_id,
                    &evidence_ids,
                    &format!("{base}/name/evidenceId"),
                    "evidence",
                );
            }

            validate_observed_rect(
                node.geometry.layout_box.as_ref(),
                &format!("{base}/geometry/layoutBox"),
                &canvas_ids,
                &evidence_ids,
                &mut validator,
            );
            validate_observed_rect(
                node.geometry.render_box.as_ref(),
                &format!("{base}/geometry/renderBox"),
                &canvas_ids,
                &evidence_ids,
                &mut validator,
            );
            validate_observed_rect(
                node.geometry.ink_box.as_ref(),
                &format!("{base}/geometry/inkBox"),
                &canvas_ids,
                &evidence_ids,
                &mut validator,
            );
            validate_observed_rect(
                node.geometry.hit_box.as_ref(),
                &format!("{base}/geometry/hitBox"),
                &canvas_ids,
                &evidence_ids,
                &mut validator,
            );
        }

        validate_hierarchy(self, &node_ids, &mut validator);

        for (index, relation) in self.relations.iter().enumerate() {
            let base = format!("/relations/{index}");
            validator.require_reference(
                relation.evidence_id(),
                &evidence_ids,
                &format!("{base}/evidenceId"),
                "evidence",
            );
            validate_relation(relation, &base, &node_ids, &mut validator);
        }

        for (index, evidence) in self.evidence.iter().enumerate() {
            validate_evidence(
                evidence,
                &format!("/evidence/{index}"),
                &canvas_ids,
                &mut validator,
            );
        }

        validator.finish()
    }
}

#[derive(Default)]
struct Validator {
    issues: Vec<ValidationIssue>,
    identifiers: BTreeMap<Identifier, String>,
}

impl Validator {
    fn issue(&mut self, code: ValidationCode, path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }

    fn register_id(&mut self, id: &Identifier, path: &str) {
        if id.is_empty() {
            self.issue(
                ValidationCode::EmptyIdentifier,
                path,
                "identifiers must not be empty",
            );
            return;
        }

        match self.identifiers.entry(id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(path.to_owned());
            }
            Entry::Occupied(entry) => {
                let first_path = entry.get().clone();
                self.issue(
                    ValidationCode::DuplicateIdentifier,
                    path,
                    format!("identifier {id} was already declared at {first_path}"),
                );
            }
        }
    }

    fn require_reference(
        &mut self,
        id: &Identifier,
        known: &BTreeSet<Identifier>,
        path: &str,
        target: &str,
    ) {
        if !known.contains(id) {
            self.issue(
                ValidationCode::InvalidReference,
                path,
                format!("referenced {target} {id} does not exist"),
            );
        }
    }

    fn finish(self) -> Result<(), ValidationErrors> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(self.issues))
        }
    }
}

fn validate_positive_finite(value: f64, path: &str, validator: &mut Validator) {
    if !value.is_finite() {
        validator.issue(
            ValidationCode::NonFiniteNumber,
            path,
            "the value must be finite",
        );
    } else if value < 0.0 {
        validator.issue(
            ValidationCode::NegativeDimension,
            path,
            "the value must not be negative",
        );
    } else if value == 0.0 {
        validator.issue(
            ValidationCode::EmptyCanvas,
            path,
            "canvas dimensions must be greater than zero",
        );
    }
}

fn validate_non_negative_finite(value: f64, path: &str, validator: &mut Validator) {
    if !value.is_finite() {
        validator.issue(
            ValidationCode::NonFiniteNumber,
            path,
            "the value must be finite",
        );
    } else if value < 0.0 {
        validator.issue(
            ValidationCode::NegativeDimension,
            path,
            "the value must not be negative",
        );
    }
}

fn validate_rect(rect: Rect, path: &str, validator: &mut Validator) {
    for (name, value) in [("x", rect.x), ("y", rect.y)] {
        if !value.is_finite() {
            validator.issue(
                ValidationCode::NonFiniteNumber,
                format!("{path}/{name}"),
                "the coordinate must be finite",
            );
        }
    }
    validate_non_negative_finite(rect.width, &format!("{path}/width"), validator);
    validate_non_negative_finite(rect.height, &format!("{path}/height"), validator);
}

fn validate_observed_rect(
    observed: Option<&ObservedRect>,
    path: &str,
    canvas_ids: &BTreeSet<Identifier>,
    evidence_ids: &BTreeSet<Identifier>,
    validator: &mut Validator,
) {
    let Some(observed) = observed else {
        return;
    };

    validator.require_reference(
        &observed.coordinate_space_id,
        canvas_ids,
        &format!("{path}/coordinateSpaceId"),
        "canvas",
    );
    validator.require_reference(
        &observed.evidence_id,
        evidence_ids,
        &format!("{path}/evidenceId"),
        "evidence",
    );
    validate_rect(observed.rect, &format!("{path}/rect"), validator);
}

fn validate_hierarchy(
    document: &ArtifactIr,
    node_ids: &BTreeSet<Identifier>,
    validator: &mut Validator,
) {
    let parents = document
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.parent_id.clone()))
        .collect::<BTreeMap<_, _>>();

    for (index, node) in document.nodes.iter().enumerate() {
        if !node_ids.contains(&node.id) {
            continue;
        }

        let mut visited = BTreeSet::new();
        let mut cursor = Some(node.id.clone());
        while let Some(current) = cursor {
            if !visited.insert(current.clone()) {
                validator.issue(
                    ValidationCode::HierarchyCycle,
                    format!("/nodes/{index}/parentId"),
                    format!("parent hierarchy for node {} contains a cycle", node.id),
                );
                break;
            }
            cursor = parents.get(&current).cloned().flatten();
        }
    }
}

fn validate_relation(
    relation: &Relation,
    path: &str,
    node_ids: &BTreeSet<Identifier>,
    validator: &mut Validator,
) {
    let members = relation.node_ids();
    if members.len() < 2 {
        validator.issue(
            ValidationCode::InvalidRelation,
            format!("{path}/nodeIds"),
            "a relation must contain at least two nodes",
        );
    }

    let mut unique = BTreeSet::new();
    for (index, node_id) in members.iter().enumerate() {
        validator.require_reference(
            node_id,
            node_ids,
            &format!("{path}/nodeIds/{index}"),
            "node",
        );
        if !unique.insert(node_id) {
            validator.issue(
                ValidationCode::InvalidRelation,
                format!("{path}/nodeIds/{index}"),
                format!("node {node_id} appears more than once in the relation"),
            );
        }
    }

    match relation {
        Relation::NonOverlapping { tolerance, .. } => {
            validate_non_negative_finite(*tolerance, &format!("{path}/tolerance"), validator);
        }
        Relation::PeerSequence {
            expected_gap,
            tolerance,
            ..
        } => {
            validate_non_negative_finite(*tolerance, &format!("{path}/tolerance"), validator);
            if let Some(expected_gap) = expected_gap {
                validate_non_negative_finite(
                    *expected_gap,
                    &format!("{path}/expectedGap"),
                    validator,
                );
            }
        }
    }
}

fn validate_evidence(
    evidence: &Evidence,
    path: &str,
    canvas_ids: &BTreeSet<Identifier>,
    validator: &mut Validator,
) {
    if evidence.source.adapter.trim().is_empty()
        || evidence.source.adapter_version.trim().is_empty()
    {
        validator.issue(
            ValidationCode::InvalidEvidenceSource,
            format!("{path}/source"),
            "adapter and adapterVersion must not be empty",
        );
    }

    if matches!(evidence.class, EvidenceClass::VisionInferred) && evidence.confidence.is_none() {
        validator.issue(
            ValidationCode::MissingConfidence,
            format!("{path}/confidence"),
            "vision-inferred evidence requires calibrated confidence",
        );
    }

    if let Some(confidence) = evidence.confidence {
        validate_confidence(confidence, &format!("{path}/confidence"), validator);
    }

    if let Some(selector) = &evidence.selector {
        match selector {
            Selector::JsonPointer { pointer } => {
                if !pointer.is_empty() && !pointer.starts_with('/') {
                    validator.issue(
                        ValidationCode::InvalidSelector,
                        format!("{path}/selector/pointer"),
                        "a non-empty JSON Pointer must start with '/'",
                    );
                }
            }
            Selector::NativeId { native_id } => {
                if native_id.trim().is_empty() {
                    validator.issue(
                        ValidationCode::InvalidSelector,
                        format!("{path}/selector/nativeId"),
                        "a native selector identifier must not be empty",
                    );
                }
            }
            Selector::Region {
                coordinate_space_id,
                rect,
            } => {
                validator.require_reference(
                    coordinate_space_id,
                    canvas_ids,
                    &format!("{path}/selector/coordinateSpaceId"),
                    "canvas",
                );
                validate_rect(*rect, &format!("{path}/selector/rect"), validator);
            }
            Selector::TextRange { start, end } => {
                if start > end {
                    validator.issue(
                        ValidationCode::InvalidSelector,
                        format!("{path}/selector"),
                        "text range start must not exceed end",
                    );
                }
            }
        }
    }

    if let Some(uncertainty) = &evidence.uncertainty {
        validate_uncertainty(uncertainty, &format!("{path}/uncertainty"), validator);
    }
}

fn validate_confidence(value: f64, path: &str, validator: &mut Validator) {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        validator.issue(
            ValidationCode::InvalidConfidence,
            path,
            "confidence must be finite and between zero and one inclusive",
        );
    }
}

fn validate_uncertainty(uncertainty: &Uncertainty, path: &str, validator: &mut Validator) {
    match uncertainty {
        Uncertainty::ScalarRange { lower, upper } => {
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                validator.issue(
                    ValidationCode::InvalidUncertainty,
                    path,
                    "a scalar range requires finite bounds with lower not greater than upper",
                );
            }
        }
        Uncertainty::RectTolerance {
            x,
            y,
            width,
            height,
        } => {
            for (name, value) in [("x", *x), ("y", *y), ("width", *width), ("height", *height)] {
                if !value.is_finite() || value < 0.0 {
                    validator.issue(
                        ValidationCode::InvalidUncertainty,
                        format!("{path}/{name}"),
                        "rectangle tolerance must be finite and non-negative",
                    );
                }
            }
        }
        Uncertainty::CategoricalAlternatives { alternatives } => {
            if alternatives.is_empty() {
                validator.issue(
                    ValidationCode::InvalidUncertainty,
                    format!("{path}/alternatives"),
                    "categorical alternatives must not be empty",
                );
            }
            let mut values = BTreeSet::new();
            for (index, alternative) in alternatives.iter().enumerate() {
                if alternative.value.trim().is_empty() || !values.insert(&alternative.value) {
                    validator.issue(
                        ValidationCode::InvalidUncertainty,
                        format!("{path}/alternatives/{index}/value"),
                        "categorical alternative values must be non-empty and unique",
                    );
                }
                validate_confidence(
                    alternative.confidence,
                    &format!("{path}/alternatives/{index}/confidence"),
                    validator,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ValidationCode, ValidationErrors, ValidationIssue};

    #[test]
    fn validation_errors_are_sorted_and_deduplicated() {
        let duplicate = ValidationIssue {
            code: ValidationCode::EmptyIdentifier,
            path: "/z".to_owned(),
            message: "empty".to_owned(),
        };
        let errors = ValidationErrors::new(vec![
            duplicate.clone(),
            ValidationIssue {
                code: ValidationCode::InvalidReference,
                path: "/a".to_owned(),
                message: "missing".to_owned(),
            },
            duplicate,
        ]);

        assert_eq!(errors.len(), 2);
        assert_eq!(errors.issues()[0].path, "/a");
    }
}
