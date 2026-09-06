//! Public-binary evaluation of exact source-alpha acquisition over realistic local assets.
//!
//! Acquisition truth and rule truth are separate reviewed documents. This suite does not claim
//! an executable padding rule, composited visibility, or real-world UI/UX accuracy.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Map, Value, json};

const EXTENSION: &str = "org.sightlint.adapter.png";
const CORPUS_PATH: &str = "evaluation/image-alpha/corpus.json";
const ACQUISITION_PATH: &str = "evaluation/image-alpha/annotations/acquisition.json";
const RULES_PATH: &str = "evaluation/image-alpha/annotations/rules.json";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn load_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read committed evaluation JSON"))
        .expect("valid committed evaluation JSON")
}

fn object<'a>(value: &'a Value, context: &str) -> &'a Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"))
}

fn cases_by_id<'a>(document: &'a Value, context: &str) -> BTreeMap<&'a str, &'a Value> {
    let mut result = BTreeMap::new();
    for case in document["cases"].as_array().expect("case array") {
        let id = case["caseId"]
            .as_str()
            .unwrap_or_else(|| panic!("{context} case ID"));
        assert!(
            result.insert(id, case).is_none(),
            "duplicate {context} {id}"
        );
    }
    result
}

fn resolve_asset(root: &Path, relative: &str) -> PathBuf {
    let relative = Path::new(relative);
    assert!(!relative.is_absolute());
    assert!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    );
    let path = root.join(relative).canonicalize().expect("asset path");
    assert!(path.starts_with(root));
    path
}

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn run_stdin(args: &[&str], bytes: &[u8]) -> Output {
    let mut child = binary()
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
        .write_all(bytes)
        .expect("write PNG");
    child.wait_with_output().expect("collect public output")
}

fn run_file(verb: &str, path: &Path) -> Output {
    let mut command = binary();
    command.arg(verb).arg(path).stdin(Stdio::null());
    if verb == "check-image" {
        command.args(["--format", "json"]);
    }
    command.output().expect("run public file command")
}

fn success_json(output: &Output, id: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{id}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{id}: unexpected stderr");
    serde_json::from_slice(&output.stdout).expect("public JSON")
}

fn expected_alpha(annotation: &Value) -> Value {
    let mut expected = annotation["alpha"].clone();
    object_mut(&mut expected, "alpha oracle").extend([
        ("version".to_owned(), json!("0.1.0")),
        ("status".to_owned(), json!("available")),
        (
            "sourceAlphaEncoding".to_owned(),
            json!("unassociatedPngEncodedAlpha8"),
        ),
        ("visiblePredicate".to_owned(), json!("alphaGreaterThanZero")),
        ("opaquePredicate".to_owned(), json!("alphaEquals255")),
        ("coordinateSpaceId".to_owned(), json!("canvas")),
    ]);
    object_mut(&mut expected, "alpha oracle").remove("expectedInkBox");
    expected
}

fn object_mut<'a>(value: &'a mut Value, context: &str) -> &'a mut Map<String, Value> {
    value
        .as_object_mut()
        .unwrap_or_else(|| panic!("{context} must be an object"))
}

fn assert_ink_box(ir: &Value, annotation: &Value, id: &str) {
    let actual = ir["nodes"][0]["geometry"].get("inkBox");
    match annotation["alpha"]["expectedInkBox"].as_array() {
        Some(bounds) => {
            let actual = actual.expect("visible source alpha needs inkBox");
            assert_eq!(actual["coordinateSpaceId"], "canvas", "{id}");
            assert_eq!(actual["evidenceId"], "evidence:png-alpha", "{id}");
            for (field, index) in [("x", 0), ("y", 1), ("width", 2), ("height", 3)] {
                assert_eq!(
                    actual["rect"][field].as_f64(),
                    bounds[index].as_f64(),
                    "{id}: inkBox {field}"
                );
            }
        }
        None => assert!(actual.is_none(), "{id}: invented empty inkBox"),
    }
}

