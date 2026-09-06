//! Public-process evaluation for the bounded iOS capture adapter.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

const EXIT_SUCCESS: i32 = 0;
const EXIT_FINDINGS: i32 = 1;
const EXIT_ERROR: i32 = 2;
const BOUNDS_RULE: &str = "visual.bounds.within-canvas";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sightlint-ios-e2e-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create iOS E2E temporary directory");
        Self(path)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn python() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn load_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read JSON fixture")).expect("parse JSON fixture")
}

fn adapter(request: &Path, output: &Path) -> Output {
    adapter_in_root(request, output, &repository_root())
}

fn adapter_in_root(request: &Path, output: &Path, repository: &Path) -> Output {
    Command::new(python())
        .arg(repository_root().join("adapters/ios/sightlint_ios.py"))
        .arg("--request")
        .arg(request)
        .arg("--repository-root")
        .arg(repository)
        .arg("--sightlint-binary")
        .arg(env!("CARGO_BIN_EXE_sightlint"))
        .arg("--artifact-ir-out")
        .arg(output)
        .output()
        .expect("execute iOS adapter")
}

fn check(artifact_ir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
        .arg("check")
        .arg(artifact_ir)
        .arg("--profile")
        .arg("base")
        .arg("--format")
        .arg("json")
        .output()
        .expect("execute public check command")
}

fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.contains(&b'\r'),
        "diagnostics must be LF-stable"
    );
}

fn item_by<'a>(items: &'a Value, field: &str, expected: &Value) -> &'a Value {
    items
        .as_array()
        .expect("items are an array")
        .iter()
        .find(|item| item.get(field) == Some(expected))
        .unwrap_or_else(|| panic!("missing item with {field}={expected}"))
}

fn acquisition_node_facts(document: &Value, extension: &Value, expected: &Value) -> u32 {
    let identifier = &expected["identifier"];
    let actual = item_by(&extension["sourceNodes"], "identifier", identifier);
    for field in ["className", "mappingStatus", "layoutBoundsPoints"] {
        assert_eq!(
            actual[field], expected[field],
            "node {identifier} field {field}"
        );
    }
    assert_eq!(
        !actual["windowIntersectionPoints"].is_null(),
        expected["windowVisible"].as_bool().unwrap(),
        "node {identifier} window visibility"
    );
    assert_eq!(actual["xcuiReconciliation"], expected["xcuiReconciliation"]);
    let xcui = extension["xcuiNodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["identifier"] == *identifier);
    if expected["xcuiFrameStatus"] == "unavailable" {
        assert!(xcui.is_none());
        assert!(expected["xcuiFramePoints"].is_null());
        assert!(expected["xcuiHittable"].is_null());
    } else {
        let xcui = xcui.expect("reviewed XCUI node is present");
        assert_eq!(xcui["frameStatus"], expected["xcuiFrameStatus"]);
        assert_eq!(xcui["framePoints"], expected["xcuiFramePoints"]);
        assert_eq!(xcui["hittable"], expected["xcuiHittable"]);
        let evidence = item_by(&document["evidence"], "id", &xcui["evidenceId"]);
        assert_eq!(evidence["class"], "platformSemantics");
        assert_eq!(
            evidence["selector"]["nativeId"],
            json!(format!("ios:xcui:{}", identifier.as_str().unwrap()))
        );
    }
    for field in ["label", "value"] {
        if let Some(value) = expected.get(field) {
            assert_eq!(actual[field], *value, "node {identifier} field {field}");
            if let Some(xcui) = xcui {
                assert_eq!(xcui[field], *value, "XCUI node {identifier} field {field}");
            }
        }
    }

    let core_id = json!(format!("ios:view:{}", identifier.as_str().unwrap()));
    let core_node = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == core_id);
    if expected["mappingStatus"] == "mappedExactLayout" {
        let core_node = core_node.expect("mapped acquisition node has a core node");
        assert_eq!(core_node["kind"]["value"], expected["coreKind"]);
        for coordinate in ["x", "y", "width", "height"] {
            assert_eq!(
                core_node["geometry"]["layoutBox"]["rect"][coordinate].as_f64(),
                expected["layoutBoundsPoints"][coordinate].as_f64(),
                "node {identifier} core layout coordinate {coordinate}"
            );
        }
        assert!(core_node["geometry"].get("hitBox").is_none());
        assert!(core_node["geometry"].get("renderBox").is_none());
        assert!(core_node["geometry"].get("inkBox").is_none());
        let evidence = item_by(&document["evidence"], "id", &actual["evidenceId"]);
        assert_eq!(evidence["class"], "exactSource");
        assert_eq!(evidence["selector"]["nativeId"], core_id);
    } else {
        assert!(
            core_node.is_none(),
            "excluded acquisition node became a core node"
        );
    }
    10 + u32::from(expected.get("label").is_some()) + u32::from(expected.get("value").is_some())
}

