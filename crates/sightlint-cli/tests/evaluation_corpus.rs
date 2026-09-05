//! Product-evaluation E2E for the public `sightlint` binary.
//!
//! This suite is intentionally separate from format and API conformance tests. It verifies that
//! versioned evaluation cases produce their declared rule outcomes, stay deterministic, and kill
//! targeted synthetic mutations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Map, Value};

const EVALUATION_SCHEMA_VERSION: &str = "0.1.0";
const EVALUATION_SCHEMA_ID: &str = "urn:sightlint:schema:evaluation-corpus:0.1.0";
const MANIFEST_PATH: &str = "evaluation/corpus.json";
const SCHEMA_PATH: &str = "evaluation/corpus.schema.json";
const SMOKE_SPLIT: &str = "smoke";

type Outcomes = BTreeMap<String, BTreeSet<String>>;
type RequiredOutcomes = BTreeSet<(String, String)>;

#[derive(Debug)]
struct EvaluatedCase {
    outcomes: Outcomes,
}

#[derive(Debug, Clone)]
struct MutationExpectation {
    mutant_case_id: String,
    baseline_case_id: String,
    target_rule_id: String,
}

#[derive(Debug)]
struct CaseSpec {
    id: String,
    split: String,
    medium: String,
    input_path: PathBuf,
    expected_exit: i32,
    required: RequiredOutcomes,
    forbid_unexpected_failures: bool,
    forbid_unexpected_cant_tell: bool,
    forbid_unexpected_untested: bool,
    mutation: Option<MutationExpectation>,
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
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("{context} must be a JSON array"))
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

fn assert_fields(value: &Value, allowed: &[&str], required: &[&str], context: &str) {
    let fields = object(value, context);
    for name in fields.keys() {
        assert!(
            allowed.contains(&name.as_str()),
            "{context} contains unsupported field {name:?}"
        );
    }
    for name in required {
        assert!(
            fields.contains_key(*name),
            "{context} is missing required field {name:?}"
        );
    }
}

fn valid_outcome(outcome: &str) -> bool {
    matches!(
        outcome,
        "passed" | "failed" | "inapplicable" | "cantTell" | "untested"
    )
}

fn valid_split(split: &str) -> bool {
    matches!(split, "smoke" | "development" | "holdout")
}

fn valid_medium(medium: &str) -> bool {
    matches!(
        medium,
        "web" | "mobile" | "slide" | "document" | "pdf" | "image" | "other"
    )
}

fn validate_manifest_header(manifest: &Value, repository_root: &Path) {
    assert_fields(
        manifest,
        &[
            "$schema",
            "schemaVersion",
            "corpus",
            "sources",
            "gates",
            "cases",
        ],
        &[
            "$schema",
            "schemaVersion",
            "corpus",
            "sources",
            "gates",
            "cases",
        ],
        "evaluation manifest",
    );
    assert_eq!(
        string_field(manifest, "$schema", "evaluation manifest"),
        "./corpus.schema.json"
    );
    assert_eq!(
        string_field(manifest, "schemaVersion", "evaluation manifest"),
        EVALUATION_SCHEMA_VERSION
    );

    let corpus = field(manifest, "corpus", "evaluation manifest");
    assert_fields(
        corpus,
        &["id", "version", "description", "defaultSplit"],
        &["id", "version", "description", "defaultSplit"],
        "evaluation corpus",
    );
    for name in ["id", "version", "description"] {
        assert!(
            !string_field(corpus, name, "evaluation corpus").is_empty(),
            "evaluation corpus {name} must not be empty"
        );
    }
    assert_eq!(
        string_field(corpus, "defaultSplit", "evaluation corpus"),
        SMOKE_SPLIT
    );

    let schema_path = repository_root.join(SCHEMA_PATH);
    let schema = load_json(&schema_path, "evaluation JSON Schema");
    assert_eq!(
        string_field(&schema, "$id", "evaluation JSON Schema"),
        EVALUATION_SCHEMA_ID
    );
}