fn assert_acquisition(ir: &Value, annotation: &Value, id: &str) {
    let extension = &ir["extensions"][EXTENSION];
    assert_eq!(extension["version"], "0.2.0", "{id}");
    assert_eq!(
        extension["alphaGeometry"],
        expected_alpha(annotation),
        "{id}"
    );
    assert!(extension["alphaGeometry"].get("pixels").is_none());
    assert!(extension["encodedRgba8Raster"].get("pixels").is_none());
    assert_ink_box(ir, annotation, id);
    let evidence = ir["evidence"]
        .as_array()
        .expect("evidence")
        .iter()
        .find(|item| item["id"] == "evidence:png-alpha")
        .expect("alpha evidence");
    assert_eq!(evidence["class"], "exactSource", "{id}");
    assert_eq!(evidence["source"]["externalProcessing"], false, "{id}");
    assert_eq!(
        evidence["selector"]["nativeId"], "IDAT/encoded-rgba8-v1/alpha8",
        "{id}"
    );
}

fn assert_governance(corpus: &Value) {
    assert_eq!(corpus["schemaVersion"], "0.1.0");
    assert_eq!(corpus["source"]["ownership"], "sightlintRepository");
    assert_eq!(corpus["source"]["license"], "MIT OR Apache-2.0");
    assert_eq!(corpus["source"]["privacyReview"], "syntheticNoPersonalData");
    assert_eq!(corpus["source"]["externalAssets"], false);
    assert_eq!(
        corpus["dataGovernance"]["implementationOutputIsOracle"],
        false
    );
    assert_eq!(corpus["splitPolicy"]["holdout"]["status"], "notEstablished");
}

fn evaluate_case(
    root: &Path,
    corpus: &Value,
    case: &Value,
    annotation: &Value,
    rule: &Value,
) -> Value {
    let id = case["id"].as_str().expect("case ID");
    assert!(rule["executableRule"].is_null(), "{id}");
    assert_eq!(rule["expectedOutcome"], "untested", "{id}");
    assert_eq!(rule["blockingAllowed"], false, "{id}");
    assert!(
        matches!(
            rule["applicabilityGroundTruth"].as_str(),
            Some("cantTell" | "inapplicable")
        ),
        "{id}"
    );

    let path = resolve_asset(root, case["path"].as_str().expect("asset path"));
    let bytes = std::fs::read(&path).expect("read asset");
    let first = run_stdin(&["adapt-image", "-"], &bytes);
    let ir = success_json(&first, id);
    assert_acquisition(&ir, annotation, id);
    for _ in 1..corpus["gates"]["determinismRuns"].as_u64().expect("runs") {
        let repeated = run_stdin(&["adapt-image", "-"], &bytes);
        success_json(&repeated, id);
        assert_eq!(first.stdout, repeated.stdout, "{id}: unstable stdin IR");
    }

    let normalized = run_stdin(&["normalize", "-"], &first.stdout);
    success_json(&normalized, id);
    assert_eq!(first.stdout, normalized.stdout, "{id}: non-canonical IR");
    let file_first = run_file("adapt-image", &path);
    assert_acquisition(&success_json(&file_first, id), annotation, id);
    let file_repeated = run_file("adapt-image", &path);
    success_json(&file_repeated, id);
    assert_eq!(
        file_first.stdout, file_repeated.stdout,
        "{id}: unstable file IR"
    );

    let checked = run_stdin(&["check-image", "-", "--format", "json"], &bytes);
    let report = success_json(&checked, id);
    assert_eq!(report["summary"]["failed"], 0, "{id}");
    assert!(
        report["results"]
            .as_array()
            .expect("rule results")
            .iter()
            .all(|result| !result["ruleId"].as_str().unwrap_or("").contains("alpha")),
        "{id}: this slice must not invent an alpha rule"
    );
    let file_checked = run_file("check-image", &path);
    success_json(&file_checked, id);
    assert_eq!(
        checked.stdout, file_checked.stdout,
        "{id}: file/stdin report"
    );
    ir
}

