//! Official, independently versioned interaction-contract and deterministic-trace extension.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ArtifactIr, EvidenceClass, Identifier, serialize_canonical};

/// Artifact IR extension key for medium-neutral interaction contracts and traces.
pub const INTERACTION_EXTENSION_KEY: &str = "org.sightlint.interaction";

/// Current official interaction extension version.
pub const INTERACTION_EXTENSION_VERSION: &str = "0.1.0";

/// Whether an effect is expected to expose an intermediate visible state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectLatency {
    /// The declared effect completes without an observably latent interval.
    Immediate,
    /// The declared effect has an observably latent interval.
    Observable,
}

/// User-visible states admitted by the first interaction slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VisibleState {
    /// Work is visibly pending.
    Pending,
    /// An optimistic result is visible before final resolution.
    Optimistic,
    /// Successful completion is visible.
    Success,
    /// Failure is visible and distinguishable from success.
    Failure,
}

/// Declared effect resolution observed through controlled instrumentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectResolution {
    /// The effect completed successfully.
    Success,
    /// The effect completed with failure.
    Failure,
}

/// Recovery alternatives normalized by the first interaction slice.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryKind {
    /// Retry the failed effect.
    Retry,
    /// Preserve the user's work as a local or server-side draft.
    SaveDraft,
}

/// Declared recovery applicability and accepted alternatives for an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "applicability",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RecoveryContract {
    /// No failure-recovery obligation applies to this action.
    Inapplicable,
    /// A failure must expose and complete one of the accepted recovery paths.
    Required {
        /// Equivalent recovery alternatives accepted by the declared contract.
        accepted_alternatives: Vec<RecoveryKind>,
    },
}

impl RecoveryContract {
    fn canonicalize(&mut self) {
        if let Self::Required {
            accepted_alternatives,
        } = self
        {
            accepted_alternatives.sort();
        }
    }

    /// Returns accepted recovery alternatives, or an empty slice when inapplicable.
    pub fn accepted_alternatives(&self) -> &[RecoveryKind] {
        match self {
            Self::Inapplicable => &[],
            Self::Required {
                accepted_alternatives,
            } => accepted_alternatives,
        }
    }
}

/// One declared action and its externally supplied interaction policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionAction {
    /// Stable action identifier within this extension.
    pub id: Identifier,
    /// Core node activated by the action.
    pub target_node_id: Identifier,
    /// Evidence for action meaning, latency, and recovery applicability.
    pub contract_evidence_id: Identifier,
    /// Declared effect latency class; never inferred from wall-clock time.
    pub effect_latency: EffectLatency,
    /// Declared failure-recovery obligation and accepted alternatives.
    pub recovery: RecoveryContract,
}

/// Whether the controlled trace was executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TraceExecution {
    /// The trace was run under the declared controlled harness.
    Captured,
    /// No trace execution was performed.
    Untested {
        /// Stable reason the execution was unavailable or intentionally omitted.
        reason: String,
    },
}

/// Reconciliation status across native, accessibility, rendered, and declared observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TraceConsistency {
    /// The evidence required by the trace contract agrees.
    Agreement,
    /// Two or more retained observations disagree.
    Conflict {
        /// Evidence records that establish the conflict.
        evidence_ids: Vec<Identifier>,
        /// Stable, nonempty description of the disagreement.
        reason: String,
    },
}

impl TraceConsistency {
    fn canonicalize(&mut self) {
        if let Self::Conflict { evidence_ids, .. } = self {
            evidence_ids.sort();
        }
    }

    /// Returns evidence linked directly to a retained conflict.
    pub fn evidence_ids(&self) -> &[Identifier] {
        match self {
            Self::Agreement => &[],
            Self::Conflict { evidence_ids, .. } => evidence_ids,
        }
    }
}

/// Typed payload carried by one ordered trace event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TraceEventDetail {
    /// The primary or recovery action was activated through the controlled adapter.
    ActionActivated,
    /// A state was observed through native structure and related capture evidence.
    StateObserved {
        /// Normalized visible state.
        state: VisibleState,
    },
    /// Instrumentation declared that an attempt resolved.
    EffectResolved {
        /// Normalized success or failure resolution.
        resolution: EffectResolution,
    },
    /// A declared accepted recovery control was observed as available.
    RecoveryOffered {
        /// Recovery alternative exposed by the interface.
        recovery: RecoveryKind,
    },
    /// A recovery alternative was activated through the controlled adapter.
    RecoveryActivated {
        /// Recovery alternative selected by the trace.
        recovery: RecoveryKind,
    },
}

