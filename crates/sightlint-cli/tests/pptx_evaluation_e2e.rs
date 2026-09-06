//! Public-process evaluation for the bounded PPTX source adapter.

use std::collections::BTreeSet;
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
            "sightlint-pptx-e2e-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create PPTX E2E temporary directory");
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
    let root = repository_root();
    adapter_in_root(request, output, &root)
}

fn adapter_in_root(request: &Path, output: &Path, repository: &Path) -> Output {
    let root = repository_root();
    Command::new(python())
        .arg(root.join("adapters/pptx/sightlint_pptx.py"))
        .arg("--request")
        .arg(request)
        .arg("--repository-root")
        .arg(repository)
        .arg("--sightlint-binary")
        .arg(env!("CARGO_BIN_EXE_sightlint"))
        .arg("--artifact-ir-out")
        .arg(output)
        .output()
        .expect("execute PPTX adapter")
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
}

fn item_by<'a>(items: &'a Value, field: &str, expected: &Value) -> &'a Value {
    items
        .as_array()
        .expect("items are an array")
        .iter()
        .find(|item| item.get(field) == Some(expected))
        .unwrap_or_else(|| panic!("missing item with {field}={expected}"))
}

fn acquisition_facts(document: &Value, oracle: &Value) -> usize {
    let extension = &document["extensions"]["org.sightlint.pptx"];
    let slide = &oracle["slide"];
    let actual_slide = item_by(&extension["slides"], "id", &slide["id"]);
    assert_eq!(actual_slide["index"], slide["index"]);
    assert_eq!(actual_slide["part"], slide["part"]);
    assert_eq!(actual_slide["widthEmu"], slide["widthEmu"]);
    assert_eq!(actual_slide["heightEmu"], slide["heightEmu"]);

    let canvas = item_by(&document["canvases"], "id", &slide["id"]);
    assert_eq!(canvas["unit"], "emu");
    assert_eq!(canvas["size"]["width"].as_f64(), slide["widthEmu"].as_f64());
    assert_eq!(
        canvas["size"]["height"].as_f64(),
        slide["heightEmu"].as_f64()
    );
    let canvas_evidence = item_by(
        &document["evidence"],
        "id",
        &actual_slide["sourceEvidenceId"],
    );
    assert_eq!(canvas_evidence["class"], slide["evidenceClass"]);

    let expected_render = &oracle["render"];
    let actual_render = &actual_slide["render"];
    for field in [
        "widthPixels",
        "heightPixels",
        "emuPerPixel",
        "extentReconciliation",
        "nodeIdentity",
    ] {
        assert_eq!(
            actual_render[field], expected_render[field],
            "render field {field}"
        );
    }
    let render_evidence = item_by(&document["evidence"], "id", &actual_render["evidenceId"]);
    assert_eq!(render_evidence["class"], expected_render["evidenceClass"]);

    let mut facts = 2;
    for expected_node in oracle["nodes"].as_array().expect("oracle nodes") {
        let identifier = &expected_node["id"];
        let extension_node = item_by(&extension["nodes"], "id", identifier);
        for field in ["nativeId", "nativeType", "zOrder", "geometryStatus", "text"] {
            assert_eq!(
                extension_node[field], expected_node[field],
                "node {identifier} field {field}"
            );
        }
        assert_eq!(
            extension_node.get("parentId"),
            expected_node.get("parentId")
        );

        let core_node = item_by(&document["nodes"], "id", identifier);
        let layout = &core_node["geometry"]["layoutBox"];
        let expected_rect = &expected_node["rectEmu"];
        for field in ["x", "y", "width", "height"] {
            assert_eq!(
                layout["rect"][field].as_f64(),
                expected_rect[field].as_f64()
            );
        }
        assert!(core_node["geometry"].get("renderBox").is_none());
        assert!(core_node["geometry"].get("inkBox").is_none());
        let evidence = item_by(
            &document["evidence"],
            "id",
            &extension_node["sourceEvidenceId"],
        );
        assert_eq!(evidence["class"], expected_node["sourceEvidenceClass"]);
        facts += 1;
    }
    facts
}