fn validate_sources(manifest: &Value) -> BTreeSet<String> {
    let mut source_ids = BTreeSet::new();
    for source in array(field(manifest, "sources", "evaluation manifest"), "sources") {
        assert_fields(
            source,
            &["id", "kind", "origin", "license", "reviewStatus"],
            &["id", "kind", "origin", "license", "reviewStatus"],
            "evaluation source",
        );
        let id = string_field(source, "id", "evaluation source");
        assert!(!id.is_empty(), "evaluation source ID must not be empty");
        assert!(
            source_ids.insert(id.to_owned()),
            "duplicate evaluation source ID {id:?}"
        );

        let kind = string_field(source, "kind", "evaluation source");
        assert!(
            matches!(kind, "synthetic" | "human-reviewed" | "imported-benchmark"),
            "evaluation source {id:?} has unknown kind {kind:?}"
        );
        let review_status = string_field(source, "reviewStatus", "evaluation source");
        assert!(
            matches!(
                review_status,
                "generated" | "maintainer-reviewed" | "dual-reviewed" | "expert-reviewed"
            ),
            "evaluation source {id:?} has unknown review status {review_status:?}"
        );
        for required in ["origin", "license"] {
            assert!(
                !string_field(source, required, "evaluation source").is_empty(),
                "evaluation source {id:?} has an empty {required}"
            );
        }
    }
    assert!(
        !source_ids.is_empty(),
        "evaluation manifest must declare at least one source"
    );
    source_ids
}

fn smoke_gate(manifest: &Value) -> usize {
    let gates = field(manifest, "gates", "evaluation manifest");
    let gate = field(gates, SMOKE_SPLIT, "evaluation gates");
    assert_fields(
        gate,
        &[
            "requireAllExpectations",
            "requireAllMutationsKilled",
            "determinismRuns",
        ],
        &[
            "requireAllExpectations",
            "requireAllMutationsKilled",
            "determinismRuns",
        ],
        "smoke gate",
    );
    assert!(
        bool_field(gate, "requireAllExpectations", "smoke gate"),
        "required smoke evaluation must enforce every declared expectation"
    );
    assert!(
        bool_field(gate, "requireAllMutationsKilled", "smoke gate"),
        "required smoke evaluation must enforce every declared mutation"
    );
    let runs = unsigned_field(gate, "determinismRuns", "smoke gate");
    assert!(
        (2..=10).contains(&runs),
        "smoke determinismRuns must be between 2 and 10"
    );
    usize::try_from(runs).expect("determinism run count fits usize")
}

fn resolve_input(repository_root: &Path, relative_path: &str, case_id: &str) -> PathBuf {
    let relative = Path::new(relative_path);
    assert!(
        !relative.is_absolute(),
        "evaluation case {case_id:?} input path must be repository-relative"
    );
    assert!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "evaluation case {case_id:?} input path must not contain prefixes, roots, or parent traversal"
    );
    let resolved = repository_root
        .join(relative)
        .canonicalize()
        .unwrap_or_else(|error| {
            panic!(
                "evaluation case {case_id:?} input {} is unavailable: {error}",
                relative.display()
            )
        });
    assert!(
        resolved.starts_with(repository_root),
        "evaluation case {case_id:?} input escapes the repository"
    );
    resolved
}

fn required_outcomes(expectation: &Value, case_id: &str) -> RequiredOutcomes {
    let mut required = BTreeSet::new();
    for item in array(
        field(expectation, "requiredOutcomes", "evaluation expectation"),
        "required outcomes",
    ) {
        assert_fields(
            item,
            &["ruleId", "outcome"],
            &["ruleId", "outcome"],
            "required outcome",
        );
        let rule_id = string_field(item, "ruleId", "required outcome");
        let outcome = string_field(item, "outcome", "required outcome");
        assert!(
            !rule_id.is_empty(),
            "evaluation case {case_id:?} contains an empty rule ID"
        );
        assert!(
            valid_outcome(outcome),
            "evaluation case {case_id:?} contains unknown outcome {outcome:?}"
        );
        assert!(
            required.insert((rule_id.to_owned(), outcome.to_owned())),
            "evaluation case {case_id:?} repeats {rule_id}={outcome}"
        );
    }
    assert!(
        !required.is_empty(),
        "evaluation case {case_id:?} must declare at least one required outcome"
    );
    required
}

fn parse_mutation(case: &Value, case_id: &str) -> Option<MutationExpectation> {
    object(case, "evaluation case")
        .get("mutation")
        .map(|value| {
            assert_fields(
                value,
                &["baselineCaseId", "targetRuleId"],
                &["baselineCaseId", "targetRuleId"],
                "mutation",
            );
            let baseline_case_id = string_field(value, "baselineCaseId", "mutation");
            let target_rule_id = string_field(value, "targetRuleId", "mutation");
            assert!(
                !baseline_case_id.is_empty() && !target_rule_id.is_empty(),
                "evaluation case {case_id:?} has an incomplete mutation relation"
            );
            MutationExpectation {
                mutant_case_id: case_id.to_owned(),
                baseline_case_id: baseline_case_id.to_owned(),
                target_rule_id: target_rule_id.to_owned(),
            }
        })
}

