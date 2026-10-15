use anyhow::{bail, Context, Result};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

const RESPONSE_BYTES: usize = 4096;
const INVOCATION_ID_BYTES: usize = 128;
const TARGET_BYTES: usize = 4096;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowLookup<'a> {
    target: &'static str,
    workflow_name: &'a str,
    workflow_key: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LookupResponse {
    invocation_id: String,
}

#[tracing::instrument(skip_all)]
pub async fn lookup_workflow(origin: &str, workflow: &str, key: &str) -> Result<String> {
    let origin = super::validated_origin(origin)?;
    if workflow.is_empty()
        || key.is_empty()
        || workflow.len() > TARGET_BYTES
        || key.len() > TARGET_BYTES
    {
        bail!("workflow lookup target is empty or exceeds its byte bound");
    }
    let response = super::http_client(super::READ_TIMEOUT)?
        .post(origin.join("restate/lookup")?)
        .json(&WorkflowLookup {
            target: "workflow",
            workflow_name: workflow,
            workflow_key: key,
        })
        .send()
        .await
        .context("workflow invocation lookup failed")?;
    if response.status() != reqwest::StatusCode::OK {
        bail!(
            "workflow invocation lookup returned HTTP status {}",
            response.status()
        );
    }
    let byte_limit =
        u64::try_from(RESPONSE_BYTES).context("workflow lookup byte limit overflow")?;
    if response
        .content_length()
        .is_some_and(|length| length > byte_limit)
    {
        bail!("workflow lookup response exceeds its byte bound");
    }
    let body = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(
            Vec::with_capacity(RESPONSE_BYTES),
            |mut body, chunk| async move {
                if chunk.len() > RESPONSE_BYTES.saturating_sub(body.len()) {
                    bail!("workflow lookup response exceeds its byte bound");
                }
                body.extend_from_slice(&chunk);
                Ok(body)
            },
        )
        .await?;
    let resolved: LookupResponse = serde_json::from_slice(&body)
        .context("workflow lookup response has an invalid invocation ID shape")?;
    validate_id(&resolved.invocation_id)?;
    Ok(resolved.invocation_id)
}

fn validate_id(id: &str) -> Result<()> {
    let valid = id.strip_prefix("inv_").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
    });
    if !valid || id.len() > INVOCATION_ID_BYTES {
        bail!("workflow lookup returned an invalid invocation ID");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
