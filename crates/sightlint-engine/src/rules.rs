//! Atomic deterministic rules for the M1 vertical slice.

use std::collections::BTreeMap;

use sightlint_ir::{Axis, BoxKind, Identifier, Relation, Unit};

use crate::geometry::{
    QueryContext, ResolvedRect, ensure_comparable, ordered_gap, overlap_extents, within_canvas,
};
use crate::report::{
    Measurement, RuleKind, RuleMaturity, RuleOutcome, RuleResult, Target, TargetKind,
};

/// Input category required by a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAspect {
    /// Canvas metadata and dimensions.
    CanvasGeometry,
    /// One or more exact or observed node rectangles.
    NodeGeometry,
    /// Explicit relations supplied by an adapter or project contract.
    DeclaredRelations,
    /// Evidence provenance records.
    Evidence,
}

/// Static, inspectable definition of an executable rule.
#[derive(Debug)]
pub struct RuleDefinition {
    /// Stable rule identifier.
    pub id: &'static str,
    /// Semantic rule version.
    pub version: &'static str,
    /// Human-readable title.
    pub title: &'static str,
    /// Required input categories.
    pub input_aspects: &'static [InputAspect],
    /// Current validation maturity.
    pub maturity: RuleMaturity,
}

/// One narrow deterministic rule.
pub trait AtomicRule {
    /// Returns static rule metadata.
    fn definition(&self) -> &'static RuleDefinition;

    /// Evaluates every applicable target in stable order.
    fn evaluate(&self, context: &QueryContext<'_>) -> Vec<RuleResult>;
}

/// Runs the built-in M1 rule pack.
pub fn run_default_rules(context: &QueryContext<'_>) -> Vec<RuleResult> {
    let rules: [&dyn AtomicRule; 3] = [
        &BoundsWithinCanvasRule,
        &DeclaredNonOverlapRule,
        &PeerSpacingConsistencyRule,
    ];
    let mut results = Vec::new();
    for rule in rules {
        results.extend(rule.evaluate(context));
    }
    results
}

struct BoundsWithinCanvasRule;

static BOUNDS_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.bounds.within-canvas",
    version: "0.1.0",
    title: "Observed bounds stay within their canvas",
    input_aspects: &[
        InputAspect::CanvasGeometry,
        InputAspect::NodeGeometry,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

impl AtomicRule for BoundsWithinCanvasRule {
    fn definition(&self) -> &'static RuleDefinition {
        &BOUNDS_DEFINITION
    }

    fn evaluate(&self, context: &QueryContext<'_>) -> Vec<RuleResult> {
        let mut results = Vec::new();
        let mut nodes = context.document().nodes.iter().collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));

        for node in nodes {
            for box_kind in [BoxKind::Layout, BoxKind::Render, BoxKind::Ink, BoxKind::Hit] {
                if let Ok(Some(observed)) = context.rect(&node.id, box_kind) {
                    results.push(evaluate_bound(
                        self.definition(),
                        context,
                        &node.id,
                        box_kind,
                        observed,
                    ));
                }
            }
        }

        if results.is_empty() {
            results.push(inapplicable_result(
                self.definition(),
                context,
                "the artifact contains no observed node bounds",
            ));
        }

        results
    }
}

fn evaluate_bound(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    node_id: &Identifier,
    box_kind: BoxKind,
    observed: ResolvedRect<'_>,
) -> RuleResult {
    let passed = within_canvas(observed.rect, observed.canvas, 0.0);
    let message = if passed {
        format!(
            "{} is fully contained by canvas {}",
            box_kind.as_str(),
            observed.canvas.id
        )
    } else {
        format!(
            "{} extends outside canvas {}",
            box_kind.as_str(),
            observed.canvas.id
        )
    };
    let mut measurements = BTreeMap::new();
    for (name, value) in [
        ("x", observed.rect.x),
        ("y", observed.rect.y),
        ("width", observed.rect.width),
        ("height", observed.rect.height),
        ("canvasWidth", observed.canvas.size.width),
        ("canvasHeight", observed.canvas.size.height),
    ] {
        insert_measurement(&mut measurements, name, value, observed.unit);
    }

    build_result(
        definition,
        Target {
            kind: TargetKind::Node,
            id: node_id.clone(),
            aspect: Some(box_kind.as_str().to_owned()),
        },
        if passed {
            RuleOutcome::Passed
        } else {
            RuleOutcome::Failed
        },
        message,
        vec![
            observed.evidence_id.clone(),
            observed.canvas.evidence_id.clone(),
        ],
        vec![node_id.clone()],
        measurements,
        context,
    )
}

