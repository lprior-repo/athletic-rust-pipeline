use super::map::{DocumentEntities, ResultStats, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult, UnresolvedCounters};
use census_domain::model::CanonicalSchool;
use census_store::Table;

mod absorb;
mod manifest;
mod run;
#[cfg(test)]
mod tests;

pub use manifest::{collect_manifest, ManifestOptions};
use run::Run;

const RETIRED_PHASE: &str = "athleticlive_results_v2";
pub(super) const CAPTURE_PHASE: &str = "athleticlive_results_capture_v1";
pub(super) const EFFECT_PHASE: &str = "athleticlive_results_effect_v2";
pub(super) const ROW_PHASE: &str = "athleticlive_projection_v4";
pub(super) const PARSER: &str = "athleticlive_results_v4";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingsCapture {
    pub run_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct ResultOptions {
    pub meet: Option<MeetTarget>,
    pub summary: Option<String>,
    pub documents: Vec<String>,
    pub standings: Vec<StandingsCapture>,
    pub observed_on: String,
    pub limit: Option<usize>,
    pub capture_metadata: std::collections::BTreeMap<String, crate::net::cache::CacheMeta>,
}

impl ResultOptions {
    pub fn for_meet(meet: MeetTarget, observed_on: impl Into<String>) -> Self {
        Self {
            meet: Some(meet),
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct EntityCounts {
    meets: usize,
    events: usize,
    teams: usize,
    athletes: usize,
    performances: usize,
}

impl EntityCounts {
    fn add(&mut self, value: Self) -> CrawlResult<()> {
        self.meets = self
            .meets
            .checked_add(value.meets)
            .ok_or_else(run::counter_error)?;
        self.events = self
            .events
            .checked_add(value.events)
            .ok_or_else(run::counter_error)?;
        self.teams = self
            .teams
            .checked_add(value.teams)
            .ok_or_else(run::counter_error)?;
        self.athletes = self
            .athletes
            .checked_add(value.athletes)
            .ok_or_else(run::counter_error)?;
        self.performances = self
            .performances
            .checked_add(value.performances)
            .ok_or_else(run::counter_error)?;
        Ok(())
    }
}

pub async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ResultOptions,
) -> CrawlResult<AdapterReport> {
    let target = options.meet.as_ref().ok_or_else(|| CrawlError::Invariant {
        detail: "athleticlive results require a source-owned meet target".to_string(),
    })?;
    let mut run = Run::new(ctx, target)?;
    run.read_captures(ctx, options)?;
    let walk = run.close()?;
    finish(walk)
}

pub(super) struct WalkResult {
    pub(super) entities: DocumentEntities,
    counts: EntityCounts,
    pub(super) failures: Vec<String>,
    failure_count: usize,
    pub(super) unfinished: Vec<String>,
    resumed: usize,
}

pub(super) struct EffectReceipt {
    pub(super) key: String,
    pub(super) payload: serde_json::Value,
}

fn append(
    ctx: &AdapterContext<'_>,
    entities: DocumentEntities,
    schools: &[CanonicalSchool],
    body: &str,
    receipt: &EffectReceipt,
) -> CrawlResult<EntityCounts> {
    let mut page = ctx.write_batch();
    let mut observations = ctx.athlete_observations(&entities.athletes, schools);
    observations
        .try_reserve(entities.source_observations.len())
        .map_err(|_| CrawlError::Resource {
            resource: "LIVE source observations",
            requested: entities.source_observations.len(),
            limit: crate::recording::MAX_RECORDED_WORK,
        })?;
    observations.extend(entities.source_observations);
    let meets = crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::Meets,
        entities.meets,
        ROW_PHASE,
    )?;
    let events = crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::Events,
        entities.events,
        ROW_PHASE,
    )?;
    let teams = crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::Teams,
        entities.teams,
        ROW_PHASE,
    )?;
    let athletes = crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::Athletes,
        entities.athletes,
        ROW_PHASE,
    )?;
    let performances = crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::Performances,
        entities.performances,
        ROW_PHASE,
    )?;
    crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::ReviewCases,
        entities.review_cases,
        ROW_PHASE,
    )?;
    crate::recording::projection::append_new(
        ctx,
        &mut page,
        Table::SourceObservations,
        observations,
        ROW_PHASE,
    )?;
    let digest = crate::net::cache::content_digest(body.as_bytes());
    if !ctx.store.journal_contains(CAPTURE_PHASE, &digest)? {
        page.journal_done(
            CAPTURE_PHASE,
            &digest,
            &serde_json::json!({"bytes": body.len(), "body": body}),
        )?;
    }
    if !ctx.store.journal_contains(EFFECT_PHASE, &receipt.key)? {
        page.journal_done(EFFECT_PHASE, &receipt.key, &receipt.payload)?;
    }
    page.commit()?;
    Ok(EntityCounts {
        meets,
        events,
        teams,
        athletes,
        performances,
    })
}

fn retired_receipts(ctx: &AdapterContext<'_>) -> CrawlResult<()> {
    for phase in [RETIRED_PHASE, "athleticlive_results_effect_v1"] {
        if !ctx.store.journal_keys(phase)?.is_empty() {
            return Err(CrawlError::Invariant {
                detail: format!("retired `{phase}` receipts cannot prove the current LIVE projection; use a fresh store"),
            });
        }
    }
    Ok(())
}

fn finish(walk: WalkResult) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(SOURCE_ID, "result rows");
    let stats = &walk.entities.stats;
    report.rows = u64::try_from(walk.counts.performances).map_err(|_| run::counter_error())?;
    report.errors = u64::try_from(walk.failure_count).map_err(|_| run::counter_error())?;
    report.unfinished = walk.unfinished;
    report.unresolved = Some(unresolved(stats)?);
    stats
        .note("")
        .into_iter()
        .for_each(|note| report.note(note));
    walk.failures
        .into_iter()
        .take(5)
        .for_each(|failure| report.note(format!("capture refused: {failure}")));
    if walk.resumed > 0 {
        report.note(format!(
            "captures already journaled by an earlier run: {}",
            walk.resumed
        ));
    }
    let counts = walk.counts;
    report.note(format!(
        "canonical entities committed: meets {} events {} teams {} athletes {} performances {}",
        counts.meets, counts.events, counts.teams, counts.athletes, counts.performances
    ));
    report.note("operator captures replayed; no network request issued; physical capture metadata is not refreshed");
    report.finish_frontier();
    Ok(report)
}

fn unresolved(stats: &ResultStats) -> CrawlResult<UnresolvedCounters> {
    let rows = [
        stats.rows_skipped_no_name,
        stats.rows_skipped_no_school,
        stats.rows_skipped_unresolved_school,
        stats.rows_skipped_no_grade,
        stats.rows_skipped_unsupported_cohort,
    ]
    .into_iter()
    .try_fold(0usize, |count, value| {
        count.checked_add(value).ok_or_else(run::counter_error)
    })?;
    let labels = stats
        .events_unmapped
        .checked_add(stats.events_unfetched)
        .ok_or_else(run::counter_error)?;
    Ok(UnresolvedCounters {
        rows: u64::try_from(rows).map_err(|_| run::counter_error())?,
        labels: u64::try_from(labels).map_err(|_| run::counter_error())?,
    })
}
