//! Deterministic execution boundary for `SightLint`.
//!
//! This crate accepts validated, medium-neutral Artifact IR and produces evidence-linked rule
//! outcomes. Artifact acquisition, browser automation, parsing, and probabilistic perception do
//! not belong in this boundary.

#![forbid(unsafe_code)]

mod geometry;
mod report;
mod rules;

pub use geometry::{
    QueryContext, QueryError, ResolvedRect, bottom, ensure_comparable, ordered_gap,
    overlap_extents, right, within_canvas,
};
pub use report::{
    CheckReport, Measurement, REPORT_SCHEMA_VERSION, ReportSummary, RuleKind, RuleMaturity,
    RuleOutcome, RuleResult, Target, TargetKind,
};
pub use rules::{AtomicRule, InputAspect, RuleDefinition, run_default_rules};

use sightlint_ir::{ArtifactIr, ValidationErrors};

/// Returns the Artifact IR schema version understood by this engine.
pub const fn supported_schema_version() -> &'static str {
    sightlint_ir::SCHEMA_VERSION
}

/// Validates and checks one Artifact IR document with the built-in rule pack.
///
/// # Errors
///
/// Returns semantic validation errors before any rule executes. Rules therefore operate over a
/// document whose identifiers, references, numbers, and provenance satisfy the core contract.
pub fn check(document: &ArtifactIr) -> Result<CheckReport, ValidationErrors> {
    document.validate()?;
    let context = QueryContext::new(document);
    Ok(CheckReport::new(document, run_default_rules(&context)))
}

#[cfg(test)]
mod tests {
    use super::supported_schema_version;

    #[test]
    fn engine_and_ir_agree_on_schema_version() {
        assert_eq!(supported_schema_version(), "0.1.0");
    }
}
