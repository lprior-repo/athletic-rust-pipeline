use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::{Store, StoreBatch, Table};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const PHASE: &str = "milesplit_roster_effects_v1";
pub(super) const MAX_ROWS: usize = 20_000;

pub(super) fn reserve<T>(rows: &mut Vec<T>, count: usize) -> CrawlResult<()> {
    if count > MAX_ROWS {
        return Err(resource(count));
    }
    rows.try_reserve_exact(count).map_err(|_| resource(count))
}

fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "roster effects",
        requested,
        limit: MAX_ROWS,
    }
}

pub(super) fn stage<T: Serialize>(
    store: &Store,
    batch: &mut StoreBatch<'_>,
    table: Table,
    rows: &[T],
    facts: &mut Sha256,
) -> CrawlResult<()> {
    if rows.len() > MAX_ROWS {
        return Err(resource(rows.len()));
    }
    let mut seen = HashSet::new();
    seen.try_reserve(rows.len())
        .map_err(|_| resource(rows.len()))?;
    rows.iter().try_for_each(|row| {
        let digest = serialized_digest(row).map_err(|source| CrawlError::Canonical {
            table: table.file().to_string(),
            source,
        })?;
        facts.update(table.file().as_bytes());
        facts.update([0]);
        facts.update(digest.as_bytes());
        let key = format!("{}/{digest}", table.file());
        if !seen.insert(digest) || store.journal_contains(PHASE, &key)? {
            return Ok(());
        }
        batch.append_many(table, std::slice::from_ref(row))?;
        batch.journal_done(PHASE, &key, &true)?;
        Ok(())
    })
}
