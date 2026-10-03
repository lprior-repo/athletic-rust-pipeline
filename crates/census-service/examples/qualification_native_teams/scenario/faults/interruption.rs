use anyhow::{ensure, Result};
use census_service::restate_services::{TeamsAttemptProgress, TeamsSourceRequest};
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};

use super::super::super::artifacts::{append, now, rows, write_json};
use super::super::super::{http, ledger, lifecycle};
use super::{revised, source, Config, OwnedProcess, RefusalProxy, SourceRepeat, SourceRun};

#[derive(Serialize)]
pub struct Interrupted {
    pub input: TeamsSourceRequest,
    pub original_id: String,
    pub key: String,
    pub armed: Value,
    pub progress: http::Observation,
    pub native_at_witness: Value,
    pub physical_at_witness: Vec<Value>,
    pub cleanup: Value,
    pub cold_before: ledger::Snapshot,
    pub release: Value,
    pub restart: Value,
    pub original_terminal: Value,
    pub settlement: SourceRun,
    pub replay: SourceRepeat,
    pub physical_after: Vec<Value>,
}

#[tracing::instrument(skip_all)]
pub(super) async fn run(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    endpoint: &mut OwnedProcess,
    input: &TeamsSourceRequest,
) -> Result<Interrupted> {
    let (key, input, armed, id) = prepare(config, client, proxy, input).await?;
    let progress = witness(config, client, proxy, &key, &id, &armed).await?;
    let native_at_witness = source::target(config, client, &key, &id).await?;
    ensure!(
        !super::terminal(&native_at_witness),
        "native invocation settled before held witness"
    );
    let physical_at_witness = proxy.observations()?;
    let cleanup = lifecycle::stop_endpoint(endpoint)?;
    trace(config, "endpoint_drained_reaped", cleanup.clone())?;
    let cold_before =
        ledger::capture_named(&config.root, &cleanup, "interruption-ledger-before.json")?;
    let release = proxy.release()?;
    *endpoint = config.restart_endpoint()?;
    let restart = json!({"at":now()?, "binary":config.serve, "data_dir":config.root.join("store"),
        "listen":format!("127.0.0.1:{}", config.endpoint_port), "old_owner_reaped":cleanup,
        "native_node_restarted":false, "configuration_changed":false});
    trace(
        config,
        "same_owner_configuration_restarted",
        restart.clone(),
    )?;
    ready(config, client, &key).await?;
    let original_terminal = super::super::wait_terminal(config, client, &id).await?;
    ensure!(
        rows(&original_terminal)?.iter().all(super::terminal),
        "original native execution is not quiescent for same-key reattachment"
    );
    let settlement = attach(config, client, &key, &input).await?;
    let replay = source::repeat(config, client, proxy, &settlement, input.clone()).await?;
    let result = Interrupted {
        input,
        original_id: id,
        key,
        armed,
        progress,
        native_at_witness,
        physical_at_witness,
        cleanup,
        cold_before,
        release,
        restart,
        original_terminal,
        settlement,
        replay,
        physical_after: proxy.observations()?,
    };
    write_json(
        &config.root.join("interrupted-third-witness.json"),
        &serde_json::to_value(&result)?,
    )?;
    Ok(result)
}

async fn prepare(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    input: &TeamsSourceRequest,
) -> Result<(String, TeamsSourceRequest, Value, String)> {
    let quiescent = http::query(client, &config.root, &config.admin,
        "SELECT id, status, target_service_name, target_service_key FROM sys_invocation WHERE target_service_name IN ('JurisdictionCensus', 'TeamsSource') AND target_handler_name = 'run' AND status NOT IN ('completed', 'killed')").await?;
    write_json(
        &config.root.join("interruption-source-quiescence.json"),
        &quiescent,
    )?;
    ensure!(
        rows(&quiescent)?.is_empty(),
        "held boundary requires quiescent parent and other source runs"
    );
    let (key, input) = revised(input, 3);
    let virgin = source::progress(config, client, &key).await?;
    ensure!(
        virgin.body.as_array().is_some_and(Vec::is_empty),
        "interrupted source key is not fresh"
    );
    let armed = proxy.arm_third(&key)?;
    let id = source::submit(config, client, &key, &input).await?;
    trace(
        config,
        "interruption_submitted",
        json!({"key":key, "invocation_id":id}),
    )?;
    Ok((key, input, armed, id))
}

#[tracing::instrument(skip_all)]
async fn witness(
    config: &Config,
    client: &Client,
    proxy: &RefusalProxy,
    key: &str,
    id: &str,
    armed: &Value,
) -> Result<http::Observation> {
    super::super::until(160, || async {
        let observed = source::progress(config, client, key).await?;
        trace(
            config,
            "actual_shared_progress",
            json!({"key":key, "invocation_id":id,
            "inspection":observed}),
        )?;
        let progress: Vec<TeamsAttemptProgress> = serde_json::from_value(observed.body.clone())?;
        let unknown = matches!(
            progress.as_slice(),
            [
                TeamsAttemptProgress::Transient { attempt: 1, .. },
                TeamsAttemptProgress::Transient { attempt: 2, .. },
                TeamsAttemptProgress::Unknown { attempt: 3 },
            ]
        );
        let held = proxy.observations()?.iter().any(|row| {
            row.get("sequence") == armed.get("boundary_sequence")
                && row.get("held").and_then(Value::as_bool) == Some(true)
                && row
                    .get("tls_handshake_record_observed")
                    .and_then(Value::as_bool)
                    == Some(true)
                && row
                    .get("accepted_at")
                    .and_then(Value::as_str)
                    .is_some_and(|at| at <= observed.sent_at.as_str())
        });
        Ok((unknown && held).then_some(observed))
    })
    .await
}

#[tracing::instrument(skip_all)]
async fn ready(config: &Config, client: &Client, key: &str) -> Result<()> {
    super::super::until(120, || async {
        match source::progress(config, client, key).await {
            Ok(observed) => Ok(Some(observed)),
            Err(error) => {
                trace(
                    config,
                    "restart_readiness_error",
                    json!({"error":format!("{error:#}")}),
                )?;
                Ok(None)
            }
        }
    })
    .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn attach(
    config: &Config,
    client: &Client,
    key: &str,
    input: &TeamsSourceRequest,
) -> Result<SourceRun> {
    let id = source::submit(config, client, key, input).await?;
    trace(
        config,
        "same_logical_operation_reattached",
        json!({"key":key, "invocation_id":id}),
    )?;
    super::super::wait_terminal(config, client, &id).await?;
    let row = source::target(config, client, key, &id).await?;
    source::capture(config, client, &input.source, key, &id, row).await
}

fn trace(config: &Config, event: &str, payload: Value) -> Result<()> {
    append(
        &config.root.join("source-fault-events.jsonl"),
        &json!({"event":event, "at":now()?, "payload":payload}),
    )
}
