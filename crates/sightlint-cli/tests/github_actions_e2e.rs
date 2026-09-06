//! Public-binary conformance tests for the deterministic GitHub Actions projection.

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

const EXIT_SUCCESS: i32 = 0;
const EXIT_FINDINGS: i32 = 1;
const EXIT_ERROR: i32 = 2;

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sightlint-github-actions-{label}-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn repo_path(relative: &str) -> PathBuf {
    repository_root().join(relative)
}

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn run(command: &mut Command, stdin: Option<&[u8]>) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(input) = stdin {
        command.stdin(Stdio::piped());
        let mut child = command.spawn().expect("spawn sightlint");
        child
            .stdin
            .take()
            .expect("stdin pipe")
            .write_all(input)
            .expect("write stdin");
        child.wait_with_output().expect("collect sightlint output")
    } else {
        command.output().expect("execute sightlint")
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

fn projection_command(input: impl AsRef<OsStr>) -> Command {
    let mut command = binary();
    command.arg("github-check").arg(input).args([
        "--repository-root",
        repository_root().to_str().expect("UTF-8 repository path"),
        "--format",
        "json",
    ]);
    command
}

fn parse_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON: {error}\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize JSON"),
    )
    .expect("write JSON test input");
}

fn dashboard_source_map(path: &str) -> Value {
    json!({
        "sourceMapSchemaVersion": "0.1.0",
        "artifactId": "web-dashboard-peer-spacing-mutant",
        "provenance": {
            "authoringBasis": "declaredExactSource",
            "implementationOutputUsedAsOracle": false,
            "externalProcessing": false
        },
        "entries": [{
            "finding": {
                "ruleId": "visual.spacing.peer-consistency",
                "ruleVersion": "0.1.0",
                "target": {
                    "kind": "relation",
                    "id": "relation-metrics",
                    "aspect": "horizontal:renderBox"
                }
            },
            "location": {
                "attribution": "declaredExactSourceLine",
                "path": path,
                "startLine": 1,
                "endLine": 1,
                "anchorLine": 1,
                "anchorText": "stable source anchor"
            }
        }]
    })
}

#[test]
fn exact_source_projection_is_complete_byte_stable_and_supports_stdin() {
    let input = repo_path("evaluation/web/inputs/dashboard-peer-spacing-mutant.json");
    let source_map =
        repo_path("evaluation/github-actions/source-maps/dashboard-peer-spacing-mutant.json");
    let mut command = projection_command(&input);
    command.arg("--source-map").arg(&source_map);
    let expected = run(&mut command, None);
    assert_code(&expected, EXIT_FINDINGS);
    assert!(expected.stderr.is_empty());

    let projection = parse_stdout(&expected);
    assert_eq!(projection["githubActionsReportSchemaVersion"], "0.1.0");
    assert_eq!(
        projection["checkReport"]["artifactId"],
        "web-dashboard-peer-spacing-mutant"
    );
    assert_eq!(projection["checkReport"]["summary"]["failed"], 1);
    assert_eq!(projection["projectionSummary"]["blockingFailures"], 1);
    assert_eq!(projection["projectionSummary"]["gateExitCode"], 1);
    assert_eq!(
        projection["projectedResults"].as_array().map(Vec::len),
        Some(1)
    );
    let projected = &projection["projectedResults"][0];
    assert_eq!(projected["outcome"], "failed");
    assert_eq!(projected["enforcement"], "blocking");
    assert_eq!(projected["annotation"]["status"], "emitted");
    assert_eq!(projected["annotation"]["annotation"]["level"], "error");
    assert_eq!(
        projected["annotation"]["annotation"]["path"],
        "evaluation/web/fixture-app/styles.css"
    );

    let input_bytes = std::fs::read(&input).expect("read Artifact IR fixture");
    let mut stdin_command = projection_command("-");
    stdin_command.arg("--source-map").arg(&source_map);
    let stdin_output = run(&mut stdin_command, Some(&input_bytes));
    assert_code(&stdin_output, EXIT_FINDINGS);
    assert_eq!(stdin_output.stdout, expected.stdout);

    for iteration in 0..5 {
        let mut repeated_command = projection_command(&input);
        repeated_command.arg("--source-map").arg(&source_map);
        let repeated = run(&mut repeated_command, None);
        assert_code(&repeated, EXIT_FINDINGS);
        assert_eq!(
            repeated.stdout, expected.stdout,
            "projection run {iteration} was not byte-identical"
        );
    }
}

