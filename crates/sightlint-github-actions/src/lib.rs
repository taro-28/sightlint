//! Versioned contracts for projecting trusted `SightLint` reports into GitHub Actions.
//!
//! This crate does not acquire artifacts, execute rules, contact GitHub, or decide whether an
//! observed condition is a defect. It defines a deterministic projection boundary around the
//! existing [`sightlint_engine::CheckReport`].

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path};

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sightlint_engine::{CheckReport, RuleEnforcement, RuleOutcome, RuleResult, Target, TargetKind};
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
/// Maximum UTF-8 source file inspected to validate one or more anchors.
pub const MAX_SOURCE_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum aggregate bytes inspected across unique source files in one declaration.
pub const MAX_SOURCE_MAP_TOTAL_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes written to one GitHub Actions step-summary file.
pub const MAX_STEP_SUMMARY_BYTES: u64 = 1024 * 1024;
/// Maximum inclusive source range accepted for one annotation.
pub const MAX_SOURCE_RANGE_LINES: u32 = 200;

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

/// Explicit gate policy applied after the trusted report is produced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionOptions {
    /// Treat `cantTell` as a process failure without changing its outcome or annotation level.
    pub deny_cant_tell: bool,
}

/// A source map that passed structural, report-join, repository, and anchor validation.
#[derive(Debug, Clone)]
pub struct ValidatedSourceMap {
    locations: BTreeMap<String, SourceLocation>,
}

/// Stable user-facing error at the GitHub Actions projection boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionError {
    message: String,
}

impl ProjectionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProjectionError {}

