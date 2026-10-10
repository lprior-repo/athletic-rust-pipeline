use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlError, CrawlResult};

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

pub(super) fn read(
    ctx: &AdapterContext<'_>,
    path: &str,
    metadata: Option<&CacheMeta>,
) -> CrawlResult<String> {
    let captured =
        match crate::athleticlive::capture::freeze(ctx, path, metadata, MAX_CAPTURE_BYTES) {
            Ok(captured) => {
                return String::from_utf8(captured.bytes)
                    .map_err(|error| schema(path, &error.to_string()))
            }
            Err(CrawlError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                archived(ctx, path, metadata)?
            }
            Err(error) => return Err(error),
        };
    Ok(captured)
}

pub(super) fn require_url(meta: &CacheMeta, expected: &str) -> CrawlResult<()> {
    if meta.url != expected {
        return Err(CrawlError::Schema {
            url: meta.url.clone(),
            detail: format!("capture producer differs from owning URL {expected}"),
        });
    }
    Ok(())
}
fn archived(
    ctx: &AdapterContext<'_>,
    path: &str,
    metadata: Option<&CacheMeta>,
) -> CrawlResult<String> {
    let meta = metadata
        .ok_or_else(|| schema(path, "archive replay requires physical producer metadata"))?;
    let payload = ctx
        .store
        .journal_payload(super::super::CAPTURE_PHASE, &meta.content_digest)?
        .ok_or_else(|| {
            schema(
                path,
                "operator capture and immutable archived body are missing",
            )
        })?;
    let body = payload
        .get("body")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| schema(path, "immutable archive has no body"))?;
    if body.len() > MAX_CAPTURE_BYTES
        || body.len() != meta.bytes
        || payload.get("bytes").and_then(serde_json::Value::as_u64)
            != u64::try_from(body.len()).ok()
        || crate::net::cache::content_digest(body.as_bytes()) != meta.content_digest
    {
        return Err(schema(
            path,
            "immutable archived body disagrees with physical capture metadata",
        ));
    }
    Ok(body.to_string())
}

fn schema(path: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: path.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod capture_tests;
