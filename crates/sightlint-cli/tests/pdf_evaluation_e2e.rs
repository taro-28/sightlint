//! Public-process evaluation for the bounded PDF source adapter.

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
            "sightlint-pdf-e2e-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create PDF E2E temporary directory");
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
    adapter_in_root(request, output, &repository_root(), false)
}

fn adapter_in_root(
    request: &Path,
    output: &Path,
    repository: &Path,
    isolated_python: bool,
) -> Output {
    let root = repository_root();
    let mut command = Command::new(python());
    if isolated_python {
        command.arg("-S");
    }
    command
        .arg(root.join("adapters/pdf/sightlint_pdf.py"))
        .arg("--request")
        .arg(request)
        .arg("--repository-root")
        .arg(repository)
        .arg("--sightlint-binary")
        .arg(env!("CARGO_BIN_EXE_sightlint"))
        .arg("--artifact-ir-out")
        .arg(output)
        .output()
        .expect("execute PDF adapter")
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

fn acquisition_facts(document: &Value, oracle: &Value) -> u32 {
    let extension = &document["extensions"]["org.sightlint.pdf"];
    assert_eq!(extension["pdfHeader"], oracle["pdfHeader"]);
    assert_eq!(extension["taggedStructure"], oracle["taggedStructure"]);
    let page = &oracle["page"];
    let actual_page = item_by(&extension["pages"], "id", &page["id"]);
    for field in [
        "index",
        "objectReference",
        "rotationDegrees",
        "geometryStatus",
        "mediaBoxPdfPoints",
        "cropBoxPdfPoints",
    ] {
        assert_eq!(actual_page[field], page[field], "page field {field}");
    }
    let canvas = item_by(&document["canvases"], "id", &page["id"]);
    assert_eq!(canvas["unit"], "pdfPoint");
    assert_eq!(
        canvas["size"]["width"].as_f64(),
        Some(
            page["cropBoxPdfPoints"]["right"].as_f64().unwrap()
                - page["cropBoxPdfPoints"]["left"].as_f64().unwrap()
        )
    );
    assert_eq!(
        canvas["size"]["height"].as_f64(),
        Some(
            page["cropBoxPdfPoints"]["top"].as_f64().unwrap()
                - page["cropBoxPdfPoints"]["bottom"].as_f64().unwrap()
        )
    );

    let expected_render = &oracle["render"];
    let actual_render = &actual_page["render"];
    for field in [
        "widthPixels",
        "heightPixels",
        "pdfPointsPerPixel",
        "extentReconciliation",
        "nodeIdentity",
    ] {
        assert_eq!(
            actual_render[field], expected_render[field],
            "render field {field}"
        );
    }

    let mut facts = 4_u32;
    for expected in oracle["annotations"].as_array().expect("annotations") {
        let actual = item_by(&extension["annotations"], "id", &expected["id"]);
        for field in [
            "objectReference",
            "subtype",
            "flags",
            "hasQuadPoints",
            "hasPath",
            "actionKind",
            "geometryStatus",
            "sourceRectPdfPoints",
        ] {
            assert_eq!(actual[field], expected[field], "annotation field {field}");
        }
        assert_eq!(
            actual.get("normalizedHitBoxPdfPoints"),
            expected.get("normalizedHitBoxPdfPoints")
        );
        let core_node = document["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == expected["id"]);
        if expected["geometryStatus"] == "exact" {
            let core_node = core_node.expect("exact annotation has a core node");
            assert_eq!(core_node["kind"]["value"], "control");
            assert_eq!(core_node["role"]["value"], "link");
            for coordinate in ["x", "y", "width", "height"] {
                assert_eq!(
                    core_node["geometry"]["hitBox"]["rect"][coordinate].as_f64(),
                    expected["normalizedHitBoxPdfPoints"][coordinate].as_f64(),
                    "hitBox coordinate {coordinate}"
                );
            }
            assert!(core_node["geometry"].get("layoutBox").is_none());
            assert!(core_node["geometry"].get("renderBox").is_none());
            assert!(core_node["geometry"].get("inkBox").is_none());
        } else {
            assert!(
                core_node.is_none(),
                "unsupported geometry became a core node"
            );
        }
        facts += 1;
    }
    facts
}

fn rule_observation(report: &Value, oracle: &Value) -> (u32, u32) {
    let expectations = oracle["expectations"].as_array().unwrap();
    let mut expected_failures = BTreeSet::new();
    for expectation in expectations {
        let result = report["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| {
                result["ruleId"] == expectation["ruleId"]
                    && result["ruleVersion"] == expectation["ruleVersion"]
                    && result["target"]["id"] == expectation["targetId"]
                    && result["target"]["aspect"] == expectation["aspect"]
            })
            .expect("expected rule result");
        assert_eq!(result["outcome"], expectation["expectedOutcome"]);
        assert_eq!(result["evidenceClasses"], json!(["exactSource"]));
        if expectation["expectedOutcome"] == "failed" {
            expected_failures.insert(expectation["targetId"].as_str().unwrap().to_owned());
        }
    }
    let actual_failures = report["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|result| result["outcome"] == "failed")
        .map(|result| {
            assert_eq!(result["ruleId"], BOUNDS_RULE, "unexpected failing rule");
            result["target"]["id"].as_str().unwrap().to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_failures, expected_failures);
    (
        u32::try_from(expected_failures.len()).unwrap(),
        u32::try_from(actual_failures.len()).unwrap(),
    )
}

fn assert_no_source_content(bytes: &[u8]) {
    let output = String::from_utf8_lossy(bytes);
    for private_value in [
        "Atlas Operations Review",
        "SIGHTLINT-PDF-CONTENT-SENTINEL",
        "SIGHTLINT-PDF-METADATA-SENTINEL",
        "Fictional repository-owned report",
        "Overview",
        "Decisions",
    ] {
        assert!(
            !output.contains(private_value),
            "source content leaked: {private_value}"
        );
    }
}

#[derive(Default)]
struct Metrics {
    facts: u32,
    cases: u32,
    expected_failures: u32,
    actual_failures: u32,
    abstentions: u32,
    negative_cases: u32,
    false_positive_cases: u32,
    mutations: u32,
    killed_mutations: u32,
}

#[test]
fn public_pdf_corpus_separates_acquisition_and_rule_ground_truth() {
    let root = repository_root();
    let corpus = load_json(root.join("evaluation/pdf/corpus.json"));
    let acquisitions = load_json(root.join("evaluation/pdf/annotations/acquisition.json"));
    let rules = load_json(root.join("evaluation/pdf/annotations/rules.json"));
    let contract = load_json(root.join("evaluation/pdf/metric-contract.json"));
    let temporary = TempDirectory::new();
    let mut metrics = Metrics::default();
    let mut response_by_case = BTreeMap::new();

    for case in corpus["cases"].as_array().unwrap() {
        let case_id = case["id"].as_str().unwrap();
        let request = root.join(case["request"]["path"].as_str().unwrap());
        let first_ir = temporary.0.join(format!("{case_id}-first.json"));
        let second_ir = temporary.0.join(format!("{case_id}-second.json"));
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
        assert_eq!(response["coverage"]["pageGeometry"], "partial");
        assert_eq!(response["coverage"]["linkAnnotations"], "partial");
        assert_eq!(response["coverage"]["renderedNodeIdentity"], "cantTell");
        assert_eq!(response["externalProcessing"], false);
        response_by_case.insert(case_id, response);

        let document: Value = serde_json::from_slice(&first_bytes).unwrap();
        assert_eq!(document["artifact"]["kind"], "pdf");
        let extension = &document["extensions"]["org.sightlint.pdf"];
        assert_eq!(extension["privacy"]["actionsFollowed"], false);
        assert_eq!(extension["privacy"]["contentPolicy"], "geometryAndTypeOnly");
        let acquisition = item_by(
            &acquisitions["annotations"],
            "id",
            &case["acquisitionAnnotationId"],
        );
        metrics.facts += acquisition_facts(&document, acquisition);
        let oracle = item_by(&rules["annotations"], "id", &case["ruleAnnotationId"]);
        let checked = check(&first_ir);
        let expected_to_fail = oracle["expectations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|expectation| expectation["expectedOutcome"] == "failed");
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
        metrics.abstentions += u32::from(
            acquisition["annotations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["geometryStatus"] == "unsupportedQuadPoints"),
        );
        match oracle["caseRole"].as_str().unwrap() {
            "targetedMutation" => {
                metrics.mutations += 1;
                metrics.killed_mutations += u32::from(actual == expected && actual > 0);
            }
            "clean" | "hardNegative" => {
                metrics.negative_cases += 1;
                metrics.false_positive_cases += u32::from(actual > 0);
            }
            role => panic!("unexpected case role {role}"),
        }
        metrics.cases += 1;
    }

    assert_eq!(metrics.cases, 3);
    assert_eq!(metrics.actual_failures, metrics.expected_failures);
    assert_eq!(metrics.abstentions, 1);
    assert_eq!(metrics.false_positive_cases, 0);
    assert_eq!(metrics.killed_mutations, metrics.mutations);
    assert_eq!(response_by_case.len(), 3);
    let observed = BTreeMap::from([
        ("abstentionRetention", f64::from(metrics.abstentions)),
        ("acquisitionFactCoverage", 1.0),
        ("evaluatedCaseCoverage", f64::from(metrics.cases) / 3.0),
        (
            "falsePositiveRate",
            f64::from(metrics.false_positive_cases) / f64::from(metrics.negative_cases),
        ),
        (
            "mutationKillRate",
            f64::from(metrics.killed_mutations) / f64::from(metrics.mutations),
        ),
        (
            "verdictPrecision",
            f64::from(metrics.expected_failures) / f64::from(metrics.actual_failures),
        ),
    ]);
    for metric in contract["metrics"].as_array().unwrap() {
        let id = metric["id"].as_str().unwrap();
        assert_eq!(observed.get(id), metric["target"].as_f64().as_ref());
    }
}

fn write_request(path: &Path, source: &str, digest: &str, limits: Value) {
    let request = json!({
        "protocolVersion": "0.1.0",
        "requestId": "pdf-error-case",
        "artifact": {"id": "pdf-error-case"},
        "input": {"reference": source, "sha256": digest},
        "renders": [],
        "privacy": {
            "externalProcessing": false,
            "retention": "none",
            "contentPolicy": "geometryAndTypeOnly"
        },
        "execution": limits
    });
    fs::write(path, serde_json::to_vec_pretty(&request).unwrap()).unwrap();
}

fn default_limits() -> Value {
    json!({
        "maxInputBytes": 1048576,
        "maxRenderBytes": 4194304,
        "maxObjects": 100,
        "maxPages": 4,
        "maxAnnotations": 16,
        "maxAnnotationsPerPage": 8,
        "maxOutputBytes": 1048576
    })
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
        diagnostic.starts_with(&format!("sightlint-pdf: {code}: ")),
        "unexpected diagnostic: {diagnostic}"
    );
    assert!(diagnostic.ends_with('\n'));
}

#[test]
fn public_pdf_adapter_has_stable_fail_closed_boundaries() {
    let root = repository_root();
    let temporary = TempDirectory::new();
    let clean = load_json(root.join("evaluation/pdf/requests/atlas-clean.json"));

    let digest_request = temporary.0.join("digest.json");
    let mut digest_mismatch = clean.clone();
    digest_mismatch["input"]["sha256"] = json!(format!("sha256:{}", "0".repeat(64)));
    fs::write(
        &digest_request,
        serde_json::to_vec_pretty(&digest_mismatch).unwrap(),
    )
    .unwrap();
    let digest_ir = temporary.0.join("digest-ir.json");
    assert_error(
        &adapter(&digest_request, &digest_ir),
        "input-digest",
        &digest_ir,
    );

    let budget_request = temporary.0.join("budget.json");
    let mut budget = clean.clone();
    budget["execution"]["maxObjects"] = json!(1);
    fs::write(&budget_request, serde_json::to_vec_pretty(&budget).unwrap()).unwrap();
    let budget_ir = temporary.0.join("budget-ir.json");
    assert_error(
        &adapter(&budget_request, &budget_ir),
        "object-budget",
        &budget_ir,
    );

    let collision_ir = temporary.0.join("collision-ir.json");
    fs::write(&collision_ir, b"owned-by-caller").unwrap();
    let collision = adapter(
        &root.join("evaluation/pdf/requests/atlas-clean.json"),
        &collision_ir,
    );
    assert_exit(&collision, EXIT_ERROR);
    assert!(collision.stdout.is_empty());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("output-collision"));
    assert_eq!(fs::read(&collision_ir).unwrap(), b"owned-by-caller");

    let dependency_ir = temporary.0.join("dependency-ir.json");
    assert_error(
        &adapter_in_root(
            &root.join("evaluation/pdf/requests/atlas-clean.json"),
            &dependency_ir,
            &root,
            true,
        ),
        "dependency-error",
        &dependency_ir,
    );

    let malformed_pdf = temporary.0.join("malformed.pdf");
    fs::write(&malformed_pdf, b"not a PDF\n").unwrap();
    let malformed_request = temporary.0.join("malformed.json");
    write_request(
        &malformed_request,
        "malformed.pdf",
        &python_sha256(&malformed_pdf),
        default_limits(),
    );
    let malformed_ir = temporary.0.join("malformed-ir.json");
    assert_error(
        &adapter_in_root(&malformed_request, &malformed_ir, &temporary.0, false),
        "pdf-invalid",
        &malformed_ir,
    );

    let encrypted_pdf = temporary.0.join("encrypted.pdf");
    let generated = Command::new(python())
        .arg("-c")
        .arg("import sys; from pypdf import PdfReader,PdfWriter; r=PdfReader(sys.argv[1],strict=True); w=PdfWriter(); w.append_pages_from_reader(r); w.encrypt('fixture-password'); w.write(sys.argv[2])")
        .arg(root.join("fixtures/pdf/atlas-clean.pdf"))
        .arg(&encrypted_pdf)
        .output()
        .expect("generate encrypted PDF test input");
    assert_exit(&generated, EXIT_SUCCESS);
    let encrypted_request = temporary.0.join("encrypted.json");
    write_request(
        &encrypted_request,
        "encrypted.pdf",
        &python_sha256(&encrypted_pdf),
        default_limits(),
    );
    let encrypted_ir = temporary.0.join("encrypted-ir.json");
    assert_error(
        &adapter_in_root(&encrypted_request, &encrypted_ir, &temporary.0, false),
        "pdf-encrypted",
        &encrypted_ir,
    );
}