/// Parses and validates an independently authored exact-source declaration.
///
/// Validation is fail-closed: the artifact and every finding must match the report, entries must
/// be sorted and unique, paths must resolve to regular UTF-8 files inside `repository_root`, and
/// line anchors must still match exactly. No annotation output should be emitted before this
/// function succeeds.
///
/// # Errors
///
/// Returns a stable projection error when JSON, provenance, identity, path, range, or anchor
/// validation fails.
pub fn validate_source_map_json(
    input: &str,
    report: &CheckReport,
    repository_root: &Path,
) -> Result<ValidatedSourceMap, ProjectionError> {
    let source_map: GithubSourceMap = serde_json::from_str(input).map_err(|error| {
        ProjectionError::new(format!("invalid GitHub source map JSON: {error}"))
    })?;

    if source_map.artifact_id != report.artifact_id {
        return Err(ProjectionError::new(format!(
            "source-map artifactId {} does not match report artifactId {}",
            source_map.artifact_id, report.artifact_id
        )));
    }
    if source_map.provenance.implementation_output_used_as_oracle {
        return Err(ProjectionError::new(
            "source-map provenance must set implementationOutputUsedAsOracle to false",
        ));
    }
    if source_map.provenance.external_processing {
        return Err(ProjectionError::new(
            "source-map provenance must set externalProcessing to false for this local integration",
        ));
    }
    if source_map.entries.is_empty() || source_map.entries.len() > MAX_SOURCE_MAP_ENTRIES {
        return Err(ProjectionError::new(format!(
            "source map must contain between 1 and {MAX_SOURCE_MAP_ENTRIES} entries"
        )));
    }

    let root = repository_root.canonicalize().map_err(|error| {
        ProjectionError::new(format!(
            "failed to resolve repository root {}: {error}",
            repository_root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(ProjectionError::new(format!(
            "repository root {} is not a directory",
            repository_root.display()
        )));
    }

    let report_keys = report
        .results
        .iter()
        .map(finding_key_for_result)
        .collect::<Vec<_>>();
    let mut previous_key: Option<String> = None;
    let mut locations = BTreeMap::new();
    let mut source_files = BTreeMap::new();
    let mut total_source_bytes = 0_u64;

    for entry in source_map.entries {
        validate_finding_identity(&entry.finding)?;
        let key = finding_key(&entry.finding);
        if let Some(previous) = &previous_key {
            if key == *previous {
                return Err(ProjectionError::new(format!(
                    "source map contains duplicate finding identity {key}"
                )));
            }
            if key < *previous {
                return Err(ProjectionError::new(
                    "source-map entries must be sorted by stable finding identity",
                ));
            }
        }
        previous_key = Some(key.clone());

        let matches = report_keys
            .iter()
            .filter(|report_key| **report_key == key)
            .count();
        if matches != 1 {
            return Err(ProjectionError::new(format!(
                "source-map finding {key} must match exactly one report result; matched {matches}"
            )));
        }

        validate_source_location(
            &entry.location,
            &root,
            &mut source_files,
            &mut total_source_bytes,
        )?;
        locations.insert(key, entry.location);
    }

    Ok(ValidatedSourceMap { locations })
}

/// Projects one authoritative kernel report into the versioned GitHub Actions contract.
///
/// Outcome and enforcement are copied unchanged. Only failed, `cantTell`, and `untested` results
/// are projected; passed and inapplicable results remain available in the embedded report.
///
/// # Errors
///
/// Returns an error if the report contains duplicate actionable finding identities.
pub fn project_report(
    report: &CheckReport,
    source_map: Option<&ValidatedSourceMap>,
    options: ProjectionOptions,
) -> Result<GithubActionsReport, ProjectionError> {
    let actionable = ordered_actionable_results(report)?;
    let (projected_results, emitted, source_unavailable, omitted) =
        build_projected_results(actionable, source_map);

    let blocking_failures = report
        .results
        .iter()
        .filter(|result| {
            result.outcome == RuleOutcome::Failed && result.enforcement == RuleEnforcement::Blocking
        })
        .count() as u64;
    let advisory_failures = report
        .results
        .iter()
        .filter(|result| {
            result.outcome == RuleOutcome::Failed && result.enforcement == RuleEnforcement::Advisory
        })
        .count() as u64;
    let cant_tell = count_outcome(report, RuleOutcome::CantTell);
    let untested = count_outcome(report, RuleOutcome::Untested);
    let inapplicable = count_outcome(report, RuleOutcome::Inapplicable);
    let gate_exit_code =
        u8::from(blocking_failures > 0 || (options.deny_cant_tell && cant_tell > 0));

    Ok(GithubActionsReport {
        github_actions_report_schema_version: GithubActionsReportSchemaVersion::V0_1_0,
        integration: IntegrationIdentity {
            name: "sightlint-github-actions".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            verdict_owner: "sightlint-engine/check-report".to_owned(),
            network: "none".to_owned(),
        },
        check_report: report.clone(),
        projection_summary: ProjectionSummary {
            blocking_failures,
            advisory_failures,
            cant_tell,
            untested,
            inapplicable,
            actionable_results: projected_results.len() as u64,
            annotations_emitted: emitted,
            source_unavailable,
            annotations_omitted: omitted,
            annotation_limit: MAX_ANNOTATIONS as u64,
            deny_cant_tell: options.deny_cant_tell,
            gate_exit_code,
        },
        projected_results,
        limitations: vec![
            "Annotations require independently declared exact source lines; no source location is inferred from selectors, bundles, or pixels.".to_owned(),
            "Workflow commands use the existing GitHub Actions job check and do not create an independent REST check run.".to_owned(),
            "No artifact, screenshot, source excerpt, telemetry, token, or credential is transmitted or embedded.".to_owned(),
            "This projection preserves rule evidence and outcomes; it does not establish real-world UI/UX accuracy or a universal score.".to_owned(),
        ],
    })
}

fn count_outcome(report: &CheckReport, outcome: RuleOutcome) -> u64 {
    report
        .results
        .iter()
        .filter(|result| result.outcome == outcome)
        .count() as u64
}

type OrderedResult<'a> = (u8, String, &'a RuleResult, AnnotationLevel);

fn ordered_actionable_results(
    report: &CheckReport,
) -> Result<Vec<OrderedResult<'_>>, ProjectionError> {
    let mut actionable = report
        .results
        .iter()
        .filter(|result| {
            matches!(
                result.outcome,
                RuleOutcome::Failed | RuleOutcome::CantTell | RuleOutcome::Untested
            )
        })
        .map(|result| {
            let key = finding_key_for_result(result);
            let level = annotation_level(result.outcome, result.enforcement);
            (annotation_priority(level), key, result, level)
        })
        .collect::<Vec<_>>();
    actionable.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut keys = BTreeSet::new();
    for (_, key, _, _) in &actionable {
        if !keys.insert(key.clone()) {
            return Err(ProjectionError::new(format!(
                "report contains duplicate actionable finding identity {key}"
            )));
        }
    }
    Ok(actionable)
}

fn build_projected_results(
    actionable: Vec<OrderedResult<'_>>,
    source_map: Option<&ValidatedSourceMap>,
) -> (Vec<ProjectedResult>, u64, u64, u64) {
    let mut projected_results = Vec::with_capacity(actionable.len());
    let mut emitted = 0_u64;
    let mut source_unavailable = 0_u64;
    let mut omitted = 0_u64;
    for (_, key, result, level) in actionable {
        let location = source_map.and_then(|map| map.locations.get(&key));
        let annotation = match (source_map, location) {
            (None, _) => {
                source_unavailable += 1;
                AnnotationDisposition::SourceUnavailable {
                    reason: SourceUnavailableReason::SourceMapNotProvided,
                }
            }
            (Some(_), None) => {
                source_unavailable += 1;
                AnnotationDisposition::SourceUnavailable {
                    reason: SourceUnavailableReason::SourceLocationNotDeclared,
                }
            }
            (Some(_), Some(_)) if emitted >= MAX_ANNOTATIONS as u64 => {
                omitted += 1;
                AnnotationDisposition::Omitted {
                    reason: AnnotationOmissionReason::AnnotationLimit,
                }
            }
            (Some(_), Some(location)) => {
                emitted += 1;
                AnnotationDisposition::Emitted {
                    annotation: GithubAnnotation {
                        finding_key: key.clone(),
                        level,
                        path: location.path.clone(),
                        start_line: location.start_line,
                        end_line: location.end_line,
                        title: format!("SightLint: {}", result.title),
                        message: annotation_message(result, &key),
                    },
                }
            }
        };
        projected_results.push(ProjectedResult {
            finding_key: key,
            finding: FindingIdentity {
                rule_id: result.rule_id.clone(),
                rule_version: result.rule_version.clone(),
                target: result.target.clone(),
            },
            outcome: result.outcome,
            enforcement: result.enforcement,
            evidence_classes: result.evidence_classes.clone(),
            evidence_ids: result.evidence_ids.clone(),
            annotation,
        });
    }
    (projected_results, emitted, source_unavailable, omitted)
}

/// Serializes a projection as recursively key-sorted, pretty JSON with a final newline.
///
/// # Errors
///
/// Returns an error if the projection cannot be represented as JSON.
pub fn to_canonical_json(report: &GithubActionsReport) -> Result<String, serde_json::Error> {
    serialize_canonical(report)
}

/// Renders exact emitted annotations as escaped GitHub Actions workflow commands.
pub fn to_workflow_commands(report: &GithubActionsReport) -> String {
    let mut output = String::new();
    for projected in &report.projected_results {
        let AnnotationDisposition::Emitted { annotation } = &projected.annotation else {
            continue;
        };
        let _ = writeln!(
            output,
            "::{} file={},line={},endLine={},title={}::{}",
            annotation_level_label(annotation.level),
            escape_workflow_property(&annotation.path),
            annotation.start_line,
            annotation.end_line,
            escape_workflow_property(&annotation.title),
            escape_workflow_data(&annotation.message),
        );
    }
    output
}

/// Renders a stable, injection-safe Markdown summary of the complete projection.
pub fn to_step_summary(report: &GithubActionsReport) -> String {
    let mut output = String::new();
    write_summary_header(&mut output, report);

    if report.projected_results.is_empty() {
        let _ = writeln!(output, "No failed, cantTell, or untested results.\n");
    } else {
        let _ = writeln!(
            output,
            "| Outcome | Enforcement | Rule and target | Policy | Evidence | Source disposition |\n|---|---|---|---|---|---|"
        );
        for projected in &report.projected_results {
            write_projected_summary_row(&mut output, report, projected);
        }
        output.push('\n');
    }

    let _ = writeln!(
        output,
        "Outcome and enforcement remain separate. Missing exact source attribution is not inferred. This report makes no universal UI/UX score or real-world accuracy claim."
    );
    output
}

fn write_summary_header(output: &mut String, report: &GithubActionsReport) {
    let summary = &report.projection_summary;
    let _ = writeln!(output, "## SightLint GitHub Actions report\n");
    let _ = writeln!(
        output,
        "Artifact: <code>{}</code>  ",
        escape_html_text(report.check_report.artifact_id.as_str())
    );
    let _ = writeln!(
        output,
        "Gate exit: <code>{}</code> · blocking failures: {} · advisory failures: {} · cantTell: {} · untested: {} · inapplicable: {}  ",
        summary.gate_exit_code,
        summary.blocking_failures,
        summary.advisory_failures,
        summary.cant_tell,
        summary.untested,
        summary.inapplicable,
    );
    let _ = writeln!(
        output,
        "Annotations: {}/{} emitted · {} exact-source results omitted · {} results summary-only\n",
        summary.annotations_emitted,
        summary.annotation_limit,
        summary.annotations_omitted,
        summary.source_unavailable,
    );
}

fn write_projected_summary_row(
    output: &mut String,
    report: &GithubActionsReport,
    projected: &ProjectedResult,
) {
    let policy = report
        .check_report
        .results
        .iter()
        .find(|result| finding_key_for_result(result) == projected.finding_key)
        .map_or_else(
            || "unavailable: projection/report identity mismatch".to_owned(),
            |result| {
                format!(
                    "{} · {} · {}@{} · {}",
                    result.policy.profile,
                    policy_source_kind_label(result.policy.source_kind),
                    result.policy.source_id,
                    result.policy.source_version,
                    result.policy.reference,
                )
            },
        );
    let evidence_classes = projected
        .evidence_classes
        .iter()
        .map(|class| evidence_class_label(*class))
        .collect::<Vec<_>>()
        .join(", ");
    let evidence_ids = projected
        .evidence_ids
        .iter()
        .map(Identifier::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let evidence = format!(
        "classes: {}; ids: {}",
        nonempty_or_none(&evidence_classes),
        nonempty_or_none(&evidence_ids)
    );
    let disposition = annotation_disposition_label(&projected.annotation);
    let identity = format!(
        "{}@{} · {}:{}{}",
        projected.finding.rule_id,
        projected.finding.rule_version,
        target_kind_label(projected.finding.target.kind),
        projected.finding.target.id,
        projected
            .finding
            .target
            .aspect
            .as_deref()
            .map_or_else(String::new, |aspect| format!("/{aspect}"))
    );
    let _ = writeln!(
        output,
        "| <code>{}</code> | <code>{}</code> | <code>{}</code> | <code>{}</code> | <code>{}</code> | <code>{}</code> |",
        outcome_label(projected.outcome),
        enforcement_label(projected.enforcement),
        escape_html_text(&identity),
        escape_html_text(&policy),
        escape_html_text(&evidence),
        escape_html_text(&disposition),
    );
}

fn nonempty_or_none(value: &str) -> &str {
    if value.is_empty() { "none" } else { value }
}

fn annotation_disposition_label(disposition: &AnnotationDisposition) -> String {
    match disposition {
        AnnotationDisposition::Emitted { annotation } => format!(
            "{} at {}:{}-{}",
            annotation_level_label(annotation.level),
            annotation.path,
            annotation.start_line,
            annotation.end_line
        ),
        AnnotationDisposition::SourceUnavailable { reason } => {
            source_unavailable_label(*reason).to_owned()
        }
        AnnotationDisposition::Omitted { reason } => annotation_omission_label(*reason).to_owned(),
    }
}

/// Appends a summary only after enforcing the explicit one-step byte budget.
///
/// # Errors
///
/// Returns an error when the existing file cannot be inspected, the append would exceed 1 MiB, or
/// the file cannot be opened or written.
pub fn append_step_summary(path: &Path, summary: &str) -> Result<(), ProjectionError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        ProjectionError::new(format!(
            "failed to inspect GitHub step summary {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProjectionError::new(format!(
            "GitHub step summary {} must be an existing regular file, not a symlink",
            path.display()
        )));
    }
    let existing = metadata.len();
    let append = summary.len() as u64;
    if existing.saturating_add(append) > MAX_STEP_SUMMARY_BYTES {
        return Err(ProjectionError::new(format!(
            "GitHub step summary would exceed the {MAX_STEP_SUMMARY_BYTES}-byte safety limit"
        )));
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| {
            ProjectionError::new(format!(
                "failed to open GitHub step summary {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(summary.as_bytes()).map_err(|error| {
        ProjectionError::new(format!(
            "failed to write GitHub step summary {}: {error}",
            path.display()
        ))
    })
}

/// Returns the process exit code encoded by the trusted report and explicit gate policy.
pub const fn gate_exit_code(report: &GithubActionsReport) -> u8 {
    report.projection_summary.gate_exit_code
}

fn validate_finding_identity(finding: &FindingIdentity) -> Result<(), ProjectionError> {
    validate_nonempty_bounded("finding ruleId", &finding.rule_id)?;
    validate_nonempty_bounded("finding ruleVersion", &finding.rule_version)?;
    if finding.target.id.is_empty() {
        return Err(ProjectionError::new("finding target id must not be empty"));
    }
    if let Some(aspect) = &finding.target.aspect {
        validate_nonempty_bounded("finding target aspect", aspect)?;
    }
    Ok(())
}

fn validate_nonempty_bounded(label: &str, value: &str) -> Result<(), ProjectionError> {
    if value.is_empty() {
        return Err(ProjectionError::new(format!("{label} must not be empty")));
    }
    if value.len() > MAX_SOURCE_ANCHOR_BYTES {
        return Err(ProjectionError::new(format!(
            "{label} exceeds the {MAX_SOURCE_ANCHOR_BYTES}-byte safety limit"
        )));
    }
    Ok(())
}

fn validate_source_location(
    location: &SourceLocation,
    root: &Path,
    source_files: &mut BTreeMap<std::path::PathBuf, String>,
    total_source_bytes: &mut u64,
) -> Result<(), ProjectionError> {
    validate_repository_relative_path(&location.path)?;
    if location.start_line == 0
        || location.end_line < location.start_line
        || location.end_line - location.start_line + 1 > MAX_SOURCE_RANGE_LINES
    {
        return Err(ProjectionError::new(format!(
            "source range for {} must be one-based, ordered, and at most {MAX_SOURCE_RANGE_LINES} lines",
            location.path
        )));
    }
    if location.anchor_line < location.start_line || location.anchor_line > location.end_line {
        return Err(ProjectionError::new(format!(
            "source anchor for {} must fall inside the declared range",
            location.path
        )));
    }
    validate_nonempty_bounded("source anchorText", &location.anchor_text)?;
    if location.anchor_text.contains(['\r', '\n']) {
        return Err(ProjectionError::new(
            "source anchorText must contain exactly one line without a terminator",
        ));
    }

    let candidate = root.join(Path::new(&location.path));
    let resolved = candidate.canonicalize().map_err(|error| {
        ProjectionError::new(format!(
            "failed to resolve declared source {}: {error}",
            location.path
        ))
    })?;
    if !resolved.starts_with(root) {
        return Err(ProjectionError::new(format!(
            "declared source {} resolves outside the repository root",
            location.path
        )));
    }
    if !resolved.is_file() {
        return Err(ProjectionError::new(format!(
            "declared source {} is not a regular file",
            location.path
        )));
    }
    if !source_files.contains_key(&resolved) {
        let source_bytes = fs::metadata(&resolved)
            .map_err(|error| {
                ProjectionError::new(format!(
                    "failed to inspect declared source {}: {error}",
                    location.path
                ))
            })?
            .len();
        if source_bytes > MAX_SOURCE_FILE_BYTES {
            return Err(ProjectionError::new(format!(
                "declared source {} exceeds the {MAX_SOURCE_FILE_BYTES}-byte safety limit",
                location.path
            )));
        }
        *total_source_bytes = total_source_bytes
            .checked_add(source_bytes)
            .ok_or_else(|| {
                ProjectionError::new("declared source aggregate byte count overflowed")
            })?;
        if *total_source_bytes > MAX_SOURCE_MAP_TOTAL_SOURCE_BYTES {
            return Err(ProjectionError::new(format!(
                "declared sources exceed the {MAX_SOURCE_MAP_TOTAL_SOURCE_BYTES}-byte aggregate safety limit"
            )));
        }
        let bytes = fs::read(&resolved).map_err(|error| {
            ProjectionError::new(format!(
                "failed to read declared source {}: {error}",
                location.path
            ))
        })?;
        let text = String::from_utf8(bytes).map_err(|_| {
            ProjectionError::new(format!(
                "declared source {} is not valid UTF-8",
                location.path
            ))
        })?;
        source_files.insert(resolved.clone(), text);
    }
    let text = source_files
        .get(&resolved)
        .expect("validated source file was cached");
    let lines = text.lines().collect::<Vec<_>>();
    if location.end_line as usize > lines.len() {
        return Err(ProjectionError::new(format!(
            "source range for {} ends after the file's last line",
            location.path
        )));
    }
    let actual = lines
        .get(location.anchor_line as usize - 1)
        .expect("validated anchor line lies inside validated file range");
    if *actual != location.anchor_text {
        return Err(ProjectionError::new(format!(
            "source anchor for {}:{} is stale",
            location.path, location.anchor_line
        )));
    }
    Ok(())
}

fn validate_repository_relative_path(value: &str) -> Result<(), ProjectionError> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains(['\r', '\n', '\0'])
        || Path::new(value).is_absolute()
    {
        return Err(ProjectionError::new(
            "declared source path must be a nonempty repository-relative path using '/' separators",
        ));
    }
    if Path::new(value).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::CurDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ProjectionError::new(
            "declared source path must not contain traversal or platform prefixes",
        ));
    }
    Ok(())
}

