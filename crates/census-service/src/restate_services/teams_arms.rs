use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census;
use census_crawl::{net::Fetcher, AdapterReport};
use census_store::Store;

use super::jobs::{adapter_context, collect_error};
mod associations;

pub(super) const TEAMS_ARMS: &[(&str, TeamsArm)] = &[
    (census::SOURCE, TeamsArm::MilesplitIndex),
    ("wiaa", TeamsArm::WiaaDirectory),
    ("mshsl", TeamsArm::MshslSchools),
    ("plain_names", TeamsArm::PlainNamesDirectories),
    ("ihsa", TeamsArm::IhsaSchools),
    ("ks", TeamsArm::KsDirectory),
    ("coach_directories", TeamsArm::CoachDirectories),
    ("arbiter_orgs", TeamsArm::ArbiterOrgs),
    ("ohsaa", TeamsArm::OhsaaSchools),
    ("mpa", TeamsArm::MpaSchools),
    ("riil", TeamsArm::RiilSchools),
    ("pa_piaa", TeamsArm::PiaaSchools),
    ("chsaa", TeamsArm::ChsaaSchools),
    ("tssaa", TeamsArm::TssaaSchools),
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
    OhsaaSchools,
    MpaSchools,
    RiilSchools,
    PiaaSchools,
    ChsaaSchools,
    TssaaSchools,
}

pub(super) fn arm_for(slug: &str) -> Option<TeamsArm> {
    TEAMS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

pub(super) async fn team_source(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    slug: &str,
) -> Result<Option<AdapterReport>, super::JobError> {
    let Some(arm) = arm_for(slug) else {
        super::jobs::require_stage_arm(slug)?;
        return Ok(None);
    };
    let context = || adapter_context(store, fetcher, season, refresh, at, None);
    let report = match arm {
        TeamsArm::MilesplitIndex => walk_milesplit(store, fetcher, jurisdiction, refresh).await,
        TeamsArm::WiaaDirectory => {
            walk_wiaa(store, fetcher, jurisdiction, season, refresh, at).await
        }
        TeamsArm::MshslSchools => {
            walk_mshsl(store, fetcher, jurisdiction, season, refresh, at).await
        }
        TeamsArm::PlainNamesDirectories => {
            walk_plain_names(store, fetcher, jurisdiction, season, refresh, at).await
        }
        TeamsArm::IhsaSchools => walk_ihsa(store, fetcher, jurisdiction, season, refresh, at).await,
        TeamsArm::KsDirectory => walk_ks(store, fetcher, jurisdiction, season, refresh, at).await,
        TeamsArm::CoachDirectories => {
            walk_coach_directories(store, fetcher, jurisdiction, season, refresh, at).await
        }
        TeamsArm::ArbiterOrgs => {
            walk_arbiter_orgs(store, fetcher, jurisdiction, season, refresh, at).await
        }
        TeamsArm::OhsaaSchools => associations::ohsaa(&context(), jurisdiction)
            .await
            .map_err(collect_error),
        TeamsArm::MpaSchools => associations::mpa(&context(), jurisdiction)
            .await
            .map_err(collect_error),
        TeamsArm::RiilSchools => associations::riil(&context()).await.map_err(collect_error),
        TeamsArm::PiaaSchools => associations::piaa(&context(), jurisdiction)
            .await
            .map_err(collect_error),
        TeamsArm::ChsaaSchools => associations::chsaa(&context(), jurisdiction)
            .await
            .map_err(collect_error),
        TeamsArm::TssaaSchools => associations::tssaa(&context(), jurisdiction)
            .await
            .map_err(collect_error),
    }?;
    Ok(Some(report))
}

async fn walk_milesplit(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    refresh: bool,
) -> Result<AdapterReport, super::JobError> {
    let teams = census::collect_state_teams(fetcher, store, jurisdiction, refresh)
        .await
        .map_err(collect_error)?;
    let mut report = AdapterReport::new(census::SOURCE, "teams");
    report.rows = u64::try_from(teams.len()).map_err(|_| super::JobError::Terminal {
        message: format!("milesplit team count {} exceeds u64", teams.len()),
    })?;
    Ok(report)
}

async fn walk_wiaa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_mshsl(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_plain_names(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_ihsa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_ks(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_coach_directories(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
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
        .map_err(collect_error)?;
    Ok(report)
}

async fn walk_arbiter_orgs(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<AdapterReport, super::JobError> {
    let options = census_crawl::arbiter::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
    };
    let context = adapter_context(store, fetcher, season, refresh, at, None);
    let report = census_crawl::arbiter::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}
