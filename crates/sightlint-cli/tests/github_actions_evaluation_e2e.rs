//! Product-evaluation E2E for the GitHub Actions projection.
//!
//! Rule truth, exact-source truth, and expected projection disposition are separate reviewed
//! inputs. This test invokes only the public binary and never rewrites an oracle from its output.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[derive(Debug, Default, PartialEq, Eq)]
struct Metrics {
    executed_cases: u64,
    reviewed_failures: u64,
    exact_source_annotations: u64,
    preserved_abstentions: u64,
    summary_only_abstentions: u64,
    killed_mutations: u64,
    clean_cases: u64,
    hard_negatives: u64,
    unexpected_failures: u64,
    false_positive_failures: u64,
    hard_negative_failures: u64,
    unexpected_annotations: u64,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn load_json(path: &Path) -> Value {
    serde_json::from_slice(
        &std::fs::read(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn cases_by_id(document: &Value) -> BTreeMap<&str, &Value> {
    document["cases"]
        .as_array()
        .expect("cases array")
        .iter()
        .map(|case| (case["caseId"].as_str().expect("case ID"), case))
        .collect()
}

fn run_case(case: &Value, format: &str) -> Output {
    let root = repository_root();
    let mut command = Command::new(env!("CARGO_BIN_EXE_sightlint"));
    command
        .arg("github-check")
        .arg(root.join(case["input"].as_str().expect("case input")))
        .arg("--repository-root")
        .arg(&root)
        .arg("--profile")
        .arg(case["profile"].as_str().expect("case profile"))
        .arg("--format")
        .arg(format);
    if let Some(source_map) = case["sourceMap"].as_str() {
        command.arg("--source-map").arg(root.join(source_map));
    }
    command.output().expect("run public sightlint binary")
}

fn assert_exit(output: &Output, expected: i32, case_id: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "case {case_id}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "case {case_id} emitted stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn target_matches(result: &Value, expected: &Value) -> bool {
    result["target"]["kind"] == expected["kind"]
        && result["target"]["id"] == expected["id"]
        && result["target"].get("aspect") == expected.get("aspect")
}

fn finding_matches(projected: &Value, expected: &Value) -> bool {
    projected["finding"]["ruleId"] == expected["ruleId"]
        && projected["finding"]["ruleVersion"] == expected["ruleVersion"]
        && projected["finding"]["target"]["kind"] == expected["targetKind"]
        && projected["finding"]["target"]["id"] == expected["targetId"]
        && projected["finding"]["target"].get("aspect") == expected.get("targetAspect")
}

fn rule_result<'a>(report: &'a Value, expected: &Value) -> &'a Value {
    let matches = report["results"]
        .as_array()
        .expect("check results")
        .iter()
        .filter(|result| {
            result["ruleId"] == expected["ruleId"]
                && result["ruleVersion"] == expected["ruleVersion"]
                && target_matches(result, &expected["target"])
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "rule expectation must match once: {expected}"
    );
    matches[0]
}

fn projected_result<'a>(projection: &'a Value, expected: &Value) -> &'a Value {
    let matches = projection["projectedResults"]
        .as_array()
        .expect("projected results")
        .iter()
        .filter(|result| finding_matches(result, expected))
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "projection expectation must match once: {expected}"
    );
    matches[0]
}

fn paired_clean_result<'a>(report: &'a Value, expected: &Value) -> &'a Value {
    let matches = report["results"]
        .as_array()
        .expect("check results")
        .iter()
        .filter(|result| {
            result["ruleId"] == expected["ruleId"]
                && result["ruleVersion"] == expected["ruleVersion"]
                && result["target"]["kind"] == expected["target"]["kind"]
                && result["target"].get("aspect") == expected["target"].get("aspect")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "paired-clean expectation must match once: {expected}"
    );
    matches[0]
}

