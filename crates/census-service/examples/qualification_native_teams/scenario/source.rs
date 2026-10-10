use anyhow::{ensure, Context, Result};
use census_service::restate_services::{
    JurisdictionRequest, JurisdictionState, TeamsAttemptProgress, TeamsFailure, TeamsSourceOutcome,
    TeamsSourceRequest, TeamsStage,
};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde::Serialize;
use serde_json::{json, Value};

use super::super::artifacts::{rows, write_json};
use super::super::config::Config;
use super::super::http::{self, Observation};
use super::super::proxy::RefusalProxy;

#[derive(Serialize)]
pub struct SourceRun {
    pub source: String,
    pub key: String,
    pub id: String,
    pub invocation: Value,
    pub output: Observation,
    pub shared_state: Observation,
    pub outcome: Option<TeamsSourceOutcome>,
    pub journal: Value,
    pub events: Value,
}

#[derive(Serialize)]
pub struct SourceRepeat {
    pub original_id: String,
    pub input: TeamsSourceRequest,
    pub run: SourceRun,
    pub before: Vec<Value>,
    pub after: Vec<Value>,
}

pub fn route(ingress: &str, mode: &str, key: &str, handler: &str) -> Result<String> {
    let mut url = url::Url::parse(ingress)?;
    let suffix = match mode {
        "call" => None,
        "send" => Some("send"),
        mode => anyhow::bail!("unknown invocation mode {mode}"),
    };
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|()| anyhow::anyhow!("ingress URL cannot accept path segments"))?;
        segments
            .pop_if_empty()
            .extend(["TeamsSource", key, handler]);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
    }
    Ok(url.into())
}

#[tracing::instrument(skip(config, client))]
pub(super) async fn discover(
    config: &Config,
    client: &Client,
    parent_key: &str,
    parent_id: &str,
) -> Result<(Value, Vec<SourceRun>)> {
    let query = http::query(client, &config.root, &config.admin, &format!("SELECT id, status, completion_result, completion_failure, target_service_name, target_service_key, target_handler_name, invoked_by, invoked_by_service_name, invoked_by_id, retry_count, last_failure FROM sys_invocation WHERE target_service_name = 'TeamsSource' AND target_handler_name = 'run' AND invoked_by_id = '{parent_id}'")).await?;
    write_json(
        &config
            .root
            .join(format!("{parent_id}-source-invocations.json")),
        &query,
    )?;
    let entries = rows(&query)?;
    ensure!(
        entries.len() <= 8,
        "teams child discovery exceeds armed-source budget"
    );
    let prefix = format!("{parent_key}/teams/");
    let children = stream::iter(entries)
        .then(|row| async {
            let key = row
                .get("target_service_key")
                .and_then(Value::as_str)
                .context("native child target_service_key missing")?;
            let source = key
                .strip_prefix(&prefix)
                .context("child key has another parent")?;
            ensure!(
                !source.is_empty() && !source.contains('/'),
                "invalid source slug in child key"
            );
            ensure!(
                row.get("invoked_by_service_name").and_then(Value::as_str)
                    == Some("JurisdictionCensus"),
                "source invocation has another parent service"
            );
            let id = row
                .get("id")
                .and_then(Value::as_str)
                .context("child invocation ID missing")?;
            super::validate_id(id)?;
            capture(config, client, source, key, id, row.clone()).await
        })
        .try_collect()
        .await?;
    Ok((query, children))
}

#[tracing::instrument(skip(config, client, invocation))]
pub(super) async fn capture(
    config: &Config,
    client: &Client,
    source: &str,
    key: &str,
    id: &str,
    invocation: Value,
) -> Result<SourceRun> {
    let output = http::request(
        client,
        &config.root,
        Method::GET,
        &format!("{}restate/invocation/{id}/output", config.ingress),
        None,
    )
    .await?;
    let shared_state = http::request(
        client,
        &config.root,
        Method::POST,
        &route(&config.ingress, "call", key, "state")?,
        None,
    )
    .await?;
    let (journal, events) = journal(config, client, id).await?;
    write_json(
        &config.root.join(format!("{id}-source-snapshot.json")),
        &json!({"source":source, "key":key, "invocation_id":id, "invocation":invocation, "output":output, "shared_state":shared_state, "journal":journal, "events":events}),
    )?;
    ensure!(
        (200..300).contains(&shared_state.status),
        "source shared state inspection failed: {}",
        shared_state.body
    );
    let outcome = serde_json::from_value::<Option<TeamsSourceOutcome>>(shared_state.body.clone())
        .context("decoding actual typed TeamsSource settled state")?;
    Ok(SourceRun {
        source: source.to_string(),
        key: key.to_string(),
        id: id.to_string(),
        invocation,
        output,
        shared_state,
        outcome,
        journal,
        events,
    })
}

