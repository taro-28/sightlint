//! Public-binary execution of the committed layered image evaluation corpus.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_ERROR: i32 = 2;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("CLI crate lives two levels below repository root")
        .to_path_buf()
}

fn corpus_directory() -> PathBuf {
    repository_root().join("fixtures/evaluation/image")
}

fn manifest() -> Value {
    let path = corpus_directory().join("manifest.json");
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("failed to parse {path:?}: {error}"))
}

fn run(args: &[&str]) -> Output {
    binary()
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute sightlint {args:?}: {error}"))
}

fn case_file(case: &Value) -> PathBuf {
    let relative = case["file"].as_str().expect("case file is a string");
    assert!(!relative.contains('/'), "corpus files stay in one directory");
    assert!(!relative.contains('\\'), "corpus files stay in one directory");
    corpus_directory().join(relative)
}

fn run_adapt(path: &Path) -> Output {
    run(&[
        "adapt-image",
        path.to_str().expect("corpus path is UTF-8"),
    ])
}

fn run_check(path: &Path) -> Output {
    run(&[
        "check-image",
        path.to_str().expect("corpus path is UTF-8"),
        "--format",
        "json",
    ])
}

fn json_equivalent(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left.as_f64().zip(right.as_f64()).is_some_and(|(left, right)| left == right)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_equivalent(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|right| json_equivalent(value, right))
                })
        }
        _ => left == right,
    }
}

fn assert_current_assertion(document: &Value, assertion: &Value, case_id: &str) {
    let pointer = assertion["pointer"]
        .as_str()
        .expect("assertion pointer is a string");
    let actual = document.pointer(pointer);

    if let Some(expected_exists) = assertion.get("exists").and_then(Value::as_bool) {
        assert_eq!(
            actual.is_some(),
            expected_exists,
            "case {case_id}: pointer {pointer} existence mismatch"
        );
    }
    if let Some(expected) = assertion.get("equals") {
        let actual = actual.unwrap_or_else(|| {
            panic!("case {case_id}: pointer {pointer} is missing; expected {expected}")
        });
        assert!(
            json_equivalent(actual, expected),
            "case {case_id}: pointer {pointer}\nactual: {actual}\nexpected: {expected}"
        );
    }
    if let Some(expected_length) = assertion.get("length").and_then(Value::as_u64) {
        let actual = actual.unwrap_or_else(|| {
            panic!("case {case_id}: pointer {pointer} is missing; expected length {expected_length}")
        });
        let actual_length = match actual {
            Value::Array(values) => values.len(),
            Value::Object(values) => values.len(),
            other => panic!(
                "case {case_id}: pointer {pointer} is {other}, not an array or object"
            ),
        };
        assert_eq!(
            u64::try_from(actual_length).expect("JSON collection length fits u64"),
            expected_length,
            "case {case_id}: pointer {pointer} length mismatch"
        );
    }

    assert!(
        assertion.get("equals").is_some()
            || assertion.get("exists").is_some()
            || assertion.get("length").is_some(),
        "case {case_id}: assertion for {pointer} has no supported expectation"
    );
}

fn validate_rect(case_id: &str, dimensions: &Value, region: &Value) {
    let width = dimensions["width"].as_u64().expect("canvas width");
    let height = dimensions["height"].as_u64().expect("canvas height");
    let rect = &region["rect"];
    let x = rect["x"].as_u64().expect("region x");
    let y = rect["y"].as_u64().expect("region y");
    let region_width = rect["width"].as_u64().expect("region width");
    let region_height = rect["height"].as_u64().expect("region height");
    assert!(region_width > 0 && region_height > 0, "case {case_id}: empty region");
    assert!(
        x.checked_add(region_width).is_some_and(|right| right <= width),
        "case {case_id}: region exceeds canvas width: {region}"
    );
    assert!(
        y.checked_add(region_height)
            .is_some_and(|bottom| bottom <= height),
        "case {case_id}: region exceeds canvas height: {region}"
    );
}