fn expected_finding_key(expected: &Value) -> String {
    format!(
        "{}@{}:{:?}:{}:{:?}",
        expected["ruleId"].as_str().expect("rule ID"),
        expected["ruleVersion"].as_str().expect("rule version"),
        expected["target"]["kind"],
        expected["target"]["id"].as_str().expect("target ID"),
        expected["target"].get("aspect")
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn reviewed_corpus_preserves_truth_boundaries_and_metric_contract() {
    let root = repository_root();
    let corpus = load_json(&root.join("evaluation/github-actions/corpus.json"));
    let rules = load_json(&root.join("evaluation/github-actions/annotations/rules.json"));
    let projection_oracle =
        load_json(&root.join("evaluation/github-actions/annotations/projection.json"));
    let metric_contract = load_json(&root.join("evaluation/github-actions/metric-contract.json"));
    let rule_cases = cases_by_id(&rules);
    let projection_cases = cases_by_id(&projection_oracle);
    let cases = corpus["cases"].as_array().expect("corpus cases");
    assert_eq!(
        cases.len() as u64,
        metric_contract["caseCount"].as_u64().expect("case count")
    );

    let mut metrics = Metrics::default();
    let mut reports_by_case = BTreeMap::new();
    let mut mutation_targets = Vec::new();

    for case in cases {
        let case_id = case["caseId"].as_str().expect("case ID");
        let rule_oracle = rule_cases
            .get(case_id)
            .unwrap_or_else(|| panic!("missing rule oracle for {case_id}"));
        let expected_projection = projection_cases
            .get(case_id)
            .unwrap_or_else(|| panic!("missing projection oracle for {case_id}"));
        assert_eq!(rule_oracle["profile"], case["profile"]);
        assert_eq!(
            rule_oracle["expectedExit"], expected_projection["expectedExit"],
            "separate authorities disagree about gate exit for {case_id}"
        );
        let expected_exit = i32::try_from(
            expected_projection["expectedExit"]
                .as_i64()
                .expect("expected exit"),
        )
        .expect("expected exit fits i32");

        let first = run_case(case, "json");
        assert_exit(&first, expected_exit, case_id);
        let second = run_case(case, "json");
        assert_exit(&second, expected_exit, case_id);
        assert_eq!(
            first.stdout, second.stdout,
            "case {case_id} was not byte-stable"
        );
        let actual: Value = serde_json::from_slice(&first.stdout).expect("projection JSON");
        assert_eq!(actual["projectionSummary"]["gateExitCode"], expected_exit);
        metrics.executed_cases += 1;

        let mut expected_failures = BTreeSet::new();
        for expectation in rule_oracle["expectations"]
            .as_array()
            .expect("rule expectations")
        {
            let result = rule_result(&actual["checkReport"], expectation);
            assert_eq!(
                result["outcome"], expectation["outcome"],
                "wrong outcome for {case_id}: {expectation}"
            );
            assert_eq!(
                result["enforcement"], expectation["enforcement"],
                "wrong enforcement for {case_id}: {expectation}"
            );
            match expectation["outcome"].as_str().expect("expected outcome") {
                "failed" => {
                    metrics.reviewed_failures += 1;
                    expected_failures.insert(expected_finding_key(expectation));
                    if case["classification"] == "targetedMutation" {
                        mutation_targets.push((
                            case_id.to_owned(),
                            case["pairedCleanCase"]
                                .as_str()
                                .expect("mutation baseline")
                                .to_owned(),
                            expectation.clone(),
                        ));
                    }
                }
                "cantTell" | "untested" => metrics.preserved_abstentions += 1,
                "passed" | "inapplicable" => {}
                other => panic!("unsupported expected outcome {other}"),
            }
        }

        let actual_failures = actual["checkReport"]["results"]
            .as_array()
            .expect("check results")
            .iter()
            .filter(|result| result["outcome"] == "failed")
            .map(expected_finding_key)
            .collect::<BTreeSet<_>>();
        metrics.unexpected_failures +=
            actual_failures.difference(&expected_failures).count() as u64;
        if matches!(
            case["classification"].as_str(),
            Some("clean" | "hardNegative")
        ) {
            metrics.false_positive_failures += actual_failures.len() as u64;
        }

        let dispositions = expected_projection["dispositions"]
            .as_array()
            .expect("projection dispositions");
        for disposition in dispositions {
            let projected = projected_result(&actual, &disposition["finding"]);
            assert_eq!(
                projected["annotation"]["status"], disposition["status"],
                "wrong disposition for {case_id}: {disposition}"
            );
            match disposition["status"].as_str().expect("disposition status") {
                "emitted" => {
                    let annotation = &projected["annotation"]["annotation"];
                    assert_eq!(annotation["level"], disposition["level"]);
                    assert_eq!(annotation["path"], disposition["location"]["path"]);
                    assert_eq!(
                        annotation["startLine"],
                        disposition["location"]["startLine"]
                    );
                    assert_eq!(annotation["endLine"], disposition["location"]["endLine"]);
                    metrics.exact_source_annotations += 1;
                }
                "sourceUnavailable" => {
                    assert_eq!(projected["annotation"]["reason"], disposition["reason"]);
                    if matches!(projected["outcome"].as_str(), Some("cantTell" | "untested")) {
                        metrics.summary_only_abstentions += 1;
                    }
                }
                other => panic!("unsupported disposition {other}"),
            }
        }
        if expected_projection["forbidUnexpectedProjectedResults"] == true {
            assert_eq!(
                actual["projectedResults"].as_array().map(Vec::len),
                Some(dispositions.len()),
                "unexpected projected result for {case_id}"
            );
        }
        let emitted = actual["projectedResults"]
            .as_array()
            .expect("projected results")
            .iter()
            .filter(|result| result["annotation"]["status"] == "emitted")
            .count();
        let expected_emitted = dispositions
            .iter()
            .filter(|disposition| disposition["status"] == "emitted")
            .count();
        metrics.unexpected_annotations += emitted.saturating_sub(expected_emitted) as u64;

        if expected_emitted > 0 {
            let workflow = run_case(case, "github-actions");
            assert_exit(&workflow, expected_exit, case_id);
            assert_eq!(
                String::from_utf8(workflow.stdout)
                    .expect("workflow commands UTF-8")
                    .lines()
                    .count(),
                expected_emitted,
                "wrong workflow annotation count for {case_id}"
            );
        }

        match case["classification"].as_str().expect("classification") {
            "clean" => metrics.clean_cases += 1,
            "hardNegative" => {
                metrics.hard_negatives += 1;
                metrics.hard_negative_failures += actual_failures.len() as u64;
            }
            "targetedMutation" | "cantTell" | "untested" => {}
            other => panic!("unsupported classification {other}"),
        }
        reports_by_case.insert(case_id.to_owned(), actual);
    }

    for (mutant, baseline, expectation) in mutation_targets {
        let clean = reports_by_case
            .get(&baseline)
            .unwrap_or_else(|| panic!("missing paired clean case {baseline} for {mutant}"));
        let target = paired_clean_result(&clean["checkReport"], &expectation);
        assert_ne!(
            target["outcome"], "failed",
            "paired clean case {baseline} retained mutation failure from {mutant}"
        );
        metrics.killed_mutations += 1;
    }

    assert_minimums(&metrics, &metric_contract["minimums"]);
    assert_maximums(&metrics, &metric_contract["maximums"]);
    assert_eq!(metric_contract["aggregateScore"], false);
    eprintln!(
        "github-actions product evaluation: cases={}/{}, reviewed_failures={}/{}, exact_source_annotations={}/{}, abstentions={}/{}, summary_only_abstentions={}/{}, mutation_kill={}/{}, clean={}/{}, hard_negatives={}/{}, false_positive_failures={}/{}, unexpected_annotations={}/{}, aggregate_score=none, holdout=none",
        metrics.executed_cases,
        cases.len(),
        metrics.reviewed_failures,
        metrics.reviewed_failures,
        metrics.exact_source_annotations,
        metrics.reviewed_failures,
        metrics.preserved_abstentions,
        metrics.preserved_abstentions,
        metrics.summary_only_abstentions,
        metrics.preserved_abstentions,
        metrics.killed_mutations,
        metrics.killed_mutations,
        metrics.clean_cases,
        metrics.clean_cases,
        metrics.hard_negatives,
        metrics.hard_negatives,
        metrics.false_positive_failures,
        metrics.clean_cases + metrics.hard_negatives,
        metrics.unexpected_annotations,
        metrics.executed_cases,
    );
}

fn assert_minimums(metrics: &Metrics, minimums: &Value) {
    for (name, actual) in [
        ("executedCases", metrics.executed_cases),
        ("reviewedFailures", metrics.reviewed_failures),
        ("exactSourceAnnotations", metrics.exact_source_annotations),
        ("preservedAbstentions", metrics.preserved_abstentions),
        ("summaryOnlyAbstentions", metrics.summary_only_abstentions),
        ("killedMutations", metrics.killed_mutations),
        ("cleanCases", metrics.clean_cases),
        ("hardNegatives", metrics.hard_negatives),
    ] {
        assert!(
            actual >= minimums[name].as_u64().expect("minimum metric"),
            "minimum {name} was not met: {actual}"
        );
    }
}

fn assert_maximums(metrics: &Metrics, maximums: &Value) {
    for (name, actual) in [
        ("unexpectedFailures", metrics.unexpected_failures),
        ("falsePositiveFailures", metrics.false_positive_failures),
        ("hardNegativeFailures", metrics.hard_negative_failures),
        ("unexpectedAnnotations", metrics.unexpected_annotations),
    ] {
        assert!(
            actual <= maximums[name].as_u64().expect("maximum metric"),
            "maximum {name} was exceeded: {actual}"
        );
    }
}
