use crate::{AdapterContext, CrawlResult};
use census_store::Table;
use serde::Serialize;

pub(crate) fn append_once<T: Serialize>(
    ctx: &AdapterContext<'_>,
    phase: &str,
    table: Table,
    row: &T,
) -> CrawlResult<bool> {
    super::budget::check(
        "source effect phase bytes",
        phase.len(),
        census_store::MAX_OPERATION_BYTES,
    )?;
    let identity = (phase, table.file(), row);
    super::budget::preflight(&identity)?;
    let digest = super::apply::digest(&identity)?;
    let operation = super::apply::operation_key(phase, &digest)?;
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(false);
    }
    let mut batch = ctx.write_batch();
    batch.append_many(table, std::slice::from_ref(row))?;
    Ok(batch.commit_once(&operation, &digest)?.written())
}
