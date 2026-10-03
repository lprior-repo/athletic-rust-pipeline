use super::super::super::{artifacts, GUEST};
use super::super::http::{query, request, rows};
use super::input::{self, Original};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::{JurisdictionState, TeamsSourceInspection};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(super) struct Observation {
    pub(super) before: Value,
    pub(super) parent_journal: Value,
    pub(super) children: Vec<Child>,
    pub(super) parent_state: JurisdictionState,
    pub(super) after: Value,
    pub(super) clock: Value,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Child {
    pub(super) id: String,
    pub(super) key: String,
    pub(super) inspection_before: TeamsSourceInspection,
    pub(super) journal: Value,
    pub(super) inspection_after: TeamsSourceInspection,
}

#[tracing::instrument(skip(client, original))]
pub(super) async fn read(client: &Client, original: &Original) -> Result<Observation> {
    let before = statuses(client, &original.id).await?;
    let (parent_journal, children, parent_state) = details(client, original, &before).await?;
    let after = statuses(client, &original.id).await?;
    Ok(Observation {
        before,
        parent_journal,
        children,
        parent_state,
        after,
        clock: super::super::clock::clock()?,
    })
}

#[tracing::instrument(skip(client, original, before), fields(original_id = %original.id))]
async fn details(
    client: &Client,
    original: &Original,
    before: &Value,
) -> Result<(Value, Vec<Child>, JurisdictionState)> {
    let journal = journal(client, &original.id).await?;
    let children = stream::iter(rows(before)?.iter().filter(|row| {
        input::text(row, "target_service_name").is_ok_and(|name| name == "TeamsSource")
    }))
    .then(|row| child(client, row))
    .try_collect::<Vec<_>>()
    .await?;
    let state = request(
        client,
        Method::POST,
        &input::target("call", "JurisdictionCensus", &original.key, "state")?,
        None,
    )
    .await?;
    Ok((journal, children, serde_json::from_value(state)?))
}

#[tracing::instrument(skip(client))]
async fn child(client: &Client, row: &Value) -> Result<Child> {
    let id = input::text(row, "id")?.to_owned();
    let key = input::text(row, "target_service_key")?.to_owned();
    input::safe_id(&id)?;
    let before = inspection(client, &key).await?;
    let journal = journal(client, &id).await?;
    let after = inspection(client, &key).await?;
    Ok(Child {
        id,
        key,
        inspection_before: before,
        journal,
        inspection_after: after,
    })
}

#[tracing::instrument(skip(client))]
async fn inspection(client: &Client, key: &str) -> Result<TeamsSourceInspection> {
    let value = request(
        client,
        Method::POST,
        &input::target("call", "TeamsSource", key, "inspection")?,
        None,
    )
    .await?;
    serde_json::from_value(value).context("production atomic source inspection malformed")
}

#[tracing::instrument(skip(client))]
async fn journal(client: &Client, id: &str) -> Result<Value> {
    input::safe_id(id)?;
    query(client, &format!("SELECT id, index, version, entry_type, entry_json FROM sys_journal WHERE id = '{id}' ORDER BY index LIMIT 2049")).await
}

#[tracing::instrument(skip(client))]
pub(super) async fn statuses(client: &Client, id: &str) -> Result<Value> {
    input::safe_id(id)?;
    let value = query(client, &format!("SELECT id, target_service_name, target_service_key, target_handler_name, status, invoked_by, invoked_by_id, restarted_from, pinned_deployment_id, pinned_service_protocol_version, journal_size, modified_at, suspended_waiting_future_json, last_awaiting_on_future_json, retry_count, last_start_at, next_retry_at, last_attempt_deployment_id, last_attempt_server, completion_result, completion_failure FROM sys_invocation WHERE id = '{id}' OR (invoked_by_id = '{id}' AND target_service_name = 'TeamsSource' AND target_handler_name = 'run') ORDER BY id LIMIT 18")).await?;
    ensure!(
        rows(&value)?.len() <= 17,
        "source child witness exceeds sixteen-child budget"
    );
    Ok(value)
}

pub(super) fn find_status<'a>(value: &'a Value, id: &str) -> Result<Option<&'a Value>> {
    let mut matches = rows(value)?
        .iter()
        .filter(|row| row.get("id").and_then(Value::as_str) == Some(id));
    let status = matches.next();
    ensure!(
        matches.next().is_none(),
        "original invocation {id} duplicated"
    );
    Ok(status)
}

pub(super) fn status<'a>(value: &'a Value, id: &str) -> Result<&'a Value> {
    find_status(value, id)?.with_context(|| format!("original invocation {id} absent"))
}

pub(super) fn publish(label: &str, value: &impl Serialize) -> Result<()> {
    artifacts::publish(
        &Path::new(GUEST).join(format!("jurisdiction-recovery-{label}.json")),
        value,
    )
}
