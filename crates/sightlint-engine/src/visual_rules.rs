//! Deterministic M2 rules over the official visual extension.

use std::collections::BTreeMap;

use sightlint_ir::{
    AlignmentAnchor, Axis, BoxKind, ExtentDimension, HorizontalDirection, Identifier, Length, Unit,
    VerticalDirection, VisualContract, VisualExtension,
};

use crate::geometry::{QueryContext, ResolvedRect, bottom, ensure_comparable, right};
use crate::report::{
    Measurement, RuleKind, RuleMaturity, RuleOutcome, RuleResult, Target, TargetKind,
};
use crate::rules::{InputAspect, RuleDefinition};

/// Runs every built-in M2 rule against a validated official visual extension.
pub(crate) fn run_visual_rules(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
) -> Vec<RuleResult> {
    let mut results = evaluate_parent_containment(context);
    results.extend(evaluate_peer_alignment(context, extension));
    results.extend(evaluate_peer_extent(context, extension));
    results.extend(evaluate_peer_font_size(context, extension));
    results.extend(evaluate_minimum_font_size(context, extension));
    results
}

static PARENT_CONTAINMENT_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.geometry.parent-containment",
    version: "0.1.0",
    title: "Child bounds stay within parent bounds",
    input_aspects: &[InputAspect::NodeGeometry, InputAspect::Evidence],
    maturity: RuleMaturity::Experimental,
};

static PEER_ALIGNMENT_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.alignment.peer-consistency",
    version: "0.1.0",
    title: "Declared peer alignment is consistent",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::DeclaredVisualContracts,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

static PEER_EXTENT_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.extent.peer-consistency",
    version: "0.1.0",
    title: "Declared peer extent is consistent",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::DeclaredVisualContracts,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

