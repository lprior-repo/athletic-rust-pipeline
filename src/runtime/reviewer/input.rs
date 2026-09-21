use super::model::{build_request, ChatRequest};
use crate::runtime::{
    protocol::{DocumentReceipt, ReviewInput, ReviewJob, MAX_REVIEW_INPUT_BYTES},
    Runtime,
};
use crate::runtime::clock::Clock;
use anyhow::{anyhow, bail, Result};
use std::sync::Arc;

pub struct PreparedReview {
    pub input: ReviewInput,
    pub request: ChatRequest,
    pub endpoint: String,
}

pub async fn prepare(runtime: &Runtime, job: &ReviewJob) -> Result<PreparedReview> {
    let store = runtime.store.clone();
    let digest = job.input.clone();
    let model = runtime.config.model_id(job.lane).to_owned();
    let endpoint = runtime
        .config
        .model_origin(job.lane)
        .join("v1/chat/completions")
        .map_err(|error| anyhow!(error.to_string()))?
        .to_string();
    runtime
        .blocking(move || {
            let bytes = store.get_bytes(&digest)?;
            if bytes.len() > MAX_REVIEW_INPUT_BYTES {
                bail!("review input exceeds 64 KiB");
            }
            let input: ReviewInput = serde_json::from_slice(&bytes)
                .map_err(|error| anyhow!("decoding review input: {error}"))?;
            let request = build_request(&input, &model)?;
            let encoded = serde_json::to_vec(&request)?;
            if encoded.len() > MAX_REVIEW_INPUT_BYTES {
                bail!("review prompt exceeds 64 KiB");
            }
            Ok(PreparedReview {
                input,
                request,
                endpoint,
            })
        })
        .await
}

pub async fn store_request(
    runtime: &Runtime,
    request: &ChatRequest,
) -> Result<crate::domain::identity::EvidenceDigest> {
    let bytes = serde_json::to_vec(request)?;
    let store = runtime.store.clone();
    runtime.blocking(move || Ok(store.put_bytes(&bytes)?)).await
}

/// Build the immutable HTTP receipt for one local model response.
///
/// The timestamp is wall clock, not the monotonic instant used for the elapsed measurement: a
/// receipt is read back by operators and by the export bundle.
pub fn receipt(
    clock: &Arc<dyn Clock>,
    digest: crate::domain::identity::EvidenceDigest,
    source_url: String,
    status: u16,
    media_type: &str,
    bytes: usize,
    elapsed_ms: u64,
) -> Result<DocumentReceipt> {
    let fetched_at_unix_ms = clock
        .now_unix_ms()
        .map_err(|error| anyhow!(error.to_string()))?;
    let bytes = u64::try_from(bytes).map_err(|error| anyhow!(error.to_string()))?;
    Ok(DocumentReceipt {
        digest,
        source_url,
        http_status: status,
        media_type: media_type.to_owned(),
        bytes,
        fetched_at_unix_ms,
        elapsed_ms,
        rankings: None,
    })
}
