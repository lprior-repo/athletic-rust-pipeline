
use crate::restate_services::job_error;
use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::HandlerError;

use crate::census::{self, MeetCensus};
use census_crawl::net::Fetcher;
use census_crawl::Recording;
use census_store::Store;

use super::super::jobs::{adapter_context, collect_error, rows_written};
use super::{take_recorded, MeetsArm, RecordedSource};

#[derive(Clone, Copy)]
pub(super) struct Walk<'a> {
    store: &'a Arc<Store>,
    fetcher: &'a Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &'a str,
}

impl<'a> Walk<'a> {
    pub(super) fn new(
        store: &'a Arc<Store>,
        fetcher: &'a Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        season: SchoolYear,
        refresh: bool,
        at: &'a str,
    ) -> Self {
        Self {
            store,
            fetcher,
            jurisdiction,
            season,
            refresh,
            at,
        }
    }

    pub(super) async fn armed(
        self,
        arm: MeetsArm,
        recording: &Recording,
    ) -> Result<usize, HandlerError> {
        let (store, fetcher) = (self.store, self.fetcher);
        let (jurisdiction, season) = (self.jurisdiction, self.season);
        let (refresh, at) = (self.refresh, self.at);
        match arm {
            MeetsArm::WiaaResults => {
                walk_wiaa_results(
                    store,
                    fetcher,
                    jurisdiction,
                    season,
                    refresh,
                    at,
                    Some(recording),
                )
                .await
            }
            MeetsArm::Wayzata => {
                walk_wayzata(
                    store,
                    fetcher,
                    jurisdiction,
                    season,
                    refresh,
                    at,
                    Some(recording),
                )
                .await
            }
        }
    }
}

pub(super) async fn walk_index(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: &str,
) -> Result<(MeetCensus, Option<RecordedSource>), HandlerError> {
    let index = Recording::new();
    let census = census::collect_state_meets(
        fetcher,
        store,
        jurisdiction,
        year,
        at,
        refresh,
        Some(&index),
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    let mut recorded = Vec::new();
    take_recorded(census::SOURCE, index.drain(), &mut recorded);
    Ok((census, recorded.pop()))
}

async fn walk_wiaa_results(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    recording: Option<&Recording>,
) -> Result<usize, HandlerError> {
    let options = census_crawl::wiaa_results::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        seasons: vec![season.get()],
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, recording);
    let report = census_crawl::wiaa_results::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    rows_written(&report)
}

async fn walk_wayzata(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    recording: Option<&Recording>,
) -> Result<usize, HandlerError> {
    let options = census_crawl::wayzata::Options {
        years: vec![season.get()],
        limit: None,
        refresh,
        observed_on: Some(at.to_string()),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, recording);
    let report = census_crawl::wayzata::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    let _ = jurisdiction;
    rows_written(&report)
}
