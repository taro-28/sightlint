//! Public-binary coverage for the evaluation-only segmentation benchmark surface.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use sightlint_adapter_png::segmentation::benchmark_png_segmentation;

const POLICIES: [&str; 3] = [
    "qualified-corner-95-row-runs-v1",
    "ranked-exact-border-flood-v1",
    "strict-uniform-perimeter-flood-v1",
];

fn corpus(directory: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(directory)
        .join("corpus.json");
    let value: Value = serde_json::from_slice(&fs::read(path).expect("committed corpus"))
        .expect("valid corpus JSON");
    value["cases"].as_array().expect("case array").clone()
}

fn bytes(case: &Value, rasters: &[Value]) -> Vec<u8> {
    let source = if let Some(id) = case["rasterCase"].as_str() {
        rasters
            .iter()
            .find(|item| item["id"] == id)
            .expect("source raster case")
    } else {
        case
    };
    let hex = source["pngHex"].as_str().expect("native PNG hex");
    (0..hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).expect("hex byte"))
        .collect()
}

fn case<'a>(cases: &'a [Value], id: &str) -> &'a Value {
    cases
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("missing case {id}"))
}

fn run(arguments: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sightlint"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn public binary");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write PNG input");
    child.wait_with_output().expect("collect public output")
}

fn policy<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["policies"]
        .as_array()
        .expect("policy array")
        .iter()
        .find(|item| item["policyId"] == id)
        .unwrap_or_else(|| panic!("missing policy {id}"))
}

fn benchmark(input: &[u8]) -> Value {
    let first = run(&["benchmark-image-segmentation", "-"], input);
    assert_eq!(first.status.code(), Some(0));
    assert!(first.stderr.is_empty());
    for _ in 0..2 {
        let repeated = run(&["benchmark-image-segmentation", "-"], input);
        assert_eq!(repeated.status.code(), first.status.code());
        assert_eq!(repeated.stdout, first.stdout);
        assert_eq!(repeated.stderr, first.stderr);
    }
    let native = benchmark_png_segmentation(input).expect("native benchmark");
    assert_eq!(
        first.stdout,
        native.to_canonical_json().unwrap().as_bytes(),
        "public and native APIs must agree byte-for-byte"
    );
    serde_json::from_slice(&first.stdout).expect("benchmark JSON")
}

#[test]
fn public_benchmark_compares_named_policies_without_rule_results() {
    let rasters = corpus("png-raster");
    let inspections = corpus("image-inspection");
    let clean = bytes(case(&inspections, "cards-clean"), &rasters);
    let report = benchmark(&clean);
    assert_eq!(report["benchmarkSchemaVersion"], "0.1.0");
    assert_eq!(report["mode"], "evaluationOnly");
    assert_eq!(report["blocking"], false);
    assert_eq!(report["ruleOutcome"], "untested");
    assert_eq!(report["source"]["externalProcessing"], false);
    let identifiers = report["policies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["policyId"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(identifiers, POLICIES.into_iter().collect());
    for id in POLICIES {
        let result = policy(&report, id);
        assert_eq!(result["status"], "observed");
        assert_eq!(result["semanticApplicability"], "cantTell");
        assert_eq!(result["ruleOutcome"], "untested");
        assert_eq!(result["backgroundSelection"]["confirmed"], false);
        assert!(result["backgroundSelection"]["semanticConfidence"].is_null());
    }
    assert_eq!(
        policy(&report, "strict-uniform-perimeter-flood-v1")["regions"],
        policy(&report, "qualified-corner-95-row-runs-v1")["regions"],
        "row-run and flood-fill implementations must agree under one candidate"
    );
}

#[test]
fn candidate_and_resource_abstention_are_explicit_and_partial_free() {
    let rasters = corpus("png-raster");
    let inspections = corpus("image-inspection");
    let noisy = benchmark(&bytes(case(&inspections, "border-noise"), &rasters));
    assert_eq!(
        policy(&noisy, "strict-uniform-perimeter-flood-v1")["reason"],
        "nonUniformBorder"
    );
    for id in [
        "ranked-exact-border-flood-v1",
        "qualified-corner-95-row-runs-v1",
    ] {
        assert_eq!(policy(&noisy, id)["status"], "observed");
    }

    let alpha = benchmark(&bytes(case(&inspections, "alpha-one"), &rasters));
    for id in POLICIES {
        let result = policy(&alpha, id);
        assert_eq!(result["status"], "unavailable");
        assert_eq!(result["reason"], "nonOpaqueRaster");
        assert_eq!(result["regions"], serde_json::json!([]));
        assert!(result["backgroundSelection"]["selectedCandidateId"].is_null());
    }

    let indexed = benchmark(&bytes(case(&inspections, "indexed"), &rasters));
    assert_eq!(indexed["evidence"][0]["id"], "source");
    for id in POLICIES {
        assert_eq!(policy(&indexed, id)["reason"], "indexedColor");
        assert_eq!(policy(&indexed, id)["regions"], serde_json::json!([]));
    }
}

#[test]
fn malformed_input_and_usage_keep_the_public_error_contract() {
    for arguments in [
        vec!["benchmark-image-segmentation"],
        vec!["benchmark-image-segmentation", "-", "--format", "json"],
    ] {
        let output = run(&arguments, &[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let malformed = run(&["benchmark-image-segmentation", "-"], b"not a PNG");
    assert_eq!(malformed.status.code(), Some(2));
    assert!(malformed.stdout.is_empty());
    assert!(!malformed.stderr.is_empty());
}
