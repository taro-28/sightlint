//! Stable, evidence-linked rule reports.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sightlint_ir::{
    ArtifactIr, ArtifactKind, EvidenceClass, Identifier, Unit, serialize_canonical,
};

/// Current serialized report schema version.
pub const REPORT_SCHEMA_VERSION: &str = "0.2.0";

/// ACT-inspired result of evaluating one applicable target.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum RuleOutcome {
    /// The expectation was satisfied with sufficient evidence.
    Passed,
    /// The expectation was violated with sufficient evidence.
    Failed,
    /// The target did not meet the rule's applicability conditions.
    Inapplicable,
    /// Available evidence was insufficient or incomparable.
    CantTell,
    /// A required observation or execution was not performed.
    Untested,
}

impl RuleOutcome {
    /// Returns the stable human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passed => "PASS",
            Self::Failed => "FAIL",
            Self::Inapplicable => "INAPPLICABLE",
            Self::CantTell => "CANT_TELL",
            Self::Untested => "UNTESTED",
        }
    }
}

/// Executable rule composition kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuleKind {
    /// One narrow, independently testable expectation.
    Atomic,
    /// A deterministic composition of other rule outcomes.
    Composite,
}

/// Validation maturity of a rule implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuleMaturity {
    /// Rule semantics and fixtures are still being established.
    Experimental,
    /// Rule is useful for review but not a default blocking gate.
    Advisory,
    /// Rule has earned eligibility for explicit blocking policy.
    BlockingEligible,
}

/// Stable kind of target evaluated by a rule.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum TargetKind {
    /// Whole artifact.
    Artifact,
    /// Canvas, page, screen, or viewport.
    Canvas,
    /// Visual or semantic node.
    Node,
    /// Explicit source or inferred relation.
    Relation,
}

/// Stable reference to one evaluated target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    /// Target category.
    pub kind: TargetKind,
    /// Stable target identifier.
    pub id: Identifier,
    /// Optional aspect such as `renderBox` or a node pair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<String>,
}

/// Numeric value reported by a rule with an explicit unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Measurement {
    /// Numeric value.
    pub value: f64,
    /// Unit of the value.
    pub unit: Unit,
}

/// Result of one rule evaluation against one target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleResult {
    /// Stable rule identifier.
    pub rule_id: String,
    /// Semantic rule version.
    pub rule_version: String,
    /// Human-readable rule title.
    pub title: String,
    /// Rule composition kind.
    pub kind: RuleKind,
    /// Current validation maturity.
    pub maturity: RuleMaturity,
    /// Evaluated target.
    pub target: Target,
    /// ACT-inspired outcome.
    pub outcome: RuleOutcome,
    /// Stable explanation of the observed result.
    pub message: String,
    /// Evidence records used by the rule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_ids: Vec<Identifier>,
    /// Evidence classes represented by `evidenceIds`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_classes: Vec<EvidenceClass>,
    /// Nodes involved in a relation-level result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_node_ids: Vec<Identifier>,
    /// Named, explicitly unit-qualified measurements.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub measurements: BTreeMap<String, Measurement>,
}

/// Deterministic counts of rule outcomes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportSummary {
    /// Number of passing results.
    pub passed: u64,
    /// Number of failing results.
    pub failed: u64,
    /// Number of inapplicable results.
    pub inapplicable: u64,
    /// Number of ambiguous results.
    pub cant_tell: u64,
    /// Number of unexecuted results.
    pub untested: u64,
}

impl ReportSummary {
    fn observe(&mut self, outcome: RuleOutcome) {
        match outcome {
            RuleOutcome::Passed => self.passed += 1,
            RuleOutcome::Failed => self.failed += 1,
            RuleOutcome::Inapplicable => self.inapplicable += 1,
            RuleOutcome::CantTell => self.cant_tell += 1,
            RuleOutcome::Untested => self.untested += 1,
        }
    }

    /// Returns the total number of emitted results.
    pub const fn total(self) -> u64 {
        self.passed + self.failed + self.inapplicable + self.cant_tell + self.untested
    }
}

