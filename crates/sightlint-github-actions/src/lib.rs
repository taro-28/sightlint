//! Versioned contracts for projecting trusted SightLint reports into GitHub Actions.
//!
//! This crate does not acquire artifacts, execute rules, contact GitHub, or decide whether an
//! observed condition is a defect. It defines a deterministic projection boundary around the
//! existing [`sightlint_engine::CheckReport`].

#![forbid(unsafe_code)]

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sightlint_engine::{CheckReport, RuleEnforcement, RuleOutcome, Target};
use sightlint_ir::{EvidenceClass, Identifier, serialize_canonical};

/// Current source-location declaration schema version.
pub const GITHUB_SOURCE_MAP_SCHEMA_VERSION: &str = "0.1.0";
/// Current GitHub Actions projection report schema version.
pub const GITHUB_ACTIONS_REPORT_SCHEMA_VERSION: &str = "0.1.0";
/// Maximum exact source-location entries accepted from one declaration.
pub const MAX_SOURCE_MAP_ENTRIES: usize = 512;
/// Maximum annotations emitted into one GitHub Actions step.
pub const MAX_ANNOTATIONS: usize = 50;
/// Maximum UTF-8 bytes allowed in one stable source anchor.
pub const MAX_SOURCE_ANCHOR_BYTES: usize = 4 * 1024;
/// Maximum bytes written to one GitHub Actions step-summary file.
pub const MAX_STEP_SUMMARY_BYTES: u64 = 1024 * 1024;

/// Serialized source-map version discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SourceMapSchemaVersion {
    /// Source map contract `0.1.0`.
    #[serde(rename = "0.1.0")]
    V0_1_0,
}

/// Serialized projection-report version discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GithubActionsReportSchemaVersion {
    /// Projection report contract `0.1.0`.
    #[serde(rename = "0.1.0")]
    V0_1_0,
}

/// How exact source locations were authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceMapAuthoringBasis {
    /// A repository owner declared and reviewed the source relation independently of rule output.
    DeclaredExactSource,
}

/// Provenance and trust boundary of one source-location declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMapProvenance {
    /// Authority used to establish the source relation.
    pub authoring_basis: SourceMapAuthoringBasis,
    /// Must remain false: current implementation output is not an annotation oracle.
    pub implementation_output_used_as_oracle: bool,
    /// Whether artifact or source content was sent to an external processor while authoring.
    pub external_processing: bool,
}

/// Stable identity of a rule result, excluding its outcome and enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FindingIdentity {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Semantic rule version.
    pub rule_version: String,
    /// Exact target and optional aspect evaluated by the rule.
    pub target: Target,
}

/// Source attribution grade supported by the first GitHub Actions contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceAttribution {
    /// A reviewed repository path and line range with a stable exact line anchor.
    DeclaredExactSourceLine,
}

/// Exact repository source range used for a GitHub annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLocation {
    /// Attribution grade; version `0.1.0` accepts only declared exact source lines.
    pub attribution: SourceAttribution,
    /// Repository-relative path using `/` separators.
    pub path: String,
    /// One-based first annotated line.
    pub start_line: u32,
    /// One-based final annotated line, inclusive.
    pub end_line: u32,
    /// One-based line inside the annotated range used to detect location drift.
    pub anchor_line: u32,
    /// Exact UTF-8 text expected at `anchorLine`, excluding its line terminator.
    pub anchor_text: String,
}

/// One independently declared rule-result-to-source relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMapEntry {
    /// Result identity to which the source range applies.
    pub finding: FindingIdentity,
    /// Independently declared exact source location.
    pub location: SourceLocation,
}

/// Strict, artifact-specific source-location declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubSourceMap {
    /// Source-map schema version.
    pub source_map_schema_version: SourceMapSchemaVersion,
    /// Artifact identifier that must match the trusted report.
    pub artifact_id: Identifier,
    /// Authority, oracle, and processing provenance.
    pub provenance: SourceMapProvenance,
    /// Sorted, semantically unique source relations.
    pub entries: Vec<SourceMapEntry>,
}

/// Identity of the GitHub Actions projection boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationIdentity {
    /// Stable integration name.
    pub name: String,
    /// Package implementation version.
    pub version: String,
    /// Component that remains authoritative for every rule verdict.
    pub verdict_owner: String,
    /// Network behavior of the projection process.
    pub network: String,
}

/// GitHub annotation level derived only from outcome plus enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationLevel {
    /// Failed blocking result.
    Error,
    /// Failed advisory result.
    Warning,
    /// `cantTell` or `untested` coverage signal.
    Notice,
}

/// One exact, escaped GitHub Actions annotation before workflow-command rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubAnnotation {
    /// Stable projection key used for ordering and deduplication.
    pub finding_key: String,
    /// GitHub annotation level.
    pub level: AnnotationLevel,
    /// Repository-relative exact source path.
    pub path: String,
    /// One-based first line.
    pub start_line: u32,
    /// One-based final line, inclusive.
    pub end_line: u32,
    /// Human-readable title derived from the trusted result.
    pub title: String,
    /// Evidence- and policy-bearing message derived from the trusted result.
    pub message: String,
}

/// Explicit reason an actionable result has no source annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceUnavailableReason {
    /// The caller supplied no source-location declaration.
    SourceMapNotProvided,
    /// A valid declaration did not contain this exact finding identity.
    SourceLocationNotDeclared,
}

