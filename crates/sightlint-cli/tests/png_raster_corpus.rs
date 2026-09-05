//! Committed native PNG bytes -> real CLI -> versioned source-raster observations.
//! Pixel conformance is not a claim of screenshot-only UI/UX defect detection.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use sightlint_adapter_png::{PngRasterStatus, observe_png_raster};

const CORPUS: &str = include_str!("../../../fixtures/png-raster/corpus.json");
const EXTENSION: &str = "org.sightlint.adapter.png";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn cases() -> Vec<Value> {
    let corpus: Value = serde_json::from_str(CORPUS).expect("committed corpus JSON");
    assert_eq!(corpus["version"], "0.1.0");
    assert_eq!(corpus["scope"], "source-raster-conformance");
    assert_eq!(corpus["reviewStatus"], "synthetic-not-human-validated");
    corpus["cases"].as_array().expect("corpus cases").clone()
}

fn text<'a>(case: &'a Value, field: &str) -> &'a str {
    case[field].as_str().expect("required corpus string")
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.is_ascii());
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII pair");
            u8::from_str_radix(pair, 16).expect("hex byte")
        })
        .collect()
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sightlint"))
}

fn stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn public binary");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input)
        .expect("write native fixture bytes");
    child.wait_with_output().expect("collect public output")
}

fn assert_exit(output: &Output, code: i32, id: &str) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{id}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_success(output: &Output, id: &str) -> Value {
    assert_exit(output, 0, id);
    assert!(output.stderr.is_empty(), "{id}: unexpected stderr");
    serde_json::from_slice(&output.stdout).expect("public JSON")
}

struct TempPng(PathBuf);

impl TempPng {
    fn new(id: &str, bytes: &[u8]) -> Self {
        assert!(id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'));
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sightlint-raster-{}-{sequence}-{id}.png",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create unique temporary PNG without overwriting");
        file.write_all(bytes).expect("materialize committed PNG bytes");
        Self(path)
    }

    fn run(&self, verb: &str, json: bool) -> Output {
        let mut command = command();
        command.arg(verb).arg(&self.0).stdin(Stdio::null());
        if json {
            command.args(["--format", "json"]);
        }
        command.output().expect("run public file command")
    }
}

