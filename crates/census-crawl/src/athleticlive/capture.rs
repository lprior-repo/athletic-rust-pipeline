use crate::net::cache::{write_archive, CacheMeta};
use crate::{AdapterContext, CrawlError, CrawlResult};
use std::io::Read;
use std::path::PathBuf;

pub(super) struct Captured {
    pub(super) path: PathBuf,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn freeze(
    ctx: &AdapterContext<'_>,
    input: &str,
    metadata: Option<&CacheMeta>,
    limit: usize,
) -> CrawlResult<Captured> {
    if input.len() > 4096 {
        return Err(resource("LIVE input path bytes", input.len(), 4096));
    }
    let meta = metadata.ok_or_else(|| schema(input, "capture has no producer metadata"))?;
    validate(input, meta, limit)?;
    let bytes = read_bytes(input, limit)?;
    let key = crate::net::Fetcher::key_for(&meta.method, &meta.url, "");
    let (body_path, meta_path) = ctx.fetcher.cache_paths(&key);
    let path = write_archive(&body_path, &meta_path, &bytes, meta)?;
    Ok(Captured { path, bytes })
}

fn validate(input: &str, meta: &CacheMeta, limit: usize) -> CrawlResult<()> {
    let fields = [
        &meta.url,
        &meta.method,
        &meta.content_digest,
        &meta.fetched_at,
    ];
    if fields.iter().any(|field| field.len() > 4096) {
        return Err(resource("LIVE metadata field bytes", 4097, 4096));
    }
    if meta.bytes > limit {
        return Err(resource("LIVE capture bytes", meta.bytes, limit));
    }
    if meta.method != "GET"
        || meta.status != 200
        || chrono::DateTime::parse_from_rfc3339(&meta.fetched_at).is_err()
    {
        return Err(schema(
            input,
            "capture has no successful GET and physical acquisition timestamp",
        ));
    }
    let url = url::Url::parse(&meta.url)
        .map_err(|_| schema(input, "capture has no absolute producer URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(schema(input, "capture producer is not a public HTTP URL"));
    }
    Ok(())
}

fn read_bytes(input: &str, limit: usize) -> CrawlResult<Vec<u8>> {
    let mut file = std::fs::File::open(input).map_err(|source| CrawlError::Io {
        path: PathBuf::from(input),
        source,
    })?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    let steps = limit
        .checked_div(chunk.len())
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| resource("LIVE read-step arithmetic", limit, limit))?;
    (0..steps)
        .find_map(|_| match file.read(&mut chunk) {
            Ok(0) => Some(Ok(())),
            Ok(count) => append(&mut bytes, &chunk, count, limit).err().map(Err),
            Err(source) => Some(Err(CrawlError::Io {
                path: PathBuf::from(input),
                source,
            })),
        })
        .ok_or_else(|| resource("LIVE read steps", steps, steps))??;
    Ok(bytes)
}

fn append(bytes: &mut Vec<u8>, chunk: &[u8], count: usize, limit: usize) -> CrawlResult<()> {
    let requested = bytes
        .len()
        .checked_add(count)
        .ok_or_else(|| resource("LIVE byte arithmetic", count, limit))?;
    if requested > limit {
        return Err(resource("LIVE capture bytes", requested, limit));
    }
    let chunk = chunk
        .get(..count)
        .ok_or_else(|| resource("LIVE read length", count, limit))?;
    bytes
        .try_reserve(count)
        .map_err(|_| resource("LIVE capture allocation", requested, limit))?;
    bytes.extend_from_slice(chunk);
    Ok(())
}

fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

fn schema(input: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: input.to_string(),
        detail: detail.to_string(),
    }
}
