//! Realistic Web evaluation foundation for the public `sightlint` binary.
//!
//! This suite validates the separation between acquisition and rule annotations, preserves
//! explicit untested acquisition coverage, and executes only independently reviewed Artifact IR
//! projections. It does not evaluate a browser adapter or claim real-world UI/UX accuracy.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Map, Value};

const CORPUS_SCHEMA_VERSION: &str = "0.1.0";
const CORPUS_SCHEMA_ID: &str = "urn:sightlint:schema:web-evaluation-corpus:0.1.0";
const ANNOTATION_SCHEMA_ID: &str = "urn:sightlint:schema:web-evaluation-annotation:0.1.0";
const CORPUS_PATH: &str = "evaluation/web/corpus.json";
const CORPUS_SCHEMA_PATH: &str = "evaluation/web/corpus.schema.json";
const ANNOTATION_SCHEMA_PATH: &str = "evaluation/web/annotation.schema.json";
const ACQUISITION_PATH: &str = "evaluation/web/annotations/acquisition.json";
const RULES_PATH: &str = "evaluation/web/annotations/rules.json";
const TARGET_RULE: &str = "visual.spacing.peer-consistency";

#[derive(Debug)]
struct RuleOracle {
    rule_id: String,
    rule_version: String,
    target_relation_id: Option<String>,
    applicability: String,
    expected_outcome: String,
}

#[derive(Debug, Default)]
struct Metrics {
    labeled_cases: usize,
    runnable_cases: usize,
    applicable_runnable: usize,
    covered_pass_fail: usize,
    expected_failures: usize,
    true_positive_failures: usize,
    false_positive_failures: usize,
    expected_abstentions: usize,
    deferred_abstentions: usize,
    mutations: usize,
    killed_mutations: usize,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root must be available")
}

fn load_json(path: &Path, context: &str) -> Value {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {context} at {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("failed to decode {context} at {}: {error}", path.display()))
}

fn object<'a>(value: &'a Value, context: &str) -> &'a Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be a JSON object"))
}

fn array<'a>(value: &'a Value, context: &str) -> &'a [Value] {
    value
        .as_array()
        .map_or_else(|| panic!("{context} must be a JSON array"), Vec::as_slice)
}

fn field<'a>(value: &'a Value, name: &str, context: &str) -> &'a Value {
    object(value, context)
        .get(name)
        .unwrap_or_else(|| panic!("{context} is missing required field {name:?}"))
}

fn string_field<'a>(value: &'a Value, name: &str, context: &str) -> &'a str {
    field(value, name, context)
        .as_str()
        .unwrap_or_else(|| panic!("{context}.{name} must be a string"))
}

fn bool_field(value: &Value, name: &str, context: &str) -> bool {
    field(value, name, context)
        .as_bool()
        .unwrap_or_else(|| panic!("{context}.{name} must be a boolean"))
}

fn unsigned_field(value: &Value, name: &str, context: &str) -> u64 {
    field(value, name, context)
        .as_u64()
        .unwrap_or_else(|| panic!("{context}.{name} must be a non-negative integer"))
}

fn resolve_repository_path(root: &Path, relative: &str, context: &str) -> PathBuf {
    let relative = Path::new(relative);
    assert!(!relative.is_absolute(), "{context} must be relative");
    assert!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "{context} must not contain a root, prefix, or parent traversal"
    );
    let resolved = root
        .join(relative)
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to resolve {context}: {error}"));
    assert!(
        resolved.starts_with(root),
        "{context} must remain inside the repository"
    );
    resolved
}

fn index_by_case_id<'a>(document: &'a Value, context: &str) -> BTreeMap<String, &'a Value> {
    let mut indexed = BTreeMap::new();
    let mut source_order = Vec::new();
    for case in array(field(document, "cases", context), context) {
        let id = string_field(case, "caseId", context);
        source_order.push(id.to_owned());
        assert!(
            indexed.insert(id.to_owned(), case).is_none(),
            "{context} contains duplicate case ID {id:?}"
        );
    }
    assert_eq!(
        source_order,
        indexed.keys().cloned().collect::<Vec<_>>(),
        "{context} cases must be sorted by case ID"
    );
    indexed
}

