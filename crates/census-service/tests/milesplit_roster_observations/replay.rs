use census_crawl::milesplit::TeamRef;
use census_crawl::net::Fetcher;
use census_domain::UsJurisdiction;
use census_service::census::{self, CollectOptions};
use census_store::{Store, Table};
use std::path::Path;

pub(super) fn digest(store: &Store) -> super::TestResult<String> {
    Ok(store.snapshot().tables_digest(&[
        Table::Schools,
        Table::Teams,
        Table::Athletes,
        Table::SourceObservations,
    ])?)
}

pub(super) async fn assert_reopened_replay(
    root: &Path,
    fetcher: &Fetcher,
    team: &TeamRef,
    options: &CollectOptions,
    before: &str,
    synthetic_body: &str,
) -> super::TestResult {
    let store = Store::open(root)?;
    check!(eq; digest(&store)?, before);
    super::projection::assert_published(&store)?;
    super::projection::assert_capture(&store, options, synthetic_body)?;
    let resumed = census::collect_state_rosters(
        fetcher,
        &store,
        std::slice::from_ref(team),
        options,
        UsJurisdiction::Wisconsin,
    )
    .await?;
    check!(eq; resumed.errors, Vec::<String>::new());
    check!(eq; resumed.rosters_committed, 1);
    check!(eq; resumed.rosters_remaining, 0);
    check!(eq; resumed.rosters_skipped, 1);
    check!(eq; digest(&store)?, before);
    super::projection::assert_published(&store)?;
    super::projection::assert_capture(&store, options, synthetic_body)?;
    Ok(())
}
