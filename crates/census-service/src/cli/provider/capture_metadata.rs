use anyhow::{bail, Context, Result};
use census_crawl::net::cache::CacheMeta;
use std::io::Read;

const MAX_METADATA_BYTES: usize = 65_536;

pub(super) fn read(path: &str) -> Result<CacheMeta> {
    if path.len() > 4096 {
        bail!("producer metadata path exceeds 4096 bytes");
    }
    let file =
        std::fs::File::open(path).with_context(|| format!("opening producer metadata {path}"))?;
    let requested = MAX_METADATA_BYTES
        .checked_add(1)
        .context("metadata capacity arithmetic")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(requested)
        .context("allocating bounded producer metadata")?;
    let limit = u64::try_from(requested).context("metadata read capacity conversion")?;
    file.take(limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading producer metadata {path}"))?;
    if bytes.len() > MAX_METADATA_BYTES {
        bail!("producer metadata exceeds {MAX_METADATA_BYTES} bytes");
    }
    serde_json::from_slice(&bytes).with_context(|| format!("decoding producer metadata {path}"))
}