#[test]
fn workflow_commands_keep_kernel_outcome_and_enforcement_mapping() {
    let dashboard = repo_path("evaluation/web/inputs/dashboard-peer-spacing-mutant.json");
    let dashboard_map =
        repo_path("evaluation/github-actions/source-maps/dashboard-peer-spacing-mutant.json");
    let mut error_command = binary();
    error_command
        .arg("github-check")
        .arg(dashboard)
        .arg("--source-map")
        .arg(dashboard_map)
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--format", "github-actions"]);
    let error = run(&mut error_command, None);
    assert_code(&error, EXIT_FINDINGS);
    let error_text = String::from_utf8(error.stdout).expect("workflow command UTF-8");
    assert!(error_text.starts_with("::error file="));
    assert!(error_text.contains("outcome=failed; enforcement=blocking"));

    let mut warning_command = binary();
    warning_command
        .arg("github-check")
        .arg(repo_path("fixtures/e2e/m6-fail-async-feedback.json"))
        .arg("--source-map")
        .arg(repo_path(
            "evaluation/github-actions/source-maps/settings-missing-pending-mutant.json",
        ))
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--profile", "base", "--format", "github-actions"]);
    let warning = run(&mut warning_command, None);
    assert_code(&warning, EXIT_SUCCESS);
    let warning_text = String::from_utf8(warning.stdout).expect("workflow command UTF-8");
    assert!(warning_text.starts_with("::warning file="));
    assert!(warning_text.contains("outcome=failed; enforcement=advisory"));

    let cant_tell_input = repo_path("fixtures/e2e/m6-cant-tell-conflict.json");
    let mut default_command = projection_command(&cant_tell_input);
    default_command.args(["--profile", "base"]);
    let default_output = run(&mut default_command, None);
    assert_code(&default_output, EXIT_SUCCESS);
    let default_projection = parse_stdout(&default_output);
    assert_eq!(default_projection["projectionSummary"]["cantTell"], 2);
    for result in default_projection["projectedResults"]
        .as_array()
        .expect("projected results")
    {
        assert_eq!(result["outcome"], "cantTell");
        assert_eq!(result["annotation"]["status"], "sourceUnavailable");
        assert_eq!(result["annotation"]["reason"], "sourceMapNotProvided");
    }

    let mut strict_command = projection_command(&cant_tell_input);
    strict_command.args(["--profile", "base", "--deny-cant-tell"]);
    let strict_output = run(&mut strict_command, None);
    assert_code(&strict_output, EXIT_FINDINGS);
    let strict_projection = parse_stdout(&strict_output);
    assert_eq!(strict_projection["projectionSummary"]["gateExitCode"], 1);
    assert_eq!(
        strict_projection["checkReport"],
        default_projection["checkReport"]
    );
    assert_eq!(
        strict_projection["projectedResults"],
        default_projection["projectedResults"]
    );

    let mut untested_command = projection_command(repo_path("fixtures/e2e/m6-untested.json"));
    untested_command.args(["--profile", "base"]);
    let untested = run(&mut untested_command, None);
    assert_code(&untested, EXIT_SUCCESS);
    let untested = parse_stdout(&untested);
    assert_eq!(untested["projectionSummary"]["untested"], 2);
    assert!(
        untested["projectedResults"]
            .as_array()
            .expect("projected results")
            .iter()
            .all(|result| result["outcome"] == "untested")
    );

    let mut clean_command = projection_command(repo_path(
        "evaluation/web/inputs/dashboard-peer-spacing-clean.json",
    ));
    let clean = run(&mut clean_command, None);
    assert_code(&clean, EXIT_SUCCESS);
    assert_eq!(
        parse_stdout(&clean)["projectedResults"],
        json!([]),
        "passed and inapplicable results must not be projected as findings"
    );
}