impl Drop for TempPng {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn assert_api(case: &Value, png: &[u8]) {
    let id = text(case, "id");
    let observation = observe_png_raster(png);
    if case["exitCode"] == 2 {
        let message = observation.expect_err("malformed fixture must fail").to_string();
        assert!(message.contains(text(case, "errorContains")), "{id}: {message}");
        return;
    }
    let observation = observation.unwrap_or_else(|error| panic!("{id}: {error}"));
    match observation.status {
        PngRasterStatus::Available(raster) => {
            assert_eq!(case["status"], "available", "{id}");
            assert_eq!(Value::from(raster.width), case["width"], "{id}");
            assert_eq!(Value::from(raster.height), case["height"], "{id}");
            assert_eq!(raster.pixels, decode_hex(text(case, "rgbaHex")), "{id}");
        }
        PngRasterStatus::Unavailable(reason) => {
            assert_eq!(case["status"], "unavailable", "{id}");
            assert_eq!(reason.code(), text(case, "reason"), "{id}");
        }
    }
}

fn assert_metadata(case: &Value, ir: &Value) {
    let id = text(case, "id");
    let raster = &ir["extensions"][EXTENSION]["encodedRgba8Raster"];
    assert_eq!(raster["version"], "0.1.0", "{id}");
    assert_eq!(raster["status"], case["status"], "{id}");
    assert_eq!(raster["encoding"], "pngEncodedRgba8", "{id}");
    assert_eq!(raster["colorManagementApplied"], false, "{id}");
    assert!(raster.get("pixels").is_none(), "{id}: raw samples leaked into IR");
    if case["status"] == "available" {
        assert_eq!(raster["width"], case["width"], "{id}");
        assert_eq!(raster["height"], case["height"], "{id}");
        assert_eq!(raster["byteCrc32"], case["byteCrc32"], "{id}");
        assert_eq!(raster["byteCount"], text(case, "rgbaHex").len() / 2, "{id}");
        assert!(raster.get("reason").is_none(), "{id}");
    } else {
        assert_eq!(raster["reason"], case["reason"], "{id}");
        assert!(raster.get("byteCrc32").is_none(), "{id}: invented samples");
        assert!(raster.get("byteCount").is_none(), "{id}: invented sample count");
    }
    let evidence = ir["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .find(|item| item["id"] == raster["evidenceId"])
        .expect("raster provenance resolves");
    assert_eq!(evidence["class"], "exactSource", "{id}");
    assert_eq!(evidence["source"]["externalProcessing"], false, "{id}");
    assert!(evidence["selector"].to_string().contains("IDAT/encoded-rgba8-v1"));
    assert_eq!(ir["nodes"].as_array().expect("nodes").len(), 1, "{id}");
    assert!(ir["relations"].as_array().expect("relations").is_empty());
    assert!(ir["nodes"][0].get("role").is_none());
    assert!(ir["nodes"][0]["geometry"].get("inkBox").is_none());
}

fn without_source_name(mut ir: Value) -> Value {
    ir["artifact"]
        .as_object_mut()
        .expect("artifact object")
        .remove("sourceName");
    ir
}

fn assert_success_paths(case: &Value, png: &[u8], file: &TempPng) {
    let id = text(case, "id");
    let adapted = stdin(&["adapt-image", "-"], png);
    let ir = json_success(&adapted, id);
    assert_metadata(case, &ir);
    let direct = stdin(&["check-image", "-", "--format", "json"], png);
    let report = json_success(&direct, id);
    assert_eq!(report["reportSchemaVersion"], "0.2.0");
    assert_eq!(report["artifactKind"], "image");
    assert_eq!(report["summary"]["failed"], 0);
    assert!(report["results"].as_array().is_some());
    let through_ir = stdin(&["check", "-", "--format", "json"], &adapted.stdout);
    json_success(&through_ir, id);
    assert_eq!(direct.stdout, through_ir.stdout, "{id}: command paths diverged");
    let normalized = stdin(&["normalize", "-"], &adapted.stdout);
    json_success(&normalized, id);
    assert_eq!(adapted.stdout, normalized.stdout, "{id}: IR not canonical");

    let file_adapted = file.run("adapt-image", false);
    let file_ir = json_success(&file_adapted, id);
    assert_metadata(case, &file_ir);
    assert_eq!(without_source_name(ir), without_source_name(file_ir), "{id}");
    let file_checked = file.run("check-image", true);
    json_success(&file_checked, id);
    assert_eq!(direct.stdout, file_checked.stdout, "{id}: file/stdin differ");
    for _ in 0..3 {
        let repeated_ir = stdin(&["adapt-image", "-"], png);
        json_success(&repeated_ir, id);
        assert_eq!(adapted.stdout, repeated_ir.stdout, "{id}: unstable IR");
        let repeated_report = stdin(&["check-image", "-", "--format", "json"], png);
        json_success(&repeated_report, id);
        assert_eq!(direct.stdout, repeated_report.stdout, "{id}: unstable report");
    }
}

fn assert_error_paths(case: &Value, png: &[u8], file: &TempPng) {
    let id = text(case, "id");
    let expected = text(case, "errorContains");
    for verb in ["adapt-image", "check-image"] {
        let first = stdin(&[verb, "-"], png);
        let repeated = stdin(&[verb, "-"], png);
        for output in [&first, &repeated, &file.run(verb, false)] {
            assert_exit(output, 2, id);
            assert!(output.stdout.is_empty(), "{id}: partial successful output");
            assert!(String::from_utf8_lossy(&output.stderr).contains(expected), "{id}");
        }
        assert_eq!(first.stderr, repeated.stderr, "{id}: unstable error");
    }
}

#[test]
fn native_bytes_match_pixel_oracles_through_api_and_public_binary() {
    let cases = cases();
    for case in &cases {
        let id = text(case, "id");
        let png = decode_hex(text(case, "pngHex"));
        assert_api(case, &png);
        let file = TempPng::new(id, &png);
        if case["exitCode"] == 2 {
            assert_error_paths(case, &png, &file);
        } else {
            assert_success_paths(case, &png, &file);
        }
    }
    println!("{} committed native cases verified via API, file, stdin, and reports", cases.len());
}

#[test]
fn corpus_cannot_silently_drop_the_required_format_and_failure_matrix() {
    let cases = cases();
    assert_eq!(cases.len(), 38);
    let ids: BTreeSet<&str> = cases.iter().map(|case| text(case, "id")).collect();
    assert_eq!(ids.len(), cases.len());
    for color in [0, 2, 4, 6] {
        for filter in 0..5 {
            assert!(ids.contains(format!("color-{color}-filter-{filter}").as_str()));
        }
        assert!(ids.contains(format!("adam7-color-{color}").as_str()));
    }
    for id in [
        "adam7-1x1", "adam7-1x5", "adam7-5x1", "adam7-8x8", "unmanaged-gamma",
        "indexed", "packed", "sixteen-bit", "trns", "animation-control",
        "invalid-filter", "invalid-crc", "cards-clean", "cards-mutated",
    ] {
        assert!(ids.contains(id), "missing {id}");
    }
    for case in &cases {
        assert!([0, 2].iter().any(|code| case["exitCode"] == *code));
        if case["status"] == "available" {
            let width = case["width"].as_u64().expect("width");
            let height = case["height"].as_u64().expect("height");
            assert!(width > 0 && height > 0);
            let length = u64::try_from(decode_hex(text(case, "rgbaHex")).len()).expect("length");
            assert_eq!(length, width * height * 4);
        }
    }
}

#[test]
fn future_spacing_oracles_are_consistent_but_not_counted_as_detected() {
    let cases = cases();
    let clean = cases.iter().find(|case| case["id"] == "cards-clean").expect("baseline");
    let mutant = cases.iter().find(|case| case["id"] == "cards-mutated").expect("mutant");
    assert_eq!(mutant["future"]["baseline"], clean["id"]);
    assert_ne!(clean["rgbaHex"], mutant["rgbaHex"]);
    for case in [clean, mutant] {
        let future = &case["future"];
        assert_eq!(future["status"], "untested");
        assert_eq!(future["capability"], "peer-spacing");
        let bounds = future["peerBounds"].as_array().expect("peer bounds");
        assert_eq!(bounds.len(), 3);
        let mut gaps = Vec::new();
        for pair in bounds.windows(2) {
            let left = pair[0][0].as_i64().expect("left");
            let width = pair[0][2].as_i64().expect("width");
            let next = pair[1][0].as_i64().expect("next");
            gaps.push(next - left - width);
        }
        assert_eq!(Value::from(gaps.clone()), future["gaps"]);
        assert_eq!(Value::from(gaps[0] != gaps[1]), future["expectedDefect"]);
        for bounds in bounds {
            let x = bounds[0].as_u64().expect("x");
            let y = bounds[1].as_u64().expect("y");
            let width = bounds[2].as_u64().expect("width");
            let height = bounds[3].as_u64().expect("height");
            assert!(width > 0 && height > 0);
            assert!(x + width <= case["width"].as_u64().expect("canvas width"));
            assert!(y + height <= case["height"].as_u64().expect("canvas height"));
        }
    }
    println!("Future spacing oracle: clean [1,1], mutant [1,2]; detection remains UNTESTED");
}
