use super::{artifacts, oracle, GUEST};
use anyhow::{bail, ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

mod clock;
pub(super) mod clock_acquisition;
mod configuration;
mod http;
mod jurisdiction_recovery;
pub(super) mod sleep;
mod workflow;

use clock::{clock, set_clock};
use http::request;
const ADMIN: &str = "http://127.0.0.1:19095/";

#[tracing::instrument]
pub async fn action(action: &str) -> Result<()> {
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(10))
        .build()?;
    let value = match action {
        "ready" => readiness(&client).await?,
        "register" => register(&client).await?,
        "ingest-first" | "repeat-reboot" | "repeat-clock" => ingest(&client, action).await?,
        "sweep-reboot-start" => workflow::start(&client, super::WORKFLOW).await?,
        "sweep-clock-start" => workflow::start(&client, super::CLOCK_WORKFLOW).await?,
        "sweep-reboot-finish" => workflow::finish(&client, super::WORKFLOW).await?,
        "sweep-clock-finish" => workflow::finish(&client, super::CLOCK_WORKFLOW).await?,
        "jurisdiction-reboot-start" => jurisdiction_recovery::start(&client).await?,
        "jurisdiction-reboot-finish" => jurisdiction_recovery::finish(&client).await?,
        "jurisdiction-quiescence" => jurisdiction_recovery::quiescence(&client).await?,
        "clock-acquire-before" => clock_acquisition::acquire("before").await?,
        "clock-acquire-after" => clock_acquisition::acquire("after").await?,
        "clock-set" => set_clock()?,
        "clock" => clock()?,
        "snapshot" => tokio::task::spawn_blocking(|| {
            let mut value = oracle::snapshot::read()?;
            value
                .as_object_mut()
                .context("physical snapshot malformed")?
                .insert(
                    "source_recovery".to_owned(),
                    jurisdiction_recovery::offline()?,
                );
            Ok::<_, anyhow::Error>(value)
        })
        .await
        .context("offline snapshot worker failed")??,
        "drain-certificate" => oracle::snapshot::drain_certificate()?,
        other => bail!("unknown guest action {other}"),
    };
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}

#[tracing::instrument(skip(client))]
async fn readiness(client: &Client) -> Result<Value> {
    let candidates = stream::iter(0..180)
        .then(|attempt| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            match request(client, Method::GET, &format!("{ADMIN}deployments"), None).await {
                Ok(value) => Ok::<_, anyhow::Error>(Some(value)),
                Err(error) => {
                    tracing::warn!(attempt, %error, "native readiness pending");
                    Ok(None)
                }
            }
        })
        .try_filter_map(|candidate| futures::future::ready(Ok(candidate)));
    futures::pin_mut!(candidates);
    let deployments = candidates
        .try_next()
        .await?
        .context("native readiness exhausted")?;
    Ok(
        json!({"deployments":deployments,"clock":clock()?,"manifest":artifacts::json(&Path::new(GUEST).join("manifest.json"))?}),
    )
}

#[tracing::instrument(skip(client))]
async fn register(client: &Client) -> Result<Value> {
    let path = Path::new(GUEST).join("deployment.json");
    ensure!(
        !path.exists(),
        "recovery must not register a new deployment"
    );
    let mut deployment = request(
        client,
        Method::POST,
        &format!("{ADMIN}deployments"),
        Some(&json!({"uri":"http://127.0.0.1:18096/"})),
    )
    .await?;
    ensure!(
        deployment.get("id").and_then(Value::as_str).is_some(),
        "deployment identity absent"
    );
    let configuration = configuration::configure(client, &deployment).await?;
    deployment
        .as_object_mut()
        .context("native deployment is not an object")?
        .insert(
            "qualification_sweep_configuration".to_string(),
            configuration,
        );
    artifacts::publish(&path, &deployment)?;
    Ok(deployment)
}

#[tracing::instrument(skip(client))]
async fn ingest(client: &Client, label: &str) -> Result<Value> {
    let payload = artifacts::json(&Path::new(GUEST).join("request.json"))?;
    let reply = request(
        client,
        Method::POST,
        &format!(
            "{}restate/call/Ingest/{}/record",
            http::INGRESS,
            super::ENDPOINT
        ),
        Some(&payload),
    )
    .await?;
    let expected = if label == "ingest-first" {
        u64::try_from(
            payload
                .get("rows")
                .and_then(Value::as_array)
                .context("request rows absent")?
                .len(),
        )?
    } else {
        0
    };
    ensure!(
        reply.get("appended").and_then(Value::as_u64) == Some(expected),
        "production Ingest physical effect mismatch: {reply}"
    );
    let value = json!({"reply":reply,"payload_digest":oracle::payload_digest(&payload)?,"operation_id":payload.get("operation_id"),"clock":clock()?});
    artifacts::publish(&Path::new(GUEST).join(format!("{label}.json")), &value)?;
    Ok(value)
}