fn parse_case(case: &Value, repository_root: &Path, source_ids: &BTreeSet<String>) -> CaseSpec {
    assert_fields(
        case,
        &[
            "id", "split", "medium", "sourceId", "input", "expect", "mutation",
        ],
        &["id", "split", "medium", "sourceId", "input", "expect"],
        "evaluation case",
    );
    let case_id = string_field(case, "id", "evaluation case");
    let split = string_field(case, "split", "evaluation case");
    let medium = string_field(case, "medium", "evaluation case");
    let source_id = string_field(case, "sourceId", "evaluation case");
    assert!(!case_id.is_empty(), "evaluation case ID must not be empty");
    assert!(
        valid_split(split),
        "evaluation case {case_id:?} has unknown split {split:?}"
    );
    assert!(
        valid_medium(medium),
        "evaluation case {case_id:?} has unknown medium {medium:?}"
    );
    assert!(
        source_ids.contains(source_id),
        "evaluation case {case_id:?} references unknown source {source_id:?}"
    );

    let input = field(case, "input", "evaluation case");
    assert_fields(
        input,
        &["kind", "path"],
        &["kind", "path"],
        "evaluation input",
    );
    assert_eq!(
        string_field(input, "kind", "evaluation input"),
        "artifactIr",
        "evaluation corpus 0.1 supports Artifact IR inputs only"
    );
    let input_path = resolve_input(
        repository_root,
        string_field(input, "path", "evaluation input"),
        case_id,
    );

    let expectation = field(case, "expect", "evaluation case");
    assert_fields(
        expectation,
        &[
            "exitCode",
            "forbidUnexpectedFailures",
            "forbidUnexpectedCantTell",
            "forbidUnexpectedUntested",
            "requiredOutcomes",
        ],
        &[
            "exitCode",
            "forbidUnexpectedFailures",
            "forbidUnexpectedCantTell",
            "forbidUnexpectedUntested",
            "requiredOutcomes",
        ],
        "evaluation expectation",
    );
    let expected_exit = unsigned_field(expectation, "exitCode", "evaluation expectation");
    assert!(
        expected_exit <= 1,
        "product evaluation cases must be valid inputs with exit code 0 or 1"
    );

    CaseSpec {
        id: case_id.to_owned(),
        split: split.to_owned(),
        medium: medium.to_owned(),
        input_path,
        expected_exit: i32::try_from(expected_exit).expect("exit code fits i32"),
        required: required_outcomes(expectation, case_id),
        forbid_unexpected_failures: bool_field(
            expectation,
            "forbidUnexpectedFailures",
            "evaluation expectation",
        ),
        forbid_unexpected_cant_tell: bool_field(
            expectation,
            "forbidUnexpectedCantTell",
            "evaluation expectation",
        ),
        forbid_unexpected_untested: bool_field(
            expectation,
            "forbidUnexpectedUntested",
            "evaluation expectation",
        ),
        mutation: parse_mutation(case, case_id),
    }
}

fn run_check(input: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
        .arg("check")
        .arg(input)
        .args(["--format", "json"])
        .output()
        .expect("failed to execute sightlint")
}

fn collect_outcomes(report: &Value, case_id: &str) -> Outcomes {
    let mut outcomes = BTreeMap::<String, BTreeSet<String>>::new();
    for result in array(field(report, "results", "check report"), "report results") {
        let rule_id = string_field(result, "ruleId", "rule result");
        let outcome = string_field(result, "outcome", "rule result");
        assert!(
            valid_outcome(outcome),
            "evaluation case {case_id:?} emitted unknown outcome {outcome:?}"
        );
        outcomes
            .entry(rule_id.to_owned())
            .or_default()
            .insert(outcome.to_owned());
    }
    outcomes
}

fn assert_required_outcomes(case_id: &str, actual: &Outcomes, required: &RequiredOutcomes) {
    for (rule_id, outcome) in required {
        let observed = actual.get(rule_id);
        assert!(
            observed.is_some_and(|values| values.contains(outcome)),
            "evaluation case {case_id:?} expected {rule_id}={outcome}, observed {observed:?}"
        );
    }
}

fn assert_no_unexpected_outcome(
    case_id: &str,
    actual: &Outcomes,
    required: &RequiredOutcomes,
    outcome: &str,
) {
    for (rule_id, observed) in actual {
        assert!(
            !observed.contains(outcome)
                || required.contains(&(rule_id.clone(), outcome.to_owned())),
            "evaluation case {case_id:?} produced unexpected {rule_id}={outcome}"
        );
    }
}

