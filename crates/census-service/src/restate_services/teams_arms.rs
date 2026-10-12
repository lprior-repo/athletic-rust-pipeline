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
    ("ihsa_tournament", TeamsArm::IhsaTournament),
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
    IhsaTournament,
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
        TeamsArm::IhsaTournament => walk_ihsa_tournament(crawl).await,
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
    walk_arbiter_orgs, walk_coach_directories, walk_ihsa, walk_ihsa_tournament, walk_ks,
    walk_milesplit, walk_mshsl, walk_plain_names, walk_wiaa,
};
#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn ihsa_tournament_arm_is_registered_and_sweepable_for_il() {
        let arm = arm_for("ihsa_tournament");
        assert!(
            arm.is_some(),
            "ihsa_tournament must be registered in the durable arm table"
        );
        assert!(
            matches!(arm, Some(TeamsArm::IhsaTournament)),
            "arm_for must return the tournament arm variant"
        );
    }

    #[test]
    fn ihsa_tournament_arm_is_distinct_from_directory_arm() {
        let directory = arm_for("ihsa");
        let tournament = arm_for("ihsa_tournament");
        assert!(directory.is_some(), "ihsa directory arm must be registered");
        assert!(
            tournament.is_some(),
            "ihsa_tournament arm must be registered"
        );
        assert_ne!(
            directory, tournament,
            "directory and tournament arms must be distinct variants"
        );
        assert!(
            matches!(directory, Some(TeamsArm::IhsaSchools)),
            "ihsa slug must map to IhsaSchools"
        );
        assert!(
            matches!(tournament, Some(TeamsArm::IhsaTournament)),
            "ihsa_tournament slug must map to IhsaTournament"
        );
    }

    #[test]
    fn ihsa_tournament_slug_is_sweepable_under_il_plan() {
        assert!(
            TEAMS_ARMS
                .iter()
                .any(|(slug, arm)| *slug == "ihsa_tournament" && *arm == TeamsArm::IhsaTournament),
            "the ihsa_tournament entry must be in the sweepable arms table"
        );
    }

    #[test]
    fn ihsa_tournament_pass_attempts_its_sources_and_records_the_outcome() -> TestResult {
        use census_domain::model::SchoolYear;
        use census_store::Store;
        use std::sync::Arc;

        let dir = tempfile::tempdir()?;
        let store_path = dir.path().join("store");
        let store = Arc::new(Store::open(&store_path)?);

        let cache_path = dir.path().join("http");
        let fetcher = Arc::new(census_crawl::net::Fetcher::new(
            &cache_path,
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )?);

        let spawner = Arc::new(crate::spawn::Spawner::new());
        let runtime = SourceRuntime {
            store: &store,
            fetcher: &fetcher,
            region: &spawner,
        };

        let scope = super::super::jobs::AdapterScope {
            season: SchoolYear::new(2025).ok_or("school year 2025")?,
            refresh: false,
            at: "2026-09-20",
            as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 22).ok_or("as of date")?,
        };

        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(team_source(
                runtime,
                census_domain::UsJurisdiction::Illinois,
                scope,
                "ihsa_tournament",
            ));

        check!(
            result.is_ok(),
            "an unreachable source is a typed outcome, not a job failure"
        );
        let report = result?.ok_or("must produce a report")?;
        check!(eq; report.adapter, "ihsa");
        check!(eq; report.unit, "performances");
        check!(
            report.requests >= 1,
            "the pass must attempt its sources: {report:?}"
        );
        check!(
            report.rows + report.errors > 0,
            "the outcome is recorded either way: {report:?}"
        );
        Ok(())
    }
}