fn parse_rule_oracles(document: &Value) -> BTreeMap<String, RuleOracle> {
    assert_eq!(
        string_field(document, "documentType", "rule annotations"),
        "ruleOracle"
    );
    let mut result = BTreeMap::new();
    for case in array(
        field(document, "cases", "rule annotations"),
        "rule annotations",
    ) {
        let case_id = string_field(case, "caseId", "rule annotation").to_owned();
        assert_eq!(
            string_field(case, "maturity", "rule annotation"),
            "experimental"
        );
        assert!(
            !bool_field(case, "blocking", "rule annotation"),
            "Web evaluation 0.1 must not make the target rule blocking"
        );
        let target_relation_id = field(case, "targetRelationId", "rule annotation")
            .as_str()
            .map(ToOwned::to_owned);
        let oracle = RuleOracle {
            rule_id: string_field(case, "ruleId", "rule annotation").to_owned(),
            rule_version: string_field(case, "ruleVersion", "rule annotation").to_owned(),
            target_relation_id,
            applicability: string_field(
                field(case, "applicability", "rule annotation"),
                "status",
                "rule applicability",
            )
            .to_owned(),
            expected_outcome: string_field(case, "expectedOutcome", "rule annotation").to_owned(),
        };
        assert_eq!(oracle.rule_id, TARGET_RULE);
        assert!(
            result.insert(case_id.clone(), oracle).is_none(),
            "duplicate rule annotation for {case_id:?}"
        );
    }
    result
}

fn assert_acquisition_boundary(document: &Value) -> BTreeSet<String> {
    assert_eq!(
        string_field(document, "documentType", "acquisition annotations"),
        "acquisitionOracle"
    );
    let indexed = index_by_case_id(document, "acquisition annotations");
    for (case_id, case) in &indexed {
        let unavailable = array(
            field(case, "unavailableAspects", "acquisition annotation"),
            "unavailable acquisition aspects",
        );
        assert!(
            unavailable.len() >= 3,
            "case {case_id:?} must expose browser, accessibility, and pixel coverage"
        );
        let mut aspects = BTreeSet::new();
        for item in unavailable {
            assert_eq!(
                string_field(item, "status", "unavailable acquisition aspect"),
                "untested"
            );
            assert_eq!(
                unsigned_field(item, "trackingIssue", "unavailable acquisition aspect"),
                23
            );
            aspects.insert(string_field(
                item,
                "aspect",
                "unavailable acquisition aspect",
            ));
        }
        assert!(aspects.contains("computedLayoutRenderAndHitGeometry"));
        assert!(aspects.contains("accessibilityTree"));
        assert!(aspects.contains("screenshotAndNativePixelReconciliation"));
    }
    indexed.into_keys().collect()
}

fn run_check(input: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
        .arg("check")
        .arg(input)
        .args(["--format", "json"])
        .output()
        .expect("failed to execute sightlint")
}

fn result_outcome(report: &Value, oracle: &RuleOracle, case_id: &str) -> String {
    let mut matches = array(field(report, "results", "check report"), "check results")
        .iter()
        .filter(|result| {
            string_field(result, "ruleId", "rule result") == oracle.rule_id
                && oracle
                    .target_relation_id
                    .as_deref()
                    .is_none_or(|target_id| {
                        string_field(field(result, "target", "rule result"), "id", "rule target")
                            == target_id
                    })
        });
    let result = matches
        .next()
        .unwrap_or_else(|| panic!("case {case_id:?} did not emit the annotated rule target"));
    assert!(
        matches.next().is_none(),
        "case {case_id:?} emitted more than one annotated rule target"
    );
    assert_eq!(
        string_field(result, "ruleVersion", "rule result"),
        oracle.rule_version,
        "case {case_id:?} emitted the wrong rule version"
    );
    string_field(result, "outcome", "rule result").to_owned()
}

fn evaluate_runnable_case(
    input: &Path,
    oracle: &RuleOracle,
    case_id: &str,
    determinism_runs: usize,
) -> String {
    let first = run_check(input);
    let expected_exit = i32::from(oracle.expected_outcome == "failed");
    assert_eq!(
        first.status.code(),
        Some(expected_exit),
        "case {case_id:?} returned the wrong exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        first.stderr.is_empty(),
        "case {case_id:?} wrote unexpected stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    for run_index in 1..determinism_runs {
        let repeated = run_check(input);
        assert_eq!(
            repeated.status.code(),
            first.status.code(),
            "case {case_id:?} changed exit code on run {run_index}"
        );
        assert_eq!(
            repeated.stdout, first.stdout,
            "case {case_id:?} changed report bytes on run {run_index}"
        );
        assert_eq!(
            repeated.stderr, first.stderr,
            "case {case_id:?} changed stderr bytes on run {run_index}"
        );
    }

    let report: Value = serde_json::from_slice(&first.stdout)
        .unwrap_or_else(|error| panic!("case {case_id:?} emitted invalid JSON: {error}"));
    let actual = result_outcome(&report, oracle, case_id);
    assert_eq!(
        actual, oracle.expected_outcome,
        "case {case_id:?} disagreed with its reviewed rule oracle"
    );
    actual
}

type MutationRecord = (String, String, String);

