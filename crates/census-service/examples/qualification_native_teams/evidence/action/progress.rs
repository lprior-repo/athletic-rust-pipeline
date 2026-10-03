use anyhow::Result;
use census_service::restate_services::{StageOutcome, TeamsSourceOutcome};
use serde_json::{json, Value};

use super::Slot;

pub(super) fn matches(
    outcome: Option<&TeamsSourceOutcome>,
    slots: &[Slot<'_>],
    at: &str,
) -> Result<bool> {
    let Some(outcome) = outcome else {
        return Ok(false);
    };
    let settled = serde_json::to_value(outcome)?;
    let expected = slots
        .iter()
        .filter(|(_, reserved, _)| reserved.is_some())
        .map(|(attempt, _, recorded)| entry(*attempt, *recorded, at))
        .collect::<Result<Option<Vec<_>>>>()?;
    Ok(expected.is_some_and(|expected| settled.get("progress") == Some(&json!(expected))))
}

fn entry(attempt: u8, recorded: Option<&Value>, at: &str) -> Result<Option<Value>> {
    let Some(recorded) = recorded else {
        return Ok(Some(json!({"status":"unknown", "attempt":attempt})));
    };
    let status = recorded.get("status").and_then(Value::as_str);
    let raw = recorded
        .get(if status == Some("completed") {
            "outcome"
        } else {
            "progress"
        })
        .filter(|value| !value.is_null());
    if let Some(raw) = raw {
        let outcome: StageOutcome = serde_json::from_value(raw.clone())?;
        if outcome.at != at {
            return Ok(None);
        }
    }
    Ok(match status {
        Some("completed") if raw.is_some() => {
            Some(json!({"status":"completed", "attempt":attempt, "outcome":raw}))
        }
        Some("terminal" | "transient")
            if recorded.get("message").and_then(Value::as_str).is_some() =>
        {
            Some(
                json!({"status":status, "attempt":attempt, "outcome":raw, "message":recorded.get("message")}),
            )
        }
        _ => None,
    })
}
