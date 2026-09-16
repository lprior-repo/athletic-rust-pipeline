use super::{
    export::{Issue, RowResult},
    state::{self, Intent},
};
use crate::restate_types::{RowInput, RowOutput};
use anyhow::{bail, Context, Result};
use futures::TryStreamExt;
use reqwest::{Client, Url};
use std::path::PathBuf;

const BODY_LIMIT: usize = 8 * 1024 * 1024;

async fn call(
    client: &Client,
    mut base: Url,
    key: &str,
    method: &str,
    body: Option<&RowInput>,
    identity: Option<&str>,
) -> Result<Vec<u8>> {
    base.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("invalid ingress URL"))?
        .clear()
        .extend(["restate", "call", "AthleteRow", key, method]);
    let mut request = client.post(base);
    if let Some(body) = body {
        request = request.json(body);
    }
    if let Some(identity) = identity {
        request = request.header("idempotency-key", identity);
    }
    let response = request.send().await.context("Restate transport failure")?;
    let status = response.status();
    let bytes = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, chunk| async move {
            let size = bytes
                .len()
                .checked_add(chunk.len())
                .context("response size overflow")?;
            if size > BODY_LIMIT {
                bail!("Restate response exceeds body limit");
            }
            bytes
                .try_reserve(chunk.len())
                .context("reserving bounded Restate response")?;
            bytes.extend_from_slice(&chunk);
            Ok(bytes)
        })
        .await?;
    if !status.is_success() {
        // Do not copy arbitrary upstream bodies containing private data to diagnostics.
        bail!("Restate returned HTTP {status}");
    }
    Ok(bytes)
}

fn validate(input: &RowInput, output: &RowOutput) -> Result<()> {
    if output.attempt == 0
        || output.record.source_key != input.prospect.source_key
        || serde_json::to_value(&output.record.prospect)? != serde_json::to_value(&input.prospect)?
    {
        bail!("Restate output immutable source metadata mismatch");
    }
    if !matches!(
        output.record.status.as_str(),
        "MATCH"
            | "CLOSE_MATCH"
            | "REVIEW"
            | "NO_MATCH"
            | "INPUT_ERROR"
            | "SEARCH_ERROR"
            | "AI_ERROR"
    ) {
        bail!("unknown Restate row status");
    }
    if output.address.requires_review()
        && matches!(output.record.status.as_str(), "MATCH" | "CLOSE_MATCH")
    {
        bail!("positive attribution contradicts address review requirement");
    }
    Ok(())
}

pub fn final_output(output: &RowOutput) -> bool {
    crate::restate_types::final_record(&output.record)
}

pub async fn execute(client: Client, base: Url, directory: PathBuf, input: RowInput) -> RowResult {
    let key = crate::restate_types::row_key(&input.run_fingerprint, &input.prospect.source_key);
    let mut attempt = 0;
    let result = execute_inner(&client, base, directory, &input, &key, &mut attempt).await;
    match result {
        Ok(output) => RowResult {
            source_key: input.prospect.source_key,
            row_key: key,
            failed: output.as_ref().is_some_and(|value| !final_output(value)),
            output,
            issue: None,
        },
        Err(error) => RowResult {
            source_key: input.prospect.source_key.clone(),
            row_key: key.clone(),
            output: None,
            failed: true,
            issue: Some(Issue {
                source_key: input.prospect.source_key,
                row_key: key,
                stage: "client".to_owned(),
                code: "RESTATE_RECONCILIATION_REQUIRED".to_owned(),
                message: format!("{error:#}"),
                retryable: true,
                attempt,
            }),
        },
    }
}

async fn execute_inner(
    client: &Client,
    base: Url,
    directory: PathBuf,
    input: &RowInput,
    key: &str,
    attempt: &mut u32,
) -> Result<Option<RowOutput>> {
    let previous = state::load_intent(directory.clone(), key.to_owned()).await?;
    if let Some(intent) = &previous {
        *attempt = intent.attempt;
    }
    let bytes = call(client, base.clone(), key, "status", None, None).await?;
    let status: Option<RowOutput> =
        serde_json::from_slice(&bytes).context("decoding shared status")?;
    if let Some(output) = &status {
        validate(input, output)?;
    }
    if let Some(output) = &status {
        let reconciled = previous
            .as_ref()
            .is_some_and(|intent| intent.pending && output.attempt >= intent.attempt);
        if final_output(output)
            || reconciled
            || output
                .issues
                .iter()
                .any(|issue| issue.code == "ROW_ATTEMPTS_EXHAUSTED")
        {
            *attempt = output.attempt;
            state::save_intent(
                directory,
                Intent {
                    row_key: key.to_owned(),
                    attempt: output.attempt,
                    pending: false,
                },
            )
            .await?;
            return Ok(status);
        }
    }
    let next = match &previous {
        Some(intent) if intent.pending => {
            let observed = status.as_ref().map_or(0, |value| value.attempt);
            if observed.checked_add(1) != Some(intent.attempt) {
                bail!("durable status regressed behind pending submission intent");
            }
            intent.attempt
        }
        _ => status
            .as_ref()
            .map_or(0, |value| value.attempt)
            .checked_add(1)
            .context("row attempt exhausted")?,
    };
    if previous
        .as_ref()
        .is_some_and(|intent| !intent.pending && intent.attempt >= next)
    {
        bail!("durable status regressed behind acknowledged invocation");
    }
    *attempt = next;
    state::save_intent(
        directory.clone(),
        Intent {
            row_key: key.to_owned(),
            attempt: next,
            pending: true,
        },
    )
    .await?;
    let identity = format!(
        "restate-v{}-{key}-attempt-{next}",
        crate::restate_types::RESTATE_SCHEMA_VERSION
    );
    let bytes = call(client, base, key, "process", Some(input), Some(&identity)).await?;
    let output: RowOutput = serde_json::from_slice(&bytes).context("decoding process result")?;
    validate(input, &output)?;
    if output.attempt < next {
        bail!("process returned an older attempt");
    }
    state::save_intent(
        directory,
        Intent {
            row_key: key.to_owned(),
            attempt: output.attempt,
            pending: false,
        },
    )
    .await?;
    Ok(Some(output))
}
