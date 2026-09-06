//! Deterministic advisory rules over the official interaction extension.

use std::collections::BTreeMap;

use sightlint_ir::{
    EffectLatency, EffectResolution, Identifier, InteractionAction, InteractionExtension,
    InteractionTrace, RecoveryContract, RecoveryKind, TraceConsistency, TraceEvent,
    TraceEventDetail, TraceExecution, VisibleState,
};

use crate::geometry::QueryContext;
use crate::report::{
    PolicyProvenance, PolicySourceKind, RuleEnforcement, RuleKind, RuleMaturity, RuleOutcome,
    RuleResult, Target, TargetKind,
};
use crate::rules::{InputAspect, RuleDefinition, RulePolicyDefinition};

static ASYNC_FEEDBACK_DEFINITION: RuleDefinition = RuleDefinition {
    id: "interaction.async-feedback",
    version: "0.1.0",
    title: "Latent actions expose visible intermediate feedback",
    input_aspects: &[
        InputAspect::InteractionActionContract,
        InputAspect::InteractionTrace,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Advisory,
    policy: RulePolicyDefinition {
        profile: "sightlint:base",
        source_kind: PolicySourceKind::ConservativeBuiltIn,
        source_id: "sightlint:observable-latency-feedback",
        source_version: "0.1.0",
        reference: "docs/decisions/0047-deterministic-interaction-contracts-and-traces.md",
        enforcement: RuleEnforcement::Advisory,
    },
};

static FAILURE_RECOVERY_DEFINITION: RuleDefinition = RuleDefinition {
    id: "interaction.failure-recovery",
    version: "0.1.0",
    title: "Failed actions expose and complete an accepted recovery path",
    input_aspects: &[
        InputAspect::InteractionActionContract,
        InputAspect::InteractionTrace,
        InputAspect::Evidence,
    ],
    maturity: RuleMaturity::Advisory,
    policy: RulePolicyDefinition {
        profile: "sightlint:base",
        source_kind: PolicySourceKind::DeclaredContract,
        source_id: "org.sightlint.interaction:failure-recovery",
        source_version: "0.1.0",
        reference: "docs/decisions/0047-deterministic-interaction-contracts-and-traces.md",
        enforcement: RuleEnforcement::Advisory,
    },
};

pub(crate) fn run_interaction_rules(
    context: &QueryContext<'_>,
    extension: &InteractionExtension,
) -> Vec<RuleResult> {
    let traces = extension
        .traces
        .iter()
        .map(|trace| (&trace.action_id, trace))
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::with_capacity(extension.actions.len() * 2);
    for action in &extension.actions {
        let trace = traces
            .get(&action.id)
            .copied()
            .expect("validated interaction action has exactly one trace");
        results.push(evaluate_async_feedback(context, action, trace));
        results.push(evaluate_failure_recovery(context, action, trace));
    }
    results
}

fn evaluate_async_feedback(
    context: &QueryContext<'_>,
    action: &InteractionAction,
    trace: &InteractionTrace,
) -> RuleResult {
    let base_evidence = vec![action.contract_evidence_id.clone()];
    if action.effect_latency == EffectLatency::Immediate {
        return build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::Inapplicable,
            "the action contract declares an immediate effect, so latent feedback is not applicable",
            base_evidence,
        );
    }
    if let TraceExecution::Untested { reason } = &trace.execution {
        return build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::Untested,
            format!("the required controlled trace was not executed: {reason}"),
            base_evidence,
        );
    }
    if let TraceConsistency::Conflict { reason, .. } = &trace.consistency {
        return build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::CantTell,
            format!("conflicting trace evidence prevents a feedback verdict: {reason}"),
            trace_evidence(action, trace),
        );
    }

    let Some(activation) = trace
        .events
        .iter()
        .find(|event| matches!(event.detail, TraceEventDetail::ActionActivated))
    else {
        return build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::CantTell,
            "the captured trace has no action activation event",
            trace_evidence(action, trace),
        );
    };
    let resolution = trace.events.iter().find(|event| {
        event.attempt_id == activation.attempt_id
            && event.sequence > activation.sequence
            && matches!(event.detail, TraceEventDetail::EffectResolved { .. })
    });
    let Some(resolution) = resolution else {
        return build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::CantTell,
            "the captured primary attempt has no effect resolution event",
            trace_evidence(action, trace),
        );
    };
    let feedback = trace.events.iter().find(|event| {
        event.attempt_id == activation.attempt_id
            && event.sequence > activation.sequence
            && event.sequence < resolution.sequence
            && matches!(
                event.detail,
                TraceEventDetail::StateObserved {
                    state: VisibleState::Pending | VisibleState::Optimistic
                }
            )
    });
    match feedback {
        Some(_) => build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::Passed,
            "the controlled trace observes pending or optimistic feedback before effect resolution",
            trace_evidence(action, trace),
        ),
        None => build_result(
            &ASYNC_FEEDBACK_DEFINITION,
            context,
            action,
            RuleOutcome::Failed,
            "the effect resolved without observed pending or optimistic feedback between activation and resolution",
            trace_evidence(action, trace),
        ),
    }
}

