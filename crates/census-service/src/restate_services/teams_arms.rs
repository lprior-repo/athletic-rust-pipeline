//! The team-index stage's arms: the walk the stage runs for each planned source.
//!
//! Split from [`super::jobs`] because the stage's own wrappers are one concern and the walks they
//! dispatch are another: what lives here is the arm table, one arm per source, and the
//! [`AdapterContext`] each association walk runs under. The stage itself stays in `jobs`, and it
//! re-exports [`teams_stage`] so its caller names one module.
//!
//! Every arm is a walk that publishes a school or team universe and needs no seed from another
//! source. A source whose walk needs one — a name list, a meet id, an athlete profile — is refused
//! by the plan instead of being run here, because a stage that invented its own seed would be a
//! second opinion about what the run covers. The registered sources this stage leaves owed are the
//! ones whose first input only an operator can supply: `ohsaa` the name of a school to search for,
//! `coach_contacts` a dataset path, `tfrrs` a performance list and a roster URL, and
//! `athleticlive` with `athleticlive_athletes` a harvest and the meet ids its athlete index is
//! keyed on.

use crate::restate_services::job_error;
use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json};

use crate::census;
use census_crawl::net::Fetcher;
use census_store::Store;

use super::jobs::{adapter_context, assert_some_stage_arms, collect_error, rows_written};
use super::wire::StageOutcome;

/// The walks the team-index stage can run, one per planned source.
///
/// The plan asks [`super::jurisdiction::DISPATCHED`] whether any stage sweeps a source; this table
/// is what the teams stage answers that claim with. The slugs are the *registry's* spellings — what
/// a plan and a refusal carry — not the evidence tag each walk stamps (`wiaa_directory`,
/// `mshsl`), and `tests::the_arms_are_the_dispatched_slugs` holds the two lists to each other so a
/// slug planned without an arm fails a test instead of failing a run.
pub(super) const TEAMS_ARMS: &[(&str, TeamsArm)] = &[
    (census::SOURCE, TeamsArm::MilesplitIndex),
    ("wiaa", TeamsArm::WiaaDirectory),
    ("mshsl", TeamsArm::MshslSchools),
    ("plain_names", TeamsArm::PlainNamesDirectories),
    ("ihsa", TeamsArm::IhsaSchools),
    ("ks", TeamsArm::KsDirectory),
];

/// One arm per walk: what the stage runs for a planned source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TeamsArm {
    /// The state team index the roster stage reads back.
    MilesplitIndex,
    /// The WIAA member-school directory: schools, activities directors and coaches.
    WiaaDirectory,
    /// The MSHSL school listing: schools, activities directors and per-team coaches.
    MshslSchools,
    /// The Nebraska and North Dakota half each: association directories of schools and staff.
    PlainNamesDirectories,
    /// The IHSA school list and its per-school staff: schools, their coaches and activities
    /// directors. One request returns every Illinois member school, so the walk needs no seed.
    IhsaSchools,
    /// The KSHSAA directory: one request returns every Kansas member school with its athletic
    /// director.
    KsDirectory,
}

/// The arm for one planned slug, or `None` when the stage has no walk for it.
///
/// `None` is a build bug rather than a source condition: every refusal in the plan names a source no
/// stage dispatches, so a sweepable unit without an arm is the two lists disagreeing. The stage
/// turns it into a terminal error naming the slug.
pub(super) fn arm_for(slug: &str) -> Option<TeamsArm> {
    TEAMS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

/// Run one planned source's team-index walk and report the rows it wrote, or `None` when the slug
/// is another stage's work.
async fn sweep_team_source(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    slug: &str,
) -> Result<Option<usize>, HandlerError> {
    let Some(arm) = arm_for(slug) else {
        assert_some_stage_arms(slug)?;
        return Ok(None);
    };
    match arm {
        TeamsArm::MilesplitIndex => {
            return Ok(Some(
                census::collect_state_teams(fetcher, store, jurisdiction, refresh)
                    .await
                    .map_err(|error| job_error(collect_error(error)))
                    .map(|t| t.len())?,
            ));
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
    }
}

/// The WIAA member-school directory. Its bulk index is the per-letter listing, so an empty
/// `school_names` walks every letter — the whole state — rather than nothing.
async fn walk_wiaa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<usize, HandlerError> {
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
    rows_written(&report)
}

/// The MSHSL school listing, which publishes every member school in one paginated list.
async fn walk_mshsl(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<usize, HandlerError> {
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
    rows_written(&report)
}

/// One adapter over two associations, split by state: a run of Nebraska walks the NSAA half alone
/// and a run of North Dakota the NDHSAA half alone, each publishing its own school index.
async fn walk_plain_names(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<usize, HandlerError> {
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
    rows_written(&report)
}

/// The IHSA school list and its per-school staff. One request returns every Illinois member school,
/// so an empty `school_names` walks the whole state rather than nothing.
async fn walk_ihsa(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<usize, HandlerError> {
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
    rows_written(&report)
}

/// The KSHSAA directory. Its one endpoint returns every Kansas member school with its athletic
/// director, and the walk names no school of its own.
async fn walk_ks(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
) -> Result<usize, HandlerError> {
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
    rows_written(&report)
}

/// The team-index stage: enumerate one jurisdiction's school and team universe, one walk per
/// planned source, and report how many rows they wrote together.
///
/// The stage runs the plan, not a list of its own: `sweepable` is the recorded plan's units, in
/// plan order, so widening the sources a state sweeps is the same edit as widening
/// [`TEAMS_ARMS`] — and a unit the plan calls sweepable that no arm serves is a terminal error
/// rather than a silence. Every walk is journalled, so a re-invocation that reaches this stage
/// again resumes instead of re-fetching, and the more expensive ones answer from the HTTP cache.
///
/// The team index itself stays where `collect_state_teams` put it — the store's observations and
/// the fetch cache — because the roster stage re-reads it. Returning the count rather than the refs
/// is what keeps the journal entry small on a state with thousands of teams.
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
    for slug in &sweepable {
        let Some(written) =
            sweep_team_source(&store, &fetcher, jurisdiction, season, refresh, &at, slug).await?
        else {
            continue;
        };
        records = records.saturating_add(written);
    }
    Ok(Json(StageOutcome { records, at }))
}