struct DeclaredNonOverlapRule;

static NON_OVERLAP_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.geometry.declared-non-overlap",
    version: "0.1.0",
    title: "Declared peers do not overlap",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::DeclaredRelations,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

impl AtomicRule for DeclaredNonOverlapRule {
    fn definition(&self) -> &'static RuleDefinition {
        &NON_OVERLAP_DEFINITION
    }

    fn evaluate(&self, context: &QueryContext<'_>) -> Vec<RuleResult> {
        let mut relations = context
            .document()
            .relations
            .iter()
            .filter(|relation| matches!(relation, Relation::NonOverlapping { .. }))
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.id().cmp(right.id()));

        let mut results = relations
            .into_iter()
            .map(|relation| evaluate_non_overlap_relation(self.definition(), context, relation))
            .collect::<Vec<_>>();

        if results.is_empty() {
            results.push(inapplicable_result(
                self.definition(),
                context,
                "the artifact declares no non-overlap relation",
            ));
        }

        results
    }
}

#[derive(Debug, Default)]
struct NonOverlapAnalysis {
    evidence_ids: Vec<Identifier>,
    ambiguous: Vec<String>,
    overlaps: Vec<String>,
    maximum_horizontal: f64,
    maximum_vertical: f64,
    unit: Option<Unit>,
}

fn evaluate_non_overlap_relation(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    relation: &Relation,
) -> RuleResult {
    let Relation::NonOverlapping {
        id,
        node_ids,
        box_kind,
        tolerance,
        evidence_id,
    } = relation
    else {
        unreachable!("relation was filtered by variant")
    };

    let analysis = analyze_non_overlap(context, node_ids, *box_kind, *tolerance, evidence_id);
    let (outcome, message) = non_overlap_outcome(&analysis, node_ids.len(), *tolerance);
    let measurements = non_overlap_measurements(&analysis, *tolerance);

    build_result(
        definition,
        Target {
            kind: TargetKind::Relation,
            id: id.clone(),
            aspect: Some(box_kind.as_str().to_owned()),
        },
        outcome,
        message,
        analysis.evidence_ids,
        node_ids.clone(),
        measurements,
        context,
    )
}

fn analyze_non_overlap(
    context: &QueryContext<'_>,
    node_ids: &[Identifier],
    box_kind: BoxKind,
    tolerance: f64,
    relation_evidence_id: &Identifier,
) -> NonOverlapAnalysis {
    let mut analysis = NonOverlapAnalysis {
        evidence_ids: vec![relation_evidence_id.clone()],
        ..NonOverlapAnalysis::default()
    };

    for first_index in 0..node_ids.len() {
        for second_index in (first_index + 1)..node_ids.len() {
            analyze_non_overlap_pair(
                context,
                &node_ids[first_index],
                &node_ids[second_index],
                box_kind,
                tolerance,
                &mut analysis,
            );
        }
    }

    analysis
}

fn analyze_non_overlap_pair(
    context: &QueryContext<'_>,
    first_id: &Identifier,
    second_id: &Identifier,
    box_kind: BoxKind,
    tolerance: f64,
    analysis: &mut NonOverlapAnalysis,
) {
    let pair = format!("{first_id} and {second_id}");
    let first = context.rect(first_id, box_kind);
    let second = context.rect(second_id, box_kind);

    match (first, second) {
        (Ok(Some(first)), Ok(Some(second))) => {
            analysis.evidence_ids.push(first.evidence_id.clone());
            analysis.evidence_ids.push(second.evidence_id.clone());
            if let Err(error) = ensure_comparable(first, second) {
                analysis.ambiguous.push(format!("{pair}: {error}"));
                return;
            }
            analysis.unit.get_or_insert(first.unit);
            let (horizontal, vertical) = overlap_extents(first.rect, second.rect);
            analysis.maximum_horizontal = analysis.maximum_horizontal.max(horizontal);
            analysis.maximum_vertical = analysis.maximum_vertical.max(vertical);
            if horizontal > tolerance && vertical > tolerance {
                analysis.overlaps.push(format!(
                    "{pair} overlap by {horizontal} × {vertical} {}",
                    unit_label(first.unit)
                ));
            }
        }
        (Ok(None), _) | (_, Ok(None)) => analysis.ambiguous.push(format!(
            "{pair}: {} is missing for at least one node",
            box_kind.as_str()
        )),
        (Err(error), _) | (_, Err(error)) => {
            analysis.ambiguous.push(format!("{pair}: {error}"));
        }
    }
}

