//! Atomic deterministic rules for the M1 vertical slice.

use std::collections::BTreeMap;

use sightlint_ir::{Axis, BoxKind, Identifier, Relation, Unit};

use crate::geometry::{
    QueryContext, ensure_comparable, ordered_gap, overlap_extents, within_canvas,
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
                let Ok(Some(observed)) = context.rect(&node.id, box_kind) else {
                    continue;
                };
                let tolerance = 0.0;
                let passed = within_canvas(observed.rect, observed.canvas, tolerance);
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
                insert_measurement(&mut measurements, "x", observed.rect.x, observed.unit);
                insert_measurement(&mut measurements, "y", observed.rect.y, observed.unit);
                insert_measurement(
                    &mut measurements,
                    "width",
                    observed.rect.width,
                    observed.unit,
                );
                insert_measurement(
                    &mut measurements,
                    "height",
                    observed.rect.height,
                    observed.unit,
                );
                insert_measurement(
                    &mut measurements,
                    "canvasWidth",
                    observed.canvas.size.width,
                    observed.unit,
                );
                insert_measurement(
                    &mut measurements,
                    "canvasHeight",
                    observed.canvas.size.height,
                    observed.unit,
                );

                results.push(build_result(
                    self.definition(),
                    Target {
                        kind: TargetKind::Node,
                        id: node.id.clone(),
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
                    vec![node.id.clone()],
                    measurements,
                    context,
                ));
            }
        }

        if results.is_empty() {
            results.push(build_result(
                self.definition(),
                Target {
                    kind: TargetKind::Artifact,
                    id: context.document().artifact.id.clone(),
                    aspect: None,
                },
                RuleOutcome::Inapplicable,
                "the artifact contains no observed node bounds".to_owned(),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
                context,
            ));
        }

        results
    }
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
        let mut results = Vec::new();
        let mut relations = context
            .document()
            .relations
            .iter()
            .filter_map(|relation| match relation {
                Relation::NonOverlapping { .. } => Some(relation),
                Relation::PeerSequence { .. } => None,
            })
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.id().cmp(right.id()));

        for relation in relations {
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

            let mut evidence_ids = vec![evidence_id.clone()];
            let mut ambiguous = Vec::new();
            let mut overlaps = Vec::new();
            let mut maximum_horizontal: f64 = 0.0;
            let mut maximum_vertical: f64 = 0.0;
            let mut unit = None;

            for first_index in 0..node_ids.len() {
                for second_index in (first_index + 1)..node_ids.len() {
                    let first_id = &node_ids[first_index];
                    let second_id = &node_ids[second_index];
                    let pair = format!("{first_id} and {second_id}");
                    let first = context.rect(first_id, *box_kind);
                    let second = context.rect(second_id, *box_kind);
                    match (first, second) {
                        (Ok(Some(first)), Ok(Some(second))) => {
                            evidence_ids.push(first.evidence_id.clone());
                            evidence_ids.push(second.evidence_id.clone());
                            if let Err(error) = ensure_comparable(first, second) {
                                ambiguous.push(format!("{pair}: {error}"));
                                continue;
                            }
                            unit.get_or_insert(first.unit);
                            let (horizontal, vertical) = overlap_extents(first.rect, second.rect);
                            maximum_horizontal = maximum_horizontal.max(horizontal);
                            maximum_vertical = maximum_vertical.max(vertical);
                            if horizontal > *tolerance && vertical > *tolerance {
                                overlaps.push(format!(
                                    "{pair} overlap by {horizontal} × {vertical} {}",
                                    unit_label(first.unit)
                                ));
                            }
                        }
                        (Ok(None), _) | (_, Ok(None)) => ambiguous.push(format!(
                            "{pair}: {} is missing for at least one node",
                            box_kind.as_str()
                        )),
                        (Err(error), _) | (_, Err(error)) => {
                            ambiguous.push(format!("{pair}: {error}"));
                        }
                    }
                }
            }

            let (outcome, message) = if !overlaps.is_empty() {
                (
                    RuleOutcome::Failed,
                    format!(
                        "{} declared pair(s) overlap; first violation: {}",
                        overlaps.len(),
                        overlaps[0]
                    ),
                )
            } else if !ambiguous.is_empty() {
                (
                    RuleOutcome::CantTell,
                    format!(
                        "non-overlap could not be established for {} pair(s); first reason: {}",
                        ambiguous.len(),
                        ambiguous[0]
                    ),
                )
            } else {
                (
                    RuleOutcome::Passed,
                    format!(
                        "all {} declared node pair(s) stay within the {} tolerance",
                        pair_count(node_ids.len()),
                        tolerance
                    ),
                )
            };

            let mut measurements = BTreeMap::new();
            if let Some(unit) = unit {
                insert_measurement(
                    &mut measurements,
                    "maximumHorizontalOverlap",
                    maximum_horizontal,
                    unit,
                );
                insert_measurement(
                    &mut measurements,
                    "maximumVerticalOverlap",
                    maximum_vertical,
                    unit,
                );
                insert_measurement(&mut measurements, "tolerance", *tolerance, unit);
            }

            results.push(build_result(
                self.definition(),
                Target {
                    kind: TargetKind::Relation,
                    id: id.clone(),
                    aspect: Some(box_kind.as_str().to_owned()),
                },
                outcome,
                message,
                evidence_ids,
                node_ids.clone(),
                measurements,
                context,
            ));
        }

        if results.is_empty() {
            results.push(build_result(
                self.definition(),
                Target {
                    kind: TargetKind::Artifact,
                    id: context.document().artifact.id.clone(),
                    aspect: None,
                },
                RuleOutcome::Inapplicable,
                "the artifact declares no non-overlap relation".to_owned(),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
                context,
            ));
        }

        results
    }
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
        let mut results = Vec::new();
        let mut relations = context
            .document()
            .relations
            .iter()
            .filter_map(|relation| match relation {
                Relation::PeerSequence { .. } => Some(relation),
                Relation::NonOverlapping { .. } => None,
            })
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.id().cmp(right.id()));

        for relation in relations {
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

            let mut evidence_ids = vec![evidence_id.clone()];
            let mut resolved = Vec::with_capacity(node_ids.len());
            let mut reason = None;
            for node_id in node_ids {
                match context.rect(node_id, *box_kind) {
                    Ok(Some(rect)) => {
                        evidence_ids.push(rect.evidence_id.clone());
                        resolved.push(rect);
                    }
                    Ok(None) => {
                        reason = Some(format!(
                            "node {node_id} has no {} observation",
                            box_kind.as_str()
                        ));
                        break;
                    }
                    Err(error) => {
                        reason = Some(error.to_string());
                        break;
                    }
                }
            }

            let mut gaps = Vec::new();
            if reason.is_none() {
                for pair in resolved.windows(2) {
                    match ordered_gap(pair[0], pair[1], *axis) {
                        Ok(gap) => gaps.push(gap),
                        Err(error) => {
                            reason = Some(error.to_string());
                            break;
                        }
                    }
                }
            }

            let unit = resolved.first().map(|rect| rect.unit);
            let (outcome, message, baseline) = if let Some(reason) = reason {
                (
                    RuleOutcome::CantTell,
                    format!("peer spacing cannot be compared: {reason}"),
                    None,
                )
            } else if expected_gap.is_none() && gaps.len() < 2 {
                (
                    RuleOutcome::CantTell,
                    "at least three peers or an explicit expectedGap are required".to_owned(),
                    None,
                )
            } else {
                let baseline = expected_gap.unwrap_or_else(|| median(&gaps));
                let maximum_deviation = gaps
                    .iter()
                    .map(|gap| (gap - baseline).abs())
                    .fold(0.0_f64, f64::max);
                if maximum_deviation > *tolerance {
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
                            gaps.len()
                        ),
                        Some(baseline),
                    )
                }
            };

            let mut measurements = BTreeMap::new();
            if let Some(unit) = unit {
                if let Some(baseline) = baseline {
                    insert_measurement(&mut measurements, "baselineGap", baseline, unit);
                    let maximum_deviation = gaps
                        .iter()
                        .map(|gap| (gap - baseline).abs())
                        .fold(0.0_f64, f64::max);
                    insert_measurement(
                        &mut measurements,
                        "maximumDeviation",
                        maximum_deviation,
                        unit,
                    );
                }
                if let Some(minimum) = gaps.iter().copied().reduce(f64::min) {
                    insert_measurement(&mut measurements, "minimumGap", minimum, unit);
                }
                if let Some(maximum) = gaps.iter().copied().reduce(f64::max) {
                    insert_measurement(&mut measurements, "maximumGap", maximum, unit);
                }
                insert_measurement(&mut measurements, "tolerance", *tolerance, unit);
            }

            results.push(build_result(
                self.definition(),
                Target {
                    kind: TargetKind::Relation,
                    id: id.clone(),
                    aspect: Some(format!("{}:{}", axis_label(*axis), box_kind.as_str())),
                },
                outcome,
                message,
                evidence_ids,
                node_ids.clone(),
                measurements,
                context,
            ));
        }

        if results.is_empty() {
            results.push(build_result(
                self.definition(),
                Target {
                    kind: TargetKind::Artifact,
                    id: context.document().artifact.id.clone(),
                    aspect: None,
                },
                RuleOutcome::Inapplicable,
                "the artifact declares no peer sequence".to_owned(),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
                context,
            ));
        }

        results
    }
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
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
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
        assert_eq!(median(&[9.0, 1.0, 5.0]), 5.0);
        assert_eq!(median(&[9.0, 1.0, 5.0, 3.0]), 4.0);
    }

    #[test]
    fn pair_count_matches_complete_graph_edges() {
        assert_eq!(pair_count(0), 0);
        assert_eq!(pair_count(2), 1);
        assert_eq!(pair_count(4), 6);
    }
}
