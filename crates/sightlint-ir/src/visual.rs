//! Official, independently versioned visual-style and visual-contract extension.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ArtifactIr, Axis, BoxKind, Identifier, Observed, Unit, serialize_canonical};

/// Artifact IR extension key for the official visual contract.
pub const VISUAL_EXTENSION_KEY: &str = "org.sightlint.visual";

/// Current official visual extension version.
pub const VISUAL_EXTENSION_VERSION: &str = "0.1.0";

/// Exact or observed length with an explicit unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Length {
    /// Numeric length value.
    pub value: f64,
    /// Explicit length unit.
    pub unit: Unit,
}

/// Visual style observations attached to one node.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualStyle {
    /// Observed font size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<Observed<Length>>,
}

/// Directional or geometric anchor compared by a peer-alignment contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AlignmentAnchor {
    /// Logical start edge for the canvas direction.
    Start,
    /// Geometric center.
    Center,
    /// Logical end edge for the canvas direction.
    End,
}

/// Rectangle dimension compared by a peer-extent contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ExtentDimension {
    /// Rectangle width.
    Width,
    /// Rectangle height.
    Height,
}

/// Explicit, evidenced visual expectation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum VisualContract {
    /// Nodes are expected to share one directional alignment anchor.
    PeerAlignment {
        /// Nodes belonging to the comparison set.
        node_ids: Vec<Identifier>,
        /// Axis along which anchor coordinates are measured.
        axis: Axis,
        /// Logical or geometric anchor compared on that axis.
        anchor: AlignmentAnchor,
        /// Rectangle observation used by the comparison.
        box_kind: BoxKind,
        /// Maximum absolute deviation in the resolved coordinate-space unit.
        tolerance: f64,
        /// Evidence supporting the comparison expectation.
        evidence_id: Identifier,
    },
    /// Nodes are expected to have a consistent width or height.
    PeerExtent {
        /// Nodes belonging to the comparison set.
        node_ids: Vec<Identifier>,
        /// Width or height to compare.
        dimension: ExtentDimension,
        /// Rectangle observation used by the comparison.
        box_kind: BoxKind,
        /// Maximum absolute deviation in the resolved geometry unit.
        tolerance: f64,
        /// Evidence supporting the comparison expectation.
        evidence_id: Identifier,
    },
    /// Nodes are expected to use a consistent observed font size.
    PeerFontSize {
        /// Nodes belonging to the comparison set.
        node_ids: Vec<Identifier>,
        /// Maximum absolute deviation in the shared font-size unit.
        tolerance: f64,
        /// Evidence supporting the comparison expectation.
        evidence_id: Identifier,
    },
    /// Nodes are expected to meet an explicitly declared minimum font size.
    MinimumFontSize {
        /// Nodes governed by the policy.
        node_ids: Vec<Identifier>,
        /// Declared minimum and its unit.
        minimum: Length,
        /// Evidence supporting the policy threshold and target set.
        evidence_id: Identifier,
    },
}

impl VisualContract {
    /// Returns the compared or governed node identifiers.
    pub fn node_ids(&self) -> &[Identifier] {
        match self {
            Self::PeerAlignment { node_ids, .. }
            | Self::PeerExtent { node_ids, .. }
            | Self::PeerFontSize { node_ids, .. }
            | Self::MinimumFontSize { node_ids, .. } => node_ids,
        }
    }

    /// Returns the evidence supporting the expectation.
    pub const fn evidence_id(&self) -> &Identifier {
        match self {
            Self::PeerAlignment { evidence_id, .. }
            | Self::PeerExtent { evidence_id, .. }
            | Self::PeerFontSize { evidence_id, .. }
            | Self::MinimumFontSize { evidence_id, .. } => evidence_id,
        }
    }