fn non_overlap_outcome(
    analysis: &NonOverlapAnalysis,
    member_count: usize,
    tolerance: f64,
) -> (RuleOutcome, String) {
    if let Some(first) = analysis.overlaps.first() {
        return (
            RuleOutcome::Failed,
            format!(
                "{} declared pair(s) overlap; first violation: {first}",
                analysis.overlaps.len()
            ),
        );
    }
    if let Some(first) = analysis.ambiguous.first() {
        return (
            RuleOutcome::CantTell,
            format!(
                "non-overlap could not be established for {} pair(s); first reason: {first}",
                analysis.ambiguous.len()
            ),
        );
    }
    (
        RuleOutcome::Passed,
        format!(
            "all {} declared node pair(s) stay within the {tolerance} tolerance",
            pair_count(member_count)
        ),
    )
}

fn non_overlap_measurements(
    analysis: &NonOverlapAnalysis,
    tolerance: f64,
) -> BTreeMap<String, Measurement> {
    let mut measurements = BTreeMap::new();
    if let Some(unit) = analysis.unit {
        insert_measurement(
            &mut measurements,
            "maximumHorizontalOverlap",
            analysis.maximum_horizontal,
            unit,
        );
        insert_measurement(
            &mut measurements,
            "maximumVerticalOverlap",
            analysis.maximum_vertical,
            unit,
        );
        insert_measurement(&mut measurements, "tolerance", tolerance, unit);
    }
    measurements
}

struct PeerSpacingConsistencyRule;

static PEER_SPACING_DEFINITION: RuleDefinition = RuleDefinition {
    id: "visual.spacing.peer-consistency",
    version: "0.1.0",
    title: "Declared peer gaps are consistent",
    input_aspects: &[
        InputAspect::NodeGeometry,
        InputAspect::DeclaredRelations,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Experimental,
};

impl AtomicRule for PeerSpacingConsistencyRule {
    fn definition(&self) -> &'static RuleDefinition {
        &PEER_SPACING_DEFINITION
    }

    fn evaluate(&self, context: &QueryContext<'_>) -> Vec<RuleResult> {
        let mut relations = context
            .document()
            .relations
            .iter()
            .filter(|relation| matches!(relation, Relation::PeerSequence { .. }))
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.id().cmp(right.id()));

        let mut results = relations
            .into_iter()
            .map(|relation| evaluate_peer_sequence(self.definition(), context, relation))
            .collect::<Vec<_>>();

        if results.is_empty() {
            results.push(inapplicable_result(
                self.definition(),
                context,
                "the artifact declares no peer sequence",
            ));
        }

        results
    }
}

#[derive(Debug, Default)]
struct PeerSpacingAnalysis<'a> {
    evidence_ids: Vec<Identifier>,
    rects: Vec<ResolvedRect<'a>>,
    gaps: Vec<f64>,
    reason: Option<String>,
}

fn evaluate_peer_sequence(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    relation: &Relation,
) -> RuleResult {
    let Relation::PeerSequence {
        id,
        node_ids,
        axis,
        box_kind,
        expected_gap,
        tolerance,
        evidence_id,
    } = relation
    else {
        unreachable!("relation was filtered by variant")
    };

    let analysis = analyze_peer_spacing(context, node_ids, *axis, *box_kind, evidence_id);
    let (outcome, message, baseline) =
        peer_spacing_outcome(&analysis, *expected_gap, *tolerance);
    let measurements = peer_spacing_measurements(&analysis, baseline, *tolerance);

    build_result(
        definition,
        Target {
            kind: TargetKind::Relation,
            id: id.clone(),
            aspect: Some(format!("{}:{}", axis_label(*axis), box_kind.as_str())),
        },
        outcome,
        message,
        analysis.evidence_ids,
        node_ids.clone(),
        measurements,
        context,
    )
}

