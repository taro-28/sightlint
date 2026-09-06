//! Strict decoding and semantic validation for the official Web acquisition extension.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::Deserialize;
use sightlint_ir::{
    ArtifactIr, ArtifactKind, Evidence, EvidenceClass, Identifier, Rect, Selector, Unit,
};

pub(crate) const WEB_EXTENSION_KEY: &str = "org.sightlint.web";
pub(crate) const LEGACY_WEB_EXTENSION_VERSION: &str = "0.3.0";
pub(crate) const WEB_EXTENSION_VERSION: &str = "0.4.0";
const WEB_ADAPTER_NAME: &str = "sightlint-playwright";
const LEGACY_WEB_ADAPTER_VERSION: &str = "0.3.0";
const WEB_ADAPTER_VERSION: &str = "0.4.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WebValidationCode {
    InvalidPayload,
    UnsupportedVersion,
    InvalidArtifactKind,
    InvalidReference,
    InvalidEvidenceClass,
    InvalidEvidenceProvenance,
    DuplicateIdentifier,
    InconsistentObservation,
    InvalidValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WebValidationIssue {
    pub(crate) code: WebValidationCode,
    pub(crate) path: String,
    pub(crate) message: String,
}

/// Ordered failures returned when a recognized Web extension is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebExtensionErrors {
    issues: Vec<WebValidationIssue>,
}

impl WebExtensionErrors {
    fn new(mut issues: Vec<WebValidationIssue>) -> Self {
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
        issues.dedup();
        Self { issues }
    }

    fn decode(message: impl Into<String>) -> Self {
        Self::new(vec![WebValidationIssue {
            code: WebValidationCode::InvalidPayload,
            path: String::new(),
            message: message.into(),
        }])
    }

    fn unsupported(version: &str) -> Self {
        Self::new(vec![WebValidationIssue {
            code: WebValidationCode::UnsupportedVersion,
            path: "/extensionVersion".to_owned(),
            message: format!(
                "expected Web extension version {LEGACY_WEB_EXTENSION_VERSION} or {WEB_EXTENSION_VERSION}, found {version}"
            ),
        }])
    }

    /// Returns the number of deterministic validation issues.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Returns whether no validation issues are present.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }
}

impl fmt::Display for WebExtensionErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "official Web extension validation failed with {} issue(s):",
            self.issues.len()
        )?;
        for issue in &self.issues {
            let path = if issue.path.is_empty() {
                "/"
            } else {
                issue.path.as_str()
            };
            writeln!(
                formatter,
                "- {:?} at {WEB_EXTENSION_KEY}{path}: {}",
                issue.code, issue.message
            )?;
        }
        Ok(())
    }
}

