//! Fixture-driven end-to-end tests for the public `sightlint` binary.

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_FINDINGS: i32 = 1;
const EXIT_ERROR: i32 = 2;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e2e")
        .join(name)
}

fn run(command: &mut Command, stdin: Option<&[u8]>) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(input) = stdin {
        command.stdin(Stdio::piped());
        let mut child = command.spawn().expect("failed to spawn sightlint");
        child
            .stdin
            .take()
            .expect("stdin pipe")
            .write_all(input)
            .expect("failed to write test input");
        child.wait_with_output().expect("failed to collect output")
    } else {
        command.output().expect("failed to execute sightlint")
    }
}

fn assert_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn check_json(name: &str, extra: &[&str]) -> Output {
    let mut command = binary();
    command
        .arg("check")
        .arg(fixture(name))
        .arg("--format")
        .arg("json")
        .args(extra);
    run(&mut command, None)
}

fn parse_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn result_outcome<'a>(report: &'a Value, rule_id: &str) -> Option<&'a str> {
    report["results"].as_array()?.iter().find_map(|result| {
        (result["ruleId"] == rule_id)
            .then(|| result["outcome"].as_str())
            .flatten()
    })
}

#[test]
fn clean_fixture_runs_the_complete_pipeline_in_both_report_formats() {
    let json = check_json("pass-web.json", &[]);
    assert_code(&json, EXIT_SUCCESS);
    assert!(json.stderr.is_empty());

    let report = parse_stdout(&json);
    assert_eq!(report["artifactId"], "artifact-pass-web");
    assert_eq!(report["artifactKind"], "web");
    assert_eq!(report["summary"]["passed"], 5);
    assert_eq!(report["summary"]["failed"], 0);
    assert_eq!(report["summary"]["cantTell"], 0);
    assert_eq!(
        result_outcome(&report, "visual.bounds.within-canvas"),
        Some("passed")
    );
    assert_eq!(
        result_outcome(&report, "visual.geometry.declared-non-overlap"),
        Some("passed")
    );
    assert_eq!(
        result_outcome(&report, "visual.spacing.peer-consistency"),
        Some("passed")
    );

    let mut command = binary();
    command.arg("check").arg(fixture("pass-web.json"));
    let human = run(&mut command, None);
    assert_code(&human, EXIT_SUCCESS);
    let human = String::from_utf8(human.stdout).expect("human report is UTF-8");
    assert!(human.contains("5 result(s): 5 passed, 0 failed"));
    assert!(human.contains("PASS visual.spacing.peer-consistency"));
    assert!(human.contains("evidence: e-contract, e-render"));
}

#[test]
fn mutation_fixtures_kill_each_initial_rule() {
    for (fixture_name, expected_rule) in [
        ("fail-bounds.json", "visual.bounds.within-canvas"),
        ("fail-overlap.json", "visual.geometry.declared-non-overlap"),
        ("fail-spacing.json", "visual.spacing.peer-consistency"),
    ] {
        let output = check_json(fixture_name, &[]);
        assert_code(&output, EXIT_FINDINGS);
        let report = parse_stdout(&output);
        assert_eq!(
            result_outcome(&report, expected_rule),
            Some("failed"),
            "fixture {fixture_name} did not kill {expected_rule}"
        );
        assert_eq!(report["summary"]["failed"], 1);
    }
}

#[test]
fn ambiguous_evidence_abstains_and_only_fails_under_explicit_strict_policy() {
    for name in ["cant-tell-missing-box.json", "cant-tell-cross-canvas.json"] {
        let advisory = check_json(name, &[]);
        assert_code(&advisory, EXIT_SUCCESS);
        let report = parse_stdout(&advisory);
        assert_eq!(report["summary"]["cantTell"], 1);

        let strict = check_json(name, &["--deny-cant-tell"]);
        assert_code(&strict, EXIT_FINDINGS);
        assert_eq!(advisory.stdout, strict.stdout);
    }
}