/// One canonically ordered event in a controlled interaction trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceEvent {
    /// Stable event identifier within the trace.
    pub id: Identifier,
    /// One-based canonical order assigned by the adapter, never a wall-clock timestamp.
    pub sequence: u64,
    /// Stable attempt identifier shared by causally related activation, state, and resolution events.
    pub attempt_id: Identifier,
    /// Optional earlier event that directly caused this event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause_event_id: Option<Identifier>,
    /// Evidence records supporting this event.
    pub evidence_ids: Vec<Identifier>,
    /// Typed event payload.
    #[serde(flatten)]
    pub detail: TraceEventDetail,
}

/// One action trace, including explicit untested and conflict states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionTrace {
    /// Stable trace identifier.
    pub id: Identifier,
    /// Action governed by this trace.
    pub action_id: Identifier,
    /// Whether the controlled execution ran.
    pub execution: TraceExecution,
    /// Cross-source agreement or retained conflict.
    pub consistency: TraceConsistency,
    /// Canonically ordered events; empty only for an untested trace.
    pub events: Vec<TraceEvent>,
}

/// Typed payload stored under [`INTERACTION_EXTENSION_KEY`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionExtension {
    /// Independent extension contract version.
    pub extension_version: String,
    /// Declared actions in stable identifier order after canonicalization.
    pub actions: Vec<InteractionAction>,
    /// One captured or explicitly untested trace per action.
    pub traces: Vec<InteractionTrace>,
}

impl InteractionExtension {
    /// Returns a clone with every set-like collection in canonical order.
    #[must_use]
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        for action in &mut canonical.actions {
            action.recovery.canonicalize();
        }
        canonical
            .actions
            .sort_by(|left, right| left.id.cmp(&right.id));
        for trace in &mut canonical.traces {
            trace.consistency.canonicalize();
            trace.events.sort_by(|left, right| {
                left.sequence
                    .cmp(&right.sequence)
                    .then_with(|| left.id.cmp(&right.id))
            });
            for event in &mut trace.events {
                event.evidence_ids.sort();
            }
        }
        canonical
            .traces
            .sort_by(|left, right| left.id.cmp(&right.id));
        canonical
    }

    /// Validates references, evidence authority, ordering, and trace invariants.
    ///
    /// # Errors
    ///
    /// Returns every deterministic issue in stable order.
    pub fn validate(&self, document: &ArtifactIr) -> Result<(), InteractionExtensionErrors> {
        let mut validator = InteractionValidator::new(document);
        if self.extension_version != INTERACTION_EXTENSION_VERSION {
            validator.issue(
                InteractionValidationCode::UnsupportedExtensionVersion,
                "/extensionVersion",
                format!(
                    "expected interaction extension version {INTERACTION_EXTENSION_VERSION}, found {}",
                    self.extension_version
                ),
            );
        }

        let mut actions = BTreeMap::new();
        for (index, action) in self.actions.iter().enumerate() {
            let path = format!("/actions/{index}");
            validator.nonempty_identifier(&action.id, &format!("{path}/id"));
            if actions.insert(action.id.clone(), action).is_some() {
                validator.issue(
                    InteractionValidationCode::DuplicateIdentifier,
                    format!("{path}/id"),
                    format!("action {} is duplicated", action.id),
                );
            }
            validator.require_node(&action.target_node_id, &format!("{path}/targetNodeId"));
            validator.require_evidence_class(
                &action.contract_evidence_id,
                EvidenceClass::DeclaredContract,
                &format!("{path}/contractEvidenceId"),
            );
            if let RecoveryContract::Required {
                accepted_alternatives,
            } = &action.recovery
            {
                if accepted_alternatives.is_empty() {
                    validator.issue(
                        InteractionValidationCode::InvalidValue,
                        format!("{path}/recovery/acceptedAlternatives"),
                        "required recovery must declare at least one accepted alternative",
                    );
                }
                validator.unique_recoveries(
                    accepted_alternatives,
                    &format!("{path}/recovery/acceptedAlternatives"),
                );
            }
        }
        if self.actions.is_empty() {
            validator.issue(
                InteractionValidationCode::InvalidValue,
                "/actions",
                "the interaction extension must contain at least one action",
            );
        }

        let mut trace_ids = BTreeSet::new();
        let mut traced_actions = BTreeSet::new();
        for (index, trace) in self.traces.iter().enumerate() {
            let path = format!("/traces/{index}");
            validator.nonempty_identifier(&trace.id, &format!("{path}/id"));
            if !trace_ids.insert(trace.id.clone()) {
                validator.issue(
                    InteractionValidationCode::DuplicateIdentifier,
                    format!("{path}/id"),
                    format!("trace {} is duplicated", trace.id),
                );
            }
            let action = actions.get(&trace.action_id).copied();
            if action.is_none() {
                validator.issue(
                    InteractionValidationCode::InvalidActionReference,
                    format!("{path}/actionId"),
                    format!("referenced action {} does not exist", trace.action_id),
                );
            } else if !traced_actions.insert(trace.action_id.clone()) {
                validator.issue(
                    InteractionValidationCode::DuplicateActionTrace,
                    format!("{path}/actionId"),
                    format!("action {} has more than one trace", trace.action_id),
                );
            }
            validate_trace(trace, action, &path, &mut validator);
        }
        for action_id in actions.keys() {
            if !traced_actions.contains(action_id) {
                validator.issue(
                    InteractionValidationCode::MissingActionTrace,
                    "/traces",
                    format!("action {action_id} has no captured or untested trace"),
                );
            }
        }
        validator.finish()
    }
}

