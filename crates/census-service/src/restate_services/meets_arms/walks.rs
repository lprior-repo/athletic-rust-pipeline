use super::super::{
    history_stage::HistoricalStageScope,
    jobs::{adapter_context, collect_error, AdapterScope},
};
use super::{report_census, MeetsArm};
use crate::census::{self, MeetCensus, MeetWalkRequest};
use census_crawl::{net::Fetcher, Recording};
use census_store::Store;
use restate_sdk::prelude::HandlerError;
use std::sync::Arc;

pub(super) async fn armed(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
    arm: MeetsArm,
    recording: &Recording,
) -> Result<MeetCensus, HandlerError> {
    let at = scope.observed_on.to_string();
    let adapter = AdapterScope {
        season: super::season_of(scope.year)?,
        refresh: scope.refresh,
        at: &at,
        as_of: scope.window.as_of(),
    };
    let context = adapter_context(store, fetcher, adapter, Some(recording));
    match arm {
        MeetsArm::MilesplitIndex => {
            let options = context.fetch_options();
            let request = MeetWalkRequest::new(scope.jurisdiction, scope.year, &at, &options);
            census::collect_state_meets(fetcher, store, &request, Some(recording))
                .await
                .map_err(|error| super::super::job_error(collect_error(error)))
        }
        MeetsArm::WiaaResults => walk_wiaa(&context, scope).await,
        MeetsArm::Wayzata => walk_wayzata(&context, scope).await,
    }
}

async fn walk_wiaa(
    context: &census_crawl::AdapterContext<'_>,
    scope: HistoricalStageScope,
) -> Result<MeetCensus, HandlerError> {
    let options = census_crawl::wiaa_results::Options {
        limit: None,
        refresh: scope.refresh,
        observed_on: context.observed_on.clone(),
        seasons: vec![source_year(scope.year)?],
        states: vec![scope.jurisdiction],
        school_names: Vec::new(),
    };
    let report = census_crawl::wiaa_results::collect(context, &options)
        .await
        .map_err(|error| super::super::job_error(collect_error(error)))?;
    report_census("wiaa_results", report)
}

async fn walk_wayzata(
    context: &census_crawl::AdapterContext<'_>,
    scope: HistoricalStageScope,
) -> Result<MeetCensus, HandlerError> {
    if !census_crawl::applicability::applicable_sources(scope.jurisdiction)
        .iter()
        .any(|source| source.slug == "wayzata")
    {
        return Ok(super::failed_census(
            "wayzata",
            scope,
            "source geography does not admit this jurisdiction".to_string(),
        ));
    }
    let options = census_crawl::wayzata::Options {
        years: vec![source_year(scope.year)?],
        jurisdictions: vec![scope.jurisdiction],
        limit: None,
        refresh: scope.refresh,
        observed_on: Some(context.observed_on.clone()),
    };
    let report = census_crawl::wayzata::collect(context, &options)
        .await
        .map_err(|error| super::super::job_error(collect_error(error)))?;
    report_census("wayzata", report)
}

fn source_year(calendar: u16) -> Result<i16, HandlerError> {
    i16::try_from(calendar).map_err(|_| super::not_a_season(calendar))
}
