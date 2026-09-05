//! Native image-to-region-to-gap observations, not a claimed semantic UX classifier.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use sightlint_adapter_png::inspection::inspect_png;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn corpus(directory: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(directory)
        .join("corpus.json");
    let value: Value = serde_json::from_slice(&fs::read(path).expect("committed corpus"))
        .expect("valid corpus JSON");
    assert_eq!(value["version"], "0.1.0");
    value["cases"].as_array().expect("case array").clone()
}

fn bytes(case: &Value, rasters: &[Value]) -> Vec<u8> {
    let source = if let Some(id) = case["rasterCase"].as_str() {
        rasters
            .iter()
            .find(|item| item["id"] == id)
            .expect("source case")
    } else {
        case
    };
    let hex = source["pngHex"].as_str().expect("committed native bytes");
    assert_eq!(hex.len() % 2, 0);
    assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    (0..hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).expect("hex byte"))
        .collect()
}

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sightlint"))
        .args(args)
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
        .expect("write input");
    child.wait_with_output().expect("collect output")
}

fn success(output: &Output, id: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{id}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{id}: unexpected stderr");
}

struct TempImage(PathBuf);

impl TempImage {
    fn new(bytes: &[u8]) -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sightlint-inspection-{}-{sequence}.png",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("unique temporary PNG");
        file.write_all(bytes).expect("write native fixture");
        Self(path)
    }

    fn inspect(&self, format: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sightlint"))
            .arg("inspect-image")
            .arg(&self.0)
            .args(["--format", format])
            .output()
            .expect("file inspection")
    }
}

impl Drop for TempImage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn assert_observations(case: &Value, report: &Value) {
    let expected = &case["expected"];
    let id = case["id"].as_str().expect("case id");
    assert_eq!(report["inspectionSchemaVersion"], "0.1.0", "{id}");
    assert_eq!(report["mode"], "advisory", "{id}");
    assert_eq!(report["blocking"], false, "{id}");
    assert_eq!(report["status"], expected["status"], "{id}");
    assert_eq!(report["source"]["externalProcessing"], false, "{id}");
    assert_eq!(report["canvas"]["unit"], "devicePixel", "{id}");
    let regions = report["regions"].as_array().expect("regions");
    let groups = report["groups"].as_array().expect("groups");
    let bounds: Vec<Value> = regions
        .iter()
        .map(|region| region["bounds"].clone())
        .collect();
    assert_eq!(json!(bounds), expected["bounds"], "{id}: acquired bounds");
    let measured: Vec<Value> = groups
        .iter()
        .map(|group| {
            json!({
                "axis": group["axis"], "gaps": group["gaps"], "pattern": group["pattern"]
            })
        })
        .collect();
    assert_eq!(json!(measured), expected["groups"], "{id}: acquired gaps");
    assert_eq!(report["summary"]["regionCount"], regions.len(), "{id}");
    assert_eq!(report["summary"]["groupCount"], groups.len(), "{id}");
    let unequal = groups
        .iter()
        .filter(|group| group["pattern"] == "unequal")
        .count();
    assert_eq!(report["summary"]["unequalGapGroupCount"], unequal, "{id}");
    if expected["status"] == "unavailable" {
        assert_eq!(report["reason"], expected["reason"], "{id}");
        assert_eq!(report["uxVerdict"], "untested", "{id}");
        assert!(report.get("backgroundHypothesis").is_none());
    } else {
        assert_eq!(report["uxVerdict"], "cantTell", "{id}");
        assert_eq!(report["backgroundHypothesis"]["confirmed"], false);
        assert_eq!(
            report["backgroundHypothesis"]["calibration"],
            "notCalibrated"
        );
        assert_eq!(
            report["backgroundHypothesis"]["semanticConfidence"],
            Value::Null
        );
        assert_links_and_measurements(report);
    }
}

