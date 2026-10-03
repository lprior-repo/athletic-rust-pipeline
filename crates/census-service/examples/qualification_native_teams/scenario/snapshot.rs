use anyhow::{ensure, Result};
use reqwest::{Client, Method};
use serde_json::Value;

use super::super::artifacts::write_json;
use super::super::config::Config;
use super::super::http;
use super::Run;

#[tracing::instrument(skip(config, client, invocation))]
pub(super) async fn capture(
    config: &Config,
    client: &Client,
    key: &str,
    id: &str,
    invocation: Value,
) -> Result<Run> {
    let state = http::request(
        client,
        &config.root,
        Method::POST,
        &format!(
            "{}restate/call/JurisdictionCensus/{key}/state",
            config.ingress
        ),
        None,
    )
    .await?;
    ensure!(
        (200..300).contains(&state.status),
        "native shared state inspection failed: {}",
        state.body
    );
    let output = http::request(
        client,
        &config.root,
        Method::GET,
        &format!("{}restate/output/{id}", config.ingress),
        None,
    )
    .await?;
    let (journal, events, admin_state, status) = inspect(config, client, key, id).await?;
    let (source_invocations, sources) = super::source::discover(config, client, key, id).await?;
    write_json(
        &config.root.join(format!("{id}-invocation-status.json")),
        &invocation,
    )?;
    Ok(Run {
        id: id.to_string(),
        state: state.body,
        output,
        journal,
        events,
        admin_state,
        status,
        invocation,
        sources,
        source_invocations,
        physical: Vec::new(),
    })
}

#[tracing::instrument(skip(config, client))]
async fn inspect(
    config: &Config,
    client: &Client,
    key: &str,
    id: &str,
) -> Result<(Value, Value, Value, Value)> {
    let journal = http::query(client, &config.root, &config.admin, &format!("SELECT index, entry_type, completed, entry_json, entry_lite_json FROM sys_journal WHERE id = '{id}' ORDER BY index")).await?;
    let events = http::query(client, &config.root, &config.admin, &format!("SELECT after_journal_entry_index, event_type, event_json, appended_at FROM sys_journal_events WHERE id = '{id}' ORDER BY appended_at")).await?;
    inspect_store(config, client, key, journal, events).await
}

#[tracing::instrument(skip(config, client, journal, events))]
async fn inspect_store(
    config: &Config,
    client: &Client,
    key: &str,
    journal: Value,
    events: Value,
) -> Result<(Value, Value, Value, Value)> {
    let state = http::query(client, &config.root, &config.admin, &format!("SELECT service_key, key, value_utf8 FROM state WHERE service_name = 'JurisdictionCensus' AND service_key = '{key}'")).await?;
    let status = http::request(
        client,
        &config.root,
        Method::POST,
        &format!("{}restate/call/Census/status", config.ingress),
        None,
    )
    .await?;
    ensure!(
        (200..300).contains(&status.status),
        "serving store status inspection failed"
    );
    Ok((journal, events, state, status.body))
}
