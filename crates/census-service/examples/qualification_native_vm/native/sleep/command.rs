use super::entry::Entry;
use super::shape::{self, field, fields, integer, result, text, variant};
use anyhow::{ensure, Result};
use serde_json::Value;

mod request;

pub(super) fn parse(value: &Value, row_type: &str) -> Result<Entry> {
    let (kind, payload) = variant(value, "command")?;
    let required = required_fields(kind)?;
    ensure!(
        row_type.strip_prefix("Command: ") == Some(kind),
        "journal entry type mismatch"
    );
    fields(payload, required, &[])?;
    required
        .iter()
        .try_for_each(|key| validate_field(kind, key, field(payload, key)?))?;
    if kind != "Sleep" {
        return Ok(Entry::Other);
    }
    let completion = shape::completion(payload)?;
    let wake = integer(
        field(payload, "wake_up_time")?,
        u64::MAX,
        "Sleep wake time malformed",
    )?;
    Ok(Entry::Sleep { completion, wake })
}

fn required_fields(kind: &str) -> Result<&'static [&'static str]> {
    match kind {
        "Input" => Ok(&["headers", "payload", "name"]),
        "Output" => Ok(&["result", "name"]),
        "GetLazyState" | "GetPromise" | "PeekPromise" => Ok(&["key", "completion_id", "name"]),
        "SetState" => Ok(&["key", "value", "name"]),
        "ClearState" => Ok(&["key", "name"]),
        "ClearAllState" => Ok(&["name"]),
        "GetLazyStateKeys" | "Run" => Ok(&["completion_id", "name"]),
        "GetEagerState" => Ok(&["key", "result", "name"]),
        "GetEagerStateKeys" => Ok(&["state_keys", "name"]),
        "CompletePromise" => Ok(&["key", "value", "completion_id", "name"]),
        "Sleep" => Ok(&["wake_up_time", "completion_id", "name"]),
        "Call" => Ok(&[
            "request",
            "invocation_id_completion_id",
            "result_completion_id",
            "name",
        ]),
        "OneWayCall" => Ok(&[
            "request",
            "invoke_time",
            "invocation_id_completion_id",
            "name",
        ]),
        "SendSignal" => Ok(&["target_invocation_id", "signal_id", "result", "name"]),
        "AttachInvocation" | "GetInvocationOutput" => Ok(&["target", "completion_id", "name"]),
        "CompleteAwakeable" => Ok(&["id", "result", "name"]),
        _ => anyhow::bail!("unknown command variant"),
    }
}

fn validate_field(kind: &str, key: &str, value: &Value) -> Result<()> {
    match key {
        "name" | "key" | "id" | "target_invocation_id" => text(value),
        "completion_id" | "invocation_id_completion_id" | "result_completion_id" => {
            integer(value, u64::from(u32::MAX), "completion ID outside u32").map(|_| ())
        }
        "wake_up_time" | "invoke_time" => {
            integer(value, u64::MAX, "command time malformed").map(|_| ())
        }
        "headers" => shape::headers(value),
        "payload" => shape::bytes(value),
        "state_keys" => shape::strings(value),
        "value" if kind == "CompletePromise" => result(value, &["Success", "Failure"]),
        "value" => shape::bytes(value),
        "result" if kind == "GetEagerState" => result(value, &["Void", "Success"]),
        "result" if kind == "SendSignal" => result(value, &["Void", "Success", "Failure"]),
        "result" => result(value, &["Success", "Failure"]),
        "signal_id" => shape::signal(value),
        "request" => request::parse(value),
        "target" => request::attachment(value),
        _ => anyhow::bail!("unknown command field"),
    }
}
