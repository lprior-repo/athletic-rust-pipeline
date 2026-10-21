use super::super::super::super::http::{query, rows};
use super::super::{safe_id, text};
use anyhow::{ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

mod transition;
use transition::Transition;

pub(super) async fn national(client: &Client, id: &str, key: &str) -> Result<Value> {
    change(client, id, key, "NationalCensus", Transition::Pause).await
}

pub(in super::super::super) async fn pause_jurisdiction(
    client: &Client,
    id: &str,
    key: &str,
) -> Result<Value> {
    change(client, id, key, "JurisdictionCensus", Transition::Pause).await
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
    safe_id(id)?;
    let before = state(client, id).await?;
    target(&before, id, key, service)?;
    ensure!(
        action.accepts(text(&before, "status")?),
        "original invocation is not eligible for bounded control"
    );
    let acknowledgement = action.send(client, id).await?;
    let after = settled(client, id, key, service, action).await?;
    for field in [
        "pinned_deployment_id",
        "pinned_service_protocol_version",
        "retry_count",
    ] {
        ensure!(
            before.get(field) == after.get(field),
            "native control changed {field}"
        );
    }
    Ok(
        json!({"id":id,"service":service,"before":before,"after":after,
        "action":action.name(),"acknowledgement":acknowledgement,
        "reason":"control the same original invocation without retry, restart or deployment override"}),
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
        "SELECT id, target_service_name, target_service_key, target_handler_name, status, restarted_from, pinned_deployment_id, pinned_service_protocol_version, retry_count FROM sys_invocation WHERE id = '{id}' LIMIT 2"
    )).await?;
    let [row] = rows(&value)? else {
        anyhow::bail!("original native control inventory differs");
    };
    Ok(row.clone())
}