impl Error for WebExtensionErrors {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebExtension {
    pub(crate) extension_version: String,
    document: WebDocument,
    environment: WebEnvironment,
    capture: WebCapture,
    pub(crate) nodes: Vec<WebNode>,
    pub(crate) reconciliation: WebReconciliation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebDocument {
    id: String,
    frame_id: String,
    frames: Vec<WebFrame>,
    frame_count: u64,
    source_path: String,
    state: String,
    readiness_selector: String,
    scroll: PointWithUnit,
    document_size: SizeWithUnit,
    viewport_size: SizeWithUnit,
    direction: WebDirection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebFrame {
    id: String,
    parent_id: Option<String>,
    kind: String,
    source_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WebDirection {
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebEnvironment {
    viewport: SizeWithUnit,
    device_scale_factor: f64,
    text_scale: f64,
    locale: String,
    timezone_id: String,
    color_scheme: String,
    reduced_motion: String,
    browser_name: String,
    browser_version: String,
    playwright_version: String,
    platform: String,
    architecture: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebCapture {
    source_kind: Option<String>,
    source_files: Vec<String>,
    source_digest: String,
    loopback_responses: Option<LoopbackResponses>,
    screenshot: WebScreenshot,
    network: WebNetwork,
    privacy: WebPrivacy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebScreenshot {
    reference: String,
    sha256: String,
    byte_length: u64,
    pixel_size: IntegerSize,
    format: String,
    scale: String,
    animations: String,
    caret: String,
    color_assumptions: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebNetwork {
    mode: String,
    external_requests: Vec<String>,
    blocked_web_socket_count: Option<u64>,
    blocked_service_worker_count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LoopbackResponses {
    digest: String,
    request_count: u64,
    response_bytes: u64,
    route_path: String,
    target_digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WebPrivacy {
    accessible_name_mode: String,
    descendants_redacted: bool,
    external_processing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebNode {
    pub(crate) node_id: Identifier,
    pub(crate) locator: WebLocator,
    pub(crate) tag_name: String,
    pub(crate) dom_evidence_id: Identifier,
    pub(crate) render_evidence_id: Identifier,
    pub(crate) accessibility_evidence_id: Option<Identifier>,
    pub(crate) disabled: bool,
    pub(crate) interactive: bool,
    layout_method: LayoutMethod,
    layout_unavailable_reason: Option<String>,
    render_method: String,
    client_size: MeasuredSize,
    scroll_size: MeasuredSize,
    overflow_measurement: OverflowMeasurement,
    pub(crate) center_hit_sample: CenterHitSample,
    hit_region: CantTell,
    pub(crate) computed_style: ComputedStyle,
    pub(crate) clipping_ancestors: Vec<ClippingAncestor>,
    pub(crate) accessibility: AccessibilitySummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebLocator {
    pub(crate) r#type: LocatorType,
    pub(crate) value: String,
    selector: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LocatorType {
    TestId,
    Id,
    Css,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum LayoutMethod {
    OffsetParentBorderBoxToDocument,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Presence {
    Present,
    Absent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OverflowMeasurement {
    horizontal: Presence,
    vertical: Presence,
    method: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CenterHitSample {
    pub(crate) point: PointWithUnit,
    pub(crate) outcome: CenterHitOutcome,
    pub(crate) hit_locator: Option<String>,
    pub(crate) method: CenterHitMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CenterHitOutcome {
    Hit,
    NotInteractive,
    ZeroArea,
    OffViewport,
    Occluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CenterHitMethod {
    ElementFromPointAtRenderBoxCenter,
    NotSampled,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ComputedStyle {
    pub(crate) display: String,
    pub(crate) visibility: String,
    pub(crate) opacity: f64,
    overflow_x: String,
    overflow_y: String,
    white_space: String,
    text_overflow: String,
    font_size: String,
    line_height: String,
    font_weight: String,
    direction: String,
    writing_mode: String,
    pub(crate) transform: String,
    pub(crate) pointer_events: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ClippingAncestor {
    pub(crate) locator: String,
    pub(crate) overflow_x: String,
    pub(crate) overflow_y: String,
    pub(crate) rect: Rect,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AccessibilitySummary {
    pub(crate) status: AccessibilityStatus,
    pub(crate) role: Option<String>,
    pub(crate) name: Option<String>,
    states: Vec<String>,
    root_line: Option<String>,
    snapshot_digest: String,
    descendants_redacted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AccessibilityStatus {
    Observed,
    CantTell,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WebReconciliation {
    screenshot_viewport: ScreenshotViewport,
    pub(crate) nodes: Vec<ReconciliationNode>,
    pixel_content_comparison: CantTell,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScreenshotViewport {
    status: AgreementStatus,
    viewport_css_pixels: SizeWithUnit,
    screenshot_pixels: IntegerSize,
    screenshot_scale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AgreementStatus {
    Agreement,
    Conflict,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReconciliationNode {
    pub(crate) node_id: Identifier,
    pub(crate) screenshot_geometry_coverage: ScreenshotGeometryCoverage,
    native_visibility: NativeVisibility,
    layout_render: LayoutRender,
    pub(crate) ancestor_clip: AncestorClip,
    pixel_content_match: CantTell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ScreenshotGeometryCoverage {
    ZeroArea,
    OutsideScreenshot,
    PartiallyOutsideScreenshot,
    InsideScreenshotExtent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeVisibility {
    display: String,
    visibility: String,
    opacity: f64,
    center_hit_test: CenterHitOutcome,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum LayoutRender {
    Agreement {
        tolerance: LengthWithUnit,
        layout_box: Rect,
        render_box: Rect,
    },
    Conflict {
        tolerance: LengthWithUnit,
        layout_box: Rect,
        render_box: Rect,
    },
    CantTell {
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum AncestorClip {
    NotClipped {
        clipping_ancestor_locators: Vec<String>,
        method: String,
    },
    PartiallyClipped {
        clipping_ancestor_locators: Vec<String>,
        method: String,
    },
    FullyClipped {
        clipping_ancestor_locators: Vec<String>,
        method: String,
    },
    CantTell {
        reason: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CantTell {
    status: CantTellStatus,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum CantTellStatus {
    CantTell,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PointWithUnit {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) unit: Unit,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SizeWithUnit {
    width: u64,
    height: u64,
    unit: Unit,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MeasuredSize {
    width: u64,
    height: u64,
    unit: Unit,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IntegerSize {
    width: u64,
    height: u64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LengthWithUnit {
    value: f64,
    unit: Unit,
}

pub(crate) fn decode_web_extension(
    document: &ArtifactIr,
) -> Result<Option<WebExtension>, WebExtensionErrors> {
    let Some(value) = document.extensions.get(WEB_EXTENSION_KEY) else {
        return Ok(None);
    };
    let version = value
        .get("extensionVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| WebExtensionErrors::decode("extensionVersion must be a string"))?;
    if version != LEGACY_WEB_EXTENSION_VERSION && version != WEB_EXTENSION_VERSION {
        return Err(WebExtensionErrors::unsupported(version));
    }
    let extension = serde_json::from_value::<WebExtension>(value.clone())
        .map_err(|error| WebExtensionErrors::decode(error.to_string()))?;
    extension.validate(document)?;
    Ok(Some(extension))
}

impl WebExtension {
    fn validate(&self, document: &ArtifactIr) -> Result<(), WebExtensionErrors> {
        let mut validator = WebValidator::default();
        debug_assert!(
            self.extension_version == LEGACY_WEB_EXTENSION_VERSION
                || self.extension_version == WEB_EXTENSION_VERSION
        );
        if document.artifact.kind != ArtifactKind::Web {
            validator.issue(
                WebValidationCode::InvalidArtifactKind,
                "/",
                "the official Web extension requires artifact kind web",
            );
        }
        validate_environment(self, &mut validator);

        let core_nodes = document
            .nodes
            .iter()
            .map(|node| (&node.id, node))
            .collect::<BTreeMap<_, _>>();
        let evidence = document
            .evidence
            .iter()
            .map(|item| (&item.id, item))
            .collect::<BTreeMap<_, _>>();
        let mut node_ids = BTreeSet::new();
        let mut locators = BTreeSet::new();
        let adapter_version = if self.extension_version == WEB_EXTENSION_VERSION {
            WEB_ADAPTER_VERSION
        } else {
            LEGACY_WEB_ADAPTER_VERSION
        };
        for (index, node) in self.nodes.iter().enumerate() {
            let path = format!("/nodes/{index}");
            if !node_ids.insert(node.node_id.clone()) {
                validator.issue(
                    WebValidationCode::DuplicateIdentifier,
                    format!("{path}/nodeId"),
                    format!("Web node {} is duplicated", node.node_id),
                );
            }
            if !locators.insert(node.locator.value.clone()) {
                validator.issue(
                    WebValidationCode::DuplicateIdentifier,
                    format!("{path}/locator/value"),
                    format!("Web locator {} is duplicated", node.locator.value),
                );
            }
            validate_node(
                node,
                &path,
                core_nodes.get(&node.node_id).copied(),
                &evidence,
                &self.capture.source_digest,
                adapter_version,
                &mut validator,
            );
        }

        let mut reconciliation_ids = BTreeSet::new();
        let node_by_id = self
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect::<BTreeMap<_, _>>();
        for (index, reconciliation) in self.reconciliation.nodes.iter().enumerate() {
            let path = format!("/reconciliation/nodes/{index}");
            if !reconciliation_ids.insert(reconciliation.node_id.clone()) {
                validator.issue(
                    WebValidationCode::DuplicateIdentifier,
                    format!("{path}/nodeId"),
                    format!(
                        "reconciliation node {} is duplicated",
                        reconciliation.node_id
                    ),
                );
            }
            let Some(node) = node_by_id.get(&reconciliation.node_id).copied() else {
                validator.issue(
                    WebValidationCode::InvalidReference,
                    format!("{path}/nodeId"),
                    format!("Web node {} does not exist", reconciliation.node_id),
                );
                continue;
            };
            validate_reconciliation(node, reconciliation, &path, &mut validator);
        }
        if node_ids != reconciliation_ids {
            validator.issue(
                WebValidationCode::InconsistentObservation,
                "/reconciliation/nodes",
                "Web nodes and reconciliation nodes must have identical identifiers",
            );
        }
        validator.finish()
    }
}

fn validate_environment(extension: &WebExtension, validator: &mut WebValidator) {
    let document = &extension.document;
    let environment = &extension.environment;
    let screenshot_viewport = &extension.reconciliation.screenshot_viewport;
    for (path, value) in [
        ("/document/scroll/x", document.scroll.x),
        ("/document/scroll/y", document.scroll.y),
        (
            "/environment/deviceScaleFactor",
            environment.device_scale_factor,
        ),
        ("/environment/textScale", environment.text_scale),
    ] {
        validator.finite(path, value);
    }
    for (path, unit) in [
        ("/document/scroll/unit", document.scroll.unit),
        ("/document/documentSize/unit", document.document_size.unit),
        ("/document/viewportSize/unit", document.viewport_size.unit),
        ("/environment/viewport/unit", environment.viewport.unit),
        (
            "/reconciliation/screenshotViewport/viewportCssPixels/unit",
            screenshot_viewport.viewport_css_pixels.unit,
        ),
    ] {
        validator.css_pixel(path, unit);
    }
    validate_document_environment(extension, validator);
    validate_capture_environment(extension, validator);
    validate_screenshot_reconciliation(extension, validator);
    if self_consistency_strings(extension).is_err() {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/",
            "Web extension constant and required string fields are inconsistent",
        );
    }
}

fn validate_document_environment(extension: &WebExtension, validator: &mut WebValidator) {
    let document = &extension.document;
    let environment = &extension.environment;
    if document.frame_count != 1
        || document.frames.len() != 1
        || document.frames[0].parent_id.is_some()
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/document/frames",
            "the Web extension requires exactly one main frame",
        );
    }
    if document.id != "document-main"
        || document.frame_id != "frame-main"
        || document
            .frames
            .first()
            .is_none_or(|frame| frame.id != "frame-main" || frame.kind != "main")
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/document",
            "the Web extension requires the stable document-main/frame-main identity",
        );
    }
    if document.document_size.width == 0
        || document.document_size.height == 0
        || document.viewport_size.width == 0
        || document.viewport_size.height == 0
        || environment.viewport.width == 0
        || environment.viewport.height == 0
        || environment.viewport.width != document.viewport_size.width
        || environment.viewport.height != document.viewport_size.height
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/environment/viewport",
            "viewport and document viewport sizes must be positive and equal",
        );
    }
    if environment.device_scale_factor < 1.0 || environment.device_scale_factor > 2.0 {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/environment/deviceScaleFactor",
            "deviceScaleFactor must be from 1 through 2",
        );
    }
}

fn validate_capture_environment(extension: &WebExtension, validator: &mut WebValidator) {
    let capture = &extension.capture;
    if capture.privacy.external_processing || !capture.privacy.descendants_redacted {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/capture",
            "the local Web extension requires redacted descendants and no external processing",
        );
    }
    if capture.screenshot.byte_length == 0
        || capture.screenshot.pixel_size.width == 0
        || capture.screenshot.pixel_size.height == 0
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/capture",
            "positive screenshot dimensions are required",
        );
    }
    if extension.extension_version == LEGACY_WEB_EXTENSION_VERSION {
        if capture.source_kind.is_some()
            || capture.loopback_responses.is_some()
            || capture.source_files.is_empty()
            || capture.network.mode != "deny"
            || !capture.network.external_requests.is_empty()
            || capture.network.blocked_web_socket_count.is_some()
            || capture.network.blocked_service_worker_count.is_some()
        {
            validator.issue(
                WebValidationCode::InvalidValue,
                "/capture",
                "Web extension 0.3 requires repository files and denied network access",
            );
        }
        return;
    }
    let valid_loopback = capture.source_kind.as_deref() == Some("loopbackResponses")
        && capture.source_files.is_empty()
        && is_sha256_digest(&capture.source_digest)
        && is_sha256_digest(&capture.screenshot.sha256)
        && capture.network.mode == "sameOriginLoopback"
        && capture.network.external_requests.is_empty()
        && capture.network.blocked_web_socket_count.is_some()
        && capture.network.blocked_service_worker_count.is_some()
        && capture
            .loopback_responses
            .as_ref()
            .is_some_and(|responses| {
                responses.digest == capture.source_digest
                    && is_sha256_digest(&responses.digest)
                    && responses.request_count > 0
                    && responses.request_count <= 512
                    && responses.response_bytes <= 64 * 1024 * 1024
                    && is_route_path(&responses.route_path)
                    && is_sha256_digest(&responses.target_digest)
                    && extension.document.source_path == responses.route_path
                    && extension
                        .document
                        .frames
                        .first()
                        .is_some_and(|frame| frame.source_path == responses.route_path)
            });
    if !valid_loopback {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/capture",
            "Web extension 0.4 requires bounded same-origin loopback response evidence without source-file attribution",
        );
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn is_route_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2048
        && value.starts_with('/')
        && !value.starts_with("//")
        && !value
            .bytes()
            .any(|byte| byte <= 0x1f || byte == 0x7f || matches!(byte, b'?' | b'#' | b'\\'))
}

fn validate_screenshot_reconciliation(extension: &WebExtension, validator: &mut WebValidator) {
    let environment = &extension.environment;
    let capture = &extension.capture;
    let screenshot_viewport = &extension.reconciliation.screenshot_viewport;
    if screenshot_viewport.viewport_css_pixels.width != environment.viewport.width
        || screenshot_viewport.viewport_css_pixels.height != environment.viewport.height
        || screenshot_viewport.screenshot_pixels.width != capture.screenshot.pixel_size.width
        || screenshot_viewport.screenshot_pixels.height != capture.screenshot.pixel_size.height
        || screenshot_viewport.screenshot_scale != "css"
        || !matches!(
            screenshot_viewport.status,
            AgreementStatus::Agreement | AgreementStatus::Conflict
        )
    {
        validator.issue(
            WebValidationCode::InconsistentObservation,
            "/reconciliation/screenshotViewport",
            "screenshot reconciliation must match the captured viewport and PNG dimensions",
        );
    }
    if !matches!(
        extension.reconciliation.pixel_content_comparison.status,
        CantTellStatus::CantTell
    ) || extension
        .reconciliation
        .pixel_content_comparison
        .reason
        .is_empty()
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            "/reconciliation/pixelContentComparison",
            "pixel-content comparison must remain cantTell with a reason",
        );
    }
}

#[allow(clippy::too_many_lines)]
fn self_consistency_strings(extension: &WebExtension) -> Result<(), ()> {
    let document = &extension.document;
    let environment = &extension.environment;
    let capture = &extension.capture;
    let screenshot = &capture.screenshot;
    let frame = document.frames.first().ok_or(())?;
    let values = [
        document.source_path.as_str(),
        document.state.as_str(),
        document.readiness_selector.as_str(),
        frame.source_path.as_str(),
        environment.browser_version.as_str(),
        environment.platform.as_str(),
        environment.architecture.as_str(),
        capture.source_digest.as_str(),
        screenshot.reference.as_str(),
        screenshot.sha256.as_str(),
        screenshot.color_assumptions.as_str(),
    ];
    let locale_valid = environment.locale == "en-US"
        || (extension.extension_version == WEB_EXTENSION_VERSION && environment.locale == "ja-JP");
    if values.iter().any(|value| value.is_empty())
        || !locale_valid
        || environment.timezone_id != "UTC"
        || environment.color_scheme != "light"
        || environment.reduced_motion != "reduce"
        || environment.browser_name != "chromium"
        || environment.playwright_version != "1.63.0"
        || capture.privacy.accessible_name_mode != "selectedNodes"
        || screenshot.format != "png"
        || screenshot.scale != "css"
        || screenshot.animations != "disabled"
        || screenshot.caret != "hide"
        || !matches!(document.direction, WebDirection::Ltr | WebDirection::Rtl)
    {
        return Err(());
    }
    Ok(())
}

fn validate_node(
    node: &WebNode,
    path: &str,
    core: Option<&sightlint_ir::Node>,
    evidence: &BTreeMap<&Identifier, &Evidence>,
    source_digest: &str,
    adapter_version: &str,
    validator: &mut WebValidator,
) {
    let Some(core) = core else {
        validator.issue(
            WebValidationCode::InvalidReference,
            format!("{path}/nodeId"),
            format!("core node {} does not exist", node.node_id),
        );
        return;
    };
    validate_node_evidence(
        node,
        path,
        core,
        evidence,
        source_digest,
        adapter_version,
        validator,
    );
    validate_node_values(node, path, validator);
    for (index, ancestor) in node.clipping_ancestors.iter().enumerate() {
        validator.rect(
            &format!("{path}/clippingAncestors/{index}/rect"),
            ancestor.rect,
        );
    }
}

fn validate_node_evidence(
    node: &WebNode,
    path: &str,
    core: &sightlint_ir::Node,
    evidence: &BTreeMap<&Identifier, &Evidence>,
    source_digest: &str,
    adapter_version: &str,
    validator: &mut WebValidator,
) {
    let expected_source = EvidenceSourceExpectation {
        locator: &node.locator.value,
        source_digest,
        adapter_version,
    };
    validator.evidence(
        &node.dom_evidence_id,
        EvidenceClass::ExactSource,
        &format!("{path}/domEvidenceId"),
        expected_source,
        evidence,
    );
    validator.evidence(
        &node.render_evidence_id,
        EvidenceClass::ExactRender,
        &format!("{path}/renderEvidenceId"),
        expected_source,
        evidence,
    );
    if core.kind.evidence_id != node.dom_evidence_id
        || core
            .geometry
            .render_box
            .as_ref()
            .is_none_or(|rect| rect.evidence_id != node.render_evidence_id)
        || core
            .geometry
            .layout_box
            .as_ref()
            .is_some_and(|rect| rect.evidence_id != node.render_evidence_id)
    {
        validator.issue(
            WebValidationCode::InconsistentObservation,
            path,
            "core node observations must reference the Web node DOM/render evidence",
        );
    }
    match node.accessibility.status {
        AccessibilityStatus::Observed => {
            let Some(accessibility_id) = node.accessibility_evidence_id.as_ref() else {
                validator.issue(
                    WebValidationCode::InconsistentObservation,
                    format!("{path}/accessibilityEvidenceId"),
                    "observed accessibility requires a platform evidence identifier",
                );
                return;
            };
            validator.evidence(
                accessibility_id,
                EvidenceClass::PlatformSemantics,
                &format!("{path}/accessibilityEvidenceId"),
                expected_source,
                evidence,
            );
            if node.accessibility.role.is_none()
                || core.role.as_ref().is_none_or(|role| {
                    Some(role.value.as_str()) != node.accessibility.role.as_deref()
                        || &role.evidence_id != accessibility_id
                })
                || !matching_name(core, &node.accessibility, accessibility_id)
            {
                validator.issue(
                    WebValidationCode::InconsistentObservation,
                    format!("{path}/accessibility"),
                    "observed platform role/name must match the core node",
                );
            }
        }
        AccessibilityStatus::CantTell => {
            if node.accessibility_evidence_id.is_some()
                || node.accessibility.role.is_some()
                || node.accessibility.name.is_some()
            {
                validator.issue(
                    WebValidationCode::InconsistentObservation,
                    format!("{path}/accessibility"),
                    "cantTell accessibility cannot carry platform role/name evidence",
                );
            }
        }
    }
}

fn validate_node_values(node: &WebNode, path: &str, validator: &mut WebValidator) {
    if !node.computed_style.opacity.is_finite()
        || node.computed_style.opacity < 0.0
        || node.computed_style.opacity > 1.0
        || !node.center_hit_sample.point.x.is_finite()
        || !node.center_hit_sample.point.y.is_finite()
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            path,
            "opacity and center-hit coordinates must be finite and bounded",
        );
    }
    validator.css_pixel(
        &format!("{path}/centerHitSample/point/unit"),
        node.center_hit_sample.point.unit,
    );
    let sampled =
        node.center_hit_sample.method == CenterHitMethod::ElementFromPointAtRenderBoxCenter;
    let needs_sample = matches!(
        node.center_hit_sample.outcome,
        CenterHitOutcome::Hit | CenterHitOutcome::Occluded
    );
    if sampled != needs_sample
        || (!node.interactive && node.center_hit_sample.outcome != CenterHitOutcome::NotInteractive)
    {
        validator.issue(
            WebValidationCode::InconsistentObservation,
            format!("{path}/centerHitSample"),
            "center-hit method/outcome must match DOM interactivity and sampling",
        );
    }
    if node.locator.value.is_empty()
        || node.locator.selector.is_empty()
        || node.tag_name.is_empty()
        || node.render_method != "getBoundingClientRect"
        || node.client_size.unit != Unit::CssPixel
        || node.scroll_size.unit != Unit::CssPixel
        || node.overflow_measurement.method != "scrollSizeComparedWithClientSize"
        || !matches!(node.hit_region.status, CantTellStatus::CantTell)
        || node.hit_region.reason.is_empty()
        || !node.accessibility.descendants_redacted
        || node.accessibility.snapshot_digest.is_empty()
        || node.accessibility.states.iter().any(String::is_empty)
        || node
            .accessibility
            .root_line
            .as_ref()
            .is_some_and(String::is_empty)
        || matches!(node.layout_method, LayoutMethod::Unavailable)
            != node.layout_unavailable_reason.is_some()
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            path,
            "Web node constant, method, unit, or required string fields are invalid",
        );
    }
    let _ = (
        node.client_size.width,
        node.client_size.height,
        node.scroll_size.width,
        node.scroll_size.height,
        node.overflow_measurement.horizontal,
        node.overflow_measurement.vertical,
        node.computed_style.overflow_x.as_str(),
        node.computed_style.overflow_y.as_str(),
        node.computed_style.white_space.as_str(),
        node.computed_style.text_overflow.as_str(),
        node.computed_style.font_size.as_str(),
        node.computed_style.line_height.as_str(),
        node.computed_style.font_weight.as_str(),
        node.computed_style.direction.as_str(),
        node.computed_style.writing_mode.as_str(),
    );
}

fn matching_name(
    core: &sightlint_ir::Node,
    accessibility: &AccessibilitySummary,
    evidence_id: &Identifier,
) -> bool {
    match (&core.name, &accessibility.name) {
        (None, None) => true,
        (Some(core_name), Some(web_name)) => {
            core_name.value == *web_name && core_name.evidence_id == *evidence_id
        }
        _ => false,
    }
}

fn validate_reconciliation(
    node: &WebNode,
    reconciliation: &ReconciliationNode,
    path: &str,
    validator: &mut WebValidator,
) {
    if reconciliation.native_visibility.display != node.computed_style.display
        || reconciliation.native_visibility.visibility != node.computed_style.visibility
        || reconciliation.native_visibility.opacity.to_bits()
            != node.computed_style.opacity.to_bits()
        || reconciliation.native_visibility.center_hit_test != node.center_hit_sample.outcome
    {
        validator.issue(
            WebValidationCode::InconsistentObservation,
            format!("{path}/nativeVisibility"),
            "native visibility must match the corresponding Web node",
        );
    }
    match &reconciliation.layout_render {
        LayoutRender::Agreement {
            tolerance,
            layout_box,
            render_box,
        }
        | LayoutRender::Conflict {
            tolerance,
            layout_box,
            render_box,
        } => {
            validator.finite(
                &format!("{path}/layoutRender/tolerance/value"),
                tolerance.value,
            );
            validator.css_pixel(
                &format!("{path}/layoutRender/tolerance/unit"),
                tolerance.unit,
            );
            validator.rect(&format!("{path}/layoutRender/layoutBox"), *layout_box);
            validator.rect(&format!("{path}/layoutRender/renderBox"), *render_box);
        }
        LayoutRender::CantTell { reason } if reason.is_empty() => validator.issue(
            WebValidationCode::InvalidValue,
            format!("{path}/layoutRender/reason"),
            "cantTell requires a reason",
        ),
        LayoutRender::CantTell { .. } => {}
    }
    match &reconciliation.ancestor_clip {
        AncestorClip::NotClipped {
            clipping_ancestor_locators,
            method,
        }
        | AncestorClip::PartiallyClipped {
            clipping_ancestor_locators,
            method,
        }
        | AncestorClip::FullyClipped {
            clipping_ancestor_locators,
            method,
        } => {
            if method != "rectangularOverflowAncestorIntersection"
                || clipping_ancestor_locators.iter().any(String::is_empty)
            {
                validator.issue(
                    WebValidationCode::InvalidValue,
                    format!("{path}/ancestorClip"),
                    "ancestor clipping requires the declared method and non-empty locators",
                );
            }
        }
        AncestorClip::CantTell { reason } if reason.is_empty() => validator.issue(
            WebValidationCode::InvalidValue,
            format!("{path}/ancestorClip/reason"),
            "cantTell requires a reason",
        ),
        AncestorClip::CantTell { .. } => {}
    }
    if !matches!(
        reconciliation.pixel_content_match.status,
        CantTellStatus::CantTell
    ) || reconciliation.pixel_content_match.reason.is_empty()
    {
        validator.issue(
            WebValidationCode::InvalidValue,
            format!("{path}/pixelContentMatch"),
            "pixel-content matching must remain cantTell with a reason",
        );
    }
}

#[derive(Default)]
struct WebValidator {
    issues: Vec<WebValidationIssue>,
}

#[derive(Clone, Copy)]
struct EvidenceSourceExpectation<'a> {
    locator: &'a str,
    source_digest: &'a str,
    adapter_version: &'a str,
}

impl WebValidator {
    fn issue(
        &mut self,
        code: WebValidationCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(WebValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }

    fn evidence(
        &mut self,
        id: &Identifier,
        expected: EvidenceClass,
        path: &str,
        expected_source: EvidenceSourceExpectation<'_>,
        evidence: &BTreeMap<&Identifier, &Evidence>,
    ) {
        match evidence.get(id) {
            None => self.issue(
                WebValidationCode::InvalidReference,
                path,
                format!("evidence {id} does not exist"),
            ),
            Some(actual) if actual.class != expected => self.issue(
                WebValidationCode::InvalidEvidenceClass,
                path,
                format!(
                    "evidence {id} must be {expected:?}, found {:?}",
                    actual.class
                ),
            ),
            Some(actual)
                if actual.source.adapter != WEB_ADAPTER_NAME
                    || actual.source.adapter_version != expected_source.adapter_version
                    || actual.source.input_digest.as_deref()
                        != Some(expected_source.source_digest)
                    || actual.source.model.is_some()
                    || actual.source.external_processing
                    || !matches!(
                        actual.selector.as_ref(),
                        Some(Selector::NativeId { native_id }) if native_id == expected_source.locator
                    ) =>
            {
                self.issue(
                    WebValidationCode::InvalidEvidenceProvenance,
                    path,
                    format!(
                        "evidence {id} must identify the local {WEB_ADAPTER_NAME}@{} source digest and native locator",
                        expected_source.adapter_version
                    ),
                );
            }
            Some(_) => {}
        }
    }

    fn finite(&mut self, path: &str, value: f64) {
        if !value.is_finite() {
            self.issue(
                WebValidationCode::InvalidValue,
                path,
                "value must be finite",
            );
        }
    }

    fn css_pixel(&mut self, path: &str, unit: Unit) {
        if unit != Unit::CssPixel {
            self.issue(
                WebValidationCode::InvalidValue,
                path,
                "unit must be cssPixel",
            );
        }
    }

    fn rect(&mut self, path: &str, rect: Rect) {
        if !rect.x.is_finite()
            || !rect.y.is_finite()
            || !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width < 0.0
            || rect.height < 0.0
        {
            self.issue(
                WebValidationCode::InvalidValue,
                path,
                "rectangle coordinates must be finite and dimensions non-negative",
            );
        }
    }

    fn finish(self) -> Result<(), WebExtensionErrors> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(WebExtensionErrors::new(self.issues))
        }
    }
}