fn finding_key_for_result(result: &RuleResult) -> String {
    finding_key(&FindingIdentity {
        rule_id: result.rule_id.clone(),
        rule_version: result.rule_version.clone(),
        target: result.target.clone(),
    })
}

fn finding_key(finding: &FindingIdentity) -> String {
    let aspect = finding.target.aspect.as_deref().map_or_else(
        || "none".to_owned(),
        |value| format!("value/{}", percent_encode(value)),
    );
    format!(
        "rule/{}/version/{}/target/{}/id/{}/aspect/{}",
        percent_encode(&finding.rule_id),
        percent_encode(&finding.rule_version),
        target_kind_label(finding.target.kind),
        percent_encode(finding.target.id.as_str()),
        aspect,
    )
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else {
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

const fn target_kind_label(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Artifact => "artifact",
        TargetKind::Canvas => "canvas",
        TargetKind::Node => "node",
        TargetKind::Relation => "relation",
    }
}

const fn annotation_level(outcome: RuleOutcome, enforcement: RuleEnforcement) -> AnnotationLevel {
    match (outcome, enforcement) {
        (RuleOutcome::Failed, RuleEnforcement::Blocking) => AnnotationLevel::Error,
        (RuleOutcome::Failed, RuleEnforcement::Advisory) => AnnotationLevel::Warning,
        (
            RuleOutcome::CantTell
            | RuleOutcome::Untested
            | RuleOutcome::Passed
            | RuleOutcome::Inapplicable,
            _,
        ) => AnnotationLevel::Notice,
    }
}

const fn annotation_priority(level: AnnotationLevel) -> u8 {
    match level {
        AnnotationLevel::Error => 0,
        AnnotationLevel::Warning => 1,
        AnnotationLevel::Notice => 2,
    }
}

const fn annotation_level_label(level: AnnotationLevel) -> &'static str {
    match level {
        AnnotationLevel::Error => "error",
        AnnotationLevel::Warning => "warning",
        AnnotationLevel::Notice => "notice",
    }
}