#[test]
fn invalid_and_malformed_inputs_are_rejected_with_stable_error_exit_code() {
    for (name, expected_message) in [
        ("invalid-json.json", "failed to decode Artifact IR JSON"),
        ("invalid-schema-version.json", "UnsupportedSchemaVersion"),
        ("invalid-reference.json", "InvalidReference"),
        ("invalid-cycle.json", "HierarchyCycle"),
        ("invalid-confidence.json", "InvalidConfidence"),
        ("invalid-uncertainty.json", "InvalidUncertainty"),
        ("invalid-negative-geometry.json", "NegativeDimension"),
        ("invalid-missing-confidence.json", "MissingConfidence"),
        ("invalid-empty-identifier.json", "EmptyIdentifier"),
        ("invalid-duplicate-identifier.json", "DuplicateIdentifier"),
    ] {
        let output = check_json(name, &[]);
        assert_code(&output, EXIT_ERROR);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected_message),
            "fixture {name} did not report {expected_message}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn normalization_is_canonical_idempotent_and_insertion_order_independent() {
    let mut canonical_command = binary();
    canonical_command
        .arg("normalize")
        .arg(fixture("pass-web.json"));
    let canonical = run(&mut canonical_command, None);
    assert_code(&canonical, EXIT_SUCCESS);

    let mut shuffled_command = binary();
    shuffled_command
        .arg("normalize")
        .arg(fixture("pass-web-shuffled.json"));
    let shuffled = run(&mut shuffled_command, None);
    assert_code(&shuffled, EXIT_SUCCESS);
    assert_eq!(canonical.stdout, shuffled.stdout);

    let mut stdin_command = binary();
    stdin_command.arg("normalize").arg(OsStr::new("-"));
    let second_pass = run(&mut stdin_command, Some(&canonical.stdout));
    assert_code(&second_pass, EXIT_SUCCESS);
    assert_eq!(canonical.stdout, second_pass.stdout);
}

#[test]
fn report_bytes_are_deterministic_across_ordering_stdin_and_repeated_runs() {
    let expected = check_json("pass-web.json", &[]);
    assert_code(&expected, EXIT_SUCCESS);

    let shuffled = check_json("pass-web-shuffled.json", &[]);
    assert_code(&shuffled, EXIT_SUCCESS);
    assert_eq!(expected.stdout, shuffled.stdout);

    let input = std::fs::read(fixture("pass-web.json")).expect("fixture exists");
    let mut stdin_command = binary();
    stdin_command
        .arg("check")
        .arg("-")
        .arg("--format")
        .arg("json");
    let stdin_output = run(&mut stdin_command, Some(&input));
    assert_code(&stdin_output, EXIT_SUCCESS);
    assert_eq!(expected.stdout, stdin_output.stdout);

    for iteration in 0..20 {
        let actual = check_json("pass-web.json", &[]);
        assert_code(&actual, EXIT_SUCCESS);
        assert_eq!(
            expected.stdout, actual.stdout,
            "run {iteration} was not byte-identical"
        );
    }
}

#[test]
fn the_same_core_contract_accepts_every_planned_static_artifact_kind() {
    for kind in [
        "web", "mobile", "slide", "document", "pdf", "image", "other",
    ] {
        let output = check_json(&format!("pass-{kind}.json"), &[]);
        assert_code(&output, EXIT_SUCCESS);
        let report = parse_stdout(&output);
        assert_eq!(report["artifactKind"], kind);
        assert_eq!(report["summary"]["failed"], 0);
        assert_eq!(report["summary"]["cantTell"], 0);
    }
}

#[test]
fn artifacts_without_applicable_targets_are_explicitly_inapplicable() {
    let output = check_json("inapplicable.json", &[]);
    assert_code(&output, EXIT_SUCCESS);
    let report = parse_stdout(&output);
    assert_eq!(report["summary"]["passed"], 0);
    assert_eq!(report["summary"]["inapplicable"], 3);
}

#[test]
fn schema_and_version_commands_expose_machine_and_human_contract_versions() {
    let mut schema_command = binary();
    schema_command.arg("schema");
    let schema = run(&mut schema_command, None);
    assert_code(&schema, EXIT_SUCCESS);
    let schema_json = parse_stdout(&schema);
    assert_eq!(schema_json["$id"], "urn:sightlint:schema:artifact-ir:0.1.0");

    let mut version_command = binary();
    version_command.arg("version");
    let version = run(&mut version_command, None);
    assert_code(&version, EXIT_SUCCESS);
    let version = String::from_utf8(version.stdout).expect("version output is UTF-8");
    assert!(version.contains("Artifact IR schema 0.1.0"));
    assert!(version.contains("Report schema 0.1.0"));
}

#[test]
fn safety_and_usage_failures_return_exit_two() {
    let mut invalid_utf8_command = binary();
    invalid_utf8_command.arg("check").arg("-");
    let invalid_utf8 = run(&mut invalid_utf8_command, Some(&[0xff, 0xfe]));
    assert_code(&invalid_utf8, EXIT_ERROR);
    assert!(String::from_utf8_lossy(&invalid_utf8.stderr).contains("not valid UTF-8"));

    let oversized = vec![b' '; 16 * 1024 * 1024 + 1];
    let mut oversized_command = binary();
    oversized_command.arg("check").arg("-");
    let oversized_output = run(&mut oversized_command, Some(&oversized));
    assert_code(&oversized_output, EXIT_ERROR);
    assert!(String::from_utf8_lossy(&oversized_output.stderr).contains("safety limit"));

    let mut usage_command = binary();
    usage_command.arg("check");
    let usage = run(&mut usage_command, None);
    assert_code(&usage, EXIT_ERROR);
    assert!(String::from_utf8_lossy(&usage.stderr).contains("Usage:"));
}
