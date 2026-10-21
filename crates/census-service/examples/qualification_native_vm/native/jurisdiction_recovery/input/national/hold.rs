use super::super::super::super::http::{query, rows};
use super::super::{safe_id, text};
use anyhow::{ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

mod budget;
mod transition;
mod wait;
use transition::Transition;

pub(super) async fn national(
    client: &Client,
    id: &str,
    key: &str,
    child: &str,
    request: &census_service::restate_services::JurisdictionRequest,
) -> Result<Value> {
    pause(client, id, key, &wait::Expected::national(child, request)?).await
}

pub(in super::super::super) async fn pause_jurisdiction(
    client: &Client,
    id: &str,
    key: &str,
    call: &super::super::super::journal::SourceCall,
) -> Result<Value> {
    pause(client, id, key, &wait::Expected::source(call)?).await
}

pub(in super::super::super) async fn resume_jurisdiction(
    client: &Client,
    id: &str,
    key: &str,
) -> Result<Value> {
    change(client, id, key, "JurisdictionCensus", Transition::Resume).await
}

async fn change(
    client: &Client,
    id: &str,
    key: &str,
    service: &str,
    action: Transition,
) -> Result<Value> {
    tokio::time::timeout(
        Duration::from_secs(6),
        change_observed(client, id, key, service, action),
    )
    .await
    .context("native control six-second deadline expired")?
}

async fn change_observed(
    client: &Client,
    id: &str,
    key: &str,
    service: &str,
    action: Transition,
) -> Result<Value> {
    safe_id(id)?;
    let before = state(client, id).await?;
    target(&before, id, key, service)?;
    ensure!(
        action.accepts(text(&before, "status")?),
        "original invocation is not eligible for bounded control"
    );
    let budget_before = budget::read(client, id).await?;
    ensure!(
        budget_before.get("deployment") == before.get("pinned_deployment_id"),
        "persistent invocation deployment differs"
    );
    let journal_before = wait::journal(client, id).await?;
    wait::complete(&before, &journal_before, id)?;
    let acknowledgement = action.send(client, id).await?;
    let after = settled(client, id, key, service, action).await?;
    let budget_after = budget::read(client, id).await?;
    budget::preserved(&budget_before, &budget_after, action)?;
    let journal_after = wait::journal(client, id).await?;
    wait::prefix(&journal_before, &journal_after)?;
    for field in ["pinned_deployment_id", "pinned_service_protocol_version"] {
        ensure!(
            before.get(field) == after.get(field),
            "native control changed {field}"
        );
    }
    Ok(
        json!({"id":id,"service":service,"before":before,"after":after,
        "action":action.name(),"acknowledgement":acknowledgement,
        "budget_before":budget_before,"budget_after":budget_after,
        "journal_before":journal_before,"journal_after":journal_after,
        "retry_count_semantics":"diagnostic ephemeral invoker start_count; lifetime source budget is qualified separately by the original Store ledger",
        "reason":"control the same original invocation with unchanged persistent retry metadata, original journal prefix and deployment"}),
    )
}

async fn settled(
    client: &Client,
    id: &str,
    key: &str,
    service: &str,
    action: Transition,
) -> Result<Value> {
    let probes = stream::iter(0..120u32)
        .then(|_| async move {
            let row = state(client, id).await?;
            target(&row, id, key, service)?;
            if action.reached(text(&row, "status")?) {
                return Ok(Some(row));
            }
            ensure!(
                text(&row, "status")? != "completed",
                "original completed before native control settled"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<_, anyhow::Error>(None)
        })
        .try_filter_map(|row| futures::future::ready(Ok(row)));
    futures::pin_mut!(probes);
    probes
        .try_next()
        .await?
        .context("original native control did not settle")
}

fn target(row: &Value, id: &str, key: &str, service: &str) -> Result<()> {
    ensure!(
        text(row, "id")? == id
            && text(row, "target_service_name")? == service
            && text(row, "target_service_key")? == key
            && text(row, "target_handler_name")? == "run"
            && !text(row, "pinned_deployment_id")?.is_empty()
            && row.get("restarted_from").is_none_or(Value::is_null)
            && row
                .get("pinned_service_protocol_version")
                .and_then(Value::as_u64)
                == Some(7),
        "original native control identity differs"
    );
    Ok(())
}

pub(in super::super::super) async fn state(client: &Client, id: &str) -> Result<Value> {
    safe_id(id)?;
    let value = query(client, &format!(
        "SELECT id, target_service_name, target_service_key, target_handler_name, status, restarted_from, pinned_deployment_id, pinned_service_protocol_version, retry_count, journal_size, suspended_waiting_future_json FROM sys_invocation WHERE id = '{id}' LIMIT 2"
    )).await?;
    let [row] = rows(&value)? else {
        anyhow::bail!("original native control inventory differs");
    };
    Ok(row.clone())
}

async fn pause(client: &Client, id: &str, key: &str, expected: &wait::Expected) -> Result<Value> {
    let quiescent = wait::suspended(client, id, key, expected).await?;
    let service = text(
        quiescent.get("state").context("quiescent state absent")?,
        "target_service_name",
    )?;
    let mut control = change(client, id, key, service, Transition::Pause).await?;
    let before = control
        .get("before")
        .context("controlled original state absent")?;
    ensure!(
        before == quiescent.get("state").context("quiescent state absent")?,
        "original left exact child wait before pause"
    );
    wait::prefix(
        quiescent
            .get("journal")
            .context("quiescent journal absent")?,
        control
            .get("journal_after")
            .context("paused journal absent")?,
    )?;
    control
        .as_object_mut()
        .context("control proof malformed")?
        .insert("quiescent_child_wait".to_owned(), quiescent);
    Ok(control)
}

pub(in super::super::super) async fn unchanged_budget(
    client: &Client,
    id: &str,
    original: &Value,
) -> Result<Value> {
    let current = budget::read(client, id).await?;
    budget::held(original, &current)?;
    Ok(current)
}