/// Stable category for one official interaction extension problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InteractionValidationCode {
    /// The recognized extension uses an unsupported contract version.
    UnsupportedExtensionVersion,
    /// The recognized extension payload cannot be decoded.
    InvalidExtensionPayload,
    /// An identifier is empty or duplicated.
    DuplicateIdentifier,
    /// A required identifier is empty.
    EmptyIdentifier,
    /// An action references a core node that does not exist.
    InvalidNodeReference,
    /// A trace references an action that does not exist.
    InvalidActionReference,
    /// An event or conflict references evidence that does not exist.
    InvalidEvidenceReference,
    /// Evidence does not have the authority required by the field.
    InvalidEvidenceClass,
    /// An event references a cause that is not earlier in the same trace.
    InvalidCausalReference,
    /// Event sequence values are not contiguous and one-based.
    InvalidSequence,
    /// One action has more than one trace.
    DuplicateActionTrace,
    /// One action has no captured or untested trace.
    MissingActionTrace,
    /// A value violates an interaction semantic invariant.
    InvalidValue,
}

/// One deterministic official interaction extension validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionValidationIssue {
    /// Stable issue category.
    pub code: InteractionValidationCode,
    /// JSON Pointer relative to the interaction extension payload.
    pub path: String,
    /// Stable human-readable explanation.
    pub message: String,
}

/// Ordered collection of official interaction extension issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionExtensionErrors {
    issues: Vec<InteractionValidationIssue>,
}

impl InteractionExtensionErrors {
    /// Creates a sorted, duplicate-free error collection.
    pub fn new(mut issues: Vec<InteractionValidationIssue>) -> Self {
        issues.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.message.cmp(&right.message))
        });
        issues.dedup();
        Self { issues }
    }

    /// Creates one stable payload-decoding error.
    pub fn decode(message: impl Into<String>) -> Self {
        Self::new(vec![InteractionValidationIssue {
            code: InteractionValidationCode::InvalidExtensionPayload,
            path: String::new(),
            message: message.into(),
        }])
    }

    /// Returns ordered validation issues.
    pub fn issues(&self) -> &[InteractionValidationIssue] {
        &self.issues
    }
}

impl fmt::Display for InteractionExtensionErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "official interaction extension validation failed with {} issue(s):",
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
                "- {:?} at {}{}: {}",
                issue.code, INTERACTION_EXTENSION_KEY, path, issue.message
            )?;
        }
        Ok(())
    }
}

impl Error for InteractionExtensionErrors {}

impl ArtifactIr {
    /// Decodes and validates the official interaction extension when present.
    ///
    /// # Errors
    ///
    /// Returns stable extension decoding or semantic validation issues.
    pub fn interaction_extension(
        &self,
    ) -> Result<Option<InteractionExtension>, InteractionExtensionErrors> {
        let Some(value) = self.extensions.get(INTERACTION_EXTENSION_KEY) else {
            return Ok(None);
        };
        let extension: InteractionExtension =
            serde_json::from_value(value.clone()).map_err(|error| {
                InteractionExtensionErrors::decode(format!(
                    "failed to decode interaction extension: {error}"
                ))
            })?;
        extension.validate(self)?;
        Ok(Some(extension.canonicalized()))
    }
}

