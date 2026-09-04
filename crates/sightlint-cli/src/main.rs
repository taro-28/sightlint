fn main() {
    println!(
        "SightLint {} foundation (Artifact IR schema {})",
        env!("CARGO_PKG_VERSION"),
        sightlint_engine::supported_schema_version()
    );
}