/// Complete deterministic check report for one artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckReport {
    /// Version of the report schema.
    pub report_schema_version: String,
    /// Version of the engine that produced the report.
    pub engine_version: String,
    /// Artifact identifier.
    pub artifact_id: Identifier,
    /// Artifact medium.
    pub artifact_kind: ArtifactKind,
    /// Independently versioned official extensions consumed by the engine.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extension_versions: BTreeMap<String, String>,
    /// Outcome counts.
    pub summary: ReportSummary,
    /// Results in stable rule and target order.
    pub results: Vec<RuleResult>,
}

impl CheckReport {
    /// Builds a canonical report and derives its summary.
    pub fn new(
        document: &ArtifactIr,
        extension_versions: BTreeMap<String, String>,
        mut results: Vec<RuleResult>,
    ) -> Self {
        for result in &mut results {
            result.evidence_ids.sort();
            result.evidence_ids.dedup();
            result
                .evidence_classes
                .sort_by_key(|class| evidence_class_order(*class));
            result.evidence_classes.dedup();
            result.related_node_ids.sort();
            result.related_node_ids.dedup();
        }
        results.sort_by(|left, right| {
            left.rule_id
                .cmp(&right.rule_id)
                .then_with(|| left.target.kind.cmp(&right.target.kind))
                .then_with(|| left.target.id.cmp(&right.target.id))
                .then_with(|| left.target.aspect.cmp(&right.target.aspect))
        });

        let mut summary = ReportSummary::default();
        for result in &results {
            summary.observe(result.outcome);
        }

        Self {
            report_schema_version: REPORT_SCHEMA_VERSION.to_owned(),
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            artifact_id: document.artifact.id.clone(),
            artifact_kind: document.artifact.kind,
            extension_versions,
            summary,
            results,
        }
    }

    /// Serializes the report as canonical, pretty-printed JSON.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serialize_canonical(self)
    }

    /// Formats a stable, color-free human report.
    pub fn to_human(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(
            output,
            "SightLint {} — artifact {} ({:?})",
            self.engine_version, self.artifact_id, self.artifact_kind
        );
        let _ = writeln!(
            output,
            "{} result(s): {} passed, {} failed, {} cantTell, {} inapplicable, {} untested",
            self.summary.total(),
            self.summary.passed,
            self.summary.failed,
            self.summary.cant_tell,
            self.summary.inapplicable,
            self.summary.untested
        );
        if !self.extension_versions.is_empty() {
            let extensions = self
                .extension_versions
                .iter()
                .map(|(key, version)| format!("{key}@{version}"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(output, "extensions: {extensions}");
        }

        for result in &self.results {
            let aspect = result
                .target
                .aspect
                .as_deref()
                .map_or_else(String::new, |value| format!("/{value}"));
            let _ = writeln!(
                output,
                "\n{} {} [{}:{}{}]",
                result.outcome.label(),
                result.rule_id,
                target_kind_label(result.target.kind),
                result.target.id,
                aspect
            );
            let _ = writeln!(output, "  {}", result.message);
            if !result.evidence_ids.is_empty() {
                let evidence = result
                    .evidence_ids
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(output, "  evidence: {evidence}");
            }
        }

        output
    }
}

fn evidence_class_order(class: EvidenceClass) -> u8 {
    match class {
        EvidenceClass::ExactSource => 0,
        EvidenceClass::ExactRender => 1,
        EvidenceClass::PlatformSemantics => 2,
        EvidenceClass::VisionMeasured => 3,
        EvidenceClass::VisionInferred => 4,
        EvidenceClass::InteractionTrace => 5,
        EvidenceClass::DeclaredContract => 6,
        EvidenceClass::Unknown => 7,
    }
}

const fn target_kind_label(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Artifact => "artifact",
        TargetKind::Canvas => "canvas",
        TargetKind::Node => "node",
        TargetKind::Relation => "relation",
    }
}

#[cfg(test)]
mod tests {
    use super::{ReportSummary, RuleOutcome};

    #[test]
    fn summary_total_includes_every_outcome() {
        let summary = ReportSummary {
            passed: 1,
            failed: 2,
            inapplicable: 3,
            cant_tell: 4,
            untested: 5,
        };
        assert_eq!(summary.total(), 15);
    }

    #[test]
    fn outcome_labels_are_stable() {
        assert_eq!(RuleOutcome::CantTell.label(), "CANT_TELL");
    }
}
