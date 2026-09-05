//! Deterministic advisory rules in the zero-setup recommended Web profile.

use std::collections::{BTreeMap, BTreeSet};

use sightlint_ir::{BoxKind, Identifier};

use crate::geometry::{QueryContext, right, within_canvas};
use crate::report::{
    Measurement, PolicyProvenance, PolicySourceKind, RuleEnforcement, RuleKind, RuleMaturity,
    RuleOutcome, RuleResult, Target, TargetKind,
};
use crate::rules::{InputAspect, RuleDefinition, RulePolicyDefinition};
use crate::web_extension::{
    AccessibilityStatus, AncestorClip, CenterHitMethod, CenterHitOutcome, ClippingAncestor,
    LocatorType, ReconciliationNode, ScreenshotGeometryCoverage, WebExtension, WebNode,
};

const RECOMMENDED_PROFILE: &str = "sightlint:recommended";

static INTERACTIVE_NAME_DEFINITION: RuleDefinition = RuleDefinition {
    id: "web.accessibility.interactive-name",
    version: "0.1.0",
    title: "Interactive controls expose a programmatic name",
    input_aspects: &[InputAspect::WebStructure, InputAspect::Evidence],
    maturity: RuleMaturity::Advisory,
    policy: RulePolicyDefinition {
        profile: RECOMMENDED_PROFILE,
        source_kind: PolicySourceKind::PlatformStandard,
        source_id: "wcag:4.1.2-name-role-value",
        source_version: "2.2",
        reference: "https://www.w3.org/TR/WCAG22/#name-role-value",
        enforcement: RuleEnforcement::Advisory,
    },
};

static CENTER_HIT_DEFINITION: RuleDefinition = RuleDefinition {
    id: "web.interaction.center-hit",
    version: "0.1.0",
    title: "Visible controls are not unexpectedly blocked at their center",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::WebStructure,
        InputAspect::WebReconciliation,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Advisory,
    policy: RulePolicyDefinition {
        profile: RECOMMENDED_PROFILE,
        source_kind: PolicySourceKind::ConservativeBuiltIn,
        source_id: "sightlint:web-center-hit",
        source_version: "0.1.0",
        reference: "docs/decisions/0035-recommended-web-profile-and-advisory-enforcement.md",
        enforcement: RuleEnforcement::Advisory,
    },
};

static ANCESTOR_CLIP_DEFINITION: RuleDefinition = RuleDefinition {
    id: "web.interaction.ancestor-clip",
    version: "0.1.0",
    title: "Visible controls are not clipped by non-scrollable ancestors",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::WebStructure,
        InputAspect::WebReconciliation,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Advisory,
    policy: RulePolicyDefinition {
        profile: RECOMMENDED_PROFILE,
        source_kind: PolicySourceKind::ConservativeBuiltIn,
        source_id: "sightlint:web-ancestor-clip",
        source_version: "0.1.0",
        reference: "docs/decisions/0035-recommended-web-profile-and-advisory-enforcement.md",
        enforcement: RuleEnforcement::Advisory,
    },
};

pub(crate) fn run_recommended_web_rules(
    context: &QueryContext<'_>,
    extension: &WebExtension,
) -> Vec<RuleResult> {
    let reconciliation = extension
        .reconciliation
        .nodes
        .iter()
        .map(|item| (&item.node_id, item))
        .collect::<BTreeMap<_, _>>();
    let locators = extension
        .nodes
        .iter()
        .map(|node| (node.locator.value.as_str(), &node.node_id))
        .collect::<BTreeMap<_, _>>();
    let mut nodes = extension
        .nodes
        .iter()
        .filter(|node| node.interactive)
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let mut results = Vec::new();
    for node in nodes {
        let reconciled = reconciliation
            .get(&node.node_id)
            .copied()
            .expect("validated Web nodes have reconciliation records");
        results.push(evaluate_interactive_name(context, node));
        results.push(evaluate_center_hit(context, node, reconciled, &locators));
        results.push(evaluate_ancestor_clip(context, node, reconciled));
    }

    if results.is_empty() {
        for definition in [
            &INTERACTIVE_NAME_DEFINITION,
            &CENTER_HIT_DEFINITION,
            &ANCESTOR_CLIP_DEFINITION,
        ] {
            results.push(build_web_result(
                definition,
                context,
                &context.document().artifact.id,
                TargetKind::Artifact,
                RuleOutcome::Inapplicable,
                "the Web artifact contains no observed interactive nodes".to_owned(),
                Vec::new(),
                BTreeMap::new(),
            ));
        }
    }
    results
}