    fn canonicalize(&mut self) {
        let node_ids = match self {
            Self::PeerAlignment { node_ids, .. }
            | Self::PeerExtent { node_ids, .. }
            | Self::PeerFontSize { node_ids, .. }
            | Self::MinimumFontSize { node_ids, .. } => node_ids,
        };
        node_ids.sort();
    }
}

/// Typed payload stored under [`VISUAL_EXTENSION_KEY`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualExtension {
    /// Independent extension contract version.
    pub extension_version: String,
    /// Exact or inferred visual styles keyed by target node identifier.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub node_styles: BTreeMap<Identifier, VisualStyle>,
    /// Explicit visual expectations keyed by stable contract identifier.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub contracts: BTreeMap<Identifier, VisualContract>,
}

impl VisualExtension {
    /// Returns a clone with all set-like node collections in canonical order.
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        for contract in canonical.contracts.values_mut() {
            contract.canonicalize();
        }
        canonical
    }

    /// Validates references, numeric values, and official extension invariants.
    ///
    /// # Errors
    ///
    /// Returns every deterministic issue in stable order.
    pub fn validate(&self, document: &ArtifactIr) -> Result<(), VisualExtensionErrors> {
        let mut validator = VisualValidator::new(document);

        if self.extension_version != VISUAL_EXTENSION_VERSION {
            validator.issue(
                VisualValidationCode::UnsupportedExtensionVersion,
                "/extensionVersion",
                format!(
                    "expected visual extension version {VISUAL_EXTENSION_VERSION}, found {}",
                    self.extension_version
                ),
            );
        }

        for (node_id, style) in &self.node_styles {
            let escaped = escape_pointer_segment(node_id.as_str());
            let path = format!("/nodeStyles/{escaped}");
            validator.require_node(node_id, &path);
            if style.font_size.is_none() {
                validator.issue(
                    VisualValidationCode::EmptyStyle,
                    &path,
                    "a visual style must contain at least one observation",
                );
            }
            if let Some(font_size) = &style.font_size {
                validator.require_evidence(
                    &font_size.evidence_id,
                    &format!("{path}/fontSize/evidenceId"),
                );
                validate_positive_length(
                    font_size.value,
                    &format!("{path}/fontSize/value"),
                    &mut validator,
                );
            }
        }

        for (contract_id, contract) in &self.contracts {
            let escaped = escape_pointer_segment(contract_id.as_str());
            let path = format!("/contracts/{escaped}");
            if contract_id.is_empty() {
                validator.issue(
                    VisualValidationCode::EmptyContractIdentifier,
                    &path,
                    "contract identifiers must not be empty",
                );
            }
            validator.require_evidence(contract.evidence_id(), &format!("{path}/evidenceId"));
            validate_contract(contract, &path, &mut validator);
        }

        validator.finish()
    }
}

/// Stable category for one official visual extension problem.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum VisualValidationCode {
    /// The recognized extension uses an unsupported contract version.
    UnsupportedExtensionVersion,
    /// The recognized extension payload cannot be decoded.
    InvalidExtensionPayload,
    /// A style references a node that does not exist.
    InvalidNodeReference,
    /// A contract or style references evidence that does not exist.
    InvalidEvidenceReference,
    /// A contract identifier is empty.
    EmptyContractIdentifier,
    /// A peer or policy target set has too few members.
    InsufficientContractMembers,
    /// A contract repeats one node identifier.
    DuplicateContractMember,
    /// A numeric value is `NaN` or infinite.
    NonFiniteNumber,
    /// A length that must be positive is zero or negative.
    NonPositiveLength,
    /// A tolerance is negative.
    NegativeTolerance,
    /// A normalized ratio is not a meaningful typography unit.
    InvalidTypographyUnit,
    /// A style payload contains no observations.
    EmptyStyle,
}

/// One deterministic official visual extension validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualValidationIssue {
    /// Stable issue category.
    pub code: VisualValidationCode,
    /// JSON Pointer relative to the visual extension payload.
    pub path: String,
    /// Stable human-readable explanation.
    pub message: String,
}