#[test]
fn exact_source_cant_tell_stays_a_notice_even_under_strict_gate_policy() {
    let temp = TestDirectory::new("cant-tell-notice");
    std::fs::write(temp.path().join("source.js"), "stable source anchor\n")
        .expect("write source fixture");
    let entries = ["interaction.async-feedback", "interaction.failure-recovery"]
        .into_iter()
        .map(|rule_id| {
            json!({
                "finding": {
                    "ruleId": rule_id,
                    "ruleVersion": "0.1.0",
                    "target": {
                        "kind": "artifact",
                        "id": "artifact-m6-cant-tell-conflict",
                        "aspect": "interaction.action:save-settings"
                    }
                },
                "location": {
                    "attribution": "declaredExactSourceLine",
                    "path": "source.js",
                    "startLine": 1,
                    "endLine": 1,
                    "anchorLine": 1,
                    "anchorText": "stable source anchor"
                }
            })
        })
        .collect::<Vec<_>>();
    let source_map = json!({
        "sourceMapSchemaVersion": "0.1.0",
        "artifactId": "artifact-m6-cant-tell-conflict",
        "provenance": {
            "authoringBasis": "declaredExactSource",
            "implementationOutputUsedAsOracle": false,
            "externalProcessing": false
        },
        "entries": entries
    });
    let source_map_path = temp.path().join("source-map.json");
    write_json(&source_map_path, &source_map);

    let mut command = binary();
    command
        .arg("github-check")
        .arg(repo_path("fixtures/e2e/m6-cant-tell-conflict.json"))
        .arg("--source-map")
        .arg(source_map_path)
        .arg("--repository-root")
        .arg(temp.path())
        .args([
            "--profile",
            "base",
            "--format",
            "github-actions",
            "--deny-cant-tell",
        ]);
    let output = run(&mut command, None);
    assert_code(&output, EXIT_FINDINGS);
    let workflow = String::from_utf8(output.stdout).expect("workflow commands UTF-8");
    assert_eq!(workflow.lines().count(), 2);
    assert!(
        workflow
            .lines()
            .all(|line| line.starts_with("::notice file=source.js")
                && line.contains("outcome=cantTell; enforcement=advisory"))
    );

    let mut partial_map = source_map;
    partial_map["entries"] = json!([partial_map["entries"][0]]);
    let partial_map_path = temp.path().join("partial-source-map.json");
    write_json(&partial_map_path, &partial_map);
    let mut partial_command = binary();
    partial_command
        .arg("github-check")
        .arg(repo_path("fixtures/e2e/m6-cant-tell-conflict.json"))
        .arg("--source-map")
        .arg(partial_map_path)
        .arg("--repository-root")
        .arg(temp.path())
        .args(["--profile", "base", "--format", "json"]);
    let partial = run(&mut partial_command, None);
    assert_code(&partial, EXIT_SUCCESS);
    let partial = parse_stdout(&partial);
    assert_eq!(partial["projectionSummary"]["annotationsEmitted"], 1);
    assert_eq!(partial["projectionSummary"]["sourceUnavailable"], 1);
    assert!(
        partial["projectedResults"]
            .as_array()
            .expect("projected results")
            .iter()
            .any(|result| {
                result["annotation"]["status"] == "sourceUnavailable"
                    && result["annotation"]["reason"] == "sourceLocationNotDeclared"
            })
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn source_map_validation_fails_closed_before_stdout() {
    let temp = TestDirectory::new("invalid-maps");
    std::fs::write(temp.path().join("source.css"), "stable source anchor\n")
        .expect("write source fixture");
    let input = repo_path("evaluation/web/inputs/dashboard-peer-spacing-mutant.json");
    let base = dashboard_source_map("source.css");

    let mut variants = Vec::new();
    let mut unknown = base.clone();
    unknown["unknownField"] = json!(true);
    variants.push(("unknown", unknown, "unknown field"));

    let mut version = base.clone();
    version["sourceMapSchemaVersion"] = json!("9.9.9");
    variants.push(("version", version, "unknown variant"));

    let mut artifact = base.clone();
    artifact["artifactId"] = json!("other-artifact");
    variants.push(("artifact", artifact, "does not match report artifactId"));

    let mut oracle = base.clone();
    oracle["provenance"]["implementationOutputUsedAsOracle"] = json!(true);
    variants.push((
        "oracle",
        oracle,
        "must set implementationOutputUsedAsOracle",
    ));

    let mut external = base.clone();
    external["provenance"]["externalProcessing"] = json!(true);
    variants.push(("external", external, "must set externalProcessing"));

    let mut empty = base.clone();
    empty["entries"] = json!([]);
    variants.push(("empty", empty, "between 1 and 512 entries"));

    let mut duplicate = base.clone();
    duplicate["entries"] = json!([base["entries"][0], base["entries"][0]]);
    variants.push(("duplicate", duplicate, "duplicate finding identity"));

    let mut dangling = base.clone();
    dangling["entries"][0]["finding"]["target"]["id"] = json!("not-a-report-target");
    variants.push(("dangling", dangling, "must match exactly one report result"));

    let mut traversal = base.clone();
    traversal["entries"][0]["location"]["path"] = json!("../source.css");
    variants.push(("traversal", traversal, "must not contain traversal"));

    let mut stale = base.clone();
    stale["entries"][0]["location"]["anchorText"] = json!("stale text");
    variants.push(("stale", stale, "is stale"));

    let mut range = base.clone();
    range["entries"][0]["location"]["startLine"] = json!(0);
    variants.push(("range", range, "must be one-based"));

    let large_source = temp.path().join("large.css");
    let large_file = std::fs::File::create(&large_source).expect("create large source fixture");
    large_file
        .set_len(16 * 1024 * 1024 + 1)
        .expect("size large source fixture");
    drop(large_file);
    let mut oversized_source = base.clone();
    oversized_source["entries"][0]["location"]["path"] = json!("large.css");
    variants.push((
        "oversized-source",
        oversized_source,
        "16777216-byte safety limit",
    ));

    std::fs::write(temp.path().join("invalid-utf8.css"), [0xff])
        .expect("write invalid UTF-8 source fixture");
    let mut invalid_utf8_source = base.clone();
    invalid_utf8_source["entries"][0]["location"]["path"] = json!("invalid-utf8.css");
    variants.push((
        "invalid-utf8-source",
        invalid_utf8_source,
        "is not valid UTF-8",
    ));

    let mut unsorted = base.clone();
    let mut earlier = base["entries"][0].clone();
    earlier["finding"] = json!({
        "ruleId": "visual.bounds.within-canvas",
        "ruleVersion": "0.1.0",
        "target": {"kind": "node", "id": "metric-activation", "aspect": "renderBox"}
    });
    unsorted["entries"] = json!([base["entries"][0], earlier]);
    variants.push(("unsorted", unsorted, "must be sorted"));

    for (label, value, expected_error) in variants {
        let path = temp.path().join(format!("{label}.json"));
        write_json(&path, &value);
        let mut command = binary();
        command
            .arg("github-check")
            .arg(&input)
            .arg("--source-map")
            .arg(path)
            .arg("--repository-root")
            .arg(temp.path())
            .args(["--format", "github-actions"]);
        let output = run(&mut command, None);
        assert_code(&output, EXIT_ERROR);
        assert!(
            output.stdout.is_empty(),
            "{label} emitted partial annotations"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected_error),
            "{label} did not report {expected_error:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let oversized_map = temp.path().join("oversized-map.json");
    std::fs::write(&oversized_map, vec![b' '; 1024 * 1024 + 1])
        .expect("write oversized source map");
    let mut command = binary();
    command
        .arg("github-check")
        .arg(input)
        .arg("--source-map")
        .arg(oversized_map)
        .arg("--repository-root")
        .arg(temp.path())
        .args(["--format", "github-actions"]);
    let output = run(&mut command, None);
    assert_code(&output, EXIT_ERROR);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1048576-byte safety limit"));
}

#[cfg(unix)]
#[test]
fn source_map_rejects_symlink_escape_before_stdout() {
    use std::os::unix::fs::symlink;

    let root = TestDirectory::new("symlink-root");
    let outside = TestDirectory::new("symlink-outside");
    let outside_source = outside.path().join("outside.css");
    std::fs::write(&outside_source, "stable source anchor\n").expect("write outside source");
    symlink(&outside_source, root.path().join("source.css")).expect("create source symlink");
    let source_map_path = root.path().join("source-map.json");
    write_json(&source_map_path, &dashboard_source_map("source.css"));

    let mut command = binary();
    command
        .arg("github-check")
        .arg(repo_path(
            "evaluation/web/inputs/dashboard-peer-spacing-mutant.json",
        ))
        .arg("--source-map")
        .arg(source_map_path)
        .arg("--repository-root")
        .arg(root.path())
        .args(["--format", "github-actions"]);
    let output = run(&mut command, None);
    assert_code(&output, EXIT_ERROR);
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("resolves outside"));

    let summary_target = outside.path().join("summary.md");
    std::fs::write(&summary_target, []).expect("create summary target");
    let summary_link = root.path().join("summary-link.md");
    symlink(summary_target, &summary_link).expect("create summary symlink");
    let mut summary_command = binary();
    summary_command
        .arg("github-check")
        .arg(repo_path(
            "evaluation/web/inputs/dashboard-peer-spacing-clean.json",
        ))
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--format", "github-actions", "--write-step-summary"])
        .env("GITHUB_STEP_SUMMARY", summary_link);
    let summary_output = run(&mut summary_command, None);
    assert_code(&summary_output, EXIT_ERROR);
    assert!(summary_output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&summary_output.stderr).contains("not a symlink"));
}

#[test]
fn workflow_values_are_escaped_and_summary_writes_are_explicit_and_bounded() {
    let temp = TestDirectory::new("escaping");
    std::fs::write(
        temp.path().join("source,fixture.css"),
        "stable source anchor\n",
    )
    .expect("write source fixture");
    let map_path = temp.path().join("source-map.json");
    write_json(&map_path, &dashboard_source_map("source,fixture.css"));

    let mut artifact: Value = serde_json::from_slice(
        &std::fs::read(repo_path(
            "evaluation/web/inputs/dashboard-peer-spacing-mutant.json",
        ))
        .expect("read Artifact IR"),
    )
    .expect("parse Artifact IR");
    replace_string(
        &mut artifact,
        "e-contract",
        "e-percent%-newline\n::error injected",
    );
    let artifact_path = temp.path().join("artifact.json");
    write_json(&artifact_path, &artifact);

    let mut command = binary();
    command
        .arg("github-check")
        .arg(&artifact_path)
        .arg("--source-map")
        .arg(&map_path)
        .arg("--repository-root")
        .arg(temp.path())
        .args(["--format", "github-actions"]);
    let output = run(&mut command, None);
    assert_code(&output, EXIT_FINDINGS);
    let workflow = String::from_utf8(output.stdout).expect("workflow output UTF-8");
    assert_eq!(
        workflow.lines().count(),
        1,
        "newline command injection escaped"
    );
    assert!(workflow.contains("file=source%2Cfixture.css"));
    assert!(workflow.contains("title=SightLint%3A"));
    assert!(workflow.contains("e-percent%25-newline%0A::error injected"));

    let clean = repo_path("evaluation/web/inputs/dashboard-peer-spacing-clean.json");
    let summary_path = temp.path().join("summary.md");
    std::fs::write(&summary_path, []).expect("create runner summary file");
    let mut summary_command = binary();
    summary_command
        .arg("github-check")
        .arg(&clean)
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--format", "github-actions", "--write-step-summary"])
        .env("GITHUB_STEP_SUMMARY", &summary_path);
    let summary_output = run(&mut summary_command, None);
    assert_code(&summary_output, EXIT_SUCCESS);
    assert!(summary_output.stdout.is_empty());
    let summary = std::fs::read_to_string(&summary_path).expect("summary written");
    assert!(summary.contains("## SightLint GitHub Actions report"));
    assert!(summary.contains("No failed, cantTell, or untested results."));
    assert!(!summary.contains("stable source anchor"));

    let mut implicit_command = binary();
    implicit_command
        .arg("github-check")
        .arg(&clean)
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--format", "github-actions", "--write-step-summary"])
        .env_remove("GITHUB_STEP_SUMMARY");
    let implicit = run(&mut implicit_command, None);
    assert_code(&implicit, EXIT_ERROR);
    assert!(implicit.stdout.is_empty());
    assert!(String::from_utf8_lossy(&implicit.stderr).contains("requires a nonempty"));

    let full_summary = temp.path().join("full-summary.md");
    std::fs::write(&full_summary, vec![b'x'; 1024 * 1024]).expect("write full summary");
    let mut bounded_command = binary();
    bounded_command
        .arg("github-check")
        .arg(clean)
        .arg("--repository-root")
        .arg(repository_root())
        .args(["--format", "github-actions", "--write-step-summary"])
        .env("GITHUB_STEP_SUMMARY", &full_summary);
    let bounded = run(&mut bounded_command, None);
    assert_code(&bounded, EXIT_ERROR);
    assert!(bounded.stdout.is_empty());
    assert!(String::from_utf8_lossy(&bounded.stderr).contains("1048576-byte safety limit"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn public_binary_caps_annotations_without_hiding_findings() {
    let temp = TestDirectory::new("annotation-cap");
    std::fs::write(temp.path().join("source.css"), "stable source anchor\n")
        .expect("write source fixture");

    let mut artifact: Value = serde_json::from_slice(
        &std::fs::read(repo_path("fixtures/e2e/fail-bounds.json")).expect("read bounds fixture"),
    )
    .expect("parse bounds fixture");
    artifact["artifact"]["id"] = json!("artifact-annotation-cap");
    artifact["artifact"]["sourceName"] = json!("annotation-cap.json");
    let template = artifact["nodes"][0].clone();
    let nodes = (0..51)
        .map(|index| {
            let mut node = template.clone();
            node["id"] = json!(format!("node-outside-{index:03}"));
            node
        })
        .collect::<Vec<_>>();
    artifact["nodes"] = Value::Array(nodes);
    let artifact_path = temp.path().join("artifact.json");
    write_json(&artifact_path, &artifact);

    let entries = (0..51)
        .map(|index| {
            json!({
                "finding": {
                    "ruleId": "visual.bounds.within-canvas",
                    "ruleVersion": "0.1.0",
                    "target": {
                        "kind": "node",
                        "id": format!("node-outside-{index:03}"),
                        "aspect": "renderBox"
                    }
                },
                "location": {
                    "attribution": "declaredExactSourceLine",
                    "path": "source.css",
                    "startLine": 1,
                    "endLine": 1,
                    "anchorLine": 1,
                    "anchorText": "stable source anchor"
                }
            })
        })
        .collect::<Vec<_>>();
    let source_map = json!({
        "sourceMapSchemaVersion": "0.1.0",
        "artifactId": "artifact-annotation-cap",
        "provenance": {
            "authoringBasis": "declaredExactSource",
            "implementationOutputUsedAsOracle": false,
            "externalProcessing": false
        },
        "entries": entries
    });
    let source_map_path = temp.path().join("source-map.json");
    write_json(&source_map_path, &source_map);

    let mut json_command = binary();
    json_command
        .arg("github-check")
        .arg(&artifact_path)
        .arg("--source-map")
        .arg(&source_map_path)
        .arg("--repository-root")
        .arg(temp.path())
        .args(["--profile", "base", "--format", "json"]);
    let json_output = run(&mut json_command, None);
    assert_code(&json_output, EXIT_FINDINGS);
    let projection = parse_stdout(&json_output);
    assert_eq!(projection["checkReport"]["summary"]["failed"], 51);
    assert_eq!(projection["projectionSummary"]["actionableResults"], 51);
    assert_eq!(projection["projectionSummary"]["annotationsEmitted"], 50);
    assert_eq!(projection["projectionSummary"]["annotationsOmitted"], 1);
    assert_eq!(
        projection["projectedResults"].as_array().map(Vec::len),
        Some(51)
    );
    assert_eq!(
        projection["projectedResults"]
            .as_array()
            .expect("projected results")
            .iter()
            .filter(|result| result["annotation"]["status"] == "omitted")
            .count(),
        1
    );

    let mut workflow_command = binary();
    workflow_command
        .arg("github-check")
        .arg(&artifact_path)
        .arg("--source-map")
        .arg(&source_map_path)
        .arg("--repository-root")
        .arg(temp.path())
        .args(["--profile", "base", "--format", "github-actions"]);
    let workflow = run(&mut workflow_command, None);
    assert_code(&workflow, EXIT_FINDINGS);
    assert_eq!(
        String::from_utf8(workflow.stdout)
            .expect("workflow commands UTF-8")
            .lines()
            .count(),
        50
    );
}

fn replace_string(value: &mut Value, old: &str, new: &str) {
    match value {
        Value::String(text) if text == old => new.clone_into(text),
        Value::Array(values) => {
            for value in values {
                replace_string(value, old, new);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                replace_string(value, old, new);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}
