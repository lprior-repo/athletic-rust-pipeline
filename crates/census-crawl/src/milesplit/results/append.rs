use super::{budget, journal_changed, Accumulator, EntityCounts, APPLICATION_PHASE};
use crate::recording::projection::append_new;
use crate::{AdapterContext, CrawlResult};
use census_store::Table;
use std::collections::HashMap;

pub(super) fn window(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    entry: &(String, serde_json::Value),
) -> CrawlResult<EntityCounts> {
    let mut batch = ctx.write_batch();
    let counts = stage(ctx, &mut batch, accumulated)?;
    journal_changed(ctx, &mut batch, &entry.0, &entry.1)?;
    batch.commit()?;
    Ok(counts)
}

fn stage(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    held: Accumulator,
) -> CrawlResult<EntityCounts> {
    let unsupported_cohorts = held
        .retained
        .values()
        .filter(|row| row["cohort"] != "published")
        .count();
    held.retained
        .iter()
        .try_for_each(|(key, value)| journal_changed(ctx, batch, key, value))?;
    append_new(
        ctx,
        batch,
        Table::SourceObservations,
        values(held.observations)?,
        APPLICATION_PHASE,
    )?;
    Ok(EntityCounts {
        meets: append_new(
            ctx,
            batch,
            Table::Meets,
            values(held.meets)?,
            APPLICATION_PHASE,
        )?,
        events: append_new(
            ctx,
            batch,
            Table::Events,
            values(held.events)?,
            APPLICATION_PHASE,
        )?,
        teams: append_new(
            ctx,
            batch,
            Table::Teams,
            values(held.teams)?,
            APPLICATION_PHASE,
        )?,
        athletes: append_new(
            ctx,
            batch,
            Table::Athletes,
            values(held.athletes)?,
            APPLICATION_PHASE,
        )?,
        performances: append_new(
            ctx,
            batch,
            Table::Performances,
            values(held.performances)?,
            APPLICATION_PHASE,
        )?,
        unsupported_cohorts,
    })
}

fn values<T>(held: HashMap<String, T>) -> CrawlResult<Vec<T>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(held.len())
        .map_err(budget::reserve)?;
    rows.extend(held.into_values());
    Ok(rows)
}
