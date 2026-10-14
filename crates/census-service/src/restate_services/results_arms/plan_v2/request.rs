use super::ResultsPlanError;
use crate::restate_services::{history_stage::HistoricalStageScope, jobs};
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::model::SourceMeetRef;
use census_store::Store;
use restate_sdk::prelude::HandlerError;
use std::sync::Arc;

pub(super) fn allocation(url: &str) -> Result<usize, ResultsPlanError> {
    url.len()
        .checked_add(1)
        .and_then(|bytes| bytes.checked_mul(256))
        .and_then(|bytes| bytes.checked_add(32_768))
        .ok_or(ResultsPlanError::Limit("target derivation"))
}

pub(super) fn options(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
    meet: &SourceMeetRef,
) -> Result<FetchOptions, HandlerError> {
    let observed_on = scope.observed_on.to_string();
    let scope = jobs::AdapterScope {
        season: super::super::selection::academic_year(meet)?,
        refresh: scope.refresh,
        at: &observed_on,
        as_of: scope.window.as_of(),
    };
    Ok(jobs::adapter_context(store, fetcher, scope, None).fetch_options())
}
