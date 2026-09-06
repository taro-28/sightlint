//! Deterministic execution boundary for `SightLint`.
//!
//! This crate accepts validated, medium-neutral Artifact IR and produces evidence-linked rule
//! outcomes. Artifact acquisition, browser automation, parsing, and probabilistic perception do
//! not belong in this boundary.

#![forbid(unsafe_code)]

mod geometry;
mod interaction_rules;
mod report;
mod rules;
mod visual_rules;
mod web_extension;
mod web_rules;

pub use geometry::{
    QueryContext, QueryError, ResolvedRect, bottom, ensure_comparable, ordered_gap,
    overlap_extents, right, within_canvas,
};
pub use report::{
    CheckReport, Measurement, PolicyProvenance, PolicySourceKind, REPORT_SCHEMA_VERSION,
    ReportSummary, RuleEnforcement, RuleKind, RuleMaturity, RuleOutcome, RuleResult, Target,
    TargetKind,
};
pub use rules::{AtomicRule, InputAspect, RuleDefinition, RulePolicyDefinition, run_default_rules};
pub use web_extension::WebExtensionErrors;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use sightlint_ir::{
    ArtifactIr, INTERACTION_EXTENSION_KEY, INTERACTION_EXTENSION_VERSION,
    InteractionExtensionErrors, VISUAL_EXTENSION_KEY, VISUAL_EXTENSION_VERSION, ValidationErrors,
    VisualExtensionErrors,
};

use interaction_rules::run_interaction_rules;
use visual_rules::run_visual_rules;
use web_extension::{WEB_EXTENSION_KEY, WEB_EXTENSION_VERSION, decode_web_extension};
use web_rules::run_recommended_web_rules;

/// Built-in rule profile selected for one deterministic check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CheckProfile {
    /// Run only the pre-existing base and explicitly declared rules.
    Base,
    /// Run base rules plus admitted zero-setup recommended rules.
    #[default]
    Recommended,
}

/// Deterministic options controlling rule-pack selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckOptions {
    /// Built-in profile selected for this execution.
    pub profile: CheckProfile,
}

/// Validation failure at the trusted engine boundary.
#[derive(Debug)]
pub enum CheckError {
    /// The core Artifact IR contract is invalid.
    Core(ValidationErrors),
    /// The recognized official visual extension is invalid.
    VisualExtension(VisualExtensionErrors),
    /// The recognized official Web extension is invalid.
    WebExtension(WebExtensionErrors),
    /// The recognized official interaction extension is invalid.
    InteractionExtension(InteractionExtensionErrors),
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(errors) => errors.fmt(formatter),
            Self::VisualExtension(errors) => errors.fmt(formatter),
            Self::WebExtension(errors) => errors.fmt(formatter),
            Self::InteractionExtension(errors) => errors.fmt(formatter),
        }
    }
}

impl Error for CheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(errors) => Some(errors),
            Self::VisualExtension(errors) => Some(errors),
            Self::WebExtension(errors) => Some(errors),
            Self::InteractionExtension(errors) => Some(errors),
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

impl From<WebExtensionErrors> for CheckError {
    fn from(errors: WebExtensionErrors) -> Self {
        Self::WebExtension(errors)
    }
}

impl From<InteractionExtensionErrors> for CheckError {
    fn from(errors: InteractionExtensionErrors) -> Self {
        Self::InteractionExtension(errors)
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

/// Returns the official Web-extension version understood by this engine.
pub const fn supported_web_extension_version() -> &'static str {
    WEB_EXTENSION_VERSION
}

/// Returns the official interaction-extension version understood by this engine.
pub const fn supported_interaction_extension_version() -> &'static str {
    INTERACTION_EXTENSION_VERSION
}

/// Validates and checks one Artifact IR document with every applicable built-in rule pack.
///
/// # Errors
///
/// Returns core or official-extension validation errors before dependent rules execute. Rules
/// therefore operate over identifiers, references, numbers, provenance, and official extension
/// payloads that satisfy their declared contracts.
pub fn check(document: &ArtifactIr) -> Result<CheckReport, CheckError> {
    check_with_options(document, CheckOptions::default())
}

/// Validates and checks one Artifact IR document with explicitly selected rule-pack options.
///
/// # Errors
///
/// Returns core or recognized official-extension validation errors before rules execute.
pub fn check_with_options(
    document: &ArtifactIr,
    options: CheckOptions,
) -> Result<CheckReport, CheckError> {
    document.validate()?;
    let visual_extension = document.visual_extension()?;
    let web_extension = decode_web_extension(document)?;
    let interaction_extension = document.interaction_extension()?;
    let context = QueryContext::new(document);
    let mut results = run_default_rules(&context);
    let mut extension_versions = BTreeMap::new();
    let mut profiles = vec!["sightlint:base".to_owned()];

    if let Some(extension) = visual_extension {
        extension_versions.insert(
            VISUAL_EXTENSION_KEY.to_owned(),
            extension.extension_version.clone(),
        );
        results.extend(run_visual_rules(&context, &extension));
    }

    if let Some(extension) = web_extension {
        extension_versions.insert(
            WEB_EXTENSION_KEY.to_owned(),
            extension.extension_version.clone(),
        );
        if options.profile == CheckProfile::Recommended {
            profiles.push("sightlint:recommended".to_owned());
            results.extend(run_recommended_web_rules(&context, &extension));
        }
    } else if options.profile == CheckProfile::Recommended {
        profiles.push("sightlint:recommended".to_owned());
    }

    if let Some(extension) = interaction_extension {
        extension_versions.insert(
            INTERACTION_EXTENSION_KEY.to_owned(),
            extension.extension_version.clone(),
        );
        results.extend(run_interaction_rules(&context, &extension));
    }

    Ok(CheckReport::new(
        document,
        extension_versions,
        profiles,
        results,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        supported_interaction_extension_version, supported_schema_version,
        supported_visual_extension_version, supported_web_extension_version,
    };

    #[test]
    fn engine_and_ir_agree_on_contract_versions() {
        assert_eq!(supported_schema_version(), "0.1.0");
        assert_eq!(supported_visual_extension_version(), "0.1.0");
        assert_eq!(supported_web_extension_version(), "0.4.0");
        assert_eq!(supported_interaction_extension_version(), "0.1.0");
    }
}