const fn outcome_label(outcome: RuleOutcome) -> &'static str {
    match outcome {
        RuleOutcome::Passed => "passed",
        RuleOutcome::Failed => "failed",
        RuleOutcome::Inapplicable => "inapplicable",
        RuleOutcome::CantTell => "cantTell",
        RuleOutcome::Untested => "untested",
    }
}

const fn enforcement_label(enforcement: RuleEnforcement) -> &'static str {
    match enforcement {
        RuleEnforcement::Advisory => "advisory",
        RuleEnforcement::Blocking => "blocking",
    }
}

const fn evidence_class_label(class: EvidenceClass) -> &'static str {
    match class {
        EvidenceClass::ExactSource => "exactSource",
        EvidenceClass::ExactRender => "exactRender",
        EvidenceClass::PlatformSemantics => "platformSemantics",
        EvidenceClass::VisionMeasured => "visionMeasured",
        EvidenceClass::VisionInferred => "visionInferred",
        EvidenceClass::InteractionTrace => "interactionTrace",
        EvidenceClass::DeclaredContract => "declaredContract",
        EvidenceClass::Unknown => "unknown",
    }
}

fn annotation_message(result: &RuleResult, finding_key: &str) -> String {
    let evidence_classes = if result.evidence_classes.is_empty() {
        "none".to_owned()
    } else {
        result
            .evidence_classes
            .iter()
            .map(|class| evidence_class_label(*class))
            .collect::<Vec<_>>()
            .join(",")
    };
    let evidence_ids = if result.evidence_ids.is_empty() {
        "none".to_owned()
    } else {
        result
            .evidence_ids
            .iter()
            .map(Identifier::as_str)
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{} [findingKey={}; rule={}@{}; outcome={}; enforcement={}; policy={}@{} ({}, {}, {}); evidenceClasses={}; evidenceIds={}]",
        result.message,
        finding_key,
        result.rule_id,
        result.rule_version,
        outcome_label(result.outcome),
        enforcement_label(result.enforcement),
        result.policy.source_id,
        result.policy.source_version,
        result.policy.profile,
        policy_source_kind_label(result.policy.source_kind),
        result.policy.reference,
        evidence_classes,
        evidence_ids,
    )
}