#[test]
fn realistic_assets_match_independent_acquisition_oracles_via_public_binary() {
    let root = repository_root();
    let corpus = load_json(&root.join(CORPUS_PATH));
    let acquisition = load_json(&root.join(ACQUISITION_PATH));
    let rules = load_json(&root.join(RULES_PATH));
    let acquisition = cases_by_id(&acquisition, "acquisition");
    let rules = cases_by_id(&rules, "rule");

    assert_governance(&corpus);

    let cases = corpus["cases"].as_array().expect("corpus cases");
    assert_eq!(cases.len(), 5);
    assert_eq!(acquisition.len(), cases.len());
    assert_eq!(rules.len(), cases.len());
    let mut outputs = BTreeMap::new();
    let mut hard_negatives = 0;
    let mut abstentions = 0;

    for case in cases {
        let id = case["id"].as_str().expect("case ID");
        let annotation = acquisition.get(id).expect("acquisition oracle");
        let rule = rules.get(id).expect("separate rule oracle");
        abstentions += 1;
        if case["classification"] == "hardNegative" {
            hard_negatives += 1;
        }
        outputs.insert(id, evaluate_case(&root, &corpus, case, annotation, rule));
    }

    assert_eq!(hard_negatives, 2);
    assert_eq!(abstentions, 5);
    println!(
        "source-alpha acquisition: 5/5 exact-oracle matches, 5/5 rule abstentions, 2/2 hard negatives nonblocking; no real-world UI/UX accuracy claimed"
    );

    let baseline = &outputs["northstar-compass"];
    let hidden = &outputs["northstar-compass-hidden-rgb"];
    assert_eq!(
        baseline["extensions"][EXTENSION]["alphaGeometry"],
        hidden["extensions"][EXTENSION]["alphaGeometry"]
    );
    assert_eq!(
        baseline["nodes"][0]["geometry"].get("inkBox"),
        hidden["nodes"][0]["geometry"].get("inkBox")
    );
    assert_ne!(
        baseline["extensions"][EXTENSION]["encodedRgba8Raster"]["byteCrc32"],
        hidden["extensions"][EXTENSION]["encodedRgba8Raster"]["byteCrc32"]
    );

    let padded = &outputs["northstar-compass-padded"];
    let base_alpha = &baseline["extensions"][EXTENSION]["alphaGeometry"];
    let padded_alpha = &padded["extensions"][EXTENSION]["alphaGeometry"];
    assert_ne!(base_alpha["visibleBounds"], padded_alpha["visibleBounds"]);
    assert_eq!(
        base_alpha["pixelCounts"]["visible"],
        padded_alpha["pixelCounts"]["visible"]
    );
    assert_eq!(
        base_alpha["pixelCounts"]["opaque"],
        padded_alpha["pixelCounts"]["opaque"]
    );
    assert_eq!(
        padded_alpha["visibleBounds"],
        json!([13, 10, 33, 31]),
        "targeted padding mutation must be acquired"
    );
    println!(
        "source-alpha mutation kill rate: 1/1 acquisition mutation; rule mutation rate untested"
    );
}

#[test]
fn evaluation_contract_keeps_splits_and_rule_truth_separate() {
    let root = repository_root();
    let corpus = load_json(&root.join(CORPUS_PATH));
    let acquisition = load_json(&root.join(ACQUISITION_PATH));
    let rules = load_json(&root.join(RULES_PATH));
    assert_eq!(acquisition["documentType"], "acquisitionOracle");
    assert_eq!(rules["documentType"], "ruleOracle");
    assert_ne!(acquisition, rules);
    let splits = corpus["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| case["split"].as_str().expect("split"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        splits,
        BTreeSet::from(["challenge", "development", "smoke"])
    );
    assert_eq!(corpus["splitPolicy"]["holdout"]["status"], "notEstablished");
    assert_eq!(
        object(&corpus["gates"], "gates")["maximumBlockingFindings"],
        0
    );
}
