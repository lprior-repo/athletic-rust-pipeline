use crate::net::FetchOutcome;
use crate::riil::SchoolExtract;
use crate::{AdapterContext, CrawlResult};
use census_domain::model::SourceObservation;
use census_store::Table;

pub(super) fn persist(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    observations: &[SourceObservation],
    key: &str,
    capture: &FetchOutcome,
    payload: &serde_json::Value,
) -> CrawlResult<bool> {
    if ctx.recording.is_some() {
        let mut batch = ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
        batch.append_many(Table::SourceObservations, observations)?;
        batch.append_many(Table::Coaches, &extract.coaches)?;
        batch.journal_done(super::JOURNAL, key, payload)?;
        batch.commit()?;
        return Ok(true);
    }
    let mut batch = ctx.store.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(Table::SourceObservations, observations)?;
    batch.append_many(Table::Coaches, &extract.coaches)?;
    batch.journal_done(super::JOURNAL, key, payload)?;
    let operation = format!("{}:{key}", super::JOURNAL);
    Ok(batch
        .commit_once(&operation, &capture.content_digest)?
        .written())
}
