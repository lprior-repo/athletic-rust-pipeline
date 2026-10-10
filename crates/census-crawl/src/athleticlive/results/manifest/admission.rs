use super::{CaptureEntry, CrawlError, CrawlResult};
use std::io::Read;

pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;
pub(super) const MAX_RECORDS: usize = 8192;

pub(super) fn read(path: &str) -> CrawlResult<String> {
    let file = std::fs::File::open(path).map_err(|source| io(path, source))?;
    let size = usize::try_from(file.metadata().map_err(|source| io(path, source))?.len())
        .map_err(|_| resource("LIVE manifest bytes", usize::MAX, MAX_BYTES))?;
    limit("LIVE manifest bytes", size, MAX_BYTES)?;
    let mut body = String::new();
    body.try_reserve_exact(size)
        .map_err(|_| resource("LIVE manifest allocation", size, MAX_BYTES))?;
    file.take(
        u64::try_from(MAX_BYTES.saturating_add(1))
            .map_err(|_| resource("LIVE manifest bytes", usize::MAX, MAX_BYTES))?,
    )
    .read_to_string(&mut body)
    .map_err(|source| io(path, source))?;
    limit("LIVE manifest bytes", body.len(), MAX_BYTES)?;
    Ok(body)
}

pub(super) fn admit(entry: &CaptureEntry, records: usize) -> CrawlResult<usize> {
    owner(entry)?;
    let added = entry
        .documents
        .len()
        .checked_add(entry.standings.len())
        .and_then(|value| value.checked_add(entry.captures.len()))
        .and_then(|value| value.checked_add(usize::from(entry.summary.is_some())))
        .ok_or_else(|| resource("LIVE manifest records", usize::MAX, MAX_RECORDS))?;
    let total = records
        .checked_add(added)
        .ok_or_else(|| resource("LIVE manifest records", usize::MAX, MAX_RECORDS))?;
    limit("LIVE manifest records", total, MAX_RECORDS)?;
    Ok(total)
}

fn owner(entry: &CaptureEntry) -> CrawlResult<()> {
    if entry.athleticlive_meet_id == 0
        || entry.tenant.is_empty()
        || entry.tenant.len() > 128
        || !entry
            .tenant
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(CrawlError::Invariant {
            detail: "manifest meet has no valid published provider owner".into(),
        });
    }
    limit("LIVE manifest meet label", entry.name.len(), 4096)?;
    limit("LIVE manifest published date", entry.date.len(), 64)?;
    entry
        .documents
        .iter()
        .chain(entry.summary.iter())
        .chain(entry.standings.iter().map(|row| &row.path))
        .try_for_each(|path| limit("LIVE manifest capture path", path.len(), 4096))
}

pub(super) fn limit(name: &'static str, requested: usize, maximum: usize) -> CrawlResult<()> {
    if requested > maximum {
        return Err(resource(name, requested, maximum));
    }
    Ok(())
}

pub(super) fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}

fn io(path: &str, source: std::io::Error) -> CrawlError {
    CrawlError::Io {
        path: path.into(),
        source,
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod admission_tests;
