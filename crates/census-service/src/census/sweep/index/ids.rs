use census_crawl::milesplit::TeamRef;
use census_crawl::{CrawlError, CrawlResult};
use std::collections::HashSet;

pub(super) const MAX_TEAMS: usize = 20_000;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ID_BYTES: usize = 4096;

pub(super) fn merge(mut prior: Vec<String>, fresh: &[TeamRef]) -> CrawlResult<Vec<String>> {
    bound(prior.len())?;
    bound(fresh.len())?;
    let capacity = prior
        .len()
        .checked_add(fresh.len())
        .ok_or_else(arithmetic)?
        .min(MAX_TEAMS);
    let mut seen = HashSet::new();
    seen.try_reserve(capacity).map_err(|_| resource(capacity))?;
    let mut bytes = prior.iter().try_fold(0usize, |bytes, id| {
        if !seen.insert(id.as_str()) {
            return Err(CrawlError::Invariant {
                detail: format!("duplicate configured roster ID {id}"),
            });
        }
        add(bytes, id)
    })?;
    let mut added = Vec::new();
    added
        .try_reserve_exact(capacity.checked_sub(prior.len()).ok_or_else(arithmetic)?)
        .map_err(|_| resource(capacity))?;
    fresh
        .iter()
        .try_for_each(|team| append(&mut seen, &mut bytes, &mut added, team))?;
    drop(seen);
    prior
        .try_reserve_exact(added.len())
        .map_err(|_| resource(capacity))?;
    prior.extend(added);
    Ok(prior)
}

fn append<'a>(
    seen: &mut HashSet<&'a str>,
    bytes: &mut usize,
    added: &mut Vec<String>,
    team: &'a TeamRef,
) -> CrawlResult<()> {
    if seen.contains(team.id.as_str()) {
        return Ok(());
    }
    bound(seen.len().checked_add(1).ok_or_else(arithmetic)?)?;
    *bytes = add(*bytes, &team.id)?;
    seen.insert(team.id.as_str());
    added.push(copy(&team.id)?);
    Ok(())
}

pub(super) fn validate(ids: &[String]) -> CrawlResult<()> {
    bound(ids.len())?;
    let mut seen = HashSet::new();
    seen.try_reserve(ids.len())
        .map_err(|_| resource(ids.len()))?;
    ids.iter().try_fold(0usize, |bytes, id| {
        if !seen.insert(id.as_str()) {
            return Err(CrawlError::Invariant {
                detail: format!("duplicate configured roster ID {id}"),
            });
        }
        add(bytes, id)
    })?;
    Ok(())
}

pub(super) fn bound(count: usize) -> CrawlResult<()> {
    if count > MAX_TEAMS {
        Err(resource(count))
    } else {
        Ok(())
    }
}
pub(super) fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "configured roster IDs",
        requested,
        limit: MAX_TEAMS,
    }
}

fn add(bytes: usize, id: &str) -> CrawlResult<usize> {
    if id.len() > MAX_ID_BYTES {
        return Err(CrawlError::Resource {
            resource: "configured roster ID bytes",
            requested: id.len(),
            limit: MAX_ID_BYTES,
        });
    }
    let bytes = bytes.checked_add(id.len()).ok_or_else(arithmetic)?;
    if bytes > MAX_BYTES {
        return Err(CrawlError::Resource {
            resource: "configured roster ID payload",
            requested: bytes,
            limit: MAX_BYTES,
        });
    }
    Ok(bytes)
}

fn copy(id: &str) -> CrawlResult<String> {
    let mut value = String::new();
    value
        .try_reserve_exact(id.len())
        .map_err(|_| resource(id.len()))?;
    value.push_str(id);
    Ok(value)
}
fn arithmetic() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "configured roster ID arithmetic overflow".to_string(),
    }
}