/// Generates the JSON Schema for the current official interaction extension.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be represented as JSON.
pub fn interaction_extension_schema_json() -> Result<String, serde_json::Error> {
    let mut schema = serde_json::to_value(schema_for!(InteractionExtension))?;
    if let Value::Object(object) = &mut schema {
        object.insert(
            "$id".to_owned(),
            Value::String(format!(
                "urn:sightlint:schema:interaction-extension:{INTERACTION_EXTENSION_VERSION}"
            )),
        );
        object.insert(
            "title".to_owned(),
            Value::String(format!(
                "SightLint Interaction Extension {INTERACTION_EXTENSION_VERSION}"
            )),
        );
    }
    serialize_canonical(&schema)
}

pub(crate) fn canonicalize_interaction_extension(value: &mut Value) {
    let Ok(extension) = serde_json::from_value::<InteractionExtension>(value.clone()) else {
        return;
    };
    if let Ok(canonical) = serde_json::to_value(extension.canonicalized()) {
        *value = canonical;
    }
}

fn validate_trace(
    trace: &InteractionTrace,
    action: Option<&InteractionAction>,
    path: &str,
    validator: &mut InteractionValidator,
) {
    validate_trace_status(trace, path, validator);
    validate_trace_events(trace, action, path, validator);
}

fn validate_trace_status(
    trace: &InteractionTrace,
    path: &str,
    validator: &mut InteractionValidator,
) {
    match &trace.execution {
        TraceExecution::Captured if trace.events.is_empty() => validator.issue(
            InteractionValidationCode::InvalidValue,
            format!("{path}/events"),
            "a captured trace must contain at least one event",
        ),
        TraceExecution::Untested { reason } => {
            if reason.is_empty() {
                validator.issue(
                    InteractionValidationCode::InvalidValue,
                    format!("{path}/execution/reason"),
                    "an untested reason must not be empty",
                );
            }
            if !trace.events.is_empty() {
                validator.issue(
                    InteractionValidationCode::InvalidValue,
                    format!("{path}/events"),
                    "an untested trace must not contain events",
                );
            }
            if !matches!(trace.consistency, TraceConsistency::Agreement) {
                validator.issue(
                    InteractionValidationCode::InvalidValue,
                    format!("{path}/consistency"),
                    "an untested trace cannot claim an observed conflict",
                );
            }
        }
        TraceExecution::Captured => {}
    }

    if let TraceConsistency::Conflict {
        evidence_ids,
        reason,
    } = &trace.consistency
    {
        if reason.is_empty() || evidence_ids.len() < 2 {
            validator.issue(
                InteractionValidationCode::InvalidValue,
                format!("{path}/consistency"),
                "a conflict requires a nonempty reason and at least two evidence records",
            );
        }
        validator.unique_evidence(evidence_ids, &format!("{path}/consistency/evidenceIds"));
    }
}

fn validate_trace_events(
    trace: &InteractionTrace,
    action: Option<&InteractionAction>,
    path: &str,
    validator: &mut InteractionValidator,
) {
    let mut events = BTreeMap::new();
    for (index, event) in trace.events.iter().enumerate() {
        let event_path = format!("{path}/events/{index}");
        validator.nonempty_identifier(&event.id, &format!("{event_path}/id"));
        validator.nonempty_identifier(&event.attempt_id, &format!("{event_path}/attemptId"));
        if events.insert(event.id.clone(), event.sequence).is_some() {
            validator.issue(
                InteractionValidationCode::DuplicateIdentifier,
                format!("{event_path}/id"),
                format!("event {} is duplicated", event.id),
            );
        }
        if event.sequence != index as u64 + 1 {
            validator.issue(
                InteractionValidationCode::InvalidSequence,
                format!("{event_path}/sequence"),
                format!("expected sequence {}, found {}", index + 1, event.sequence),
            );
        }
        if event.evidence_ids.is_empty() {
            validator.issue(
                InteractionValidationCode::InvalidValue,
                format!("{event_path}/evidenceIds"),
                "a trace event requires evidence",
            );
        }
        validator.unique_evidence(&event.evidence_ids, &format!("{event_path}/evidenceIds"));
        if !event
            .evidence_ids
            .iter()
            .any(|id| validator.evidence_class(id) == Some(EvidenceClass::InteractionTrace))
        {
            validator.issue(
                InteractionValidationCode::InvalidEvidenceClass,
                format!("{event_path}/evidenceIds"),
                "every trace event requires interactionTrace evidence",
            );
        }
        if let Some(action) = action {
            let recovery = match event.detail {
                TraceEventDetail::RecoveryOffered { recovery }
                | TraceEventDetail::RecoveryActivated { recovery } => Some(recovery),
                _ => None,
            };
            if let Some(recovery) = recovery {
                if !action.recovery.accepted_alternatives().contains(&recovery) {
                    validator.issue(
                        InteractionValidationCode::InvalidValue,
                        format!("{event_path}/recovery"),
                        "trace recovery is not an accepted alternative for the action",
                    );
                }
            }
        }
    }
    for (index, event) in trace.events.iter().enumerate() {
        if let Some(cause_id) = &event.cause_event_id {
            let valid = events
                .get(cause_id)
                .is_some_and(|cause_sequence| *cause_sequence < event.sequence);
            if !valid {
                validator.issue(
                    InteractionValidationCode::InvalidCausalReference,
                    format!("{path}/events/{index}/causeEventId"),
                    format!("cause event {cause_id} must exist earlier in the same trace"),
                );
            }
        }
    }
}

