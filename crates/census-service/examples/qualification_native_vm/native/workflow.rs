use super::super::{artifacts, ENDPOINT, GUEST};
use super::clock::clock;
use super::http::{query, request, rows, INGRESS};
use anyhow::{ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

#[tracing::instrument(skip(client))]
pub(super) async fn start(client: &Client, key: &str) -> Result<Value> {
    let path = Path::new(GUEST).join(format!("{key}-invocation.json"));
    ensure!(
        !path.exists(),
        "same workflow must be reattached, not resubmitted"
    );
    let sent = request(
        client,
        Method::POST,
        &format!("{INGRESS}Sweep/{key}/run/send"),
        Some(&json!({"endpoints":[ENDPOINT],"windows":1,"window_seconds":3600})),
    )
    .await?;
    let id = sent
        .get("invocationId")
        .and_then(Value::as_str)
        .context("accepted invocation ID absent")?;
    safe_id(id)?;
    artifacts::publish(
        &path,
        &json!({"id":id,"key":key,"accepted":sent,"starting_clock":clock()?}),
    )?;
    let boundary = sleep_boundary(client, id, key).await?;
    let evidence = json!({"invocation":artifacts::json(&path)?,"boundary":boundary,"unfinished_obligation":"one durable Sweep Sleep window"});
    artifacts::publish(
        &Path::new(GUEST).join(format!("{key}-boundary.json")),
        &evidence,
    )?;
    Ok(evidence)
}

#[tracing::instrument(skip(client))]
async fn sleep_boundary(client: &Client, id: &str, key: &str) -> Result<Value> {
    let checks = stream::iter(0..120)
        .then(|attempt| async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let observation = super::sleep::observe(client, id, key).await?;
            let sampled_clock = clock()?;
            publish_sleep_probe(attempt, id, observation, sampled_clock)
        })
        .try_filter_map(|candidate| futures::future::ready(Ok(candidate)));
    futures::pin_mut!(checks);
    checks
        .try_next()
        .await?
        .context("no independently observed unfinished durable Sleep; injection refused")
}

fn publish_sleep_probe(
    attempt: u32,
    id: &str,
    observation: Value,
    sampled_clock: Value,
) -> Result<Option<Value>> {
    let (proof, parser_error) = match super::sleep::unfinished(&observation, &sampled_clock) {
        Ok(proof) => (proof, None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    let reached = proof.is_some();
    let witness = json!({"attempt":attempt,"observation":observation,"clock":sampled_clock,"proof":proof,"parser_error":parser_error});
    if reached || parser_error.is_some() || attempt == 119 {
        artifacts::publish(
            &Path::new(GUEST).join(format!("{id}-sleep-probe.json")),
            &witness,
        )
        .with_context(|| {
            format!("Sleep witness publication failed; injection refused: {witness}")
        })?;
    }
    if parser_error.is_some() {
        anyhow::bail!("invalid unfinished-Sleep evidence; injection refused: {witness}");
    }
    if attempt == 119 && !reached {
        anyhow::bail!("no unfinished durable Sleep; injection refused: {witness}");
    }
    Ok(reached.then_some(witness))
}

#[tracing::instrument(skip(client))]
pub(super) async fn finish(client: &Client, key: &str) -> Result<Value> {
    let original = artifacts::json(&Path::new(GUEST).join(format!("{key}-invocation.json")))?;
    let id = original
        .get("id")
        .and_then(Value::as_str)
        .context("persisted invocation absent")?;
    safe_id(id)?;
    let before = query(
        client,
        &format!("SELECT id, status FROM sys_invocation WHERE id = '{id}'"),
    )
    .await?;
    ensure!(
        rows(&before)?
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(id)),
        "original invocation did not survive"
    );
    let output = interrupt_existing(client, key, id).await?;
    validate_output(&original, &output)?;
    let journal = query(client, &format!("SELECT index, entry_type, completed, entry_json, entry_lite_json FROM sys_journal WHERE id = '{id}' ORDER BY index")).await?;
    let value = json!({"original":original,"status_before_interrupt":before,"output":output,"journal":journal,"clock":clock()?});
    artifacts::publish(
        &Path::new(GUEST).join(format!("{key}-finished.json")),
        &value,
    )?;
    Ok(value)
}

#[tracing::instrument(skip(client))]
async fn wait_output(client: &Client, id: &str) -> Result<Value> {
    let checks = stream::iter(0..180)
        .then(|attempt| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            match request(
                client,
                Method::GET,
                &format!("{INGRESS}restate/invocation/{id}/output"),
                None,
            )
            .await
            {
                Ok(value) => Ok::<_, anyhow::Error>(Some(value)),
                Err(error) => {
                    tracing::warn!(attempt, %error, "same invocation output pending");
                    Ok(None)
                }
            }
        })
        .try_filter_map(|candidate| futures::future::ready(Ok(candidate)));
    futures::pin_mut!(checks);
    checks
        .try_next()
        .await?
        .context("same invocation did not finish within bounded recovery budget")
}

fn safe_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 256
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "invalid native invocation ID"
    );
    Ok(())
}

#[tracing::instrument(skip(client))]
async fn interrupt_existing(client: &Client, key: &str, id: &str) -> Result<Value> {
    request(
        client,
        Method::POST,
        &format!("{INGRESS}Sweep/{key}/interrupt"),
        Some(&json!(id)),
    )
    .await?;
    wait_output(client, id).await
}

fn validate_output(original: &Value, output: &Value) -> Result<()> {
    let start_date = original
        .get("starting_clock")
        .and_then(|value| value.get("date"))
        .context("starting date absent")?;
    ensure!(
        output.get("today") == Some(start_date),
        "journaled logical date changed across fault: {output}"
    );
    ensure!(
        output.get("interrupted").and_then(Value::as_bool) == Some(true),
        "owned stop signal did not interrupt same invocation"
    );
    Ok(())
}