fn verify_rule_results(report: &Value, oracle: &Value) -> (usize, usize) {
    let expected = oracle["expectations"]
        .as_array()
        .expect("rule expectations");
    let mut expected_failures = BTreeSet::new();
    for expectation in expected {
        let result = report["results"]
            .as_array()
            .expect("report results")
            .iter()
            .find(|result| {
                result["ruleId"] == expectation["ruleId"]
                    && result["ruleVersion"] == expectation["ruleVersion"]
                    && result["target"]["id"] == expectation["targetId"]
                    && result["target"]["aspect"] == expectation["aspect"]
            })
            .expect("expected rule target result");
        assert_eq!(result["outcome"], expectation["expectedOutcome"]);
        assert_eq!(result["evidenceClasses"], json!(["exactSource"]));
        if expectation["expectedOutcome"] == "failed" {
            expected_failures.insert(expectation["targetId"].as_str().unwrap().to_owned());
        }
    }

    let actual_failures = report["results"]
        .as_array()
        .expect("report results")
        .iter()
        .filter(|result| result["outcome"] == "failed")
        .map(|result| {
            assert_eq!(result["ruleId"], BOUNDS_RULE, "unexpected failing rule");
            result["target"]["id"].as_str().unwrap().to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_failures, expected_failures);

    for rule_id in [
        "visual.geometry.declared-non-overlap",
        "visual.spacing.peer-consistency",
    ] {
        let result = report["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| result["ruleId"] == rule_id)
            .expect("declared relation rule result");
        assert_eq!(result["outcome"], "inapplicable");
    }
    (expected_failures.len(), actual_failures.len())
}

struct CaseObservation {
    facts: u32,
    expected_failures: u32,
    actual_failures: u32,
    abstained: bool,
    role: String,
}

#[derive(Default)]
struct EvaluationMetrics {
    acquisition_facts: u32,
    evaluated_cases: u32,
    expected_failures: u32,
    actual_failures: u32,
    abstentions: u32,
    negative_cases: u32,
    false_positive_cases: u32,
    mutations: u32,
    killed_mutations: u32,
}

fn assert_no_source_text(ir_bytes: &[u8]) {
    let ir_text = String::from_utf8_lossy(ir_bytes);
    for private_source_value in [
        "Quarterly operations",
        "Response time",
        "Resolution",
        "Satisfaction",
        "Primary narrative",
        "Supporting note",
        "Next action",
        "Metrics group",
    ] {
        assert!(
            !ir_text.contains(private_source_value),
            "source text/name leaked: {private_source_value}"
        );
    }
}

fn evaluate_case(
    root: &Path,
    temporary: &Path,
    case: &Value,
    acquisitions: &Value,
    rules: &Value,
) -> CaseObservation {
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
        "{case_id} IR drift"
    );

    let response: Value = serde_json::from_slice(&first.stdout).expect("adapter response JSON");
    assert_eq!(response["status"], "partial");
    assert_eq!(response["coverage"]["sourceGeometry"], "partial");
    assert_eq!(response["coverage"]["renderedNodeIdentity"], "cantTell");
    assert_eq!(response["externalProcessing"], false);
    assert_no_source_text(&first_bytes);

    let document: Value = serde_json::from_slice(&first_bytes).expect("Artifact IR JSON");
    assert_eq!(document["artifact"]["kind"], "slide");
    let extension = &document["extensions"]["org.sightlint.pptx"];
    assert_eq!(extension["privacy"]["textPolicy"], "digestOnly");
    assert_eq!(
        extension["unsupportedFeatures"],
        json!(["masterAndLayoutObjects", "themeResolvedStyles"])
    );
    let acquisition = item_by(
        &acquisitions["annotations"],
        "id",
        &case["acquisitionAnnotationId"],
    );
    let facts = u32::try_from(acquisition_facts(&document, acquisition)).unwrap();

    let rule_oracle = item_by(&rules["annotations"], "id", &case["ruleAnnotationId"]);
    let checked = check(&first_ir);
    let has_expected_failure = rule_oracle["expectations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|expectation| expectation["expectedOutcome"] == "failed");
    assert_exit(
        &checked,
        if has_expected_failure {
            EXIT_FINDINGS
        } else {
            EXIT_SUCCESS
        },
    );
    assert!(checked.stderr.is_empty());
    let report: Value = serde_json::from_slice(&checked.stdout).expect("CheckReport JSON");
    let (expected, actual) = verify_rule_results(&report, rule_oracle);
    CaseObservation {
        facts,
        expected_failures: u32::try_from(expected).unwrap(),
        actual_failures: u32::try_from(actual).unwrap(),
        abstained: extension["slides"][0]["render"]["nodeIdentity"] == "cantTell",
        role: rule_oracle["caseRole"].as_str().unwrap().to_owned(),
    }
}

fn observe(metrics: &mut EvaluationMetrics, observation: &CaseObservation) {
    metrics.acquisition_facts += observation.facts;
    metrics.expected_failures += observation.expected_failures;
    metrics.actual_failures += observation.actual_failures;
    metrics.abstentions += u32::from(observation.abstained);
    match observation.role.as_str() {
        "targetedMutation" => {
            metrics.mutations += 1;
            metrics.killed_mutations += u32::from(
                observation.actual_failures == observation.expected_failures
                    && observation.actual_failures > 0,
            );
        }
        "clean" | "hardNegative" => {
            metrics.negative_cases += 1;
            metrics.false_positive_cases += u32::from(observation.actual_failures > 0);
        }
        other => panic!("unexpected case role {other}"),
    }
    metrics.evaluated_cases += 1;
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    f64::from(numerator) / f64::from(denominator)
}

#[test]
fn public_pptx_corpus_separates_acquisition_and_rule_ground_truth() {
    let root = repository_root();
    let corpus = load_json(root.join("evaluation/pptx/corpus.json"));
    let acquisitions = load_json(root.join("evaluation/pptx/annotations/acquisition.json"));
    let rules = load_json(root.join("evaluation/pptx/annotations/rules.json"));
    let temporary = TempDirectory::new();
    let cases = corpus["cases"].as_array().expect("corpus cases");
    let mut metrics = EvaluationMetrics::default();
    for case in cases {
        observe(
            &mut metrics,
            &evaluate_case(&root, &temporary.0, case, &acquisitions, &rules),
        );
    }

    let case_count = u32::try_from(cases.len()).unwrap();
    assert_eq!(metrics.evaluated_cases, case_count);
    assert_eq!(metrics.actual_failures, metrics.expected_failures);
    assert_eq!(metrics.abstentions, metrics.evaluated_cases);
    assert_eq!(metrics.false_positive_cases, 0);
    assert_eq!(metrics.killed_mutations, metrics.mutations);
    let observed_metrics = json!({
        "abstentionRetention": ratio(metrics.abstentions, metrics.evaluated_cases),
        "acquisitionFactCoverage": ratio(metrics.acquisition_facts, metrics.acquisition_facts),
        "evaluatedCaseCoverage": ratio(metrics.evaluated_cases, case_count),
        "falsePositiveRate": ratio(metrics.false_positive_cases, metrics.negative_cases),
        "mutationKillRate": ratio(metrics.killed_mutations, metrics.mutations),
        "verdictPrecision": ratio(metrics.expected_failures, metrics.actual_failures),
    });
    println!("PPTX public regression metrics: {observed_metrics}");
    assert_eq!(
        observed_metrics,
        json!({
            "abstentionRetention": 1.0,
            "acquisitionFactCoverage": 1.0,
            "evaluatedCaseCoverage": 1.0,
            "falsePositiveRate": 0.0,
            "mutationKillRate": 1.0,
            "verdictPrecision": 1.0,
        })
    );
}

fn write_request(path: &Path, mutator: impl FnOnce(&mut Value)) {
    let root = repository_root();
    let mut request = load_json(root.join("evaluation/pptx/requests/atlas-clean.json"));
    mutator(&mut request);
    fs::write(path, serde_json::to_vec_pretty(&request).unwrap()).unwrap();
}

#[test]
fn render_conflict_and_absence_remain_distinct_evidence_states() {
    let temporary = TempDirectory::new();

    let conflict_request = temporary.0.join("render-conflict.json");
    write_request(&conflict_request, |request| {
        request["requestId"] = json!("pptx-render-conflict");
        request["renders"][0]["emuPerPixel"] = json!(9524);
    });
    let conflict_ir = temporary.0.join("render-conflict.ir.json");
    let conflict = adapter(&conflict_request, &conflict_ir);
    assert_exit(&conflict, EXIT_SUCCESS);
    let document = load_json(&conflict_ir);
    let slides = &document["extensions"]["org.sightlint.pptx"]["slides"];
    assert_eq!(slides[0]["widthEmu"], 9_144_000);
    assert_eq!(slides[0]["render"]["widthPixels"], 960);
    assert_eq!(slides[0]["render"]["emuPerPixel"], 9524);
    assert_eq!(slides[0]["render"]["extentReconciliation"], "conflict");
    assert_eq!(slides[0]["render"]["nodeIdentity"], "cantTell");
    assert_exit(&check(&conflict_ir), EXIT_SUCCESS);

    let absent_request = temporary.0.join("render-absent.json");
    write_request(&absent_request, |request| {
        request["requestId"] = json!("pptx-render-absent");
        request["renders"] = json!([]);
    });
    let absent_ir = temporary.0.join("render-absent.ir.json");
    let absent = adapter(&absent_request, &absent_ir);
    assert_exit(&absent, EXIT_SUCCESS);
    let response: Value = serde_json::from_slice(&absent.stdout).unwrap();
    assert_eq!(response["coverage"]["renderedExtent"], "untested");
    let document = load_json(&absent_ir);
    let render = &document["extensions"]["org.sightlint.pptx"]["slides"][0]["render"];
    assert_eq!(render["status"], "untested");
    assert_eq!(render["nodeIdentity"], "cantTell");
    assert_eq!(document["canvases"].as_array().unwrap().len(), 1);
}

fn assert_stable_error(request: &Path, output: &Path, diagnostic: &str) {
    let first = adapter(request, output);
    let second = adapter(request, output);
    assert_exit(&first, EXIT_ERROR);
    assert_exit(&second, EXIT_ERROR);
    assert!(first.stdout.is_empty());
    assert!(second.stdout.is_empty());
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(String::from_utf8(first.stderr).unwrap(), diagnostic);
    assert!(!output.exists());
}

#[test]
fn public_process_rejects_digest_limits_unknown_fields_and_output_collisions() {
    let temporary = TempDirectory::new();

    let archive_limit = temporary.0.join("archive-limit.json");
    write_request(&archive_limit, |request| {
        request["execution"]["maxArchiveBytes"] = json!(1);
    });
    assert_stable_error(
        &archive_limit,
        &temporary.0.join("archive-limit.ir.json"),
        "sightlint-pptx: input-budget: PPTX input exceeds its request byte budget\n",
    );

    let render_limit = temporary.0.join("render-limit.json");
    write_request(&render_limit, |request| {
        request["execution"]["maxRenderBytes"] = json!(1);
    });
    assert_stable_error(
        &render_limit,
        &temporary.0.join("render-limit.ir.json"),
        "sightlint-pptx: input-budget: render for slide 1 exceeds its request byte budget\n",
    );

    let wrong_digest = temporary.0.join("wrong-digest.json");
    write_request(&wrong_digest, |request| {
        request["input"]["sha256"] =
            json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    });
    assert_stable_error(
        &wrong_digest,
        &temporary.0.join("wrong-digest.ir.json"),
        "sightlint-pptx: input-digest: PPTX input SHA-256 does not match the request\n",
    );

    let unknown = temporary.0.join("unknown.json");
    write_request(&unknown, |request| {
        request["unexpected"] = json!(true);
    });
    assert_stable_error(
        &unknown,
        &temporary.0.join("unknown.ir.json"),
        "sightlint-pptx: request-invalid: request has unknown fields unexpected\n",
    );

    let collision = temporary.0.join("already-exists.ir.json");
    fs::write(&collision, b"preserve me").unwrap();
    let request = repository_root().join("evaluation/pptx/requests/atlas-clean.json");
    let result = adapter(&request, &collision);
    assert_exit(&result, EXIT_ERROR);
    assert!(result.stdout.is_empty());
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        "sightlint-pptx: output-collision: artifact IR output already exists\n"
    );
    assert_eq!(fs::read(&collision).unwrap(), b"preserve me");

    let malformed_root = temporary.0.join("malformed-root");
    fs::create_dir(&malformed_root).unwrap();
    fs::write(malformed_root.join("malformed.pptx"), b"").unwrap();
    let malformed_request = temporary.0.join("malformed-request.json");
    fs::write(
        &malformed_request,
        serde_json::to_vec_pretty(&json!({
            "protocolVersion": "0.1.0",
            "requestId": "malformed-archive",
            "artifact": {"id": "malformed-archive"},
            "input": {
                "reference": "malformed.pptx",
                "sha256": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            "renders": [],
            "privacy": {"externalProcessing": false, "retention": "none", "textPolicy": "digestOnly"},
            "execution": {
                "maxArchiveBytes": 1024,
                "maxRenderBytes": 1024,
                "maxEntries": 8,
                "maxExpandedBytes": 4096,
                "maxXmlBytes": 1024,
                "maxCompressionRatio": 10,
                "maxSlides": 2,
                "maxNodes": 8,
                "maxGroupDepth": 2,
                "maxOutputBytes": 4096
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let malformed_output = temporary.0.join("malformed.ir.json");
    let malformed = adapter_in_root(&malformed_request, &malformed_output, &malformed_root);
    assert_exit(&malformed, EXIT_ERROR);
    assert!(malformed.stdout.is_empty());
    assert_eq!(
        String::from_utf8(malformed.stderr).unwrap(),
        "sightlint-pptx: archive-invalid: input is not a valid supported ZIP package\n"
    );
    assert!(!malformed_output.exists());
}
