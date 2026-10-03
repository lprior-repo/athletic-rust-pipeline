use super::http::request;
use super::ADMIN;
use anyhow::{ensure, Context, Result};
use reqwest::{Client, Method};
use serde_json::{json, Value};

#[tracing::instrument(skip(client, deployment))]
pub(super) async fn configure(client: &Client, deployment: &Value) -> Result<Value> {
    let id = deployment
        .get("id")
        .and_then(Value::as_str)
        .context("deployment identity absent")?;
    ensure!(
        id.len() <= 256
            && !id.is_empty()
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "unsafe deployment identity"
    );
    let endpoint = measure_endpoint(client, id).await?;
    let sweep = configure_sweep(client, id).await?;
    let teams = configure_teams(client, id).await?;
    Ok(
        json!({"endpoint":endpoint,"sweep":sweep,"teams":teams,"scope":"isolated guest only; unchanged production defaults; configured before first workflow acceptance"}),
    )
}

#[tracing::instrument(skip(client))]
async fn measure_endpoint(client: &Client, id: &str) -> Result<Value> {
    let endpoint = request(
        client,
        Method::GET,
        &format!("{ADMIN}deployments/{id}"),
        None,
    )
    .await?;
    ensure!(
        endpoint.get("protocol_type").and_then(Value::as_str) == Some("BidiStream")
            && endpoint.get("http_version").and_then(Value::as_str) == Some("HTTP/2.0"),
        "guest fault qualification requires measured bidirectional HTTP/2 deployment: {endpoint}"
    );
    Ok(endpoint)
}

#[tracing::instrument(skip(client))]
async fn configure_sweep(client: &Client, id: &str) -> Result<Value> {
    let before = request(client, Method::GET, &format!("{ADMIN}services/Sweep"), None).await?;
    verify_owner(&before, id, "Sweep")?;
    let changed = request(
        client,
        Method::PATCH,
        &format!("{ADMIN}services/Sweep"),
        Some(&json!({"inactivity_timeout":"1s"})),
    )
    .await?;
    let after = request(client, Method::GET, &format!("{ADMIN}services/Sweep"), None).await?;
    verify_owner(&after, id, "Sweep")?;
    ensure!(
        after.get("inactivity_timeout").and_then(Value::as_str) == Some("1s"),
        "guest inactivity override not confirmed: {after}"
    );
    ensure!(
        after.get("abort_timeout") == before.get("abort_timeout")
            && after.get("abort_timeout").is_some(),
        "guest override changed or omitted abort timeout"
    );
    Ok(json!({"before":before,"patch":changed,"after":after}))
}

fn verify_owner(service: &Value, deployment: &str, name: &str) -> Result<()> {
    ensure!(
        service.get("name").and_then(Value::as_str) == Some(name)
            && service.get("deployment_id").and_then(Value::as_str) == Some(deployment),
        "guest configuration changed service/deployment identity: {service}"
    );
    Ok(())
}

#[tracing::instrument(skip(client))]
async fn configure_teams(client: &Client, id: &str) -> Result<Value> {
    let route = format!("{ADMIN}services/TeamsSource");
    let before = request(client, Method::GET, &route, None).await?;
    verify_owner(&before, id, "TeamsSource")?;
    let changed = request(
        client,
        Method::PATCH,
        &route,
        Some(&json!({"journal_retention":"1h"})),
    )
    .await?;
    let after = request(client, Method::GET, &route, None).await?;
    verify_owner(&after, id, "TeamsSource")?;
    ensure!(
        after.get("journal_retention").and_then(Value::as_str) == Some("1h"),
        "guest source journal retention override not confirmed: {after}"
    );
    ensure!(
        after.get("retry_policy") == before.get("retry_policy")
            && after.get("retry_policy").is_some()
            && after.get("idempotency_retention") == before.get("idempotency_retention")
            && after.get("idempotency_retention").is_some(),
        "guest journal override changed or omitted source retry/idempotency policy"
    );
    Ok(json!({"before":before,"patch":changed,"after":after}))
}
