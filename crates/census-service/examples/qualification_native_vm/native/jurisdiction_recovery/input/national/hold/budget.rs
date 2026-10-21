use super::{query, rows, safe_id, text, Transition};
use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde_json::Value;

const IDENTITY: [&str; 4] = ["entry_id", "vqueue_id", "entry_kind", "deployment"];
const COUNTERS: [&str; 3] = [
    "retry_attempts",
    "retry_count_since_last_stored_command",
    "num_errors",
];

pub(super) async fn read(client: &Client, id: &str) -> Result<Value> {
    safe_id(id)?;
    let value = query(client, &format!("SELECT entry_id, vqueue_id, entry_kind, stage, deployment, retry_attempts, retry_count_since_last_stored_command, num_attempts, num_errors, num_pauses, num_suspensions, num_yields FROM sys_vqueue_entry_status WHERE entry_id = '{id}' LIMIT 2")).await?;
    let [row] = rows(&value)? else {
        anyhow::bail!("persistent invocation entry absent or duplicated");
    };
    validate(row, id)?;
    Ok(row.clone())
}

fn validate(row: &Value, id: &str) -> Result<()> {
    ensure!(
        text(row, "entry_id")? == id && text(row, "entry_kind")? == "invocation",
        "persistent invocation identity differs"
    );
    for field in IDENTITY {
        ensure!(!text(row, field)?.is_empty(), "persistent {field} empty");
    }
    for field in COUNTERS.into_iter().chain([
        "num_attempts",
        "num_pauses",
        "num_suspensions",
        "num_yields",
    ]) {
        integer(row, field)?;
    }
    ensure!(
        matches!(
            text(row, "stage")?,
            "running" | "suspended" | "paused" | "inbox" | "finished"
        ),
        "persistent invocation stage unsupported"
    );
    Ok(())
}

pub(super) fn preserved(before: &Value, after: &Value, action: Transition) -> Result<()> {
    let id = text(before, "entry_id")?;
    validate(before, id)?;
    validate(after, id)?;
    for field in IDENTITY.into_iter().chain(COUNTERS) {
        ensure!(
            before.get(field) == after.get(field),
            "native control changed persistent {field}"
        );
    }
    match action {
        Transition::Pause => {
            ensure!(
                text(before, "stage")? == "suspended" && text(after, "stage")? == "paused",
                "persistent pause did not preserve quiescent invocation"
            );
            ensure!(
                integer(before, "num_attempts")? == integer(after, "num_attempts")?,
                "pause executed another invocation attempt"
            );
            ensure!(
                integer(before, "num_pauses")?.checked_add(1)
                    == Some(integer(after, "num_pauses")?),
                "manual pause transition count differs"
            );
        }
        Transition::Resume => {
            ensure!(
                text(before, "stage")? == "paused" && text(after, "stage")? != "paused",
                "persistent resume did not leave the paused stage"
            );
            ensure!(
                integer(before, "num_pauses")? == integer(after, "num_pauses")?,
                "resume paused another invocation"
            );
            ensure!(
                integer(after, "num_attempts")? >= integer(before, "num_attempts")?,
                "resume rolled back invocation attempts"
            );
        }
    }
    Ok(())
}

pub(super) fn held(original: &Value, current: &Value) -> Result<()> {
    let id = text(original, "entry_id")?;
    validate(original, id)?;
    validate(current, id)?;
    ensure!(
        text(original, "stage")? == "paused" && original == current,
        "held persistent invocation metadata changed"
    );
    Ok(())
}

fn integer(row: &Value, field: &str) -> Result<u32> {
    Ok(u32::try_from(
        row.get(field)
            .and_then(Value::as_u64)
            .with_context(|| format!("persistent {field} absent or nonnumeric"))?,
    )?)
}

#[cfg(test)]
mod tests;
