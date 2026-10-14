use census_crawl::net::Fetcher;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::SourceAccessCondition;
use census_domain::UsJurisdiction;
use census_store::Store;

use super::{load, teams_phase, Disposition, Receipt};
use crate::census::access::{self, SourceAccessFailure};

#[tracing::instrument(skip_all, fields(state = jurisdiction.code()))]
pub(in crate::census::sweep) async fn retain_failure(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    primary: CrawlError,
) -> SourceAccessFailure {
    match fetcher.access_conditions().await {
        Ok(conditions) => record(store, jurisdiction, primary, &conditions),
        Err(secondary) => SourceAccessFailure::retention(primary, secondary),
    }
}

pub(super) fn record(
    store: &Store,
    jurisdiction: UsJurisdiction,
    primary: CrawlError,
    conditions: &[SourceAccessCondition],
) -> SourceAccessFailure {
    match persist(store, jurisdiction, conditions) {
        Ok(()) => SourceAccessFailure::Primary(primary),
        Err(secondary) => SourceAccessFailure::retention(primary, secondary),
    }
}

fn persist(
    store: &Store,
    jurisdiction: UsJurisdiction,
    conditions: &[SourceAccessCondition],
) -> CrawlResult<()> {
    let mut receipt =
        load(store, jurisdiction)?.map_or(Receipt::default(), core::convert::identity);
    receipt.disposition = Disposition::Partial;
    receipt.errors = receipt
        .errors
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "team index error count overflow".to_string(),
        })?;
    let mut batch = store.write_batch();
    access::stage(&mut batch, conditions)?;
    batch.journal_done(&teams_phase(jurisdiction), jurisdiction.code(), &receipt)?;
    batch.commit()?;
    Ok(())
}