fn acquisition_facts(document: &Value, oracle: &Value) -> u32 {
    let extension = &document["extensions"]["org.sightlint.ios"];
    for field in [
        "widthPoints",
        "heightPoints",
        "scale",
        "widthPixels",
        "heightPixels",
        "orientation",
    ] {
        assert_eq!(
            extension["screen"]["display"][field], oracle["display"][field],
            "display field {field}"
        );
    }
    for field in [
        "widthPixels",
        "heightPixels",
        "extentReconciliation",
        "nodeIdentity",
    ] {
        assert_eq!(
            extension["screen"]["screenshot"][field], oracle["screenshot"][field],
            "screenshot field {field}"
        );
    }
    assert_eq!(
        extension["sourceNodes"].as_array().unwrap().len(),
        usize::try_from(oracle["counts"]["sourceNodes"].as_u64().unwrap()).unwrap()
    );
    assert_eq!(
        extension["xcuiNodes"].as_array().unwrap().len(),
        usize::try_from(oracle["counts"]["xcuiNodes"].as_u64().unwrap()).unwrap()
    );
    assert_eq!(
        document["nodes"].as_array().unwrap().len(),
        usize::try_from(oracle["counts"]["mappedCoreNodes"].as_u64().unwrap()).unwrap()
    );
    assert_eq!(
        extension["unsupported"]["unidentifiedSourceNodeCount"],
        oracle["counts"]["unidentifiedSourceNodes"]
    );
    assert_eq!(
        extension["unsupported"]["unmatchedXcuiQueryCount"],
        oracle["counts"]["unmatchedXcuiQueries"]
    );
    assert_eq!(
        extension["screen"]["safeAreaInsetsPoints"],
        oracle["safeAreaInsetsPoints"]
    );

    let mut facts = 11_u32;
    for expected in oracle["nodes"].as_array().expect("annotated nodes") {
        facts += acquisition_node_facts(document, extension, expected);
    }
    facts
}

