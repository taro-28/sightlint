//! Command-line entry point for `SightLint`.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use sightlint_engine::{CheckOptions, CheckProfile, CheckReport, RuleOutcome};
use sightlint_ir::ArtifactIr;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BINARY_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SOURCE_MAP_INPUT_BYTES: u64 = 1024 * 1024;
const EXIT_FINDINGS: u8 = 1;
const EXIT_ERROR: u8 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "sightlint",
    version,
    about = "Deterministic visual linting for interfaces and artifacts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate an Artifact IR document and run the built-in rules.
    Check {
        /// Artifact IR JSON file, or `-` for standard input.
        input: PathBuf,
        /// Report representation written to standard output.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Treat any `cantTell` result as a failing quality gate.
        #[arg(long)]
        deny_cant_tell: bool,
        /// Built-in policy profile; recommended is zero-setup and base disables its added rules.
        #[arg(long, value_enum, default_value_t = Profile::Recommended)]
        profile: Profile,
    },
    /// Run the trusted rules and project their report into a GitHub Actions job check.
    GithubCheck {
        /// Artifact IR JSON file, or `-` for standard input.
        input: PathBuf,
        /// Optional independently declared exact source locations.
        #[arg(long)]
        source_map: Option<PathBuf>,
        /// Repository root used to contain and validate declared source paths.
        #[arg(long, default_value = ".")]
        repository_root: PathBuf,
        /// Projection representation written to standard output.
        #[arg(long, value_enum, default_value_t = GithubOutputFormat::Json)]
        format: GithubOutputFormat,
        /// Explicitly append the stable report to the `GITHUB_STEP_SUMMARY` file.
        #[arg(long)]
        write_step_summary: bool,
        /// Treat any `cantTell` result as a failing gate without changing its outcome.
        #[arg(long)]
        deny_cant_tell: bool,
        /// Built-in policy profile used by the trusted rule kernel.
        #[arg(long, value_enum, default_value_t = Profile::Recommended)]
        profile: Profile,
    },
    /// Adapt a supported image file into canonical Artifact IR JSON.
    AdaptImage {
        /// PNG file, or `-` for binary standard input.
        input: PathBuf,
    },
    /// Inspect region and spacing candidates; advisory only, never a UX pass/fail verdict.
    InspectImage {
        /// PNG file, or `-` for binary standard input.
        input: PathBuf,
        /// Observation representation written to standard output.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Compare experimental image-segmentation policies; evaluation-only and nonblocking.
    BenchmarkImageSegmentation {
        /// PNG file, or `-` for binary standard input.
        input: PathBuf,
    },
    /// Adapt a supported image and run the built-in rules.
    CheckImage {
        /// PNG file, or `-` for binary standard input.
        input: PathBuf,
        /// Report representation written to standard output.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Treat any `cantTell` result as a failing quality gate.
        #[arg(long)]
        deny_cant_tell: bool,
        /// Built-in policy profile; recommended is zero-setup and base disables its added rules.
        #[arg(long, value_enum, default_value_t = Profile::Recommended)]
        profile: Profile,
    },
    /// Validate and emit canonical Artifact IR JSON.
    Normalize {
        /// Artifact IR JSON file, or `-` for standard input.
        input: PathBuf,
    },
    /// Emit a current machine-readable JSON Schema.
    Schema {
        /// Schema contract to emit.
        #[arg(long, value_enum, default_value = "artifact-ir")]
        kind: SchemaKind,
    },
    /// Print engine and schema versions.
    Version,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GithubOutputFormat {
    Json,
    GithubActions,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Profile {
    Recommended,
    Base,
}

impl From<Profile> for CheckProfile {
    fn from(value: Profile) -> Self {
        match value {
            Profile::Recommended => Self::Recommended,
            Profile::Base => Self::Base,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SchemaKind {
    ArtifactIr,
    Visual,
    Interaction,
    GithubSourceMap,
    GithubActionsReport,
}

fn main() -> ExitCode {
    run(Cli::parse())
}

fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Check {
            input,
            format,
            deny_cant_tell,
            profile,
        } => run_check(&input, format, deny_cant_tell, profile),
        Command::GithubCheck {
            input,
            source_map,
            repository_root,
            format,
            write_step_summary,
            deny_cant_tell,
            profile,
        } => run_github_check(
            &input,
            source_map.as_deref(),
            &repository_root,
            format,
            write_step_summary,
            deny_cant_tell,
            profile,
        ),
        Command::AdaptImage { input } => run_adapt_image(&input),
        Command::InspectImage { input, format } => run_inspect_image(&input, format),
        Command::BenchmarkImageSegmentation { input } => run_benchmark_image_segmentation(&input),
        Command::CheckImage {
            input,
            format,
            deny_cant_tell,
            profile,
        } => run_check_image(&input, format, deny_cant_tell, profile),
        Command::Normalize { input } => run_normalize(&input),
        Command::Schema { kind } => {
            let schema = match kind {
                SchemaKind::ArtifactIr => sightlint_ir::artifact_ir_schema_json(),
                SchemaKind::Visual => sightlint_ir::visual_extension_schema_json(),
                SchemaKind::Interaction => sightlint_ir::interaction_extension_schema_json(),
                SchemaKind::GithubSourceMap => {
                    sightlint_github_actions::github_source_map_schema_json()
                }
                SchemaKind::GithubActionsReport => {
                    sightlint_github_actions::github_actions_report_schema_json()
                }
            };
            match schema {
                Ok(schema) => write_success(&schema),
                Err(error) => fail(format!("failed to generate JSON Schema: {error}")),
            }
        }
        Command::Version => {
            let output = format!(
                "SightLint {}\nArtifact IR schema {}\nVisual extension {}\nWeb extension {}\nInteraction extension {}\nReport schema {}\nPNG adapter {}\n",
                env!("CARGO_PKG_VERSION"),
                sightlint_engine::supported_schema_version(),
                sightlint_engine::supported_visual_extension_version(),
                sightlint_engine::supported_web_extension_version(),
                sightlint_engine::supported_interaction_extension_version(),
                sightlint_engine::REPORT_SCHEMA_VERSION,
                env!("CARGO_PKG_VERSION")
            );
            let output = format!(
                "{output}GitHub source map {}\nGitHub Actions report {}\n",
                sightlint_github_actions::GITHUB_SOURCE_MAP_SCHEMA_VERSION,
                sightlint_github_actions::GITHUB_ACTIONS_REPORT_SCHEMA_VERSION
            );
            write_success(&output)
        }
    }
}

fn run_check(
    input: &Path,
    format: OutputFormat,
    deny_cant_tell: bool,
    profile: Profile,
) -> ExitCode {
    let document = match load_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    run_document_check(&document, format, deny_cant_tell, profile)
}

#[allow(clippy::too_many_arguments)]
fn run_github_check(
    input: &Path,
    source_map_path: Option<&Path>,
    repository_root: &Path,
    format: GithubOutputFormat,
    write_step_summary: bool,
    deny_cant_tell: bool,
    profile: Profile,
) -> ExitCode {
    let document = match load_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    let report = match sightlint_engine::check_with_options(
        &document,
        CheckOptions {
            profile: profile.into(),
        },
    ) {
        Ok(report) => report,
        Err(error) => return fail(error.to_string()),
    };

    let source_map_text = match source_map_path {
        Some(path) => match read_utf8_input(path, MAX_SOURCE_MAP_INPUT_BYTES, "source map") {
            Ok(input) => Some(input),
            Err(error) => return fail(error),
        },
        None => None,
    };
    let source_map = match source_map_text.as_deref() {
        Some(input) => match sightlint_github_actions::validate_source_map_json(
            input,
            &report,
            repository_root,
        ) {
            Ok(source_map) => Some(source_map),
            Err(error) => return fail(error.to_string()),
        },
        None => None,
    };
    let projection = match sightlint_github_actions::project_report(
        &report,
        source_map.as_ref(),
        sightlint_github_actions::ProjectionOptions { deny_cant_tell },
    ) {
        Ok(projection) => projection,
        Err(error) => return fail(error.to_string()),
    };
    let output = match format {
        GithubOutputFormat::Json => {
            match sightlint_github_actions::to_canonical_json(&projection) {
                Ok(output) => output,
                Err(error) => {
                    return fail(format!(
                        "failed to serialize GitHub Actions report: {error}"
                    ));
                }
            }
        }
        GithubOutputFormat::GithubActions => {
            sightlint_github_actions::to_workflow_commands(&projection)
        }
    };

    if write_step_summary {
        let summary_path = match std::env::var_os("GITHUB_STEP_SUMMARY") {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => {
                return fail(
                    "--write-step-summary requires a nonempty GITHUB_STEP_SUMMARY environment variable",
                );
            }
        };
        let summary = sightlint_github_actions::to_step_summary(&projection);
        if let Err(error) = sightlint_github_actions::append_step_summary(&summary_path, &summary) {
            return fail(error.to_string());
        }
    }

    if let Err(error) = write_stdout(output.as_bytes()) {
        return fail(format!("failed to write standard output: {error}"));
    }
    ExitCode::from(sightlint_github_actions::gate_exit_code(&projection))
}

fn run_adapt_image(input: &Path) -> ExitCode {
    let document = match load_png_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    match document.to_canonical_json() {
        Ok(output) => write_success(&output),
        Err(error) => fail(format!("failed to serialize adapted Artifact IR: {error}")),
    }
}

fn run_inspect_image(input: &Path, format: OutputFormat) -> ExitCode {
    let bytes = match read_binary_input(input) {
        Ok(bytes) => bytes,
        Err(error) => return fail(error),
    };
    let inspection = match sightlint_adapter_png::inspection::inspect_png(&bytes) {
        Ok(inspection) => inspection,
        Err(error) => return fail(error.to_string()),
    };
    match format {
        OutputFormat::Human => write_success(&inspection.to_human()),
        OutputFormat::Json => match inspection.to_canonical_json() {
            Ok(output) => write_success(&output),
            Err(error) => fail(format!("failed to serialize image inspection: {error}")),
        },
    }
}

fn run_benchmark_image_segmentation(input: &Path) -> ExitCode {
    let bytes = match read_binary_input(input) {
        Ok(bytes) => bytes,
        Err(error) => return fail(error),
    };
    let benchmark = match sightlint_adapter_png::segmentation::benchmark_png_segmentation(&bytes) {
        Ok(benchmark) => benchmark,
        Err(error) => return fail(error.to_string()),
    };
    match benchmark.to_canonical_json() {
        Ok(output) => write_success(&output),
        Err(error) => fail(format!(
            "failed to serialize image segmentation benchmark: {error}"
        )),
    }
}

fn run_check_image(
    input: &Path,
    format: OutputFormat,
    deny_cant_tell: bool,
    profile: Profile,
) -> ExitCode {
    let document = match load_png_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    run_document_check(&document, format, deny_cant_tell, profile)
}

fn run_document_check(
    document: &ArtifactIr,
    format: OutputFormat,
    deny_cant_tell: bool,
    profile: Profile,
) -> ExitCode {
    let report = match sightlint_engine::check_with_options(
        document,
        CheckOptions {
            profile: profile.into(),
        },
    ) {
        Ok(report) => report,
        Err(error) => return fail(error.to_string()),
    };
    write_report(&report, format, deny_cant_tell)
}

fn write_report(report: &CheckReport, format: OutputFormat, deny_cant_tell: bool) -> ExitCode {
    let output = match format {
        OutputFormat::Human => report.to_human(),
        OutputFormat::Json => match report.to_canonical_json() {
            Ok(output) => output,
            Err(error) => return fail(format!("failed to serialize report: {error}")),
        },
    };

    if let Err(error) = write_stdout(output.as_bytes()) {
        return fail(format!("failed to write standard output: {error}"));
    }

    let has_failure = report.has_blocking_failure();
    let has_denied_unknown = deny_cant_tell
        && report
            .results
            .iter()
            .any(|result| result.outcome == RuleOutcome::CantTell);

    if has_failure || has_denied_unknown {
        ExitCode::from(EXIT_FINDINGS)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_normalize(input: &Path) -> ExitCode {
    let document = match load_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    match document.to_canonical_json() {
        Ok(output) => write_success(&output),
        Err(error) => fail(format!(
            "failed to serialize canonical Artifact IR: {error}"
        )),
    }
}

fn load_document(path: &Path) -> Result<ArtifactIr, String> {
    let input = read_text_input(path)?;
    ArtifactIr::from_json_str(&input).map_err(|error| error.to_string())
}

fn load_png_document(path: &Path) -> Result<ArtifactIr, String> {
    let input = read_binary_input(path)?;
    let source_name = if path.as_os_str() == "-" {
        None
    } else {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    sightlint_adapter_png::adapt_png(&input, source_name).map_err(|error| error.to_string())
}

fn read_text_input(path: &Path) -> Result<String, String> {
    read_utf8_input(path, MAX_INPUT_BYTES, "input")
}

fn read_utf8_input(path: &Path, limit: u64, label: &str) -> Result<String, String> {
    let bytes = read_input_bytes(path, limit)?;
    String::from_utf8(bytes).map_err(|error| format!("{label} is not valid UTF-8: {error}"))
}

fn read_binary_input(path: &Path) -> Result<Vec<u8>, String> {
    read_input_bytes(path, MAX_BINARY_INPUT_BYTES)
}

fn read_input_bytes(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    if path.as_os_str() == "-" {
        io::stdin()
            .lock()
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read standard input: {error}"))?;
    } else {
        File::open(path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    }

    if bytes.len() as u64 > limit {
        return Err(format!("input exceeds the {limit}-byte safety limit"));
    }
    Ok(bytes)
}

fn write_success(output: &str) -> ExitCode {
    match write_stdout(output.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(format!("failed to write standard output: {error}")),
    }
}

fn write_stdout(bytes: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()
}

fn fail(message: impl AsRef<str>) -> ExitCode {
    let _ = writeln!(io::stderr().lock(), "sightlint: {}", message.as_ref());
    ExitCode::from(EXIT_ERROR)
}

#[cfg(test)]
mod tests {
    use super::{
        EXIT_ERROR, EXIT_FINDINGS, MAX_BINARY_INPUT_BYTES, MAX_INPUT_BYTES,
        MAX_SOURCE_MAP_INPUT_BYTES,
    };

    #[test]
    fn public_exit_code_and_input_limit_contract_is_stable() {
        assert_eq!(EXIT_FINDINGS, 1);
        assert_eq!(EXIT_ERROR, 2);
        assert_eq!(MAX_INPUT_BYTES, 16 * 1024 * 1024);
        assert_eq!(MAX_BINARY_INPUT_BYTES, 64 * 1024 * 1024);
        assert_eq!(MAX_SOURCE_MAP_INPUT_BYTES, 1024 * 1024);
    }
}