const fn policy_source_kind_label(kind: sightlint_engine::PolicySourceKind) -> &'static str {
    match kind {
        sightlint_engine::PolicySourceKind::DeclaredContract => "declaredContract",
        sightlint_engine::PolicySourceKind::PlatformStandard => "platformStandard",
        sightlint_engine::PolicySourceKind::ConservativeBuiltIn => "conservativeBuiltIn",
    }
}

fn escape_workflow_data(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_workflow_property(value: &str) -> String {
    escape_workflow_data(value)
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace(['\r', '\n'], " ")
}

const fn source_unavailable_label(reason: SourceUnavailableReason) -> &'static str {
    match reason {
        SourceUnavailableReason::SourceMapNotProvided => "sourceMapNotProvided",
        SourceUnavailableReason::SourceLocationNotDeclared => "sourceLocationNotDeclared",
    }
}

const fn annotation_omission_label(reason: AnnotationOmissionReason) -> &'static str {
    match reason {
        AnnotationOmissionReason::AnnotationLimit => "annotationLimit",
    }
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
    use sightlint_engine::{Target, TargetKind};
    use sightlint_ir::Identifier;

    use super::{
        FindingIdentity, finding_key, github_actions_report_schema_json,
        github_source_map_schema_json,
    };

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

    #[test]
    fn finding_keys_are_unambiguous_and_percent_encoded() {
        let identity = |aspect: Option<&str>| FindingIdentity {
            rule_id: "rule/percent%".to_owned(),
            rule_version: "0.1.0".to_owned(),
            target: Target {
                kind: TargetKind::Node,
                id: Identifier::new("node/日本語"),
                aspect: aspect.map(str::to_owned),
            },
        };
        let missing = finding_key(&identity(None));
        let tilde = finding_key(&identity(Some("~")));
        assert_ne!(missing, tilde);
        assert!(missing.ends_with("/aspect/none"));
        assert!(tilde.ends_with("/aspect/value/~"));
        assert!(tilde.contains("rule%2Fpercent%25"));
        assert!(tilde.contains("node%2F%E6%97%A5%E6%9C%AC%E8%AA%9E"));
    }
}