fn evaluate_interactive_name(context: &QueryContext<'_>, node: &WebNode) -> RuleResult {
    let (outcome, message) = if is_rendered(node, context) {
        match node.accessibility.status {
            AccessibilityStatus::CantTell => (
                RuleOutcome::CantTell,
                "platform accessibility did not expose a supported role/name summary".to_owned(),
            ),
            AccessibilityStatus::Observed
                if !node
                    .accessibility
                    .role
                    .as_deref()
                    .is_some_and(is_admitted_ui_control_role) =>
            {
                (
                    RuleOutcome::CantTell,
                    "the observed platform role is outside the conservative UI-control applicability set"
                        .to_owned(),
                )
            }
            AccessibilityStatus::Observed
                if node.accessibility.name.as_deref().is_none_or(str::is_empty) =>
            {
                    (
                        RuleOutcome::Failed,
                        format!(
                            "platform accessibility exposed role {} but no programmatic name; provide visible text, aria-label, aria-labelledby, or another supported name source",
                            node.accessibility.role.as_deref().unwrap_or("unknown")
                        ),
                )
            }
            AccessibilityStatus::Observed => (
                RuleOutcome::Passed,
                format!(
                    "platform accessibility exposed role {} with a non-empty programmatic name",
                    node.accessibility.role.as_deref().unwrap_or("unknown")
                ),
            ),
        }
    } else {
        (
            RuleOutcome::Inapplicable,
            "the interactive node is not rendered in the captured state".to_owned(),
        )
    };
    build_web_result(
        &INTERACTIVE_NAME_DEFINITION,
        context,
        &node.node_id,
        TargetKind::Node,
        outcome,
        message,
        node_evidence(node, true),
        BTreeMap::new(),
    )
}

fn evaluate_center_hit(
    context: &QueryContext<'_>,
    node: &WebNode,
    reconciliation: &ReconciliationNode,
    locators: &BTreeMap<&str, &Identifier>,
) -> RuleResult {
    let (outcome, message, mut evidence) =
        center_hit_outcome(context, node, reconciliation, locators);
    evidence.sort();
    evidence.dedup();
    let mut measurements = BTreeMap::new();
    measurements.insert(
        "sampleX".to_owned(),
        Measurement {
            value: node.center_hit_sample.point.x,
            unit: node.center_hit_sample.point.unit,
        },
    );
    measurements.insert(
        "sampleY".to_owned(),
        Measurement {
            value: node.center_hit_sample.point.y,
            unit: node.center_hit_sample.point.unit,
        },
    );
    build_web_result(
        &CENTER_HIT_DEFINITION,
        context,
        &node.node_id,
        TargetKind::Node,
        outcome,
        message,
        evidence,
        measurements,
    )
}

