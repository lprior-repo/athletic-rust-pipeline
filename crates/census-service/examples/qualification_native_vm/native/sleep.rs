use super::http::{query, rows};
use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

mod command;
mod entry;
mod future;
mod notification;
mod shape;

#[tracing::instrument(skip(client))]
pub(super) async fn observe(client: &Client, id: &str, key: &str) -> Result<Value> {
    let status_sql = format!(
        "SELECT id, target_service_name, target_service_key, target_handler_name, status, pinned_deployment_id, pinned_service_protocol_version, journal_size, modified_at, suspended_waiting_future_json, last_awaiting_on_future_json, retry_count, last_start_at, next_retry_at, last_attempt_deployment_id, last_attempt_server FROM sys_invocation WHERE id = '{id}' LIMIT 2"
    );
    let before = query(client, &status_sql).await?;
    let journal = query(client, &format!("SELECT id, index, version, entry_type, entry_json FROM sys_journal WHERE id = '{id}' ORDER BY index LIMIT 257")).await?;
    let after = query(client, &status_sql).await?;
    Ok(json!({"before":before,"journal":journal,"after":after,"id":id,"key":key}))
}

pub(in super::super) fn unfinished(observation: &Value, clock: &Value) -> Result<Option<Value>> {
    let Some(status) = stable_status(observation)? else {
        return Ok(None);
    };
    validate_identity(status)?;
    let Some(entries) = complete_journal(observation, status)? else {
        return Ok(None);
    };
    let future = shape::decode(
        status
            .get("suspended_waiting_future_json")
            .context("suspended future absent")?,
    )?;
    let awaited = future::completion_ids(&future)?;
    ensure!(
        awaited.iter().any(Option::is_some),
        "suspended future has no awaited CompletionId; unfinished Sleep correlation refused"
    );
    let now = guest_now(clock)?;
    let threshold = now
        .checked_add(30_000)
        .context("clock safety margin overflow")?;
    Ok(entries.iter().find_map(|entry| {
        let entry::Entry::Sleep { completion, wake } = entry else { return None; };
        if *wake <= threshold || !awaited.contains(&Some(*completion)) { return None; }
        let completed = entries.iter().any(|entry| matches!(entry, entry::Entry::Completion(id) if id == completion));
        if completed { return None; }
        Some(json!({"completion_id":completion,"wake_up_time":wake,"guest_now":now,"safety_margin_ms":30000}))
    }))
}

fn stable_status(observation: &Value) -> Result<Option<&Value>> {
    let id = observation
        .get("id")
        .and_then(Value::as_str)
        .context("observation ID absent")?;
    let key = observation
        .get("key")
        .and_then(Value::as_str)
        .context("observation key absent")?;
    let before = rows(
        observation
            .get("before")
            .context("before snapshot absent")?,
    )?;
    let after = rows(observation.get("after").context("after snapshot absent")?)?;
    if before != after || before.len() != 1 {
        return Ok(None);
    }
    let status = before.first().context("invocation absent")?;
    Ok(matches_target(status, id, key).then_some(status))
}

fn complete_journal(observation: &Value, status: &Value) -> Result<Option<Vec<entry::Entry>>> {
    let id = observation
        .get("id")
        .and_then(Value::as_str)
        .context("observation ID absent")?;
    let expected = status
        .get("journal_size")
        .and_then(Value::as_u64)
        .context("journal size absent")?;
    ensure!(
        (1..=256).contains(&expected),
        "journal witness size outside bounded scope"
    );
    let journal = rows(
        observation
            .get("journal")
            .context("journal snapshot absent")?,
    )?;
    if u64::try_from(journal.len())? != expected {
        return Ok(None);
    }
    Ok(Some(entry::journal(journal, id)?))
}

fn guest_now(clock: &Value) -> Result<u64> {
    let now = clock
        .get("realtime")
        .and_then(Value::as_str)
        .context("guest clock absent")?;
    Ok(u64::try_from(
        chrono::DateTime::parse_from_rfc3339(now)?.timestamp_millis(),
    )?)
}

fn matches_target(status: &Value, id: &str, key: &str) -> bool {
    [
        ("id", id),
        ("target_service_name", "Sweep"),
        ("target_service_key", key),
        ("target_handler_name", "run"),
        ("status", "suspended"),
    ]
    .into_iter()
    .all(|(field, expected)| status.get(field).and_then(Value::as_str) == Some(expected))
}

fn validate_identity(status: &Value) -> Result<()> {
    ensure!(
        status
            .get("pinned_deployment_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty()),
        "pinned deployment absent"
    );
    ensure!(
        status
            .get("pinned_service_protocol_version")
            .and_then(Value::as_u64)
            .is_some_and(|version| (4..=7).contains(&version)),
        "pinned protocol outside journal-v2 scope"
    );
    let modified = status
        .get("modified_at")
        .and_then(Value::as_str)
        .context("invocation modification time absent")?;
    chrono::DateTime::parse_from_rfc3339(modified)?;
    Ok(())
}

#[cfg(test)]
mod tests;