static PEER_FONT_SIZE_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.typography.peer-font-size",
    version: "0.1.0",
    title: "Declared peer font sizes are consistent",
    input_aspects: &[
        InputAspect::VisualStyle,
        InputAspect::DeclaredVisualContracts,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

static MINIMUM_FONT_SIZE_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.typography.minimum-font-size",
    version: "0.1.0",
    title: "Observed font size meets an explicit minimum",
    input_aspects: &[
        InputAspect::VisualStyle,
        InputAspect::DeclaredVisualContracts,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

fn evaluate_parent_containment(context: &QueryContext<'_>) -> Vec<RuleResult> {
    let mut children = context
        .document()
        .nodes
        .iter()
        .filter(|node| node.parent_id.is_some())
        .collect::<Vec<_>>();
    children.sort_by(|left, right| left.id.cmp(&right.id));

    if children.is_empty() {
        return vec![inapplicable_result(
            &PARENT_CONTAINMENT_DEFINITION,
            context,
            "the artifact contains no parent-child node relationship",
        )];
    }

    let mut results = Vec::new();
    for child in children {
        let parent_id = child
            .parent_id
            .as_ref()
            .expect("nodes were filtered by parent identifier");
        let mut emitted = false;
        for box_kind in [BoxKind::Layout, BoxKind::Render, BoxKind::Ink, BoxKind::Hit] {
            let child_rect = context.rect(&child.id, box_kind);
            let parent_rect = context.rect(parent_id, box_kind);
            if matches!(&child_rect, Ok(None)) && matches!(&parent_rect, Ok(None)) {
                continue;
            }
            emitted = true;
            results.push(parent_containment_result(
                context,
                &child.id,
                parent_id,
                box_kind,
                child_rect,
                parent_rect,
            ));
        }
        if !emitted {
            results.push(build_result(
                &PARENT_CONTAINMENT_DEFINITION,
                Target {
                    kind: TargetKind::Node,
                    id: child.id.clone(),
                    aspect: Some(format!("parent:{parent_id}")),
                },
                RuleOutcome::CantTell,
                format!(
                    "child {} and parent {parent_id} have no matching observed bounds",
                    child.id
                ),
                Vec::new(),
                vec![child.id.clone(), parent_id.clone()],
                BTreeMap::new(),
                context,
            ));
        }
    }
    results
}

fn parent_containment_result(
    context: &QueryContext<'_>,
    child_id: &Identifier,
    parent_id: &Identifier,
    box_kind: BoxKind,
    child_result: Result<Option<ResolvedRect<'_>>, crate::QueryError>,
    parent_result: Result<Option<ResolvedRect<'_>>, crate::QueryError>,
) -> RuleResult {
    let target = Target {
        kind: TargetKind::Node,
        id: child_id.clone(),
        aspect: Some(format!("{}:parent:{parent_id}", box_kind.as_str())),
    };
    let related = vec![child_id.clone(), parent_id.clone()];

    let (child, parent) = match (child_result, parent_result) {
        (Ok(Some(child)), Ok(Some(parent))) => (child, parent),
        (Err(error), _) | (_, Err(error)) => {
            return build_result(
                &PARENT_CONTAINMENT_DEFINITION,
                target,
                RuleOutcome::CantTell,
                format!("parent containment cannot be compared: {error}"),
                Vec::new(),
                related,
                BTreeMap::new(),
                context,
            );
        }
        (Ok(None), Ok(Some(parent))) => {
            return build_result(
                &PARENT_CONTAINMENT_DEFINITION,
                target,
                RuleOutcome::CantTell,
                format!("child {child_id} has no {} observation", box_kind.as_str()),
                vec![parent.evidence_id.clone()],
                related,
                BTreeMap::new(),
                context,
            );
        }
        (Ok(Some(child)), Ok(None)) => {
            return build_result(
                &PARENT_CONTAINMENT_DEFINITION,
                target,
                RuleOutcome::CantTell,
                format!(
                    "parent {parent_id} has no {} observation",
                    box_kind.as_str()
                ),
                vec![child.evidence_id.clone()],
                related,
                BTreeMap::new(),
                context,
            );
        }
        (Ok(None), Ok(None)) => unreachable!("empty pairs are filtered before evaluation"),
    };

    if let Err(error) = ensure_comparable(child, parent) {
        return build_result(
            &PARENT_CONTAINMENT_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!("parent containment cannot be compared: {error}"),
            vec![child.evidence_id.clone(), parent.evidence_id.clone()],
            related,
            BTreeMap::new(),
            context,
        );
    }

    let contained = rect_contains(parent, child, 0.0);
    let mut measurements = BTreeMap::new();
    for (name, value) in [
        ("childX", child.rect.x),
        ("childY", child.rect.y),
        ("childWidth", child.rect.width),
        ("childHeight", child.rect.height),
        ("parentX", parent.rect.x),
        ("parentY", parent.rect.y),
        ("parentWidth", parent.rect.width),
        ("parentHeight", parent.rect.height),
    ] {
        insert_measurement(&mut measurements, name, value, child.unit);
    }

    build_result(
        &PARENT_CONTAINMENT_DEFINITION,
        target,
        if contained {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        if contained {
            format!(
                "{} for child {child_id} is contained by parent {parent_id}",
                box_kind.as_str()
            )
        } else {
            format!(
                "{} for child {child_id} extends outside parent {parent_id}",
                box_kind.as_str()
            )
        },
        vec![child.evidence_id.clone(), parent.evidence_id.clone()],
        related,
        measurements,
        context,
    )
}

fn rect_contains(parent: ResolvedRect<'_>, child: ResolvedRect<'_>, tolerance: f64) -> bool {
    child.rect.x >= parent.rect.x - tolerance
        && child.rect.y >= parent.rect.y - tolerance
        && right(child.rect) <= right(parent.rect) + tolerance
        && bottom(child.rect) <= bottom(parent.rect) + tolerance
}

fn evaluate_peer_alignment(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
) -> Vec<RuleResult> {
    let mut results = extension
        .contracts
        .iter()
        .filter_map(|(contract_id, contract)| match contract {
            VisualContract::PeerAlignment {
                node_ids,
                axis,
                anchor,
                box_kind,
                tolerance,
                evidence_id,
            } => Some(peer_alignment_result(
                context,
                contract_id,
                node_ids,
                *axis,
                *anchor,
                *box_kind,
                *tolerance,
                evidence_id,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        results.push(inapplicable_result(
            &PEER_ALIGNMENT_DEFINITION,
            context,
            "the visual extension declares no peer-alignment contract",
        ));
    }
    results
}

#[allow(clippy::too_many_arguments)]
fn peer_alignment_result(
    context: &QueryContext<'_>,
    contract_id: &Identifier,
    node_ids: &[Identifier],
    axis: Axis,
    anchor: AlignmentAnchor,
    box_kind: BoxKind,
    tolerance: f64,
    contract_evidence_id: &Identifier,
) -> RuleResult {
    let analysis = resolve_rect_set(context, node_ids, box_kind, contract_evidence_id);
    let target = Target {
        kind: TargetKind::Relation,
        id: contract_id.clone(),
        aspect: Some(format!(
            "{}:{}:{}",
            axis_label(axis),
            alignment_anchor_label(anchor),
            box_kind.as_str()
        )),
    };

    if let Some(reason) = &analysis.reason {
        return build_result(
            &PEER_ALIGNMENT_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!("peer alignment cannot be compared: {reason}"),
            analysis.evidence_ids,
            node_ids.to_vec(),
            BTreeMap::new(),
            context,
        );
    }
    let rects = &analysis.rects;

    let values = rects
        .iter()
        .copied()
        .map(|rect| alignment_coordinate(rect, axis, anchor))
        .collect::<Vec<_>>();
    let baseline = median(&values);
    let deviation = maximum_deviation(&values, baseline);
    let unit = rects[0].unit;
    let mut measurements = distribution_measurements(&values, baseline, deviation, tolerance, unit);
    insert_measurement(&mut measurements, "baselineAnchor", baseline, unit);

    build_result(
        &PEER_ALIGNMENT_DEFINITION,
        target,
        if deviation <= tolerance {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        if deviation <= tolerance {
            format!(
                "all {} peer anchor(s) are within tolerance {tolerance}",
                values.len()
            )
        } else {
            format!("maximum anchor deviation {deviation} exceeds tolerance {tolerance}")
        },
        analysis.evidence_ids,
        node_ids.to_vec(),
        measurements,
        context,
    )
}

fn alignment_coordinate(rect: ResolvedRect<'_>, axis: Axis, anchor: AlignmentAnchor) -> f64 {
    let value = match (axis, anchor) {
        (Axis::Horizontal, AlignmentAnchor::Center) => rect.rect.x + rect.rect.width / 2.0,
        (Axis::Vertical, AlignmentAnchor::Center) => rect.rect.y + rect.rect.height / 2.0,
        (Axis::Horizontal, AlignmentAnchor::Start) => match rect.canvas.horizontal_direction {
            HorizontalDirection::Right => rect.rect.x,
            HorizontalDirection::Left => right(rect.rect),
        },
        (Axis::Horizontal, AlignmentAnchor::End) => match rect.canvas.horizontal_direction {
            HorizontalDirection::Right => right(rect.rect),
            HorizontalDirection::Left => rect.rect.x,
        },
        (Axis::Vertical, AlignmentAnchor::Start) => match rect.canvas.vertical_direction {
            VerticalDirection::Down => rect.rect.y,
            VerticalDirection::Up => bottom(rect.rect),
        },
        (Axis::Vertical, AlignmentAnchor::End) => match rect.canvas.vertical_direction {
            VerticalDirection::Down => bottom(rect.rect),
            VerticalDirection::Up => rect.rect.y,
        },
    };
    canonical_zero(value)
}

fn evaluate_peer_extent(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
) -> Vec<RuleResult> {
    let mut results = extension
        .contracts
        .iter()
        .filter_map(|(contract_id, contract)| match contract {
            VisualContract::PeerExtent {
                node_ids,
                dimension,
                box_kind,
                tolerance,
                evidence_id,
            } => Some(peer_extent_result(
                context,
                contract_id,
                node_ids,
                *dimension,
                *box_kind,
                *tolerance,
                evidence_id,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        results.push(inapplicable_result(
            &PEER_EXTENT_DEFINITION,
            context,
            "the visual extension declares no peer-extent contract",
        ));
    }
    results
}

#[allow(clippy::too_many_arguments)]
fn peer_extent_result(
    context: &QueryContext<'_>,
    contract_id: &Identifier,
    node_ids: &[Identifier],
    dimension: ExtentDimension,
    box_kind: BoxKind,
    tolerance: f64,
    contract_evidence_id: &Identifier,
) -> RuleResult {
    let analysis = resolve_rect_set(context, node_ids, box_kind, contract_evidence_id);
    let target = Target {
        kind: TargetKind::Relation,
        id: contract_id.clone(),
        aspect: Some(format!(
            "{}:{}",
            extent_dimension_label(dimension),
            box_kind.as_str()
        )),
    };

    if let Some(reason) = &analysis.reason {
        return build_result(
            &PEER_EXTENT_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!("peer extent cannot be compared: {reason}"),
            analysis.evidence_ids,
            node_ids.to_vec(),
            BTreeMap::new(),
            context,
        );
    }
    let rects = &analysis.rects;

    let values = rects
        .iter()
        .map(|rect| match dimension {
            ExtentDimension::Width => rect.rect.width,
            ExtentDimension::Height => rect.rect.height,
        })
        .collect::<Vec<_>>();
    let baseline = median(&values);
    let deviation = maximum_deviation(&values, baseline);
    let unit = rects[0].unit;
    let measurements = distribution_measurements(&values, baseline, deviation, tolerance, unit);

    build_result(
        &PEER_EXTENT_DEFINITION,
        target,
        if deviation <= tolerance {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        if deviation <= tolerance {
            format!(
                "all {} peer extent(s) are within tolerance {tolerance}",
                values.len()
            )
        } else {
            format!("maximum extent deviation {deviation} exceeds tolerance {tolerance}")
        },
        analysis.evidence_ids,
        node_ids.to_vec(),
        measurements,
        context,
    )
}

fn evaluate_peer_font_size(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
) -> Vec<RuleResult> {
    let mut results = extension
        .contracts
        .iter()
        .filter_map(|(contract_id, contract)| match contract {
            VisualContract::PeerFontSize {
                node_ids,
                tolerance,
                evidence_id,
            } => Some(peer_font_size_result(
                context,
                extension,
                contract_id,
                node_ids,
                *tolerance,
                evidence_id,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        results.push(inapplicable_result(
            &PEER_FONT_SIZE_DEFINITION,
            context,
            "the visual extension declares no peer-font-size contract",
        ));
    }
    results
}

fn peer_font_size_result(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
    contract_id: &Identifier,
    node_ids: &[Identifier],
    tolerance: f64,
    contract_evidence_id: &Identifier,
) -> RuleResult {
    let analysis = resolve_font_set(extension, node_ids, contract_evidence_id);
    let target = Target {
        kind: TargetKind::Relation,
        id: contract_id.clone(),
        aspect: Some("fontSize".to_owned()),
    };

    if let Some(reason) = &analysis.reason {
        return build_result(
            &PEER_FONT_SIZE_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!("peer font size cannot be compared: {reason}"),
            analysis.evidence_ids,
            node_ids.to_vec(),
            BTreeMap::new(),
            context,
        );
    }
    let values = &analysis.values;

    let numbers = values.iter().map(|value| value.value).collect::<Vec<_>>();
    let baseline = median(&numbers);
    let deviation = maximum_deviation(&numbers, baseline);
    let unit = values[0].unit;
    let measurements = distribution_measurements(&numbers, baseline, deviation, tolerance, unit);

    build_result(
        &PEER_FONT_SIZE_DEFINITION,
        target,
        if deviation <= tolerance {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        if deviation <= tolerance {
            format!(
                "all {} peer font-size observation(s) are within tolerance {tolerance}",
                numbers.len()
            )
        } else {
            format!("maximum font-size deviation {deviation} exceeds tolerance {tolerance}")
        },
        analysis.evidence_ids,
        node_ids.to_vec(),
        measurements,
        context,
    )
}

fn evaluate_minimum_font_size(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
) -> Vec<RuleResult> {
    let mut results = Vec::new();
    for (contract_id, contract) in &extension.contracts {
        let VisualContract::MinimumFontSize {
            node_ids,
            minimum,
            evidence_id,
        } = contract
        else {
            continue;
        };

        for node_id in node_ids {
            results.push(minimum_font_size_result(
                context,
                extension,
                contract_id,
                node_id,
                *minimum,
                evidence_id,
            ));
        }
    }

    if results.is_empty() {
        results.push(inapplicable_result(
            &MINIMUM_FONT_SIZE_DEFINITION,
            context,
            "the visual extension declares no minimum-font-size contract",
        ));
    }
    results
}

fn minimum_font_size_result(
    context: &QueryContext<'_>,
    extension: &VisualExtension,
    contract_id: &Identifier,
    node_id: &Identifier,
    minimum: Length,
    contract_evidence_id: &Identifier,
) -> RuleResult {
    let target = Target {
        kind: TargetKind::Node,
        id: node_id.clone(),
        aspect: Some(format!("fontSize:contract:{contract_id}")),
    };
    let Some(observed) = extension
        .node_styles
        .get(node_id)
        .and_then(|style| style.font_size.as_ref())
    else {
        return build_result(
            &MINIMUM_FONT_SIZE_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!("node {node_id} has no observed font size"),
            vec![contract_evidence_id.clone()],
            vec![node_id.clone()],
            BTreeMap::new(),
            context,
        );
    };

    let mut evidence_ids = vec![contract_evidence_id.clone(), observed.evidence_id.clone()];
    if observed.value.unit != minimum.unit {
        evidence_ids.sort();
        evidence_ids.dedup();
        return build_result(
            &MINIMUM_FONT_SIZE_DEFINITION,
            target,
            RuleOutcome::CantTell,
            format!(
                "observed font-size unit {} cannot be compared with minimum unit {}",
                unit_label(observed.value.unit),
                unit_label(minimum.unit)
            ),
            evidence_ids,
            vec![node_id.clone()],
            BTreeMap::new(),
            context,
        );
    }

    let passed = observed.value.value >= minimum.value;
    let mut measurements = BTreeMap::new();
    insert_measurement(
        &mut measurements,
        "observedFontSize",
        observed.value.value,
        observed.value.unit,
    );
    insert_measurement(
        &mut measurements,
        "minimumFontSize",
        minimum.value,
        minimum.unit,
    );

    build_result(
        &MINIMUM_FONT_SIZE_DEFINITION,
        target,
        if passed {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        if passed {
            format!(
                "observed font size {} meets explicit minimum {}",
                observed.value.value, minimum.value
            )
        } else {
            format!(
                "observed font size {} is below explicit minimum {}",
                observed.value.value, minimum.value
            )
        },
        evidence_ids,
        vec![node_id.clone()],
        measurements,
        context,
    )
}

#[derive(Debug, Default)]
struct RectSetAnalysis<'a> {
    rects: Vec<ResolvedRect<'a>>,
    evidence_ids: Vec<Identifier>,
    reason: Option<String>,
}

fn resolve_rect_set<'a>(
    context: &QueryContext<'a>,
    node_ids: &[Identifier],
    box_kind: BoxKind,
    contract_evidence_id: &Identifier,
) -> RectSetAnalysis<'a> {
    let mut analysis = RectSetAnalysis {
        evidence_ids: vec![contract_evidence_id.clone()],
        ..RectSetAnalysis::default()
    };

    for node_id in node_ids {
        let rect = match context.rect(node_id, box_kind) {
            Ok(Some(rect)) => rect,
            Ok(None) => {
                analysis.reason = Some(format!(
                    "node {node_id} has no {} observation",
                    box_kind.as_str()
                ));
                return analysis;
            }
            Err(error) => {
                analysis.reason = Some(error.to_string());
                return analysis;
            }
        };
        if let Some(first) = analysis.rects.first().copied() {
            if let Err(error) = ensure_comparable(first, rect) {
                analysis.reason = Some(error.to_string());
                analysis.evidence_ids.push(rect.evidence_id.clone());
                return analysis;
            }
        }
        analysis.evidence_ids.push(rect.evidence_id.clone());
        analysis.rects.push(rect);
    }
    analysis
}

#[derive(Debug, Default)]
struct FontSetAnalysis {
    values: Vec<Length>,
    evidence_ids: Vec<Identifier>,
    reason: Option<String>,
}

fn resolve_font_set(
    extension: &VisualExtension,
    node_ids: &[Identifier],
    contract_evidence_id: &Identifier,
) -> FontSetAnalysis {
    let mut analysis = FontSetAnalysis {
        evidence_ids: vec![contract_evidence_id.clone()],
        ..FontSetAnalysis::default()
    };

    for node_id in node_ids {
        let Some(observed) = extension
            .node_styles
            .get(node_id)
            .and_then(|style| style.font_size.as_ref())
        else {
            analysis.reason = Some(format!("node {node_id} has no observed font size"));
            return analysis;
        };
        if let Some(first) = analysis.values.first() {
            if first.unit != observed.value.unit {
                analysis.reason = Some(format!(
                    "font-size units {} and {} are not directly comparable",
                    unit_label(first.unit),
                    unit_label(observed.value.unit)
                ));
                analysis.evidence_ids.push(observed.evidence_id.clone());
                return analysis;
            }
        }
        analysis.evidence_ids.push(observed.evidence_id.clone());
        analysis.values.push(observed.value);
    }
    analysis
}

fn inapplicable_result(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    message: &str,
) -> RuleResult {
    build_result(
        definition,
        Target {
            kind: TargetKind::Artifact,
            id: context.document().artifact.id.clone(),
            aspect: None,
        },
        RuleOutcome::Inapplicable,
        message.to_owned(),
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
        context,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_result(
    definition: &RuleDefinition,
    target: Target,
    outcome: RuleOutcome,
    message: String,
    evidence_ids: Vec<Identifier>,
    related_node_ids: Vec<Identifier>,
    measurements: BTreeMap<String, Measurement>,
    context: &QueryContext<'_>,
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
        target,
        outcome,
        message,
        evidence_ids,
        evidence_classes,
        related_node_ids,
        measurements,
    }
}

fn distribution_measurements(
    values: &[f64],
    baseline: f64,
    maximum_deviation: f64,
    tolerance: f64,
    unit: Unit,
) -> BTreeMap<String, Measurement> {
    let mut measurements = BTreeMap::new();
    insert_measurement(&mut measurements, "baseline", baseline, unit);
    insert_measurement(
        &mut measurements,
        "maximumDeviation",
        maximum_deviation,
        unit,
    );
    insert_measurement(&mut measurements, "tolerance", tolerance, unit);
    if let Some(minimum) = values.iter().copied().reduce(f64::min) {
        insert_measurement(&mut measurements, "minimum", minimum, unit);
    }
    if let Some(maximum) = values.iter().copied().reduce(f64::max) {
        insert_measurement(&mut measurements, "maximum", maximum, unit);
    }
    measurements
}

fn insert_measurement(
    measurements: &mut BTreeMap<String, Measurement>,
    name: &str,
    value: f64,
    unit: Unit,
) {
    measurements.insert(name.to_owned(), Measurement { value, unit });
}

fn median(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        values[middle - 1] / 2.0 + values[middle] / 2.0
    } else {
        values[middle]
    }
}

fn maximum_deviation(values: &[f64], baseline: f64) -> f64 {
    values
        .iter()
        .map(|value| (value - baseline).abs())
        .fold(0.0_f64, f64::max)
}

const fn axis_label(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}

const fn alignment_anchor_label(anchor: AlignmentAnchor) -> &'static str {
    match anchor {
        AlignmentAnchor::Start => "start",
        AlignmentAnchor::Center => "center",
        AlignmentAnchor::End => "end",
    }
}

const fn extent_dimension_label(dimension: ExtentDimension) -> &'static str {
    match dimension {
        ExtentDimension::Width => "width",
        ExtentDimension::Height => "height",
    }
}

const fn unit_label(unit: Unit) -> &'static str {
    match unit {
        Unit::CssPixel => "css-px",
        Unit::DevicePixel => "device-px",
        Unit::Dp => "dp",
        Unit::Point => "pt",
        Unit::Emu => "emu",
        Unit::PdfPoint => "pdf-pt",
        Unit::Normalized => "normalized",
    }
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

#[cfg(test)]
mod tests {
    use sightlint_ir::{
        Canvas, HorizontalDirection, Identifier, Rect, Size, Unit, VerticalDirection,
    };

    use super::{alignment_coordinate, rect_contains};
    use crate::ResolvedRect;

    fn canvas(horizontal_direction: HorizontalDirection) -> Canvas {
        Canvas {
            id: Identifier::from("canvas"),
            size: Size {
                width: 200.0,
                height: 200.0,
            },
            unit: Unit::CssPixel,
            horizontal_direction,
            vertical_direction: VerticalDirection::Down,
            evidence_id: Identifier::from("evidence"),
        }
    }

    #[test]
    fn containment_accepts_shared_boundary() {
        let canvas = canvas(HorizontalDirection::Right);
        let evidence = Identifier::from("evidence");
        let coordinate_space = Identifier::from("canvas");
        let parent = ResolvedRect {
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 100.0,
            },
            coordinate_space_id: &coordinate_space,
            unit: Unit::CssPixel,
            evidence_id: &evidence,
            canvas: &canvas,
        };
        let child = ResolvedRect {
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 100.0,
            },
            ..parent
        };
        assert!(rect_contains(parent, child, 0.0));
    }

    #[test]
    fn logical_start_honors_right_to_left_direction() {
        let canvas = canvas(HorizontalDirection::Left);
        let evidence = Identifier::from("evidence");
        let coordinate_space = Identifier::from("canvas");
        let rect = ResolvedRect {
            rect: Rect {
                x: 20.0,
                y: 10.0,
                width: 80.0,
                height: 20.0,
            },
            coordinate_space_id: &coordinate_space,
            unit: Unit::CssPixel,
            evidence_id: &evidence,
            canvas: &canvas,
        };
        assert_eq!(
            alignment_coordinate(
                rect,
                sightlint_ir::Axis::Horizontal,
                sightlint_ir::AlignmentAnchor::Start
            ),
            100.0
        );
    }
}
