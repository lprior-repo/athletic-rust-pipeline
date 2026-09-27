use super::map::{DocumentEntities, ResultStats, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::CanonicalSchool;
use census_store::Table;

mod absorb;
mod manifest;
mod run;
#[cfg(test)]
mod tests;

pub use manifest::{collect_manifest, ManifestOptions};

use run::Run;

const PHASE: &str = "athleticlive_results_v1";

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

struct RunSummary<'a> {
    stats: &'a ResultStats,
    counts: EntityCounts,
    failures: &'a [String],
    resumed: usize,
}

pub async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ResultOptions,
) -> CrawlResult<AdapterReport> {
    let Some(target) = options.meet.as_ref() else {
        return Err(CrawlError::Invariant {
            detail:
                "the athleticlive results route requires a meet target: the meet is minted from \
                     the harvest's own state, date and name, never from a result payload"
                    .to_string(),
        });
    };
    let mut report = AdapterReport::new(SOURCE_ID, "result rows");
    let mut run = Run::new(ctx, target, options)?;
    run.read_captures(options)?;
    let walk = run.close();
    let counts = append(ctx, &walk.entities, &walk.schools, walk.entries)?;
    finish(
        &mut report,
        RunSummary {
            stats: &walk.entities.stats,
            counts,
            failures: &walk.failures,
            resumed: walk.resumed,
        },
    );
    Ok(report)
}

pub(super) struct WalkResult {
    pub(super) entities: DocumentEntities,
    pub(super) schools: Vec<CanonicalSchool>,
    pub(super) entries: Vec<(String, serde_json::Value)>,
    pub(super) failures: Vec<String>,
    pub(super) resumed: usize,
}

fn append(
    ctx: &AdapterContext<'_>,
    entities: &DocumentEntities,
    schools: &[CanonicalSchool],
    entries: Vec<(String, serde_json::Value)>,
) -> CrawlResult<EntityCounts> {
    let mut page = ctx.store.write_batch();
    page.append_many(Table::Meets, &entities.meets)?;
    page.append_many(Table::Events, &entities.events)?;
    page.append_many(Table::Teams, &entities.teams)?;
    page.append_many(Table::Athletes, &entities.athletes)?;
    page.append_many(Table::SourceObservations, &ctx.athlete_observations(&entities.athletes, schools))?;
    page.append_many(Table::Performances, &entities.performances)?;
    for (path, payload) in &entries {
        page.journal_done(PHASE, path, payload)?;
    }
    page.commit()?;
    Ok(EntityCounts {
        meets: entities.meets.len(),
        events: entities.events.len(),
        teams: entities.teams.len(),
        athletes: entities.athletes.len(),
        performances: entities.performances.len(),
    })
}

fn finish(report: &mut AdapterReport, summary: RunSummary<'_>) {
    let counts = summary.counts;
    report.rows = u64::try_from(summary.stats.rows_mapped).unwrap_or(u64::MAX);
    report.errors = u64::try_from(summary.failures.len()).unwrap_or(u64::MAX);
    for line in summary.stats.note("") {
        report.note(line);
    }
    if summary.resumed > 0 {
        report.note(format!(
            "captures already journaled by an earlier run: {}",
            summary.resumed
        ));
    }
    for failure in summary.failures {
        report.note(format!("capture refused: {failure}"));
    }
    report.note(format!(
        "canonical entities: meets {} events {} teams {} athletes {} performances {}",
        counts.meets, counts.events, counts.teams, counts.athletes, counts.performances
    ));
    report.note(
        "no request was issued: the captures are the operator's own copies of the three routes, and \
         each entity carries the route URL it was served from as its evidence",
    );
    report.note(
        "this source is outside the core scope (`report --core`): it is an Athletic.net-derived \
         results mirror, so the core comparison stays independent of it",
    );
}
