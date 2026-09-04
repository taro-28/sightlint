//! Command-line entry point for `SightLint`.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use sightlint_engine::RuleOutcome;
use sightlint_ir::ArtifactIr;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
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
    },
    /// Validate and emit canonical Artifact IR JSON.
    Normalize {
        /// Artifact IR JSON file, or `-` for standard input.
        input: PathBuf,
    },
    /// Emit the current Artifact IR JSON Schema.
    Schema,
    /// Print engine and schema versions.
    Version,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
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
        } => run_check(&input, format, deny_cant_tell),
        Command::Normalize { input } => run_normalize(&input),
        Command::Schema => match sightlint_ir::artifact_ir_schema_json() {
            Ok(schema) => write_success(&schema),
            Err(error) => fail(format!("failed to generate Artifact IR schema: {error}")),
        },
        Command::Version => {
            let output = format!(
                "SightLint {}\nArtifact IR schema {}\nReport schema {}\n",
                env!("CARGO_PKG_VERSION"),
                sightlint_engine::supported_schema_version(),
                sightlint_engine::REPORT_SCHEMA_VERSION
            );
            write_success(&output)
        }
    }
}

fn run_check(input: &Path, format: OutputFormat, deny_cant_tell: bool) -> ExitCode {
    let document = match load_document(input) {
        Ok(document) => document,
        Err(error) => return fail(error),
    };
    let report = match sightlint_engine::check(&document) {
        Ok(report) => report,
        Err(error) => return fail(error.to_string()),
    };

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

    let has_failure = report
        .results
        .iter()
        .any(|result| result.outcome == RuleOutcome::Failed);
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
    let input = read_input(path)?;
    ArtifactIr::from_json_str(&input).map_err(|error| error.to_string())
}

fn read_input(path: &Path) -> Result<String, String> {
    let mut bytes = Vec::new();
    if path.as_os_str() == "-" {
        io::stdin()
            .lock()
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read standard input: {error}"))?;
    } else {
        File::open(path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    }

    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(format!(
            "input exceeds the {MAX_INPUT_BYTES}-byte safety limit"
        ));
    }

    String::from_utf8(bytes).map_err(|error| format!("input is not valid UTF-8: {error}"))
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
    use super::{EXIT_ERROR, EXIT_FINDINGS, MAX_INPUT_BYTES};

    #[test]
    fn public_exit_code_contract_is_stable() {
        assert_eq!(EXIT_FINDINGS, 1);
        assert_eq!(EXIT_ERROR, 2);
        assert_eq!(MAX_INPUT_BYTES, 16 * 1024 * 1024);
    }
}
