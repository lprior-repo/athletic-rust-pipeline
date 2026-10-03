use anyhow::Result;
use census_domain::model::serialized_digest;
use census_service::restate_services::{JurisdictionRequest, TeamsSourceOutcome};
use serde_json::{json, Value};

use super::super::artifacts::oracle;
use super::super::ledger::Snapshot;
use super::super::scenario::{SourceRepeat, SourceRun};

mod progress;

#[cfg(test)]
mod tests;

pub fn checks(source: &SourceRun, history: &Value, repeats: &[SourceRepeat]) -> Result<Vec<Value>> {
    let typed = output_matches(source)?;
    let settled = history.get("consistent").and_then(Value::as_bool) == Some(true);
    let repeated = repeats
        .iter()
        .find(|repeat| repeat.original_id == source.id);
    let replay = repeated
        .map(|repeat| replay_matches(source, repeat))
        .transpose()?
        .is_some_and(|value| value);
    Ok(vec![
        oracle(
            "native_child_typed_output_equals_shared_settlement",
            typed,
            json!({"source":source.source, "key":source.key, "invocation":source.invocation, "output":source.output, "shared_state":source.shared_state}),
        ),
        oracle(
            "source_ledger_metadata_history_and_settlement_consistent",
            settled,
            history.clone(),
        ),
        oracle(
            "settled_direct_source_repeat_zero_additional_SOURCE_admissions_no_fourth",
            typed && settled && replay,
            json!({"source":source.source, "repeat":repeated, "final_slot_ceiling_proved":settled,
                "scope":"isolated direct same-key handler repeat; physical admissions are not logical reservations",
                "slot_byte_identity":"UNPROVEN: no restart-separated before/after cold snapshots"}),
        ),
    ])
}

pub fn output_matches(source: &SourceRun) -> Result<bool> {
    if !(200..300).contains(&source.output.status) || source.outcome.is_none() {
        return Ok(false);
    }
    let decoded: TeamsSourceOutcome = serde_json::from_value(source.output.body.clone())?;
    Ok(serde_json::to_value(decoded)? == source.shared_state.body
        && source
            .invocation
            .get("completion_result")
            .and_then(Value::as_str)
            == Some("success")
        && source.invocation.get("status").and_then(Value::as_str) == Some("completed"))
}

fn replay_matches(original: &SourceRun, repeat: &SourceRepeat) -> Result<bool> {
    Ok(original.id != repeat.run.id
        && original.key == repeat.run.key
        && original.shared_state.body == repeat.run.shared_state.body
        && original.output.body == repeat.run.output.body
        && output_matches(&repeat.run)?
        && repeat.before == repeat.after)
}

