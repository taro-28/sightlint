//! Command-line entry point for SightLint.
//!
//! The foundation command only exposes the current package and Artifact IR schema versions.
//! Functional commands are introduced in milestone M1.

fn main() {
    println!(
        "SightLint {} foundation (Artifact IR schema {})",
        env!("CARGO_PKG_VERSION"),
        sightlint_engine::supported_schema_version()
    );
}
