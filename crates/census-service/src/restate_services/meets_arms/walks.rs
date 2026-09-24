//! The meet-index stage's walks: the state's own results index, and one walk per armed source.
//!
//! Split from [`super`] the way that module was split from `jobs`: the stage decides which walks a
//! plan routes to it and what to do with what they produce, while each walk here owns its adapter's
//! options and the recording it hands back. Both arms take the same seven arguments, so the stage
//! builds one [`Walk`] and asks it to run an arm instead of naming that list at two call sites.

use std::sync::Arc;
use crate::restate_services::job_error;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::HandlerError;

use crate::census::{self, MeetCensus};
use census_crawl::net::Fetcher;
use census_crawl::Recording;
use census_store::Store;

use super::super::jobs::{adapter_context, collect_error, rows_written};
use super::{take_recorded, MeetsArm, RecordedSource};

/// What every walk in this stage needs: this run's store and fetcher, the jurisdiction and season it
/// covers, and the request's refresh flag and observation time.
///
/// Bundled rather than spelled out per arm: both walks take the same seven arguments, and the stage
/// would otherwise repeat that list at each of its two call sites.
#[derive(Clone, Copy)]
pub(super) struct Walk<'a> {
    /// The store this run holds.
    store: &'a Arc<Store>,
    /// The fetcher every walk shares.
    fetcher: &'a Arc<Fetcher>,
    /// The jurisdiction the plan routed to this stage.
    jurisdiction: UsJurisdiction,
    /// The season year the request asked for.
    season: SchoolYear,
    /// Whether the walks may re-read a page the store already holds.
    refresh: bool,
    /// The observation date stamped on what the walks record.
    at: &'a str,
}

impl<'a> Walk<'a> {
    /// Bundle one run's walks: the store and fetcher this run holds, the jurisdiction and season it
    /// covers, and the request's refresh flag and observation time.
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

    /// Run one arm's walk under this stage's recording, and report the rows it wrote.
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

/// The state's own results index: the walk whose counts the census reports as its index fields, and
/// the one source this stage does not look up in the plan — it *is* the plan's first entry.
///
/// Returns the census the stage reports, and the walk's recording when it produced rows to post.
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
    take_recorded(store, census::SOURCE, index.drain(), &mut recorded)?;
    Ok((census, recorded.pop()))
}

/// The association's result archive: its per-season listing is the meet index, so the season the
/// request asked for is the only seed this walk needs.
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

/// The timer's published schedules for the season the request asked for. It serves its own three
/// states from one set of pages, so the walk is the same one for each of them and the store merges
/// the rows it writes by id.
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
