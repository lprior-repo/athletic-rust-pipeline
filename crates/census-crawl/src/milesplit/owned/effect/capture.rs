use super::{journal, CAPTURE_CHUNK_BYTES, OWNED_CAPTURE_PHASE};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;

#[derive(Serialize)]
struct StableCapture<'a> {
    url: &'a str,
    response_url: Option<&'a str>,
    method: &'a str,
    status: u16,
    content_digest: &'a str,
    bytes: usize,
    fetched_at: &'a str,
    content_type: Option<&'a str>,
}

pub(super) fn acquisition_manifest_key(
    meet_id: u64,
    capture: &FetchOutcome,
) -> CrawlResult<String> {
    let metadata = StableCapture {
        url: &capture.url,
        response_url: capture.response_url.as_deref(),
        method: &capture.method,
        status: capture.status,
        content_digest: &capture.content_digest,
        bytes: capture.bytes,
        fetched_at: &capture.fetched_at,
        content_type: capture.content_type.as_deref(),
    };
    Ok(format!(
        "acquisition/{meet_id}/{}/manifest",
        journal::digest(&metadata)?
    ))
}

pub(super) fn archive(
    ctx: &AdapterContext<'_>,
    key: &str,
    manifest_key: &str,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    let content_manifest = format!("{key}/manifest");
    let new_content = !ctx
        .store
        .journal_contains(OWNED_CAPTURE_PHASE, &content_manifest)?;
    if new_content {
        record_capture(&mut batch, key, capture)?;
    }
    let manifest = serde_json::json!({
        "capture": capture,
        "content_manifest": content_manifest,
        "chunk_key_prefix": key,
        "chunk_bytes": CAPTURE_CHUNK_BYTES,
        "encoding": "base64",
        "chunks": capture.body.chunks(CAPTURE_CHUNK_BYTES).len(),
    });
    let new_acquisition = journal::stage_capture(
        ctx,
        &mut batch,
        OWNED_CAPTURE_PHASE,
        manifest_key,
        &manifest,
    )?;
    if new_content || new_acquisition {
        batch.commit()?;
    }
    Ok(())
}

pub(super) fn record_capture(
    batch: &mut crate::recording::RowBatch<'_>,
    key: &str,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    capture
        .body
        .chunks(CAPTURE_CHUNK_BYTES)
        .enumerate()
        .try_for_each(|(index, bytes)| {
            batch.journal_done(
                OWNED_CAPTURE_PHASE,
                &format!("{key}/{index}"),
                &serde_json::json!({
                    "capture": capture, "chunk_index": index, "raw_base64": STANDARD.encode(bytes),
                }),
            )
        })?;
    batch.journal_done(
        OWNED_CAPTURE_PHASE,
        &format!("{key}/manifest"),
        &serde_json::json!({
            "capture": capture, "chunk_bytes": CAPTURE_CHUNK_BYTES,
            "encoding": "base64",
            "chunks": capture.body.chunks(CAPTURE_CHUNK_BYTES).len(),
        }),
    )
}
