use super::ResultSetRef;
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::model::serialized_digest;
use serde::Serialize;

const CHUNK_BYTES: usize = 32 * 1024;

pub(super) fn provenance(capture: &FetchOutcome) -> serde_json::Value {
    serde_json::json!({
        "url": capture.url, "response_url": capture.response_url,
        "method": capture.method, "status": capture.status,
        "content_digest": capture.content_digest, "bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "content_type": capture.content_type,
        "request_body": null, "request_headers": [],
    })
}

pub(super) fn digest<T: Serialize>(value: &T) -> CrawlResult<String> {
    serialized_digest(value).map_err(|error| CrawlError::Invariant {
        detail: format!("result projection evidence digest failed: {error}"),
    })
}

pub(super) fn metadata_provenance(
    reference: &ResultSetRef,
    capture: &FetchOutcome,
) -> CrawlResult<serde_json::Value> {
    let mut provenance = provenance(capture);
    let manifest = format!(
        "metadata/{}/manifest",
        digest(&(reference.site.source_id(), &provenance))?
    );
    let fields = provenance
        .as_object_mut()
        .ok_or_else(|| CrawlError::Invariant {
            detail: "metadata capture provenance is not an object".into(),
        })?;
    fields.insert(
        "archive_phase".into(),
        serde_json::json!(crate::milesplit::owned::OWNED_CAPTURE_PHASE),
    );
    fields.insert(
        "archive_manifest".into(),
        serde_json::Value::String(manifest),
    );
    Ok(provenance)
}

pub(super) fn archive_metadata(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    if capture.body.len() > crate::net::MAX_BODY_BYTES {
        return Err(crate::net::FetchError::TooLarge {
            url: capture.url.clone(),
        }
        .into());
    }
    let provenance = provenance(capture);
    let key = format!(
        "metadata/{}",
        digest(&(reference.site.source_id(), &provenance))?
    );
    let phase = crate::milesplit::owned::OWNED_CAPTURE_PHASE;
    if super::super::journal_contains(ctx, phase, &format!("{key}/manifest"))? {
        return Ok(());
    }
    capture
        .body
        .chunks(CHUNK_BYTES)
        .enumerate()
        .try_for_each(|(index, bytes)| {
            archive_chunk(ctx, reference, capture, &key, (index, bytes))
        })?;
    archive_payload(
        ctx,
        &format!("{key}/manifest"),
        &serde_json::json!({
            "capture": provenance, "source_instance": reference.site.source_id(),
            "role": "raw_metadata", "encoding": "base64", "chunk_bytes": CHUNK_BYTES,
            "chunks": capture.body.chunks(CHUNK_BYTES).len(),
        }),
    )
}

fn archive_chunk(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    capture: &FetchOutcome,
    key: &str,
    chunk: (usize, &[u8]),
) -> CrawlResult<()> {
    let key = format!("{key}/{}", chunk.0);
    if super::super::journal_contains(ctx, crate::milesplit::owned::OWNED_CAPTURE_PHASE, &key)? {
        return Ok(());
    }
    if let Some(recording) = ctx.recording {
        recording.admit(CHUNK_BYTES.saturating_mul(4), 1)?;
    }
    archive_payload(
        ctx,
        &key,
        &serde_json::json!({
            "capture": provenance(capture), "source_instance": reference.site.source_id(),
            "role": "raw_metadata", "chunk_index": chunk.0, "raw_base64": STANDARD.encode(chunk.1),
        }),
    )
}

fn archive_payload(
    ctx: &AdapterContext<'_>,
    key: &str,
    payload: &serde_json::Value,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.journal_done(crate::milesplit::owned::OWNED_CAPTURE_PHASE, key, payload)?;
    batch.commit()
}
