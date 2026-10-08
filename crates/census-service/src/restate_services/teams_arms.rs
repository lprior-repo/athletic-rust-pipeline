use std::sync::Arc;

use census_domain::UsJurisdiction;

use crate::census;
use census_crawl::{net::Fetcher, AdapterReport, CrawlResult};
use census_store::Store;

use super::jobs::{adapter_context, collect_error, AdapterScope};
mod associations;
mod known_sites;

pub(super) struct SourceRuntime<'a> {
    pub(super) store: &'a Arc<Store>,
    pub(super) fetcher: &'a Arc<Fetcher>,
    pub(super) region: &'a Arc<crate::spawn::Spawner>,
}
use associations as assoc;

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
    ("ciac", TeamsArm::CiacSchools),
    ("aia", TeamsArm::AiaProfiles),
    ("home_campus", TeamsArm::HomeCampusSections),
    ("uhsaa", TeamsArm::UhsaaDirectory),
    ("pa_piaa", TeamsArm::PiaaSchools),
    ("chsaa", TeamsArm::ChsaaSchools),
    ("tssaa", TeamsArm::TssaaSchools),
    ("bound", TeamsArm::Bound),
    ("sidearm_staff", TeamsArm::DiscoveredStaff),
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
    CiacSchools,
    AiaProfiles,
    HomeCampusSections,
    UhsaaDirectory,
    PiaaSchools,
    ChsaaSchools,
    TssaaSchools,
    Bound,
    DiscoveredStaff,
}

pub(super) fn arm_for(slug: &str) -> Option<TeamsArm> {
    TEAMS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

pub(super) async fn team_source(
    runtime: SourceRuntime<'_>,
    jurisdiction: UsJurisdiction,
    scope: AdapterScope<'_>,
    slug: &str,
) -> Result<Option<AdapterReport>, super::JobError> {
    let Some(arm) = arm_for(slug) else {
        super::jobs::require_stage_arm(slug)?;
        return Ok(None);
    };
    if arm == TeamsArm::DiscoveredStaff {
        return known_sites::collect(runtime, jurisdiction, scope)
            .await
            .map(Some);
    }
    let SourceRuntime { store, fetcher, .. } = runtime;
    let crawl = walkers::crawl(store, fetcher, jurisdiction, scope);
    let ctx = adapter_context(store, fetcher, scope, None);
    Ok(Some(dispatch(arm, crawl, &ctx).await?))
}

async fn dispatch(
    arm: TeamsArm,
    crawl: walkers::Crawl<'_>,
    ctx: &census_crawl::AdapterContext<'_>,
) -> Result<AdapterReport, super::JobError> {
    let jurisdiction = crawl.jurisdiction;
    match arm {
        TeamsArm::MilesplitIndex => walk_milesplit(crawl).await,
        TeamsArm::WiaaDirectory => walk_wiaa(crawl).await,
        TeamsArm::MshslSchools => walk_mshsl(crawl).await,
        TeamsArm::PlainNamesDirectories => walk_plain_names(crawl).await,
        TeamsArm::IhsaSchools => walk_ihsa(crawl).await,
        TeamsArm::KsDirectory => walk_ks(crawl).await,
        TeamsArm::CoachDirectories => walk_coach_directories(crawl).await,
        TeamsArm::ArbiterOrgs => walk_arbiter_orgs(crawl).await,
        TeamsArm::OhsaaSchools => as_job(assoc::ohsaa(ctx, jurisdiction).await),
        TeamsArm::MpaSchools => as_job(assoc::mpa(ctx, jurisdiction).await),
        TeamsArm::RiilSchools => as_job(assoc::riil(ctx).await),
        TeamsArm::CiacSchools => as_job(assoc::ciac(ctx, jurisdiction).await),
        TeamsArm::AiaProfiles => as_job(assoc::aia(ctx, jurisdiction).await),
        TeamsArm::HomeCampusSections => as_job(assoc::home_campus(ctx, jurisdiction).await),
        TeamsArm::UhsaaDirectory => as_job(assoc::uhsaa(ctx, jurisdiction).await),
        TeamsArm::PiaaSchools => as_job(assoc::piaa(ctx, jurisdiction).await),
        TeamsArm::ChsaaSchools => as_job(assoc::chsaa(ctx, jurisdiction).await),
        TeamsArm::TssaaSchools => as_job(assoc::tssaa(ctx, jurisdiction).await),
        TeamsArm::Bound => as_job(assoc::bound(ctx, jurisdiction).await),
        TeamsArm::DiscoveredStaff => Err(super::JobError::Terminal {
            message: "discovered staff dispatch requires its owning source runtime".to_string(),
        }),
    }
}

fn as_job(result: CrawlResult<AdapterReport>) -> Result<AdapterReport, super::JobError> {
    result.map_err(collect_error)
}

mod walkers;
use walkers::{
    walk_arbiter_orgs, walk_coach_directories, walk_ihsa, walk_ks, walk_milesplit, walk_mshsl,
    walk_plain_names, walk_wiaa,
};