fn assert_links_and_measurements(report: &Value) {
    let regions = report["regions"].as_array().expect("regions");
    let ids: BTreeSet<&str> = regions
        .iter()
        .map(|r| r["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids.len(), regions.len());
    assert_eq!(report["evidence"][0]["id"], "raster");
    for region in regions {
        assert_eq!(region["evidenceId"], "raster");
        assert_eq!(region["hypothesisId"], "border-background");
        assert_eq!(region["boundsFormat"], "xywh-half-open");
        assert!(region.get("role").is_none());
        assert!(region.get("hitBox").is_none());
    }
    for group in report["groups"].as_array().expect("groups") {
        assert_eq!(group["blocking"], false);
        assert_eq!(group["uxVerdict"], "cantTell");
        assert_eq!(group["semanticConfidence"], Value::Null);
        assert_eq!(group["calibration"], "notCalibrated");
        assert_eq!(group["evidenceIds"], json!(["raster"]));
        for member in group["regionIds"].as_array().expect("members") {
            assert!(ids.contains(member.as_str().expect("member id")));
        }
        let gaps: Vec<u64> = group["gaps"]
            .as_array()
            .expect("gaps")
            .iter()
            .map(|gap| gap.as_u64().expect("integer gap"))
            .collect();
        let minimum = *gaps.iter().min().expect("at least two gaps");
        let maximum = *gaps.iter().max().expect("at least two gaps");
        assert_eq!(group["minimumGap"], minimum);
        assert_eq!(group["maximumGap"], maximum);
        assert_eq!(group["gapSpread"], maximum - minimum);
        assert_eq!(group["unit"], "devicePixel");
    }
}

fn verify_valid_case(case: &Value, png: &[u8]) {
    let id = case["id"].as_str().expect("id");
    let first = run(&["inspect-image", "-", "--format", "json"], png);
    success(&first, id);
    let report: Value = serde_json::from_slice(&first.stdout).expect("inspection JSON");
    assert_observations(case, &report);
    let api = inspect_png(png).expect("native inspection");
    assert_eq!(
        first.stdout,
        api.to_canonical_json().unwrap().as_bytes(),
        "{id}"
    );
    let file = TempImage::new(png);
    let file_output = file.inspect("json");
    success(&file_output, id);
    assert_eq!(first.stdout, file_output.stdout, "{id}: file/stdin");
    let human = run(&["inspect-image", "-"], png);
    success(&human, id);
    assert_eq!(human.stdout, api.to_human().as_bytes(), "{id}: human/API");
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("advisory only"));
    let unequal = report["summary"]["unequalGapGroupCount"].as_u64().unwrap() > 0;
    assert_eq!(text.contains("ADVISORY unequal gaps"), unequal, "{id}");
    for _ in 0..3 {
        let repeated = run(&["inspect-image", "-", "--format", "json"], png);
        success(&repeated, id);
        assert_eq!(first.stdout, repeated.stdout, "{id}: determinism");
    }
    // A heuristic observation must not alter the existing trusted command path or exit policy.
    let adapted = run(&["adapt-image", "-"], png);
    success(&adapted, id);
    let direct = run(&["check-image", "-", "--format", "json"], png);
    let indirect = run(&["check", "-", "--format", "json"], &adapted.stdout);
    success(&direct, id);
    success(&indirect, id);
    assert_eq!(
        direct.stdout, indirect.stdout,
        "{id}: trusted pipeline drift"
    );
}

#[test]
fn committed_native_corpus_matches_regions_gaps_abstentions_and_real_cli() {
    let rasters = corpus("png-raster");
    let cases = corpus("image-inspection");
    for case in &cases {
        let png = bytes(case, &rasters);
        if case["expected"]["exitCode"] == 2 {
            assert!(inspect_png(&png).is_err());
            let file = TempImage::new(&png);
            for format in ["human", "json"] {
                let output = run(&["inspect-image", "-", "--format", format], &png);
                let repeated = run(&["inspect-image", "-", "--format", format], &png);
                for error in [&output, &repeated, &file.inspect(format)] {
                    assert_eq!(error.status.code(), Some(2));
                    assert!(error.stdout.is_empty());
                    assert!(!error.stderr.is_empty());
                }
                assert_eq!(output.stderr, repeated.stderr);
            }
        } else {
            verify_valid_case(case, &png);
        }
    }
    println!(
        "{} native inspection cases verified; no semantic UX accuracy claimed",
        cases.len()
    );
}

#[test]
fn mutation_is_observed_without_promoting_ambiguous_design_intent_to_failure() {
    let cases = corpus("image-inspection");
    let rasters = corpus("png-raster");
    let inspect = |id: &str| {
        let case = cases
            .iter()
            .find(|case| case["id"] == id)
            .expect("named case");
        let output = run(
            &["inspect-image", "-", "--format", "json"],
            &bytes(case, &rasters),
        );
        success(&output, id);
        serde_json::from_slice::<Value>(&output.stdout).expect("report")
    };
    let clean = inspect("cards-clean");
    let changed = inspect("cards-mutated");
    assert_eq!(clean["groups"][0]["gaps"], json!([1, 1]));
    assert_eq!(changed["groups"][0]["gaps"], json!([1, 2]));
    assert_eq!(changed, inspect("intentional-grouping"));
    assert_eq!(changed["uxVerdict"], "cantTell");
    for id in ["translated", "recolored"] {
        assert_eq!(
            changed["groups"][0]["gaps"],
            inspect(id)["groups"][0]["gaps"]
        );
    }
    assert_eq!(inspect("scaled")["groups"][0]["gaps"], json!([2, 4]));
    for id in ["hollow", "mixed-region"] {
        assert_eq!(inspect(id)["regions"][0]["singleColorRectangle"], false);
    }
    assert_eq!(inspect("hollow")["regions"][0]["pixelCount"], 16);
    for id in ["cards-clean", "cards-mutated"] {
        let source = rasters
            .iter()
            .find(|case| case["id"] == id)
            .expect("source");
        assert_eq!(source["future"]["status"], "untested");
    }
}

#[test]
fn corpus_integrity_and_cli_usage_are_explicit() {
    let cases = corpus("image-inspection");
    let ids: BTreeSet<&str> = cases
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 30);
    assert_eq!(cases.len(), 30);
    assert_eq!(
        cases
            .iter()
            .filter(|c| c["expected"]["status"] == "observed")
            .count(),
        19
    );
    assert_eq!(
        cases
            .iter()
            .filter(|c| c["expected"]["status"] == "unavailable")
            .count(),
        9
    );
    for id in [
        "blocker",
        "different-size",
        "different-color",
        "diagonal",
        "touching",
        "two-rows",
        "intentional-grouping",
    ] {
        assert!(ids.contains(id), "missing negative/structural control {id}");
    }
    for args in [
        vec!["inspect-image"],
        vec!["inspect-image", "-", "--format", "xml"],
        vec!["inspect-image", "-", "--deny-cant-tell"],
    ] {
        assert_eq!(run(&args, &[]).status.code(), Some(2));
    }
}