/// Ordered collection of official visual extension issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualExtensionErrors {
    issues: Vec<VisualValidationIssue>,
}

impl VisualExtensionErrors {
    /// Creates a sorted, duplicate-free error collection.
    pub fn new(mut issues: Vec<VisualValidationIssue>) -> Self {
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
        issues.dedup();
        Self { issues }
    }

    /// Creates one stable payload-decoding error.
    pub fn decode(message: impl Into<String>) -> Self {
        Self::new(vec![VisualValidationIssue {
            code: VisualValidationCode::InvalidExtensionPayload,
            path: String::new(),
            message: message.into(),
        }])
    }

    /// Returns the ordered issues.
    pub fn issues(&self) -> &[VisualValidationIssue] {
        &self.issues
    }

    /// Returns whether no issues are present.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }
}

impl fmt::Display for VisualExtensionErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "official visual extension validation failed with {} issue(s):",
            self.issues.len()
        )?;
        for issue in &self.issues {
            let path = if issue.path.is_empty() {
                "/"
            } else {
                issue.path.as_str()
            };
            writeln!(
                formatter,
                "- {:?} at {}{}: {}",
                issue.code, VISUAL_EXTENSION_KEY, path, issue.message
            )?;
        }
        Ok(())
    }
}

impl Error for VisualExtensionErrors {}

impl ArtifactIr {
    /// Decodes and validates the official visual extension when present.
    ///
    /// # Errors
    ///
    /// Returns stable extension decoding or semantic validation issues.
    pub fn visual_extension(&self) -> Result<Option<VisualExtension>, VisualExtensionErrors> {
        let Some(value) = self.extensions.get(VISUAL_EXTENSION_KEY) else {
            return Ok(None);
        };
        let extension: VisualExtension =
            serde_json::from_value(value.clone()).map_err(|error| {
                VisualExtensionErrors::decode(format!("failed to decode visual extension: {error}"))
            })?;
        extension.validate(self)?;
        Ok(Some(extension.canonicalized()))
    }
}

/// Generates the JSON Schema for the current official visual extension.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be represented as JSON.
pub fn visual_extension_schema_json() -> Result<String, serde_json::Error> {
    let mut schema = serde_json::to_value(schema_for!(VisualExtension))?;
    if let Value::Object(object) = &mut schema {
        object.insert(
            "$id".to_owned(),
            Value::String(format!(
                "urn:sightlint:schema:visual-extension:{VISUAL_EXTENSION_VERSION}"
            )),
        );
        object.insert(
            "title".to_owned(),
            Value::String(format!(
                "SightLint Visual Extension {VISUAL_EXTENSION_VERSION}"
            )),
        );
    }
    serialize_canonical(&schema)
}

/// Canonicalizes a recognized visual extension payload in-place.
///
/// Invalid payloads are left unchanged; loading validation reports them before trusted rule
/// execution. This helper exists so canonical Artifact IR output can normalize a valid official
/// extension without altering unknown extension keys.
pub(crate) fn canonicalize_visual_extension(value: &mut Value) {
    let Ok(extension) = serde_json::from_value::<VisualExtension>(value.clone()) else {
        return;
    };
    if let Ok(canonical) = serde_json::to_value(extension.canonicalized()) {
        *value = canonical;
    }
}

struct VisualValidator {
    issues: Vec<VisualValidationIssue>,
    node_ids: BTreeSet<Identifier>,
    evidence_ids: BTreeSet<Identifier>,
}

impl VisualValidator {
    fn new(document: &ArtifactIr) -> Self {
        Self {
            issues: Vec::new(),
            node_ids: document.nodes.iter().map(|node| node.id.clone()).collect(),
            evidence_ids: document
                .evidence
                .iter()
                .map(|evidence| evidence.id.clone())
                .collect(),
        }
    }