fn center_hit_outcome(
    context: &QueryContext<'_>,
    node: &WebNode,
    reconciliation: &ReconciliationNode,
    locators: &BTreeMap<&str, &Identifier>,
) -> (RuleOutcome, String, Vec<Identifier>) {
    if !is_rendered(node, context) || node.disabled {
        (
            RuleOutcome::Inapplicable,
            "the node is hidden, zero-area, or disabled in the captured state".to_owned(),
            node_evidence(node, false),
        )
    } else if !is_native_control(&node.tag_name) || node.locator.r#type != LocatorType::TestId {
        (
            RuleOutcome::Inapplicable,
            "the conservative center-sample rule requires a native control with a stable test-id locator".to_owned(),
            node_evidence(node, false),
        )
    } else if node.computed_style.transform != "none"
        || node.computed_style.pointer_events != "auto"
    {
        (
            RuleOutcome::CantTell,
            "transforms or pointer-event overrides require richer hit-region evidence".to_owned(),
            node_evidence(node, false),
        )
    } else if !matches!(
        reconciliation.ancestor_clip,
        AncestorClip::NotClipped { .. }
    ) {
        (
            RuleOutcome::CantTell,
            "ancestor clipping prevents the center sample from establishing ordinary unobstructed applicability".to_owned(),
            node_evidence(node, false),
        )
    } else if reconciliation.screenshot_geometry_coverage
        != ScreenshotGeometryCoverage::InsideScreenshotExtent
    {
        (
            RuleOutcome::CantTell,
            "the control is not fully inside the captured screenshot extent".to_owned(),
            node_evidence(node, false),
        )
    } else if node.center_hit_sample.method != CenterHitMethod::ElementFromPointAtRenderBoxCenter {
        (
            RuleOutcome::CantTell,
            "the render-box center was not sampled with elementFromPoint".to_owned(),
            node_evidence(node, false),
        )
    } else {
        match node.center_hit_sample.outcome {
            CenterHitOutcome::Hit => (
                RuleOutcome::Passed,
                "elementFromPoint hit the control or one of its descendants at the render-box center".to_owned(),
                node_evidence(node, false),
            ),
            CenterHitOutcome::Occluded => {
                let hit_locator = node.center_hit_sample.hit_locator.as_deref();
                if hit_locator.is_some_and(|locator| {
                    locators
                        .get(locator)
                        .is_some_and(|id| is_dialog_or_descendant(context, id))
                }) {
                    let mut evidence = node_evidence(node, false);
                    if let Some(blocker) = hit_locator.and_then(|locator| locators.get(locator)) {
                        evidence.extend(core_semantic_evidence(context, blocker));
                    }
                    (
                        RuleOutcome::CantTell,
                        "a source-observed dialog covers the control center; static capture cannot decide whether the overlay state is valid".to_owned(),
                        evidence,
                    )
                } else {
                    (
                        RuleOutcome::Failed,
                        format!(
                            "elementFromPoint hit {} instead of the control at its render-box center; remove or reposition an unexpected blocker, or expose an intentional modal overlay as a dialog",
                            hit_locator.unwrap_or("an unselected element")
                        ),
                        node_evidence(node, false),
                    )
                }
            }
            CenterHitOutcome::OffViewport | CenterHitOutcome::ZeroArea => (
                RuleOutcome::CantTell,
                "the render-box center was unavailable in the captured viewport".to_owned(),
                node_evidence(node, false),
            ),
            CenterHitOutcome::NotInteractive => (
                RuleOutcome::CantTell,
                "DOM interactivity and the center-hit observation conflict".to_owned(),
                node_evidence(node, false),
            ),
        }
    }
}

fn evaluate_ancestor_clip(
    context: &QueryContext<'_>,
    node: &WebNode,
    reconciliation: &ReconciliationNode,
) -> RuleResult {
    let (outcome, message) = if !is_rendered(node, context) || node.disabled {
        (
            RuleOutcome::Inapplicable,
            "the node is hidden, zero-area, or disabled in the captured state".to_owned(),
        )
    } else if !is_native_control(&node.tag_name) {
        (
            RuleOutcome::Inapplicable,
            "the conservative clipping rule applies only to native controls".to_owned(),
        )
    } else if node.computed_style.transform != "none" {
        (
            RuleOutcome::CantTell,
            "a transformed control requires non-rectangular or transformed clipping evidence"
                .to_owned(),
        )
    } else if !render_box_inside_document(context, &node.node_id) {
        (
            RuleOutcome::Inapplicable,
            "the render box is outside its document canvas and is handled by the bounds rule"
                .to_owned(),
        )
    } else {
        match &reconciliation.ancestor_clip {
            AncestorClip::NotClipped { .. } => (
                RuleOutcome::Passed,
                "rectangular overflow-ancestor reconciliation found no clipping".to_owned(),
            ),
            AncestorClip::CantTell { .. } => (
                RuleOutcome::CantTell,
                "rectangular overflow-ancestor clipping could not be established".to_owned(),
            ),
            AncestorClip::PartiallyClipped {
                clipping_ancestor_locators,
                ..
            }
            | AncestorClip::FullyClipped {
                clipping_ancestor_locators,
                ..
            } => clipping_outcome(context, node, clipping_ancestor_locators),
        }
    };
    build_web_result(
        &ANCESTOR_CLIP_DEFINITION,
        context,
        &node.node_id,
        TargetKind::Node,
        outcome,
        message,
        node_evidence(node, false),
        BTreeMap::new(),
    )
}