fn load_contract(root: &Path) -> (Value, BTreeMap<String, RuleOracle>, BTreeSet<String>) {
    let corpus = load_json(&root.join(CORPUS_PATH), "Web evaluation corpus");
    let corpus_schema = load_json(&root.join(CORPUS_SCHEMA_PATH), "Web corpus schema");
    let annotation_schema = load_json(&root.join(ANNOTATION_SCHEMA_PATH), "annotation schema");
    let acquisition = load_json(&root.join(ACQUISITION_PATH), "acquisition annotations");
    let rules = load_json(&root.join(RULES_PATH), "rule annotations");

    assert_eq!(
        string_field(&corpus, "schemaVersion", "Web evaluation corpus"),
        CORPUS_SCHEMA_VERSION
    );
    assert_eq!(
        string_field(&corpus_schema, "$id", "Web corpus schema"),
        CORPUS_SCHEMA_ID
    );
    assert_eq!(
        string_field(&annotation_schema, "$id", "annotation schema"),
        ANNOTATION_SCHEMA_ID
    );
    assert_eq!(
        string_field(&acquisition, "schemaVersion", "acquisition annotations"),
        CORPUS_SCHEMA_VERSION
    );
    assert_eq!(
        string_field(&rules, "schemaVersion", "rule annotations"),
        CORPUS_SCHEMA_VERSION
    );

    let acquisition_ids = assert_acquisition_boundary(&acquisition);
    let rule_oracles = parse_rule_oracles(&rules);
    let rule_ids = rule_oracles.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(acquisition_ids, rule_ids);
    (corpus, rule_oracles, acquisition_ids)
}

fn assert_governance_and_determinism(corpus: &Value) -> usize {
    let sources = array(field(corpus, "sources", "Web corpus"), "Web sources");
    assert_eq!(sources.len(), 1);
    assert_eq!(
        string_field(&sources[0], "ownership", "Web source"),
        "sightlintRepository"
    );
    assert_eq!(
        string_field(&sources[0], "privacyReview", "Web source"),
        "syntheticNoPersonalData"
    );
    assert!(!bool_field(&sources[0], "externalAssets", "Web source"));
    assert!(
        string_field(&sources[0], "license", "Web source").contains("pending"),
        "the corpus must not invent a license before issue 33"
    );

    let split_policy = field(corpus, "splitPolicy", "Web corpus");
    assert_eq!(
        string_field(
            field(split_policy, "holdout", "split policy"),
            "status",
            "holdout policy",
        ),
        "notEstablished"
    );
    let smoke_gate = field(field(corpus, "gates", "Web corpus"), "smoke", "gates");
    let determinism_runs =
        usize::try_from(unsigned_field(smoke_gate, "determinismRuns", "smoke gate"))
            .expect("determinism runs fit usize");
    assert!((2..=10).contains(&determinism_runs));
    assert!(bool_field(
        smoke_gate,
        "requireAllExpectations",
        "smoke gate"
    ));
    assert!(bool_field(
        smoke_gate,
        "requireAllMutationsKilled",
        "smoke gate"
    ));
    assert_eq!(
        unsigned_field(smoke_gate, "maximumFalsePositives", "smoke gate"),
        0
    );
    determinism_runs
}

fn record_runnable_metrics(metrics: &mut Metrics, oracle: &RuleOracle, actual: &str) {
    if oracle.applicability == "applicable" {
        metrics.applicable_runnable += 1;
    }
    if matches!(actual, "passed" | "failed") {
        metrics.covered_pass_fail += 1;
    }
    if oracle.expected_outcome == "failed" {
        metrics.expected_failures += 1;
        if actual == "failed" {
            metrics.true_positive_failures += 1;
        }
    } else if actual == "failed" {
        metrics.false_positive_failures += 1;
    }
}

fn evaluate_case(
    root: &Path,
    case: &Value,
    oracle: &RuleOracle,
    determinism_runs: usize,
    metrics: &mut Metrics,
    actual_outcomes: &mut BTreeMap<String, String>,
    mutations: &mut Vec<MutationRecord>,
) {
    let case_id = string_field(case, "id", "Web case");
    let capture = field(case, "capture", "Web case");
    assert_eq!(string_field(capture, "status", "capture"), "untested");
    assert_eq!(unsigned_field(capture, "trackingIssue", "capture"), 23);
    assert!(!bool_field(capture, "externalProcessing", "capture"));

    if matches!(oracle.expected_outcome.as_str(), "cantTell" | "untested") {
        metrics.expected_abstentions += 1;
    }
    let execution = field(case, "execution", "Web case");
    match string_field(execution, "status", "execution") {
        "runnable" => {
            metrics.runnable_cases += 1;
            assert_eq!(
                string_field(execution, "inputKind", "execution"),
                "artifactIr"
            );
            let input = resolve_repository_path(
                root,
                string_field(execution, "inputPath", "execution"),
                "Web evaluation input",
            );
            let actual = evaluate_runnable_case(&input, oracle, case_id, determinism_runs);
            record_runnable_metrics(metrics, oracle, &actual);
            actual_outcomes.insert(case_id.to_owned(), actual);
        }
        "untested" => {
            assert!(
                matches!(oracle.expected_outcome.as_str(), "cantTell" | "untested"),
                "case {case_id:?} cannot be skipped with a definitive oracle"
            );
            metrics.deferred_abstentions += 1;
        }
        status => panic!("unsupported execution status {status:?}"),
    }

    if let Some(mutation) = object(case, "Web case").get("mutation") {
        metrics.mutations += 1;
        mutations.push((
            case_id.to_owned(),
            string_field(mutation, "baselineCaseId", "mutation").to_owned(),
            string_field(mutation, "targetRuleId", "mutation").to_owned(),
        ));
    }
    if object(case, "Web case").contains_key("hardNegative") {
        let actual = actual_outcomes
            .get(case_id)
            .unwrap_or_else(|| panic!("hard negative {case_id:?} must be runnable"));
        assert_ne!(
            actual, "failed",
            "hard negative {case_id:?} produced a false-positive failure"
        );
    }
}