    fn issue(
        &mut self,
        code: VisualValidationCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(VisualValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }

    fn require_node(&mut self, id: &Identifier, path: &str) {
        if !self.node_ids.contains(id) {
            self.issue(
                VisualValidationCode::InvalidNodeReference,
                path,
                format!("referenced node {id} does not exist"),
            );
        }
    }

    fn require_evidence(&mut self, id: &Identifier, path: &str) {
        if !self.evidence_ids.contains(id) {
            self.issue(
                VisualValidationCode::InvalidEvidenceReference,
                path,
                format!("referenced evidence {id} does not exist"),
            );
        }
    }

    fn finish(self) -> Result<(), VisualExtensionErrors> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(VisualExtensionErrors::new(self.issues))
        }
    }
}

fn validate_contract(contract: &VisualContract, path: &str, validator: &mut VisualValidator) {
    let minimum_members = match contract {
        VisualContract::MinimumFontSize { .. } => 1,
        VisualContract::PeerAlignment { .. }
        | VisualContract::PeerExtent { .. }
        | VisualContract::PeerFontSize { .. } => 2,
    };
    validate_members(contract.node_ids(), minimum_members, path, validator);

    match contract {
        VisualContract::PeerAlignment { tolerance, .. }
        | VisualContract::PeerExtent { tolerance, .. }
        | VisualContract::PeerFontSize { tolerance, .. } => {
            validate_tolerance(*tolerance, &format!("{path}/tolerance"), validator);
        }
        VisualContract::MinimumFontSize { minimum, .. } => {
            validate_positive_length(*minimum, &format!("{path}/minimum"), validator);
        }
    }
}

fn validate_members(
    node_ids: &[Identifier],
    minimum_members: usize,
    path: &str,
    validator: &mut VisualValidator,
) {
    if node_ids.len() < minimum_members {
        validator.issue(
            VisualValidationCode::InsufficientContractMembers,
            format!("{path}/nodeIds"),
            format!("this contract requires at least {minimum_members} node(s)"),
        );
    }

    let mut unique = BTreeSet::new();
    for (index, node_id) in node_ids.iter().enumerate() {
        validator.require_node(node_id, &format!("{path}/nodeIds/{index}"));
        if !unique.insert(node_id) {
            validator.issue(
                VisualValidationCode::DuplicateContractMember,
                format!("{path}/nodeIds/{index}"),
                format!("node {node_id} appears more than once in the contract"),
            );
        }
    }
}

fn validate_tolerance(value: f64, path: &str, validator: &mut VisualValidator) {
    if !value.is_finite() {
        validator.issue(
            VisualValidationCode::NonFiniteNumber,
            path,
            "tolerance must be finite",
        );
    } else if value < 0.0 {
        validator.issue(
            VisualValidationCode::NegativeTolerance,
            path,
            "tolerance must not be negative",
        );
    }
}

fn validate_positive_length(length: Length, path: &str, validator: &mut VisualValidator) {
    if !length.value.is_finite() {
        validator.issue(
            VisualValidationCode::NonFiniteNumber,
            format!("{path}/value"),
            "length must be finite",
        );
    } else if length.value <= 0.0 {
        validator.issue(
            VisualValidationCode::NonPositiveLength,
            format!("{path}/value"),
            "length must be greater than zero",
        );
    }
    if length.unit == Unit::Normalized {
        validator.issue(
            VisualValidationCode::InvalidTypographyUnit,
            format!("{path}/unit"),
            "normalized ratios cannot express an exact font-size policy",
        );
    }
}

fn escape_pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{VISUAL_EXTENSION_VERSION, VisualExtension};

    #[test]
    fn empty_extension_canonicalization_is_idempotent() {
        let extension = VisualExtension {
            extension_version: VISUAL_EXTENSION_VERSION.to_owned(),
            node_styles: BTreeMap::new(),
            contracts: BTreeMap::new(),
        };
        assert_eq!(
            extension.canonicalized().canonicalized(),
            extension.canonicalized()
        );
    }
}
