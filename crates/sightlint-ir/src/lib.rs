//! Version anchor for `SightLint`'s language-neutral Artifact IR.
//!
//! The actual serialized data model is introduced in milestone M1. This crate exists in the
//! foundation milestone to establish ownership and dependency direction before implementation.

#![forbid(unsafe_code)]

/// The first planned schema family for the `SightLint` Artifact IR.
pub const SCHEMA_VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::SCHEMA_VERSION;

    #[test]
    fn schema_version_is_explicit() {
        assert_eq!(SCHEMA_VERSION, "0.1.0");
    }
}