fn evaluate_cases(
    root: &Path,
    corpus: &Value,
    rule_oracles: &BTreeMap<String, RuleOracle>,
    determinism_runs: usize,
) -> (Metrics, BTreeSet<String>) {
    let cases = array(field(corpus, "cases", "Web corpus"), "Web cases");
    let mut metrics = Metrics {
        labeled_cases: cases.len(),
        ..Metrics::default()
    };
    let mut previous_id: Option<String> = None;
    let mut actual_outcomes = BTreeMap::new();
    let mut case_ids = BTreeSet::new();
    let mut mutations = Vec::new();

    for case in cases {
        let case_id = string_field(case, "id", "Web case");
        if let Some(previous) = &previous_id {
            assert!(
                previous.as_str() < case_id,
                "Web cases must be sorted by ID"
            );
        }
        previous_id = Some(case_id.to_owned());
        assert!(
            case_ids.insert(case_id.to_owned()),
            "duplicate case {case_id:?}"
        );
        let oracle = rule_oracles
            .get(case_id)
            .unwrap_or_else(|| panic!("missing rule oracle for {case_id:?}"));
        evaluate_case(
            root,
            case,
            oracle,
            determinism_runs,
            &mut metrics,
            &mut actual_outcomes,
            &mut mutations,
        );
    }

    for (mutant, baseline, target_rule) in mutations {
        assert_eq!(target_rule, TARGET_RULE);
        assert_eq!(
            actual_outcomes.get(&baseline).map(String::as_str),
            Some("passed")
        );
        assert_eq!(
            actual_outcomes.get(&mutant).map(String::as_str),
            Some("failed")
        );
        metrics.killed_mutations += 1;
    }
    (metrics, case_ids)
}

fn assert_and_print_metrics(metrics: &Metrics) {
    assert_eq!(metrics.labeled_cases, 6);
    assert_eq!(metrics.runnable_cases, 3);
    assert_eq!(metrics.applicable_runnable, 3);
    assert_eq!(metrics.covered_pass_fail, 3);
    assert_eq!(metrics.expected_failures, 1);
    assert_eq!(metrics.true_positive_failures, 1);
    assert_eq!(metrics.false_positive_failures, 0);
    assert_eq!(metrics.expected_abstentions, 3);
    assert_eq!(metrics.deferred_abstentions, 3);
    assert_eq!(metrics.mutations, 1);
    assert_eq!(metrics.killed_mutations, 1);

    println!(
        "web evaluation v0: labeled={}, runnable={}, applicable_runnable={}, covered_pass_fail={}/{}, true_positive_failures={}/{}, false_positive_failures={}, deferred_abstentions={}/{}, mutation_kills={}/{}",
        metrics.labeled_cases,
        metrics.runnable_cases,
        metrics.applicable_runnable,
        metrics.covered_pass_fail,
        metrics.applicable_runnable,
        metrics.true_positive_failures,
        metrics.expected_failures,
        metrics.false_positive_failures,
        metrics.deferred_abstentions,
        metrics.expected_abstentions,
        metrics.killed_mutations,
        metrics.mutations,
    );
}

#[test]
fn realistic_web_foundation_preserves_oracles_abstention_and_public_rule_behavior() {
    let root = repository_root();
    let (corpus, rule_oracles, annotated_case_ids) = load_contract(&root);
    let determinism_runs = assert_governance_and_determinism(&corpus);
    let (metrics, evaluated_case_ids) =
        evaluate_cases(&root, &corpus, &rule_oracles, determinism_runs);
    assert_eq!(evaluated_case_ids, annotated_case_ids);
    assert_and_print_metrics(&metrics);
}
