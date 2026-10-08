use crate::recording::RowBatch;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::Table;
use serde::Serialize;

mod canonical;
use canonical::CanonicalProjection;

pub(crate) fn append_new<T: CanonicalProjection>(
    ctx: &AdapterContext<'_>,
    batch: &mut RowBatch<'_>,
    table: Table,
    mut rows: Vec<T>,
    phase: &str,
) -> CrawlResult<usize> {
    super::budget::check(
        "source projection phase bytes",
        phase.len(),
        census_store::MAX_JOURNAL_KEY_BYTES,
    )?;
    super::budget::check(
        "source projection rows",
        rows.len(),
        super::MAX_RECORDED_WORK,
    )?;
    rows.iter()
        .try_for_each(|row| super::budget::preflight(row).map(|_| ()))?;
    rows.iter_mut()
        .for_each(CanonicalProjection::canonicalize_sets);
    let mut admitted = Vec::new();
    admitted.try_reserve_exact(rows.len()).map_err(|_| {
        super::budget::resource(
            "source projection allocation",
            rows.len(),
            super::MAX_RECORDED_WORK,
        )
    })?;
    let rows = rows.into_iter().try_fold(admitted, |mut held, row| {
        if stage_witness(ctx, batch, table, &row, phase)? {
            held.push(row);
        }
        Ok::<_, CrawlError>(held)
    })?;
    if !rows.is_empty() {
        batch.append_many(table, &rows)?;
    }
    Ok(rows.len())
}

fn stage_witness<T: Serialize>(
    ctx: &AdapterContext<'_>,
    batch: &mut RowBatch<'_>,
    table: Table,
    row: &T,
    phase: &str,
) -> CrawlResult<bool> {
    let digest = serialized_digest(row).map_err(|error| CrawlError::Invariant {
        detail: format!("{} projection digest failed: {error}", table.file()),
    })?;
    let key = format!("{}/{digest}", table.file());
    if batch.journal_is_staged(phase, &key) {
        return Ok(false);
    }
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(phase, &key, |prior| Ok(prior.is_some()))? {
            return Ok(false);
        }
    }
    if ctx.store.journal_contains(phase, &key)? {
        return Ok(false);
    }
    batch.journal_done(
        phase,
        &key,
        &serde_json::json!({"table": table.file(), "content_digest": digest}),
    )?;
    Ok(true)
}