/// Explicit reason a source-located result was not emitted as an annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationOmissionReason {
    /// A higher-priority set already consumed the bounded annotation budget.
    AnnotationLimit,
}

/// Annotation disposition for one failed or abstaining rule result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnnotationDisposition {
    /// An exact source annotation is present.
    Emitted {
        /// Complete platform-neutral annotation fields.
        annotation: GithubAnnotation,
    },
    /// Exact source attribution is unavailable and the result remains summary-only.
    SourceUnavailable {
        /// Why exact attribution was unavailable.
        reason: SourceUnavailableReason,
    },
    /// Exact source attribution exists but the bounded output cap omitted the annotation.
    Omitted {
        /// Why the annotation was omitted.
        reason: AnnotationOmissionReason,
    },
}

/// Projection metadata for one failed, `cantTell`, or `untested` result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectedResult {
    /// Stable projection key.
    pub finding_key: String,
    /// Result identity excluding verdict fields.
    pub finding: FindingIdentity,
    /// Trusted kernel outcome, preserved without coercion.
    pub outcome: RuleOutcome,
    /// Trusted kernel enforcement, separate from the outcome.
    pub enforcement: RuleEnforcement,
    /// Evidence classes preserved from the trusted result.
    pub evidence_classes: Vec<EvidenceClass>,
    /// Evidence identifiers preserved from the trusted result.
    pub evidence_ids: Vec<Identifier>,
    /// Annotation or explicit reason no annotation was emitted.
    pub annotation: AnnotationDisposition,
}

/// Deterministic projection counts and gate policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectionSummary {
    /// Blocking failed results.
    pub blocking_failures: u64,
    /// Advisory failed results.
    pub advisory_failures: u64,
    /// Ambiguous results preserved as `cantTell`.
    pub cant_tell: u64,
    /// Required observations or executions that were not performed.
    pub untested: u64,
    /// Results that did not meet applicability conditions.
    pub inapplicable: u64,
    /// Failed, `cantTell`, and `untested` results considered for annotation.
    pub actionable_results: u64,
    /// Exact annotations emitted after applying the cap.
    pub annotations_emitted: u64,
    /// Actionable results without declared exact source attribution.
    pub source_unavailable: u64,
    /// Exact annotations omitted after the deterministic cap.
    pub annotations_omitted: u64,
    /// Fixed per-step annotation cap.
    pub annotation_limit: u64,
    /// Whether the caller explicitly made `cantTell` gate-failing.
    pub deny_cant_tell: bool,
    /// Public process exit code implied by the trusted report and explicit gate policy.
    pub gate_exit_code: u8,
}

/// Complete deterministic GitHub Actions projection of one trusted check report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GithubActionsReport {
    /// Projection report schema version.
    pub github_actions_report_schema_version: GithubActionsReportSchemaVersion,
    /// Projection implementation and trust boundary.
    pub integration: IntegrationIdentity,
    /// Complete authoritative report produced by the deterministic kernel.
    pub check_report: CheckReport,
    /// Projection-specific counts and exit policy.
    pub projection_summary: ProjectionSummary,
    /// Failed and abstaining results with exact annotation disposition.
    pub projected_results: Vec<ProjectedResult>,
    /// Stable limitations and non-claims of this projection.
    pub limitations: Vec<String>,
}

/// Generates the canonical JSON Schema for source-location declarations.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be represented as JSON.
pub fn github_source_map_schema_json() -> Result<String, serde_json::Error> {
    schema_json(
        schema_for!(GithubSourceMap),
        "github-source-map",
        GITHUB_SOURCE_MAP_SCHEMA_VERSION,
        "SightLint GitHub source map",
    )
}

/// Generates the canonical JSON Schema for GitHub Actions projection reports.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be represented as JSON.
pub fn github_actions_report_schema_json() -> Result<String, serde_json::Error> {
    schema_json(
        schema_for!(GithubActionsReport),
        "github-actions-report",
        GITHUB_ACTIONS_REPORT_SCHEMA_VERSION,
        "SightLint GitHub Actions report",
    )
}

fn schema_json(
    schema: schemars::Schema,
    name: &str,
    version: &str,
    title: &str,
) -> Result<String, serde_json::Error> {
    let mut schema = serde_json::to_value(schema)?;
    if let Value::Object(object) = &mut schema {
        object.insert(
            "$id".to_owned(),
            Value::String(format!("urn:sightlint:schema:{name}:{version}")),
        );
        object.insert(
            "title".to_owned(),
            Value::String(format!("{title} {version}")),
        );
    }
    serialize_canonical(&schema)
}

#[cfg(test)]
mod tests {
    use super::{github_actions_report_schema_json, github_source_map_schema_json};

    #[test]
    fn generated_schemas_expose_independent_versions() {
        let source: serde_json::Value = serde_json::from_str(
            &github_source_map_schema_json().expect("source-map schema generation"),
        )
        .expect("source-map schema JSON");
        let report: serde_json::Value = serde_json::from_str(
            &github_actions_report_schema_json().expect("report schema generation"),
        )
        .expect("report schema JSON");
        assert_eq!(
            source["$id"],
            "urn:sightlint:schema:github-source-map:0.1.0"
        );
        assert_eq!(
            report["$id"],
            "urn:sightlint:schema:github-actions-report:0.1.0"
        );
    }
}