pub fn history(
    source: &SourceRun,
    snapshot: &Snapshot,
    request: &JurisdictionRequest,
    at: &str,
) -> Result<Value> {
    let prefix = format!("{}/", source.key);
    let entries = snapshot
        .entries
        .iter()
        .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|suffix| (suffix, value)))
        .collect::<Vec<_>>();
    let identity = snapshot.entries.get(&format!("{}identity", prefix));
    let digest = serialized_digest(&(
        request.jurisdiction,
        request.season,
        request.revision,
        &source.source,
    ))?;
    let metadata = identity.is_some_and(|value| {
        value.get("request_digest").and_then(Value::as_str) == Some(digest.as_str())
            && value.get("observed_on").and_then(Value::as_str) == Some(at)
    });
    let slots = [1_u8, 2, 3].map(|attempt| {
        let reserved = snapshot
            .entries
            .get(&format!("{prefix}attempt/{attempt}/reserved"));
        let outcome = snapshot
            .entries
            .get(&format!("{prefix}attempt/{attempt}/outcome"));
        (attempt, reserved, outcome)
    });
    let reservations = slots
        .iter()
        .filter_map(|(attempt, reserved, _)| reserved.map(|_| *attempt))
        .collect::<Vec<_>>();
    let outcomes = slots
        .iter()
        .filter_map(|(attempt, _, outcome)| outcome.map(|_| *attempt))
        .collect::<Vec<_>>();
    let legal_keys = entries.iter().all(|(suffix, _)| legal_suffix(suffix));
    let contiguous = reservations
        .iter()
        .copied()
        .eq((1_u8..=3).take(reservations.len()));
    let paired = slots.iter().all(|(attempt, reserved, outcome)| {
        outcome.is_none()
            || reserved.is_some()
                && reserved.is_some_and(|value| {
                    value.get("attempt").and_then(Value::as_u64) == Some(u64::from(*attempt))
                })
    });
    let dates = slots.iter().all(|(_, reserved, _)| {
        reserved.is_none_or(|value| value.get("observed_on").and_then(Value::as_str) == Some(at))
    });
    let reservation_numbers = slots.iter().all(|(attempt, reserved, _)| {
        reserved.is_none_or(|value| {
            value.get("attempt").and_then(Value::as_u64) == Some(u64::from(*attempt))
        })
    });
    let settled = snapshot.entries.get(&format!("{prefix}settled"));
    let same = settled == Some(&source.shared_state.body) && source.outcome.is_some();
    let classification = settlement_matches(&slots, &reservations, source.outcome.as_ref())?;
    let progress_valid = progress::matches(source.outcome.as_ref(), &slots, at)?;
    let completed_valid = match &source.outcome {
        Some(TeamsSourceOutcome::Completed { outcome, .. }) => {
            outcome.errors.is_empty() && outcome.at == at
        }
        _ => true,
    };
    let valid = metadata
        && legal_keys
        && contiguous
        && paired
        && dates
        && reservation_numbers
        && same
        && classification
        && progress_valid
        && completed_valid;
    Ok(
        json!({"source":source.source, "key":source.key, "phase":snapshot.phase, "consistent":valid,
        "identity":identity, "expected_request_digest":digest, "expected_observed_on":at,
        "reservations":reservations, "recorded_outcomes":outcomes, "exact_prefix_entries":entries,
        "legal_keys_no_attempt_four_or_unknown_suffix":legal_keys, "contiguous":contiguous,
        "outcomes_paired":paired, "reservation_metadata_valid":metadata && dates && reservation_numbers,
        "settled":settled, "settled_equals_shared_state":same, "settlement_classification_matches_history":classification,
        "per_attempt_progress_matches_actual_slots":progress_valid,
        "progress_semantics":"per-attempt observed report/outcome, never summed or asserted as unique source population"}),
    )
}

fn legal_suffix(suffix: &str) -> bool {
    matches!(suffix, "identity" | "settled")
        || (1..=3).any(|attempt| {
            suffix == format!("attempt/{attempt}/reserved")
                || suffix == format!("attempt/{attempt}/outcome")
        })
}

type Slot<'a> = (u8, Option<&'a Value>, Option<&'a Value>);

fn settlement_matches(
    slots: &[Slot<'_>],
    reservations: &[u8],
    outcome: Option<&TeamsSourceOutcome>,
) -> Result<bool> {
    let Some(last) = reservations.last() else {
        return Ok(false);
    };
    let previous = slots
        .iter()
        .filter(|(attempt, reserved, _)| attempt < last && reserved.is_some())
        .all(|(_, _, outcome)| outcome.is_none_or(transient));
    let recorded = slots
        .iter()
        .find(|(attempt, _, _)| attempt == last)
        .and_then(|(_, _, value)| *value);
    Ok(previous
        && match outcome {
            Some(TeamsSourceOutcome::Completed { outcome, .. }) => {
                let expected = serde_json::to_value(outcome)?;
                recorded.is_some_and(|value| {
                    value.get("status").and_then(Value::as_str) == Some("completed")
                        && value.get("outcome") == Some(&expected)
                })
            }
            Some(TeamsSourceOutcome::Terminal { message, .. }) => recorded.is_some_and(|value| {
                value.get("status").and_then(Value::as_str) == Some("terminal")
                    && value.get("message").and_then(Value::as_str) == Some(message.as_str())
            }),
            Some(TeamsSourceOutcome::Exhausted {
                attempts,
                last_failure,
                ..
            }) => {
                *last == 3
                    && *attempts == 3
                    && slots.iter().all(|(_, reserved, value)| {
                        reserved.is_some() && value.is_some_and(transient)
                    })
                    && recorded
                        .and_then(|value| value.get("message"))
                        .and_then(Value::as_str)
                        == Some(last_failure.as_str())
            }
            Some(TeamsSourceOutcome::Interrupted {
                attempts: Some(3), ..
            }) => {
                *last == 3
                    && (recorded.is_none()
                        || recorded.is_some_and(transient)
                            && slots.iter().any(|(attempt, reserved, outcome)| {
                                attempt < last && reserved.is_some() && outcome.is_none()
                            }))
            }
            Some(TeamsSourceOutcome::Interrupted { .. }) | None => false,
        })
}

fn transient(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("transient")
        && value.get("message").and_then(Value::as_str).is_some()
}