fn rule_observation(report: &Value, oracle: &Value) -> (u32, u32) {
    let rule_results = report["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|result| result["ruleId"] == BOUNDS_RULE)
        .collect::<Vec<_>>();
    assert_eq!(
        rule_results.len(),
        usize::try_from(oracle["expectedResultCount"].as_u64().unwrap()).unwrap()
    );
    for target in oracle["expectedAbsentTargets"].as_array().unwrap() {
        assert!(
            rule_results
                .iter()
                .all(|result| result["target"]["id"] != *target),
            "excluded target produced a rule verdict: {target}"
        );
    }
    let expected_failures = oracle["expectedFailedTargets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|target| target.as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    let actual_failures = rule_results
        .iter()
        .filter(|result| result["outcome"] == "failed")
        .map(|result| {
            assert_eq!(result["evidenceClasses"], json!(["exactSource"]));
            result["target"]["id"].as_str().unwrap().to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_failures, expected_failures);
    assert!(
        rule_results
            .iter()
            .all(|result| matches!(result["outcome"].as_str(), Some("passed" | "failed")))
    );
    (
        u32::try_from(expected_failures.len()).unwrap(),
        u32::try_from(actual_failures.len()).unwrap(),
    )
}

fn assert_no_source_content(bytes: &[u8]) {
    let output = String::from_utf8_lossy(bytes);
    for private_value in [
        "Account settings",
        "Alex Morgan",
        "Workspace plan",
        "Product notifications",
        "Profile visibility",
        "Save changes",
        "Archived preferences",
    ] {
        assert!(
            !output.contains(private_value),
            "source content leaked: {private_value}"
        );
    }
}

#[derive(Default)]
struct Metrics {
    acquisition_facts: u32,
    cases: u32,
    expected_failures: u32,
    actual_failures: u32,
    retained_abstentions: u32,
    negative_cases: u32,
    false_positive_cases: u32,
    mutations: u32,
    killed_mutations: u32,
}

fn observe_case(
    root: &Path,
    temporary: &Path,
    case: &Value,
    acquisitions: &Value,
    rules: &Value,
    metrics: &mut Metrics,
) {
    let case_id = case["id"].as_str().unwrap();
    let request = root.join(case["request"]["path"].as_str().unwrap());
    let first_ir = temporary.join(format!("{case_id}-first.json"));
    let second_ir = temporary.join(format!("{case_id}-second.json"));
    let first = adapter(&request, &first_ir);
    let second = adapter(&request, &second_ir);
    assert_exit(&first, EXIT_SUCCESS);
    assert_exit(&second, EXIT_SUCCESS);
    assert!(first.stderr.is_empty() && second.stderr.is_empty());
    assert_eq!(first.stdout, second.stdout, "{case_id} response drift");
    let first_bytes = fs::read(&first_ir).unwrap();
    assert_eq!(
        first_bytes,
        fs::read(&second_ir).unwrap(),
        "{case_id} Artifact IR drift"
    );
    assert_no_source_content(&first.stdout);
    assert_no_source_content(&first_bytes);

    let response: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(response["status"], "partial");
    assert_eq!(response["coverage"]["sourceLayout"], "partial");
    assert_eq!(response["coverage"]["touchHitRegions"], "cantTell");
    assert_eq!(response["coverage"]["renderedNodeIdentity"], "cantTell");
    assert_eq!(response["coverage"]["swiftUISemantics"], "untested");
    assert_eq!(response["coverage"]["focusNavigation"], "untested");
    assert_eq!(response["externalProcessing"], false);

    let document: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(document["artifact"]["kind"], "mobile");
    let extension = &document["extensions"]["org.sightlint.ios"];
    assert_eq!(extension["privacy"]["externalProcessing"], false);
    assert_eq!(extension["privacy"]["transmittedFields"], json!([]));
    let acquisition = item_by(
        &acquisitions["annotations"],
        "id",
        &case["acquisitionAnnotationId"],
    );
    metrics.acquisition_facts += acquisition_facts(&document, acquisition);

    let oracle = item_by(&rules["annotations"], "id", &case["ruleAnnotationId"]);
    let checked = check(&first_ir);
    let expected_to_fail = !oracle["expectedFailedTargets"]
        .as_array()
        .unwrap()
        .is_empty();
    assert_exit(
        &checked,
        if expected_to_fail {
            EXIT_FINDINGS
        } else {
            EXIT_SUCCESS
        },
    );
    assert!(checked.stderr.is_empty());
    let report: Value = serde_json::from_slice(&checked.stdout).unwrap();
    let (expected, actual) = rule_observation(&report, oracle);
    metrics.expected_failures += expected;
    metrics.actual_failures += actual;

    match case["relation"]["kind"].as_str().unwrap() {
        "targetedMutation" => {
            metrics.mutations += 1;
            metrics.killed_mutations += u32::from(actual == expected && actual > 0);
        }
        "baseline" | "hardNegative" => {
            metrics.negative_cases += 1;
            metrics.false_positive_cases += u32::from(actual > 0);
        }
        role => panic!("unexpected case role {role}"),
    }
    if case["relation"]["kind"] == "hardNegative" {
        let absent = oracle["expectedAbsentTargets"].as_array().unwrap();
        assert!(
            absent
                .iter()
                .any(|target| target.as_str().unwrap().ends_with("archived_title"))
        );
        assert!(
            absent
                .iter()
                .any(|target| target.as_str().unwrap().ends_with("archived_detail"))
        );
        assert!(
            absent
                .iter()
                .any(|target| target.as_str().unwrap().ends_with("settings_content"))
        );
        metrics.retained_abstentions += 1;
    }
    metrics.cases += 1;
}

#[test]
fn public_ios_corpus_separates_acquisition_and_rule_ground_truth() {
    let root = repository_root();
    let corpus = load_json(root.join("evaluation/ios/corpus.json"));
    let acquisitions = load_json(root.join("evaluation/ios/annotations/acquisition.json"));
    let rules = load_json(root.join("evaluation/ios/annotations/rules.json"));
    let contract = load_json(root.join("evaluation/ios/metric-contract.json"));
    let temporary = TempDirectory::new();
    let mut metrics = Metrics::default();

    assert_eq!(corpus["holdout"]["status"], "notEstablished");
    assert_eq!(corpus["source"]["license"], "MIT OR Apache-2.0");
    assert_eq!(
        acquisitions["provenance"]["implementationOutputUsed"],
        false
    );
    assert_eq!(rules["provenance"]["implementationOutputUsed"], false);
    assert_eq!(contract["implementationOutputsStoredAsOracle"], false);

    for case in corpus["cases"].as_array().unwrap() {
        observe_case(
            &root,
            &temporary.0,
            case,
            &acquisitions,
            &rules,
            &mut metrics,
        );
    }

    assert_eq!(metrics.acquisition_facts, 122);
    assert_eq!(metrics.cases, 3);
    assert_eq!(metrics.actual_failures, metrics.expected_failures);
    assert_eq!(metrics.retained_abstentions, 1);
    assert_eq!(metrics.false_positive_cases, 0);
    assert_eq!(metrics.killed_mutations, metrics.mutations);
    let observed = BTreeMap::from([
        ("acquisitionFactCoverage", 1.0),
        ("evaluatedCaseCoverage", f64::from(metrics.cases) / 3.0),
        (
            "failurePrecision",
            f64::from(metrics.expected_failures) / f64::from(metrics.actual_failures),
        ),
        (
            "abstentionRetention",
            f64::from(metrics.retained_abstentions),
        ),
        (
            "falsePositiveRate",
            f64::from(metrics.false_positive_cases) / f64::from(metrics.negative_cases),
        ),
        (
            "mutationKillRate",
            f64::from(metrics.killed_mutations) / f64::from(metrics.mutations),
        ),
    ]);
    for metric in contract["metrics"].as_array().unwrap() {
        let id = metric["id"].as_str().unwrap();
        let value = observed[id];
        if let Some(minimum) = metric.get("requiredMinimum") {
            assert!(
                value >= minimum.as_f64().unwrap(),
                "metric {id} below minimum"
            );
        }
        if let Some(maximum) = metric.get("requiredMaximum") {
            assert!(
                value <= maximum.as_f64().unwrap(),
                "metric {id} above maximum"
            );
        }
    }
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn python_sha256(path: &Path) -> String {
    let output = Command::new(python())
        .arg("-c")
        .arg("import hashlib,sys; print('sha256:'+hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())")
        .arg(path)
        .output()
        .expect("compute test digest");
    assert_exit(&output, EXIT_SUCCESS);
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn assert_error(output: &Output, code: &str, artifact_ir: &Path) {
    assert_exit(output, EXIT_ERROR);
    assert!(output.stdout.is_empty());
    assert!(!artifact_ir.exists(), "error left a partial Artifact IR");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostic.starts_with(&format!("sightlint-ios: {code}: ")),
        "unexpected diagnostic: {diagnostic}"
    );
    assert!(diagnostic.ends_with('\n'));
}

fn local_case(temporary: &Path, mut capture: Value) -> (PathBuf, PathBuf) {
    let root = repository_root();
    let screenshot = temporary.join("screen.png");
    fs::copy(root.join("evaluation/ios/captures/clean.png"), &screenshot).unwrap();
    let screenshot_digest = python_sha256(&screenshot);
    capture["screenshot"]["reference"] = json!("screen.png");
    capture["screenshot"]["sha256"] = json!(screenshot_digest);
    let capture_path = temporary.join("test.capture.json");
    write_json(&capture_path, &capture);
    let capture_digest = python_sha256(&capture_path);

    let mut request = load_json(root.join("evaluation/ios/requests/ios-atlas-clean.json"));
    request["capture"]["reference"] = json!("test.capture.json");
    request["capture"]["sha256"] = json!(capture_digest);
    request["screenshot"]["reference"] = json!("screen.png");
    request["screenshot"]["sha256"] = json!(screenshot_digest);
    let request_path = temporary.join("request.json");
    write_json(&request_path, &request);
    (request_path, temporary.join("artifact-ir.json"))
}

#[test]
fn public_ios_adapter_has_stable_fail_closed_boundaries() {
    let root = repository_root();
    let clean_request = load_json(root.join("evaluation/ios/requests/ios-atlas-clean.json"));

    let digest_temp = TempDirectory::new();
    let mut digest_request = clean_request.clone();
    digest_request["capture"]["sha256"] = json!(format!("sha256:{}", "0".repeat(64)));
    let digest_request_path = digest_temp.0.join("request.json");
    write_json(&digest_request_path, &digest_request);
    let digest_ir = digest_temp.0.join("artifact-ir.json");
    assert_error(
        &adapter(&digest_request_path, &digest_ir),
        "input-digest",
        &digest_ir,
    );

    let budget_temp = TempDirectory::new();
    let mut budget_request = clean_request.clone();
    budget_request["execution"]["maxNodes"] = json!(1);
    let budget_request_path = budget_temp.0.join("request.json");
    write_json(&budget_request_path, &budget_request);
    let budget_ir = budget_temp.0.join("artifact-ir.json");
    assert_error(
        &adapter(&budget_request_path, &budget_ir),
        "node-budget",
        &budget_ir,
    );

    let output_temp = TempDirectory::new();
    let mut output_request = clean_request.clone();
    output_request["execution"]["maxOutputBytes"] = json!(1024);
    let output_request_path = output_temp.0.join("request.json");
    write_json(&output_request_path, &output_request);
    let output_ir = output_temp.0.join("artifact-ir.json");
    assert_error(
        &adapter(&output_request_path, &output_ir),
        "output-budget",
        &output_ir,
    );

    let collision_temp = TempDirectory::new();
    let collision_ir = collision_temp.0.join("artifact-ir.json");
    fs::write(&collision_ir, b"owned-by-caller").unwrap();
    let collision = adapter(
        &root.join("evaluation/ios/requests/ios-atlas-clean.json"),
        &collision_ir,
    );
    assert_exit(&collision, EXIT_ERROR);
    assert!(collision.stdout.is_empty());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("output-collision"));
    assert_eq!(fs::read(&collision_ir).unwrap(), b"owned-by-caller");

    let path_temp = TempDirectory::new();
    let mut path_request = clean_request.clone();
    path_request["capture"]["reference"] = json!("../escape.capture.json");
    let path_request_path = path_temp.0.join("request.json");
    write_json(&path_request_path, &path_request);
    let path_ir = path_temp.0.join("artifact-ir.json");
    assert_error(
        &adapter(&path_request_path, &path_ir),
        "input-invalid",
        &path_ir,
    );
}

#[test]
fn public_ios_adapter_rejects_malformed_incompatible_and_conflicting_captures() {
    let root = repository_root();

    let duplicate_temp = TempDirectory::new();
    let original =
        fs::read_to_string(root.join("evaluation/ios/requests/ios-atlas-clean.json")).unwrap();
    let duplicate = original.replacen(
        "\"protocolVersion\": \"0.1.0\"",
        "\"protocolVersion\": \"0.1.0\", \"protocolVersion\": \"0.1.0\"",
        1,
    );
    let duplicate_request = duplicate_temp.0.join("request.json");
    fs::write(&duplicate_request, duplicate).unwrap();
    let duplicate_ir = duplicate_temp.0.join("artifact-ir.json");
    assert_error(
        &adapter(&duplicate_request, &duplicate_ir),
        "input-json",
        &duplicate_ir,
    );

    let unknown_temp = TempDirectory::new();
    let mut unknown_capture = load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    unknown_capture["unexpected"] = json!(true);
    let (unknown_request, unknown_ir) = local_case(&unknown_temp.0, unknown_capture);
    assert_error(
        &adapter_in_root(&unknown_request, &unknown_ir, &unknown_temp.0),
        "input-invalid",
        &unknown_ir,
    );

    let tool_temp = TempDirectory::new();
    let mut tool_capture = load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    tool_capture["build"]["sdkVersion"] = json!("26.3");
    let (tool_request, tool_ir) = local_case(&tool_temp.0, tool_capture);
    assert_error(
        &adapter_in_root(&tool_request, &tool_ir, &tool_temp.0),
        "capture-compatibility",
        &tool_ir,
    );

    let extent_temp = TempDirectory::new();
    let mut extent_capture = load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    extent_capture["screenshot"]["widthPixels"] = json!(1205);
    let (extent_request, extent_ir) = local_case(&extent_temp.0, extent_capture);
    assert_error(
        &adapter_in_root(&extent_request, &extent_ir, &extent_temp.0),
        "extent-conflict",
        &extent_ir,
    );

    let duplicate_node_temp = TempDirectory::new();
    let mut duplicate_node_capture =
        load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    let duplicate = duplicate_node_capture["sourceHierarchy"]["nodes"][0].clone();
    duplicate_node_capture["sourceHierarchy"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let (duplicate_node_request, duplicate_node_ir) =
        local_case(&duplicate_node_temp.0, duplicate_node_capture);
    assert_error(
        &adapter_in_root(
            &duplicate_node_request,
            &duplicate_node_ir,
            &duplicate_node_temp.0,
        ),
        "duplicate-node",
        &duplicate_node_ir,
    );

    let intersection_temp = TempDirectory::new();
    let mut intersection_capture =
        load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    let save = intersection_capture["sourceHierarchy"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["identifier"] == "save_button")
        .unwrap();
    save["windowIntersectionPoints"]["width"] = json!(353);
    let (intersection_request, intersection_ir) =
        local_case(&intersection_temp.0, intersection_capture);
    assert_error(
        &adapter_in_root(
            &intersection_request,
            &intersection_ir,
            &intersection_temp.0,
        ),
        "capture-conflict",
        &intersection_ir,
    );
}

#[test]
fn public_ios_adapter_preserves_source_xcui_conflict_without_geometry_promotion() {
    let root = repository_root();
    let temporary = TempDirectory::new();
    let mut capture = load_json(root.join("evaluation/ios/captures/clean.capture.json"));
    let save = capture["xcuiHierarchy"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["identifier"] == "save_button")
        .unwrap();
    save["framePoints"]["x"] = json!(25);
    let (request, artifact_ir) = local_case(&temporary.0, capture);
    let adapted = adapter_in_root(&request, &artifact_ir, &temporary.0);
    assert_exit(&adapted, EXIT_SUCCESS);

    let document = load_json(&artifact_ir);
    let extension = &document["extensions"]["org.sightlint.ios"];
    let source = item_by(
        &extension["sourceNodes"],
        "identifier",
        &json!("save_button"),
    );
    let xcui = item_by(&extension["xcuiNodes"], "identifier", &json!("save_button"));
    let core = item_by(&document["nodes"], "id", &json!("ios:view:save_button"));
    assert_eq!(source["xcuiReconciliation"], "frameConflict");
    assert_eq!(source["layoutBoundsPoints"]["x"].as_f64(), Some(24.0));
    assert_eq!(xcui["framePoints"]["x"].as_f64(), Some(25.0));
    assert_eq!(
        core["geometry"]["layoutBox"]["rect"]["x"].as_f64(),
        Some(24.0)
    );
    assert!(core["geometry"].get("hitBox").is_none());
    assert!(core["geometry"].get("renderBox").is_none());
    assert!(core["geometry"].get("inkBox").is_none());

    let checked = check(&artifact_ir);
    assert_exit(&checked, EXIT_SUCCESS);
    let report: Value = serde_json::from_slice(&checked.stdout).unwrap();
    let save_result = report["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| {
            result["ruleId"] == BOUNDS_RULE && result["target"]["id"] == "ios:view:save_button"
        })
        .unwrap();
    assert_eq!(save_result["outcome"], "passed");
    assert_eq!(save_result["evidenceClasses"], json!(["exactSource"]));
}