fn validate_manifest_contract(manifest: &Value) {
    assert_eq!(manifest["schemaVersion"], "0.1.0");
    let cases = manifest["cases"].as_array().expect("cases array");
    assert!(cases.len() >= 8, "initial corpus must retain broad case coverage");

    let mut ids = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut by_id = BTreeMap::new();
    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let file = case["file"].as_str().expect("case file");
        assert!(ids.insert(id), "duplicate case id: {id}");
        assert!(files.insert(file), "duplicate case file: {file}");
        assert!(case_file(case).is_file(), "missing corpus artifact: {file}");
        by_id.insert(id, case);

        let ground_truth = &case["groundTruth"];
        assert!(ground_truth["regions"].is_array(), "case {id}: regions array");
        assert!(ground_truth["peerGroups"].is_array(), "case {id}: peer groups array");
        assert!(ground_truth["defects"].is_array(), "case {id}: defects array");
        let capabilities = ground_truth["targetCapabilities"]
            .as_array()
            .expect("target capabilities array");
        assert!(!capabilities.is_empty(), "case {id}: target capabilities required");
        assert!(case["currentAssertions"].is_array(), "case {id}: assertions array");
        for region in ground_truth["regions"].as_array().expect("regions") {
            validate_rect(id, &case["dimensions"], region);
        }
    }

    for case in cases {
        if let Some(mutation) = case.get("mutation") {
            let id = case["id"].as_str().expect("case id");
            let baseline = mutation["baselineCaseId"].as_str().expect("baseline id");
            assert_ne!(id, baseline, "mutation cannot reference itself");
            let baseline_case = by_id
                .get(baseline)
                .unwrap_or_else(|| panic!("case {id}: missing baseline {baseline}"));
            assert_eq!(
                case["dimensions"], baseline_case["dimensions"],
                "case {id}: mutation must preserve canvas dimensions"
            );
            assert!(
                !case["groundTruth"]["defects"]
                    .as_array()
                    .expect("defects")
                    .is_empty(),
                "case {id}: mutation requires targeted defect ground truth"
            );
        }
    }

    let spacing = by_id
        .get("opaque-dashboard-spacing-mutation")
        .expect("spacing mutation remains in corpus");
    assert_eq!(
        spacing["mutation"]["baselineCaseId"],
        "opaque-dashboard-clean"
    );
}

#[test]
fn manifest_is_layered_and_internally_consistent() {
    validate_manifest_contract(&manifest());
}

#[test]
fn every_case_executes_through_the_public_binary() {
    let manifest = manifest();
    validate_manifest_contract(&manifest);
    for case in manifest["cases"].as_array().expect("cases") {
        let id = case["id"].as_str().expect("case id");
        let path = case_file(case);
        let expected_exit = i32::try_from(case["expectedExit"].as_i64().expect("expected exit"))
            .expect("exit code fits i32");
        let first = run_adapt(&path);
        assert_eq!(
            first.status.code(),
            Some(expected_exit),
            "case {id}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&first.stdout),
            String::from_utf8_lossy(&first.stderr)
        );

        if expected_exit == EXIT_ERROR {
            assert!(first.stdout.is_empty(), "case {id}: error emitted stdout");
            let expected = case["stderrContains"].as_str().expect("error excerpt");
            let stderr = String::from_utf8_lossy(&first.stderr);
            assert!(
                stderr.contains(expected),
                "case {id}: expected {expected:?} in {stderr:?}"
            );
            let repeated = run_adapt(&path);
            assert_eq!(first.status.code(), repeated.status.code(), "case {id}");
            assert_eq!(first.stderr, repeated.stderr, "case {id}: error changed");
            continue;
        }

        assert_eq!(expected_exit, EXIT_SUCCESS, "case {id}: unsupported exit contract");
        assert!(first.stderr.is_empty(), "case {id}: valid case emitted stderr");
        let document: Value = serde_json::from_slice(&first.stdout)
            .unwrap_or_else(|error| panic!("case {id}: stdout is not JSON: {error}"));
        for assertion in case["currentAssertions"].as_array().expect("assertions") {
            assert_current_assertion(&document, assertion, id);
        }

        let repeated = run_adapt(&path);
        assert_eq!(repeated.status.code(), Some(EXIT_SUCCESS), "case {id}");
        assert_eq!(first.stdout, repeated.stdout, "case {id}: adapted IR changed");

        let first_report = run_check(&path);
        let repeated_report = run_check(&path);
        assert_eq!(first_report.status.code(), Some(EXIT_SUCCESS), "case {id}");
        assert_eq!(repeated_report.status.code(), Some(EXIT_SUCCESS), "case {id}");
        assert_eq!(
            first_report.stdout, repeated_report.stdout,
            "case {id}: report changed"
        );
    }
}
