use super::entry::Entry;
use super::shape::{self, field, fields, result, strings, text, variant};
use anyhow::{ensure, Result};
use serde_json::Value;

pub(super) fn parse(value: &Value, row_type: &str) -> Result<Entry> {
    let (kind, payload) = variant(value, "notification")?;
    match kind {
        "Completion" => completion(payload, row_type),
        "Signal" => {
            ensure!(
                row_type == "Notification: Signal",
                "journal entry type mismatch"
            );
            fields(payload, &["id", "result"], &[])?;
            shape::signal(field(payload, "id")?)?;
            result(field(payload, "result")?, &["Void", "Success", "Failure"])?;
            Ok(Entry::Other)
        }
        _ => anyhow::bail!("unknown notification variant"),
    }
}

fn completion(value: &Value, row_type: &str) -> Result<Entry> {
    let (kind, payload) = variant(value, "completion")?;
    let extra = match kind {
        "Sleep" => None,
        "GetLazyStateKeys" => Some("state_keys"),
        "CallInvocationId" => Some("invocation_id"),
        "GetLazyState"
        | "GetPromise"
        | "PeekPromise"
        | "CompletePromise"
        | "Call"
        | "Run"
        | "AttachInvocation"
        | "GetInvocationOutput" => Some("result"),
        _ => anyhow::bail!("unknown completion variant"),
    };
    ensure!(
        row_type.strip_prefix("Notification: ") == Some(kind),
        "journal entry type mismatch"
    );
    match extra {
        None => {
            fields(payload, &["completion_id"], &[])?;
        }
        Some(key) => {
            fields(payload, &["completion_id", key], &[])?;
        }
    }
    let id = shape::completion(payload)?;
    validate_result(kind, payload)?;
    Ok(Entry::Completion(id))
}

fn validate_result(kind: &str, payload: &Value) -> Result<()> {
    match kind {
        "Sleep" => Ok(()),
        "GetLazyStateKeys" => strings(field(payload, "state_keys")?),
        "CallInvocationId" => text(field(payload, "invocation_id")?),
        "GetLazyState" => result(field(payload, "result")?, &["Void", "Success"]),
        "CompletePromise" => result(field(payload, "result")?, &["Void", "Failure"]),
        "PeekPromise" | "GetInvocationOutput" => {
            result(field(payload, "result")?, &["Void", "Success", "Failure"])
        }
        "GetPromise" | "Call" | "Run" | "AttachInvocation" => {
            result(field(payload, "result")?, &["Success", "Failure"])
        }
        _ => anyhow::bail!("unknown completion variant"),
    }
}
