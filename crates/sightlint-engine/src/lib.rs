//! Deterministic execution boundary for SightLint.
//!
//! Milestone M1 will add queries, rules, and evidence-linked outcomes. Perception and artifact
//! acquisition do not belong in this crate.

#![forbid(unsafe_code)]

/// Returns the Artifact IR schema version understood by this engine foundation.
pub const fn supported_schema_version() -> &'static str {
    sightlint_ir::SCHEMA_VERSION
}

#[cfg(test)]
mod tests {
    use super::supported_schema_version;

    #[test]
    fn engine_and_ir_agree_on_schema_version() {
        assert_eq!(supported_schema_version(), "0.1.0");
    }
}