struct InteractionValidator {
    issues: Vec<InteractionValidationIssue>,
    node_ids: BTreeSet<Identifier>,
    evidence_classes: BTreeMap<Identifier, EvidenceClass>,
}

impl InteractionValidator {
    fn new(document: &ArtifactIr) -> Self {
        Self {
            issues: Vec::new(),
            node_ids: document.nodes.iter().map(|node| node.id.clone()).collect(),
            evidence_classes: document
                .evidence
                .iter()
                .map(|evidence| (evidence.id.clone(), evidence.class))
                .collect(),
        }
    }

    fn issue(
        &mut self,
        code: InteractionValidationCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(InteractionValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }

    fn nonempty_identifier(&mut self, id: &Identifier, path: &str) {
        if id.is_empty() {
            self.issue(
                InteractionValidationCode::EmptyIdentifier,
                path,
                "identifier must not be empty",
            );
        }
    }

    fn require_node(&mut self, id: &Identifier, path: &str) {
        if !self.node_ids.contains(id) {
            self.issue(
                InteractionValidationCode::InvalidNodeReference,
                path,
                format!("referenced node {id} does not exist"),
            );
        }
    }

    fn evidence_class(&self, id: &Identifier) -> Option<EvidenceClass> {
        self.evidence_classes.get(id).copied()
    }

    fn require_evidence_class(&mut self, id: &Identifier, class: EvidenceClass, path: &str) {
        match self.evidence_class(id) {
            None => self.issue(
                InteractionValidationCode::InvalidEvidenceReference,
                path,
                format!("referenced evidence {id} does not exist"),
            ),
            Some(actual) if actual != class => self.issue(
                InteractionValidationCode::InvalidEvidenceClass,
                path,
                format!("evidence {id} must be {class:?}, found {actual:?}"),
            ),
            Some(_) => {}
        }
    }

    fn unique_evidence(&mut self, ids: &[Identifier], path: &str) {
        let mut unique = BTreeSet::new();
        for (index, id) in ids.iter().enumerate() {
            if self.evidence_class(id).is_none() {
                self.issue(
                    InteractionValidationCode::InvalidEvidenceReference,
                    format!("{path}/{index}"),
                    format!("referenced evidence {id} does not exist"),
                );
            }
            if !unique.insert(id) {
                self.issue(
                    InteractionValidationCode::DuplicateIdentifier,
                    format!("{path}/{index}"),
                    format!("evidence {id} appears more than once"),
                );
            }
        }
    }

    fn unique_recoveries(&mut self, values: &[RecoveryKind], path: &str) {
        let mut unique = BTreeSet::new();
        for (index, value) in values.iter().enumerate() {
            if !unique.insert(*value) {
                self.issue(
                    InteractionValidationCode::InvalidValue,
                    format!("{path}/{index}"),
                    format!("recovery alternative {value:?} appears more than once"),
                );
            }
        }
    }

    fn finish(self) -> Result<(), InteractionExtensionErrors> {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(InteractionExtensionErrors::new(self.issues))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        INTERACTION_EXTENSION_VERSION, InteractionExtension, RecoveryContract, RecoveryKind,
    };

    #[test]
    fn canonicalization_sorts_actions_traces_events_and_set_values() {
        let mut extension = InteractionExtension {
            extension_version: INTERACTION_EXTENSION_VERSION.to_owned(),
            actions: Vec::new(),
            traces: Vec::new(),
        };
        assert_eq!(extension.canonicalized(), extension);

        let mut recovery = RecoveryContract::Required {
            accepted_alternatives: vec![RecoveryKind::SaveDraft, RecoveryKind::Retry],
        };
        recovery.canonicalize();
        assert_eq!(
            recovery.accepted_alternatives(),
            &[RecoveryKind::Retry, RecoveryKind::SaveDraft]
        );
        extension.actions.clear();
    }
}
