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

pub(super) async fn fragment(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
    step: (MeetsArm, usize),
    recording: &Recording,
) -> Result<(MeetCensus, bool), HandlerError> {
    let at = scope.observed_on.to_string();
    let adapter = AdapterScope {
        season: super::season_of(scope.year)?,
        refresh: scope.refresh,
        at: &at,
        as_of: scope.window.as_of(),
    };
    let context = adapter_context(store, fetcher, adapter, Some(recording));
    match step.0 {
        MeetsArm::AthleticnetHistoricalDiscovery => {
            super::prerequisites::athleticnet().map(|census| (census, false))
        }
        MeetsArm::MilesplitIndex => {
            let options = context.fetch_options();
            let request = MeetWalkRequest::new(scope.jurisdiction, scope.year, &at, &options);
            let census =
                census::collect_state_meet_fragment(fetcher, store, &request, Some(recording))
                    .await
                    .map_err(|error| super::super::job_error(collect_error(error)))?;
            let more = census
                .frontiers
                .iter()
                .any(|frontier| frontier.disposition == census::DiscoveryDisposition::Partial)
                && census
                    .frontiers
                    .iter()
                    .all(|frontier| frontier.failure.is_none())
                && census.pages > 0;
            Ok((census, more))
        }
        MeetsArm::WiaaResults => walk_wiaa(&context, scope, step.1).await,
        MeetsArm::Wayzata => walk_wayzata(&context, scope, step.1).await,
    }
}

pub(super) async fn unarmed(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
    slug: &str,
    recording: &Recording,
) -> Result<(MeetCensus, bool), HandlerError> {
    let Some(source) = census_crawl::registry::descriptor(slug) else {
        return Err(super::super::jobs::invariant(
            "unknown selected meet source",
        ));
    };
    if !required(source) {
        return Ok((MeetCensus::default(), false));
    }
    let at = scope.observed_on.to_string();
    let adapter = AdapterScope {
        season: super::season_of(scope.year)?,
        refresh: scope.refresh,
        at: &at,
        as_of: scope.window.as_of(),
    };
    let context = adapter_context(store, fetcher, adapter, Some(recording));
    super::prerequisites::unarmed(&context, slug, scope.jurisdiction)
        .await
        .map(|census| (census, false))
}

pub(in crate::restate_services) fn required(source: &census_crawl::SourceDescriptor) -> bool {
    source.capabilities.meet_discovery
        || source.capabilities.bulk_results
        || source.capabilities.athlete_discovery
}

async fn walk_wiaa(
    context: &census_crawl::AdapterContext<'_>,
    scope: HistoricalStageScope,
    ordinal: usize,
) -> Result<(MeetCensus, bool), HandlerError> {
    let options = census_crawl::wiaa_results::Options {
        limit: None,
        refresh: scope.refresh,
        observed_on: context.observed_on.clone(),
        seasons: vec![source_year(scope.year)?],
        states: vec![scope.jurisdiction],
        school_names: Vec::new(),
    };
    let (report, more) = census_crawl::wiaa_results::collect_fragment(context, &options, ordinal)
        .await
        .map_err(|error| super::super::job_error(collect_error(error)))?;
    Ok((report_census("wiaa_results", report)?, more))
}

async fn walk_wayzata(
    context: &census_crawl::AdapterContext<'_>,
    scope: HistoricalStageScope,
    ordinal: usize,
) -> Result<(MeetCensus, bool), HandlerError> {
    if !census_crawl::applicability::applicable_sources(scope.jurisdiction)
        .iter()
        .any(|source| source.slug == "wayzata")
    {
        return Ok((
            super::failed_census(
                "wayzata",
                scope,
                "source geography does not admit this jurisdiction".to_string(),
            ),
            false,
        ));
    }
    let options = census_crawl::wayzata::Options {
        years: vec![source_year(scope.year)?],
        jurisdictions: vec![scope.jurisdiction],
        limit: None,
        refresh: scope.refresh,
        observed_on: Some(context.observed_on.clone()),
    };
    let (report, more) = census_crawl::wayzata::collect_fragment(context, &options, ordinal)
        .await
        .map_err(|error| super::super::job_error(collect_error(error)))?;
    Ok((report_census("wayzata", report)?, more))
}

fn source_year(calendar: u16) -> Result<i16, HandlerError> {
    i16::try_from(calendar).map_err(|_| super::not_a_season(calendar))
}