fn clipping_outcome(
    context: &QueryContext<'_>,
    node: &WebNode,
    reconciled_locators: &[String],
) -> (RuleOutcome, String) {
    let Some(render) = context.rect(&node.node_id, BoxKind::Render).ok().flatten() else {
        return (
            RuleOutcome::CantTell,
            "the control render box is unavailable for clipping reconciliation".to_owned(),
        );
    };
    let reconciled = reconciled_locators.iter().collect::<BTreeSet<_>>();
    let causes = node
        .clipping_ancestors
        .iter()
        .filter(|ancestor| reconciled.contains(&ancestor.locator) && clips(ancestor, render.rect))
        .collect::<Vec<_>>();
    if causes.is_empty() {
        return (
            RuleOutcome::CantTell,
            "the reconciled clip could not be attributed to a recorded rectangular ancestor"
                .to_owned(),
        );
    }
    if causes.iter().any(|ancestor| {
        is_non_scroll_clip(&ancestor.overflow_x) || is_non_scroll_clip(&ancestor.overflow_y)
    }) {
        (
            RuleOutcome::Failed,
            "a hidden/clip overflow ancestor partially or fully clips the native control; enlarge or reposition the control/container, or use an intentional scrollable pattern"
                .to_owned(),
        )
    } else {
        (
            RuleOutcome::CantTell,
            "the control is partially visible inside an auto/scroll ancestor; static capture cannot prove harmful clipping".to_owned(),
        )
    }
}

fn clips(ancestor: &ClippingAncestor, child: sightlint_ir::Rect) -> bool {
    let horizontal = is_clipping_overflow(&ancestor.overflow_x)
        && (child.x < ancestor.rect.x || right(child) > right(ancestor.rect));
    let vertical = is_clipping_overflow(&ancestor.overflow_y)
        && (child.y < ancestor.rect.y
            || child.y + child.height > ancestor.rect.y + ancestor.rect.height);
    horizontal || vertical
}

fn is_clipping_overflow(value: &str) -> bool {
    matches!(value, "auto" | "clip" | "hidden" | "scroll")
}

fn is_non_scroll_clip(value: &str) -> bool {
    matches!(value, "clip" | "hidden")
}

fn is_native_control(tag_name: &str) -> bool {
    matches!(tag_name, "a" | "button" | "input" | "select" | "textarea")
}

fn is_admitted_ui_control_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "checkbox"
            | "combobox"
            | "link"
            | "listbox"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "option"
            | "radio"
            | "searchbox"
            | "slider"
            | "spinbutton"
            | "switch"
            | "tab"
            | "textbox"
            | "treeitem"
    )
}

fn is_rendered(node: &WebNode, context: &QueryContext<'_>) -> bool {
    node.computed_style.display != "none"
        && !matches!(
            node.computed_style.visibility.as_str(),
            "hidden" | "collapse"
        )
        && node.computed_style.opacity > 0.0
        && context
            .rect(&node.node_id, BoxKind::Render)
            .ok()
            .flatten()
            .is_some_and(|rect| rect.rect.width > 0.0 && rect.rect.height > 0.0)
}

