use crate::restate_services::job_error;
use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};

use crate::census;
use census_crawl::{net::Fetcher, AdapterReport};
use census_store::Store;

use super::jobs::{adapter_context, assert_some_stage_arms, collect_error, rows_written};
use super::wire::StageOutcome;

pub(super) const TEAMS_ARMS: &[(&str, TeamsArm)] = &[
    (census::SOURCE, TeamsArm::MilesplitIndex),
    ("wiaa", TeamsArm::WiaaDirectory),
    ("mshsl", TeamsArm::MshslSchools),
    ("plain_names", TeamsArm::PlainNamesDirectories),
    ("ihsa", TeamsArm::IhsaSchools),
    ("ks", TeamsArm::KsDirectory),
    ("coach_directories", TeamsArm::CoachDirectories),
    ("arbiter_orgs", TeamsArm::ArbiterOrgs),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TeamsArm {
    MilesplitIndex,
    WiaaDirectory,
    MshslSchools,
    PlainNamesDirectories,
    IhsaSchools,
    KsDirectory,
    CoachDirectories,
    ArbiterOrgs,
}

pub(super) fn arm_for(slug: &str) -> Option<TeamsArm> {
    TEAMS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

async fn sweep_team_source(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    slug: &str,
) -> Result<Option<AdapterReport>, HandlerError> {
    let Some(arm) = arm_for(slug) else {
        assert_some_stage_arms(slug)?;
        return Ok(None);
    };
    match arm {
        TeamsArm::MilesplitIndex => {
            let teams = census::collect_state_teams(fetcher, store, jurisdiction, refresh)
                .await
                .map_err(|error| job_error(collect_error(error)))?;
            let mut report = AdapterReport::new(census::SOURCE, "teams");
            report.rows = u64::try_from(teams.len()).map_err(|_| {
                TerminalError::new(format!("milesplit team count {} exceeds u64", teams.len()))
            })?;
            Ok(Some(report))
        }
        TeamsArm::WiaaDirectory => Ok(Some(
            walk_wiaa(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::MshslSchools => Ok(Some(
            walk_mshsl(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::PlainNamesDirectories => Ok(Some(
            walk_plain_names(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::IhsaSchools => Ok(Some(
            walk_ihsa(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::KsDirectory => Ok(Some(
            walk_ks(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::CoachDirectories => Ok(Some(
            walk_coach_directories(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
        TeamsArm::ArbiterOrgs => Ok(Some(
            walk_arbiter_orgs(store, fetcher, jurisdiction, season, refresh, at).await?,
        )),
    }
}

async fn walk_wiaa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::wiaa::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::wiaa::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_mshsl(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::mshsl::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::mshsl::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_plain_names(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::plain_names::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::plain_names::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_ihsa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::ihsa::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::ihsa::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_ks(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::ks::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::ks::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_coach_directories(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::coach_directories::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::coach_directories::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

async fn walk_arbiter_orgs(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, HandlerError> {
    let options = census_crawl::arbiter::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::arbiter::collect(&context, &options)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(report)
}

pub(super) async fn teams_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: String,
    sweepable: Vec<String>,
) -> Result<Json<StageOutcome>, HandlerError> {
    let mut records: usize = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for slug in &sweepable {
        let Some(report) =
            sweep_team_source(&store, &fetcher, jurisdiction, season, refresh, &at, slug).await?
        else {
            continue;
        };
        let written = rows_written(&report)?;
        records = records.saturating_add(written);
        if report.errors > 0 {
            errors.push(format!("{}: {} errors", slug, report.errors));
        }
        notes.extend(report.notes.iter().map(|n| format!("{}: {}", slug, n)));
    }
    Ok(Json(StageOutcome {
        records,
        at,
        errors,
        notes,
    }))
}