fn evaluate_case(spec: &CaseSpec, determinism_runs: usize) -> EvaluatedCase {
    let first = run_check(&spec.input_path);
    assert_eq!(
        first.status.code(),
        Some(spec.expected_exit),
        "evaluation case {:?} returned the wrong exit code\nstdout:\n{}\nstderr:\n{}",
        spec.id,
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        first.stderr.is_empty(),
        "evaluation case {:?} wrote unexpected stderr: {}",
        spec.id,
        String::from_utf8_lossy(&first.stderr)
    );
    for run_index in 1..determinism_runs {
        let repeated = run_check(&spec.input_path);
        assert_eq!(
            repeated.status.code(),
            first.status.code(),
            "evaluation case {:?} changed exit code on run {run_index}",
            spec.id
        );
        assert_eq!(
            repeated.stdout, first.stdout,
            "evaluation case {:?} changed report bytes on run {run_index}",
            spec.id
        );
        assert_eq!(
            repeated.stderr, first.stderr,
            "evaluation case {:?} changed stderr bytes on run {run_index}",
            spec.id
        );
    }

    let report: Value = serde_json::from_slice(&first.stdout).unwrap_or_else(|error| {
        panic!(
            "evaluation case {:?} did not emit a JSON report: {error}",
            spec.id
        )
    });
    assert_eq!(
        field(&report, "artifactKind", "check report").as_str(),
        Some(spec.medium.as_str()),
        "evaluation case {:?} emitted the wrong artifact medium",
        spec.id
    );
    let actual = collect_outcomes(&report, &spec.id);
    assert_required_outcomes(&spec.id, &actual, &spec.required);

    if spec.forbid_unexpected_failures {
        assert_no_unexpected_outcome(&spec.id, &actual, &spec.required, "failed");
    }
    if spec.forbid_unexpected_cant_tell {
        assert_no_unexpected_outcome(&spec.id, &actual, &spec.required, "cantTell");
    }
    if spec.forbid_unexpected_untested {
        assert_no_unexpected_outcome(&spec.id, &actual, &spec.required, "untested");
    }

    EvaluatedCase { outcomes: actual }
}

fn contains_outcome(case: &EvaluatedCase, rule_id: &str, outcome: &str) -> bool {
    case.outcomes
        .get(rule_id)
        .is_some_and(|values| values.contains(outcome))
}

fn verify_mutations(
    evaluated: &BTreeMap<String, EvaluatedCase>,
    mutations: &[MutationExpectation],
) {
    assert!(
        !mutations.is_empty(),
        "smoke evaluation must include targeted mutations"
    );
    for mutation in mutations {
        let baseline = evaluated
            .get(&mutation.baseline_case_id)
            .unwrap_or_else(|| {
                panic!(
                    "mutation {} references missing baseline {}",
                    mutation.mutant_case_id, mutation.baseline_case_id
                )
            });
        let mutant = evaluated
            .get(&mutation.mutant_case_id)
            .expect("evaluated mutation case exists");
        assert!(
            contains_outcome(baseline, &mutation.target_rule_id, "passed"),
            "mutation {} baseline {} did not pass target rule {}",
            mutation.mutant_case_id,
            mutation.baseline_case_id,
            mutation.target_rule_id
        );
        assert!(
            contains_outcome(mutant, &mutation.target_rule_id, "failed"),
            "mutation {} was not killed by target rule {}",
            mutation.mutant_case_id,
            mutation.target_rule_id
        );
    }
}

#[test]
fn smoke_product_evaluation_matches_versioned_oracles_and_kills_mutations() {
    let repository_root = repository_root();
    let manifest = load_json(&repository_root.join(MANIFEST_PATH), "evaluation manifest");
    validate_manifest_header(&manifest, &repository_root);

    let source_ids = validate_sources(&manifest);
    let determinism_runs = smoke_gate(&manifest);
    let cases = array(field(&manifest, "cases", "evaluation manifest"), "cases");

    let mut evaluated = BTreeMap::new();
    let mut mutations = Vec::new();
    let mut previous_case_id: Option<String> = None;
    let mut smoke_case_count = 0_usize;

    for case in cases {
        let spec = parse_case(case, &repository_root, &source_ids);
        if let Some(previous) = &previous_case_id {
            assert!(
                previous < &spec.id,
                "evaluation cases must be unique and sorted by ID: {previous:?} before {:?}",
                spec.id
            );
        }
        previous_case_id = Some(spec.id.clone());

        if spec.split != SMOKE_SPLIT {
            continue;
        }
        smoke_case_count += 1;
        let result = evaluate_case(&spec, determinism_runs);
        assert!(
            evaluated.insert(spec.id.clone(), result).is_none(),
            "duplicate evaluated case ID {:?}",
            spec.id
        );
        if let Some(mutation) = spec.mutation {
            mutations.push(mutation);
        }
    }

    assert!(
        smoke_case_count >= 10,
        "smoke evaluation must retain broad baseline and mutation coverage"
    );
    verify_mutations(&evaluated, &mutations);
}
