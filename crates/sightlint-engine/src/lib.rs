//! Deterministic execution boundary for `SightLint`.
//!
//! This crate accepts validated, medium-neutral Artifact IR and produces evidence-linked rule
//! outcomes. Artifact acquisition, browser automation, parsing, and probabilistic perception do
//! not belong in this boundary.

#![forbid(unsafe_code)]

mod geometry;
mod report;
mod rules;
mod visual_rules;

pub use geometry::{
    QueryContext, QueryError, ResolvedRect, bottom, ensure_comparable, ordered_gap,
    overlap_extents, right, within_canvas,
};
pub use report::{
    CheckReport, Measurement, REPORT_SCHEMA_VERSION, ReportSummary, RuleKind, RuleMaturity,
    RuleOutcome, RuleResult, Target, TargetKind,
};
pub use rules::{AtomicRule, InputAspect, RuleDefinition, run_default_rules};

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use sightlint_ir::{
    ArtifactIr, VISUAL_EXTENSION_KEY, VISUAL_EXTENSION_VERSION, ValidationErrors,
    VisualExtensionErrors,
};

use visual_rules::run_visual_rules;

/// Validation failure at the trusted engine boundary.
#[derive(Debug)]
pub enum CheckError {
    /// The core Artifact IR contract is invalid.
    Core(ValidationErrors),
    /// The recognized official visual extension is invalid.
    VisualExtension(VisualExtensionErrors),
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(errors) => errors.fmt(formatter),
            Self::VisualExtension(errors) => errors.fmt(formatter),
        }
    }
}

impl Error for CheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(errors) => Some(errors),
            Self::VisualExtension(errors) => Some(errors),
        }
    }
}

impl From<ValidationErrors> for CheckError {
    fn from(errors: ValidationErrors) -> Self {
        Self::Core(errors)
    }
}

impl From<VisualExtensionErrors> for CheckError {
    fn from(errors: VisualExtensionErrors) -> Self {
        Self::VisualExtension(errors)
    }
}

/// Returns the Artifact IR schema version understood by this engine.
pub const fn supported_schema_version() -> &'static str {
    sightlint_ir::SCHEMA_VERSION
}

/// Returns the official visual-extension version understood by this engine.
pub const fn supported_visual_extension_version() -> &'static str {
    VISUAL_EXTENSION_VERSION
}

/// Validates and checks one Artifact IR document with every applicable built-in rule pack.
///
/// # Errors
///
/// Returns core or official-extension validation errors before dependent rules execute. Rules
/// therefore operate over identifiers, references, numbers, provenance, and official extension
/// payloads that satisfy their declared contracts.
pub fn check(document: &ArtifactIr) -> Result<CheckReport, CheckError> {
    document.validate()?;
    let visual_extension = document.visual_extension()?;
    let context = QueryContext::new(document);
    let mut results = run_default_rules(&context);
    let mut extension_versions = BTreeMap::new();

    if let Some(extension) = visual_extension {
        extension_versions.insert(
            VISUAL_EXTENSION_KEY.to_owned(),
            extension.extension_version.clone(),
        );
        results.extend(run_visual_rules(&context, &extension));
    }

    Ok(CheckReport::new(document, extension_versions, results))
}

#[cfg(test)]
mod tests {
    use super::{supported_schema_version, supported_visual_extension_version};

    #[test]
    fn engine_and_ir_agree_on_contract_versions() {
        assert_eq!(supported_schema_version(), "0.1.0");
        assert_eq!(supported_visual_extension_version(), "0.1.0");
    }
}
