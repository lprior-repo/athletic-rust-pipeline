use anyhow::Result;
use census_service::restate_services::{
    JurisdictionRequest, JurisdictionState, TeamsFailure, TeamsSourceOutcome, TeamsStage,
};
use serde_json::{json, Value};

use super::super::artifacts::oracle;
use super::super::ledger::Snapshot;
use super::super::scenario::{Run, SourceRun};
use super::{action, Measurement};
pub(super) fn source_checks(
    measured: &Measurement,
    state: &JurisdictionState,
    ledger: Option<&Snapshot>,
) -> Result<Vec<Value>> {
    let request: JurisdictionRequest = serde_json::from_value(measured.request.clone())?;
    let at = match &state.teams {
        TeamsStage::Failed(TeamsFailure::SourceFailures { at, .. }) => Some(at.as_str()),
        TeamsStage::Completed(completed) => Some(completed.outcome().at.as_str()),
        _ => None,
    };
    let mut checks = vec![oracle(
        "actual_parent_created_TeamsSource_run_observed",
        !measured.first.sources.is_empty(),
        json!({"invocations":measured.first.source_invocations}),
    )];
    let histories = match (ledger, at) {
        (Some(ledger), Some(at)) => measured
            .first
            .sources
            .iter()
            .map(|source| -> Result<_> {
                let history = action::history(source, ledger, &request, at)?;
                checks.extend(action::checks(source, &history, &measured.source_repeats)?);
                Ok((source, history))
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            checks.push(oracle("source_cold_ledger_available", false,
                json!({"reason":"cold readback requires measured endpoint TERM/drain/reap and original source date"})));
            Vec::new()
        }
    };
    checks.extend(classification_checks(&histories));
    checks.extend(unknown_checks(measured, state, ledger));
    Ok(checks)
}

fn unknown_call_matches(first: &Run, source: &str, message: &str) -> bool {
    first
        .sources
        .iter()
        .find(|child| child.source == source)
        .is_some_and(|child| {
            !(200..300).contains(&child.output.status)
                && child.output.error_source.as_deref() == Some("invocation")
                && child
                    .output
                    .body
                    .get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|actual| !actual.is_empty() && message.contains(actual))
        })
}

pub(super) fn retained(first: &Run, state: &JurisdictionState) -> Result<bool> {
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &state.teams else {
        return Ok(false);
    };
    if failures.is_empty() {
        return Ok(false);
    }
    failures
        .iter()
        .try_fold(true, |valid, failure| -> Result<_> {
            let child = first
                .sources
                .iter()
                .find(|child| child.source == failure.source);
            let same = match (&failure.outcome, child) {
                (
                    TeamsSourceOutcome::Interrupted {
                        attempts: None,
                        message,
                        ..
                    },
                    _,
                ) => unknown_call_matches(first, &failure.source, message),
                (_, Some(child)) => {
                    child
                        .outcome
                        .as_ref()
                        .map(serde_json::to_value)
                        .transpose()?
                        == Some(serde_json::to_value(&failure.outcome)?)
                        && action::output_matches(child)?
                }
                (_, None) => false,
            };
            Ok(valid && same)
        })
}

fn classification_checks(histories: &[(&SourceRun, Value)]) -> Vec<Value> {
    let mut checks = Vec::with_capacity(3);
    let exhausted = histories.iter().any(|(source, history)| {
        source.source == "milesplit"
            && matches!(
                source.outcome,
                Some(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
            )
            && history.get("consistent").and_then(Value::as_bool) == Some(true)
    });
    checks.push(oracle("transient_source_exhaustion_exactly_three_reservations_and_outcomes_no_four", exhausted,
        json!({"histories":histories, "attempt_authority":"Fjall immutable reserved/outcome slots, not SDK CompletionId traces"})));
    checks
}

fn unknown_checks(
    measured: &Measurement,
    state: &JurisdictionState,
    ledger: Option<&Snapshot>,
) -> Vec<Value> {
    let mut checks = Vec::new();
    if let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &state.teams {
        checks.extend(failures.iter().filter_map(|failure| match &failure.outcome {
            TeamsSourceOutcome::Interrupted { attempts:None, message, .. } => Some(oracle(
                "native_child_call_failure_attempt_count_retained_as_unknown",
                unknown_call_matches(&measured.first, &failure.source, message),
                json!({"source":failure.source, "parent_retained_outcome":failure.outcome,
                    "actual_child":measured.first.sources.iter().find(|source| source.source == failure.source),
                    "durable_slots":ledger, "meaning":"native call failure does not provide a known source attempt count; null is never inferred as zero or three"}))),
            _ => None,
        }));
    }
    checks
}
