use crate::recording::RowBatch;
use crate::{CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::{Store, Table};
use serde::Serialize;

mod canonical;
use canonical::CanonicalProjection;

pub(super) const APPLICATION_PHASE: &str = "milesplit_result_set_effects_v1";

pub(super) fn append_new<T: CanonicalProjection>(
    store: &Store,
    batch: &mut RowBatch<'_>,
    table: Table,
    mut rows: Vec<T>,
) -> CrawlResult<usize> {
    rows.iter_mut()
        .for_each(CanonicalProjection::canonicalize_sets);
    let mut outcome = Ok(());
    rows.retain(|row| {
        if outcome.is_err() {
            return false;
        }
        match stage_witness(store, batch, table, row) {
            Ok(unapplied) => unapplied,
            Err(error) => {
                outcome = Err(error);
                false
            }
        }
    });
    outcome?;
    if !rows.is_empty() {
        batch.append_many(table, &rows)?;
    }
    Ok(rows.len())
}

fn stage_witness<T: Serialize>(
    store: &Store,
    batch: &mut RowBatch<'_>,
    table: Table,
    row: &T,
) -> CrawlResult<bool> {
    let digest = serialized_digest(row).map_err(|error| CrawlError::Invariant {
        detail: format!("{} projection digest failed: {error}", table.file()),
    })?;
    let key = format!("{}/{digest}", table.file());
    if store.journal_contains(APPLICATION_PHASE, &key)? {
        return Ok(false);
    }
    batch.journal_done(
        APPLICATION_PHASE,
        &key,
        &serde_json::json!({"table": table.file(), "content_digest": digest}),
    )?;
    Ok(true)
}