fn analyze_peer_spacing<'a>(
    context: &QueryContext<'a>,
    node_ids: &[Identifier],
    axis: Axis,
    box_kind: BoxKind,
    relation_evidence_id: &Identifier,
) -> PeerSpacingAnalysis<'a> {
    let mut analysis = PeerSpacingAnalysis {
        evidence_ids: vec![relation_evidence_id.clone()],
        ..PeerSpacingAnalysis::default()
    };

    for node_id in node_ids {
        match context.rect(node_id, box_kind) {
            Ok(Some(rect)) => {
                analysis.evidence_ids.push(rect.evidence_id.clone());
                analysis.rects.push(rect);
            }
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
        }
    }

    for pair in analysis.rects.windows(2) {
        match ordered_gap(pair[0], pair[1], axis) {
            Ok(gap) => analysis.gaps.push(gap),
            Err(error) => {
                analysis.reason = Some(error.to_string());
                break;
            }
        }
    }

    analysis
}

fn peer_spacing_outcome(
    analysis: &PeerSpacingAnalysis<'_>,
    expected_gap: Option<f64>,
    tolerance: f64,
) -> (RuleOutcome, String, Option<f64>) {
    if let Some(reason) = &analysis.reason {
        return (
            RuleOutcome::CantTell,
            format!("peer spacing cannot be compared: {reason}"),
            None,
        );
    }
    if expected_gap.is_none() && analysis.gaps.len() < 2 {
        return (
            RuleOutcome::CantTell,
            "at least three peers or an explicit expectedGap are required".to_owned(),
            None,
        );
    }

    let baseline = expected_gap.unwrap_or_else(|| median(&analysis.gaps));
    let maximum_deviation = maximum_deviation(&analysis.gaps, baseline);
    if maximum_deviation > tolerance {
        (
            RuleOutcome::Failed,
            format!(
                "maximum gap deviation {maximum_deviation} exceeds tolerance {tolerance}"
            ),
            Some(baseline),
        )
    } else {
        (
            RuleOutcome::Passed,
            format!(
                "all {} adjacent gap(s) are within tolerance {tolerance}",
                analysis.gaps.len()
            ),
            Some(baseline),
        )
    }
}

fn peer_spacing_measurements(
    analysis: &PeerSpacingAnalysis<'_>,
    baseline: Option<f64>,
    tolerance: f64,
) -> BTreeMap<String, Measurement> {
    let mut measurements = BTreeMap::new();
    let Some(unit) = analysis.rects.first().map(|rect| rect.unit) else {
        return measurements;
    };

    if let Some(baseline) = baseline {
        insert_measurement(&mut measurements, "baselineGap", baseline, unit);
        insert_measurement(
            &mut measurements,
            "maximumDeviation",
            maximum_deviation(&analysis.gaps, baseline),
            unit,
        );
    }
    if let Some(minimum) = analysis.gaps.iter().copied().reduce(f64::min) {
        insert_measurement(&mut measurements, "minimumGap", minimum, unit);
    }
    if let Some(maximum) = analysis.gaps.iter().copied().reduce(f64::max) {
        insert_measurement(&mut measurements, "maximumGap", maximum, unit);
    }
    insert_measurement(&mut measurements, "tolerance", tolerance, unit);
    measurements
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

const fn pair_count(member_count: usize) -> usize {
    member_count.saturating_mul(member_count.saturating_sub(1)) / 2
}

const fn axis_label(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
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

#[cfg(test)]
mod tests {
    use super::{median, pair_count};

    #[test]
    fn median_is_stable_for_even_and_odd_inputs() {
        let odd = median(&[9.0, 1.0, 5.0]);
        let even = median(&[9.0, 1.0, 5.0, 3.0]);
        assert!((odd - 5.0).abs() <= f64::EPSILON);
        assert!((even - 4.0).abs() <= f64::EPSILON);
    }

    #[test]
    fn pair_count_matches_complete_graph_edges() {
        assert_eq!(pair_count(0), 0);
        assert_eq!(pair_count(2), 1);
        assert_eq!(pair_count(4), 6);
    }
}