fn evaluate_failure_recovery(
    context: &QueryContext<'_>,
    action: &InteractionAction,
    trace: &InteractionTrace,
) -> RuleResult {
    let RecoveryContract::Required {
        accepted_alternatives,
    } = &action.recovery
    else {
        return build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::Inapplicable,
            "the action contract declares no failure-recovery obligation",
            vec![action.contract_evidence_id.clone()],
        );
    };
    if let TraceExecution::Untested { reason } = &trace.execution {
        return build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::Untested,
            format!("the required controlled failure trace was not executed: {reason}"),
            vec![action.contract_evidence_id.clone()],
        );
    }
    if let TraceConsistency::Conflict { reason, .. } = &trace.consistency {
        return build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::CantTell,
            format!("conflicting trace evidence prevents a recovery verdict: {reason}"),
            trace_evidence(action, trace),
        );
    }
    let Some(failure) = trace.events.iter().find(|event| {
        matches!(
            event.detail,
            TraceEventDetail::EffectResolved {
                resolution: EffectResolution::Failure
            }
        )
    }) else {
        return build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::Inapplicable,
            "the captured trace did not exercise a failure path",
            trace_evidence(action, trace),
        );
    };

    let valid = accepted_alternatives
        .iter()
        .copied()
        .any(|recovery| recovery_path_completes(&trace.events, failure, recovery));
    if valid {
        build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::Passed,
            "the failure becomes visible and an accepted recovery path reaches visible success",
            trace_evidence(action, trace),
        )
    } else {
        build_result(
            &FAILURE_RECOVERY_DEFINITION,
            context,
            action,
            RuleOutcome::Failed,
            "the completed failure trace does not expose and complete an accepted recovery path to visible success",
            trace_evidence(action, trace),
        )
    }
}

fn recovery_path_completes(
    events: &[TraceEvent],
    failure: &TraceEvent,
    recovery: RecoveryKind,
) -> bool {
    let failure_visible = events.iter().any(|event| {
        event.sequence > failure.sequence
            && matches!(
                event.detail,
                TraceEventDetail::StateObserved {
                    state: VisibleState::Failure
                }
            )
    });
    let offered = events.iter().find(|event| {
        event.sequence > failure.sequence
            && matches!(
                event.detail,
                TraceEventDetail::RecoveryOffered { recovery: observed }
                    if observed == recovery
            )
    });
    let activated = offered.and_then(|offered| {
        events.iter().find(|event| {
            event.sequence > offered.sequence
                && matches!(
                    event.detail,
                    TraceEventDetail::RecoveryActivated { recovery: observed }
                        if observed == recovery
                )
        })
    });
    let success = activated.and_then(|activated| {
        events.iter().find(|event| {
            event.attempt_id == activated.attempt_id
                && event.sequence > activated.sequence
                && matches!(
                    event.detail,
                    TraceEventDetail::EffectResolved {
                        resolution: EffectResolution::Success
                    }
                )
        })
    });
    let success_visible = success.is_some_and(|success| {
        events.iter().any(|event| {
            event.attempt_id == success.attempt_id
                && event.sequence > success.sequence
                && matches!(
                    event.detail,
                    TraceEventDetail::StateObserved {
                        state: VisibleState::Success
                    }
                )
        })
    });
    failure_visible && success_visible
}

fn trace_evidence(action: &InteractionAction, trace: &InteractionTrace) -> Vec<Identifier> {
    let mut evidence = vec![action.contract_evidence_id.clone()];
    evidence.extend(trace.consistency.evidence_ids().iter().cloned());
    evidence.extend(
        trace
            .events
            .iter()
            .flat_map(|event| event.evidence_ids.iter().cloned()),
    );
    evidence
}

fn build_result(
    definition: &RuleDefinition,
    context: &QueryContext<'_>,
    action: &InteractionAction,
    outcome: RuleOutcome,
    message: impl Into<String>,
    evidence_ids: Vec<Identifier>,
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
            kind: TargetKind::Artifact,
            id: context.document().artifact.id.clone(),
            aspect: Some(format!("interaction.action:{}", action.id)),
        },
        outcome,
        message: message.into(),
        evidence_ids,
        evidence_classes,
        related_node_ids: vec![action.target_node_id.clone()],
        measurements: BTreeMap::new(),
    }
}