fn render_box_inside_document(context: &QueryContext<'_>, node_id: &Identifier) -> bool {
    context
        .rect(node_id, BoxKind::Render)
        .ok()
        .flatten()
        .is_some_and(|rect| within_canvas(rect.rect, rect.canvas, 0.0))
}

fn is_dialog_or_descendant(context: &QueryContext<'_>, node_id: &Identifier) -> bool {
    let mut current = context.node(node_id).ok();
    let mut visited = BTreeSet::new();
    while let Some(node) = current {
        if !visited.insert(node.id.clone()) {
            return false;
        }
        if node
            .role
            .as_ref()
            .is_some_and(|role| role.value == "dialog")
        {
            return true;
        }
        current = node
            .parent_id
            .as_ref()
            .and_then(|parent| context.node(parent).ok());
    }
    false
}

fn core_semantic_evidence(context: &QueryContext<'_>, node_id: &Identifier) -> Vec<Identifier> {
    context.node(node_id).ok().map_or_else(Vec::new, |node| {
        node.role
            .iter()
            .map(|role| role.evidence_id.clone())
            .chain(node.name.iter().map(|name| name.evidence_id.clone()))
            .collect()
    })
}

fn node_evidence(node: &WebNode, include_accessibility: bool) -> Vec<Identifier> {
    let mut evidence = vec![
        node.dom_evidence_id.clone(),
        node.render_evidence_id.clone(),
    ];
    if include_accessibility {
        evidence.extend(node.accessibility_evidence_id.iter().cloned());
    }
    evidence
}

#[allow(clippy::too_many_arguments)]
fn build_web_result(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    target_id: &Identifier,
    target_kind: TargetKind,
    outcome: RuleOutcome,
    message: String,
    evidence_ids: Vec<Identifier>,
    measurements: BTreeMap<String, Measurement>,
) -> RuleResult {
    let evidence_classes = evidence_ids
        .iter()
        .filter_map(|id| {
            context
                .document()
                .evidence
                .iter()
                .find(|evidence| &evidence.id == id)
                .map(|evidence| evidence.class)
        })
        .collect();
    RuleResult {
        rule_id: definition.id.to_owned(),
        rule_version: definition.version.to_owned(),
        title: definition.title.to_owned(),
        kind: RuleKind::Atomic,
        maturity: definition.maturity,
        policy: PolicyProvenance {
            profile: definition.policy.profile.to_owned(),
            source_kind: definition.policy.source_kind,
            source_id: definition.policy.source_id.to_owned(),
            source_version: definition.policy.source_version.to_owned(),
            reference: definition.policy.reference.to_owned(),
        },
        enforcement: definition.policy.enforcement,
        target: Target {
            kind: target_kind,
            id: target_id.clone(),
            aspect: None,
        },
        outcome,
        message,
        evidence_ids,
        evidence_classes,
        related_node_ids: vec![target_id.clone()],
        measurements,
    }
}

#[cfg(test)]
mod tests {
    use sightlint_ir::Rect;

    use super::{clips, is_admitted_ui_control_role, is_clipping_overflow, is_non_scroll_clip};
    use crate::web_extension::ClippingAncestor;

    #[test]
    fn clipping_policy_distinguishes_hidden_from_scrollable_ancestors() {
        let child = Rect {
            x: -4.0,
            y: 5.0,
            width: 20.0,
            height: 10.0,
        };
        let ancestor = ClippingAncestor {
            locator: "testid:viewport".to_owned(),
            overflow_x: "hidden".to_owned(),
            overflow_y: "visible".to_owned(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        };
        assert!(clips(&ancestor, child));
        assert!(is_clipping_overflow("auto"));
        assert!(is_non_scroll_clip("hidden"));
        assert!(!is_non_scroll_clip("scroll"));
    }

    #[test]
    fn name_rule_abstains_outside_the_conservative_control_role_set() {
        assert!(is_admitted_ui_control_role("button"));
        assert!(is_admitted_ui_control_role("textbox"));
        assert!(!is_admitted_ui_control_role("generic"));
        assert!(!is_admitted_ui_control_role("group"));
    }
}