#[tracing::instrument(skip(config, client))]
async fn journal(config: &Config, client: &Client, id: &str) -> Result<(Value, Value)> {
    let journal = http::query(client, &config.root, &config.admin, &format!("SELECT index, entry_type, completed, entry_json, entry_lite_json FROM sys_journal WHERE id = '{id}' ORDER BY index")).await?;
    let events = http::query(client, &config.root, &config.admin, &format!("SELECT after_journal_entry_index, event_type, event_json, appended_at FROM sys_journal_events WHERE id = '{id}' ORDER BY appended_at")).await?;
    Ok((journal, events))
}

pub(super) fn input(body: &Value, state: &Value, source: &str) -> Result<TeamsSourceRequest> {
    let jurisdiction: JurisdictionRequest = serde_json::from_value(body.clone())?;
    let state: JurisdictionState = serde_json::from_value(state.clone())?;
    let at = match state.teams {
        TeamsStage::Failed(TeamsFailure::SourceFailures { at, .. }) => at,
        TeamsStage::Completed(completed) => completed.outcome().at.clone(),
        _ => anyhow::bail!("original parent source observation date unavailable"),
    };
    Ok(TeamsSourceRequest {
        jurisdiction,
        source: source.to_string(),
        observed_on: at,
    })
}

#[tracing::instrument(skip(config, client, proxy, original, input))]
pub(super) async fn repeat(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    original: &SourceRun,
    input: TeamsSourceRequest,
) -> Result<SourceRepeat> {
    let before = proxy.observations()?;
    write_json(
        &config
            .root
            .join(format!("{}-source-repeat-before.json", original.id)),
        &json!(before),
    )?;
    let id = submit(config, client, &original.key, &input).await?;
    let invocation = super::wait_terminal(config, client, &id).await?;
    let row = target(config, client, &original.key, &id).await?;
    write_json(
        &config
            .root
            .join(format!("{id}-source-repeat-terminal.json")),
        &invocation,
    )?;
    let run = capture(config, client, &original.source, &original.key, &id, row).await?;
    let result = SourceRepeat {
        original_id: original.id.clone(),
        input,
        run,
        before,
        after: proxy.observations()?,
    };
    write_json(
        &config.root.join(format!("{id}-source-repeat.json")),
        &serde_json::to_value(&result)?,
    )?;
    Ok(result)
}

#[tracing::instrument(skip(config, client, input))]
pub(super) async fn submit(
    config: &Config,
    client: &Client,
    key: &str,
    input: &TeamsSourceRequest,
) -> Result<String> {
    let accepted = http::request(
        client,
        &config.root,
        Method::POST,
        &route(&config.ingress, "send", key, "run")?,
        Some(&serde_json::to_value(input)?),
    )
    .await?;
    ensure!(
        (200..300).contains(&accepted.status),
        "source submission refused: {}",
        accepted.body
    );
    let id = accepted
        .body
        .get("invocationId")
        .and_then(Value::as_str)
        .context("source native invocation ID missing")?;
    super::validate_id(id)?;
    Ok(id.to_string())
}

#[tracing::instrument(skip(config, client))]
pub(super) async fn target(config: &Config, client: &Client, key: &str, id: &str) -> Result<Value> {
    let target = http::query(client, &config.root, &config.admin, &format!("SELECT id, status, completion_result, completion_failure, target_service_name, target_service_key, target_handler_name, invoked_by, invoked_by_service_name, invoked_by_id, invoked_by_service_name IS NULL AS caller_service_absent, invoked_by_id IS NULL AS caller_id_absent, retry_count, last_failure FROM sys_invocation WHERE id = '{id}'")).await?;
    let row = rows(&target)?.first().context("source invocation absent")?;
    ensure!(
        row.get("target_service_key").and_then(Value::as_str) == Some(key)
            && row.get("target_service_name").and_then(Value::as_str) == Some("TeamsSource")
            && row.get("target_handler_name").and_then(Value::as_str) == Some("run"),
        "encoded route did not target original source object"
    );
    Ok(row.clone())
}

#[tracing::instrument(skip(config, client))]
pub(super) async fn progress(config: &Config, client: &Client, key: &str) -> Result<Observation> {
    let observed = http::request(
        client,
        &config.root,
        Method::POST,
        &route(&config.ingress, "call", key, "progress")?,
        None,
    )
    .await?;
    ensure!(
        (200..300).contains(&observed.status),
        "shared progress refused: {}",
        observed.body
    );
    let _: Vec<TeamsAttemptProgress> = serde_json::from_value(observed.body.clone())?;
    Ok(observed)
}
