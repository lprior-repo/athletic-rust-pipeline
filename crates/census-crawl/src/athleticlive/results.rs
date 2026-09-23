//! The result-plane run: the captures an operator supplies, the entities they yield, and the report.
//!
//! The adapter issues no request. It folds captures of the three wire routes
//! ([`super::wire`] holds the measured cost of each): one event document per event, one event
//! summary per meet, and — for a race whose event document is missing — one live-standings payload
//! per run key. Every capture is read from a path the operator supplies, and the route URL it was
//! served from is stamped into the evidence it writes.
//!
//! Layout: this file owns the entry point, the run state and the table appends; `results::absorb`
//! owns the three document-level folds, `map_rows` the row-level one, and `results::tests` proves
//! what the captures measure.

use super::map::{DocumentEntities, ResultStats, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_store::Table;

mod absorb;
mod manifest;
mod run;
#[cfg(test)]
mod tests;

pub use manifest::{collect_manifest, ManifestOptions};

use run::Run;

/// The journal phase of the result-plane route, with the capture layout encoded in the name: a
/// parser change that alters what an already-journaled capture yields bumps this, so those captures
/// are read again instead of being skipped as done.
const PHASE: &str = "athleticlive_results_v1";

/// A captured live-standings payload, paired with the run key whose race it publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingsCapture {
    /// The run key the event published (`rui`: `4-1`, `19-1`).
    pub run_id: String,
    /// Path to the captured body of `liveRunStandings/<runId>.json`.
    pub path: String,
}

/// What a result-plane run is asked for: one meet, and the captures of it that were made.
#[derive(Debug, Clone, Default)]
pub struct ResultOptions {
    /// The meet the captures belong to, as the harvest publishes it. Required: the meet is minted
    /// from these facts, never from a result payload.
    pub meet: Option<MeetTarget>,
    /// Path to a captured `meet_<meetId>/event_summary.json`.
    pub summary: Option<String>,
    /// Paths to captured event documents (`ind_res_list/_doc/<eventId>`), in the order to read them.
    pub documents: Vec<String>,
    /// Captured live standings, each paired with the event's run key.
    pub standings: Vec<StandingsCapture>,
    /// Observation date stamped on every evidence row.
    pub observed_on: String,
    /// Read at most this many event documents.
    pub limit: Option<usize>,
}

impl ResultOptions {
    /// Options for one meet's captures, stamped with an observation date.
    pub fn for_meet(meet: MeetTarget, observed_on: impl Into<String>) -> Self {
        Self {
            meet: Some(meet),
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
}

/// Canonical entities one run wrote, by table.
#[derive(Debug, Default, Clone, Copy)]
struct EntityCounts {
    meets: usize,
    events: usize,
    teams: usize,
    athletes: usize,
    performances: usize,
}

/// Everything the closing notes need, once the walk is over.
struct RunSummary<'a> {
    stats: &'a ResultStats,
    counts: EntityCounts,
    failures: &'a [String],
    resumed: usize,
}

/// Read every supplied capture and write what they yield.
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
    run.read_captures(ctx, options)?;
    let walk = run.close();
    let counts = append(ctx, &walk.entities)?;
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

/// What a finished walk hands back: the entities it accumulated, what it refused, and how many
/// captures an earlier run had already journaled.
pub(super) struct WalkResult {
    pub(super) entities: DocumentEntities,
    pub(super) failures: Vec<String>,
    pub(super) resumed: usize,
}

/// Append one batch per table and return what was written.
fn append(ctx: &AdapterContext<'_>, entities: &DocumentEntities) -> CrawlResult<EntityCounts> {
    ctx.store.append_many(Table::Meets, &entities.meets)?;
    ctx.store.append_many(Table::Events, &entities.events)?;
    ctx.store.append_many(Table::Teams, &entities.teams)?;
    ctx.store.append_many(Table::Athletes, &entities.athletes)?;
    ctx.store
        .append_many(Table::Performances, &entities.performances)?;
    Ok(EntityCounts {
        meets: entities.meets.len(),
        events: entities.events.len(),
        teams: entities.teams.len(),
        athletes: entities.athletes.len(),
        performances: entities.performances.len(),
    })
}

/// Close the report: the row ledger, the entity counts, the captures that failed, and the scope.
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
