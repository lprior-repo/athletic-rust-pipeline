//! AthleticLIVE athlete-index adapter.
//!
//! AthleticLIVE is the white-label live-results platform many Midwest timers run. Its public
//! Elasticsearch endpoint `search.athletic.live/athlete_list/_search` indexes one document per
//! athlete-entry at a meet, and those documents carry, per row:
//!
//! - the competitor's name, sex, and **grade** (`y`),
//! - the **Athletic.net athlete id** (`ani`) when the operator linked the meet,
//! - the school name plus AthleticLIVE team id (`t.i`) and **Athletic.net team id** (`t.ani`).
//!
//! That makes this the second independent athlete source in the census (MileSplit rosters are the
//! first) and, more importantly, a source of *Athletic.net profile seeds*: for every row with an
//! `ani`, the profile URL is deterministic
//! (`https://www.athletic.net/athlete/{id}/track-and-field`), so no Athletic.net enumeration or
//! search is required to acquire that athlete's history.
//!
//! Query shape (verified 2026-09-20 against meets 73566 and 75742):
//!
//! ```text
//! POST https://search.athletic.live/athlete_list/_search
//! {"size":2000,"from":0,
//!  "query":{"bool":{"filter":[{"terms":{"mi":[<AthleticLIVE meet ids>]}},
//!                             {"terms":{"y":["11","12","JR","SR"]}}]}},
//!  "_source":["i","n","y","g","mi","ani","t"]}
//! ```
//!
//! `y` is a keyword and accepts the numeric encoding (`"11"`) used by track meets; letter encodings
//! (`JR`/`SR`) appear on some cross-country meets and are filtered for as well. Grade is interpreted
//! against the meet date's school year, never against "today": grade 11 at a 2025-26 meet is class of
//! 2027, grade 12 at a 2026-27 meet is also class of 2027, and both are retained as observations.
//!
//! Elasticsearch caps `from + size` at 10,000, so meet batches are split when a batch exceeds the
//! window, and pagination restarts at the top of each split.

use std::collections::{HashMap, VecDeque};

use serde_json::{json, Value};

use crate::sources::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use crate::store::Table;
use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;

mod batches;
mod map;
mod parse;
mod targets;
mod tokens;

use batches::run_batches;

pub use map::{build_entities, BatchEntities};
pub use parse::{AthleteHit, HitTeam};
pub use targets::{meet_targets, MeetSelection, MeetTarget};
pub use tokens::{gender_from_token, grade_from_token, school_year_for_date, sport_for};

/// Elasticsearch result window: `from + size` may not exceed this.
const RESULT_WINDOW: usize = 10_000;
/// Rows per page.
const PAGE_SIZE: usize = 2_000;
/// Meet ids per query.
const MEETS_PER_BATCH: usize = 40;

const ENDPOINT: &str = "https://search.athletic.live/athlete_list/_search";

/// Adapter options (uniform across provider adapters).
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to meets in these jurisdictions; empty = every state present in the meet log.
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

/// Build the Elasticsearch query for a batch of meet ids.
pub fn batch_query(meet_ids: &[u64], from: usize) -> Value {
    json!({
        "size": PAGE_SIZE,
        "from": from,
        "track_total_hits": true,
        "query": { "bool": { "filter": [
            { "terms": { "mi": meet_ids } },
            { "terms": { "y": ["11", "12", "JR", "SR", "Jr", "Sr"] } }
        ] } },
        "_source": ["i", "n", "y", "g", "mi", "ani", "t"]
    })
}

/// Collect athlete rows for every timer-published meet and emit canonical entities.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let meets: Vec<CanonicalMeet> = ctx.store.scan(Table::Meets)?;
    if meets.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no meets in the store: run the `athleticlive` adapter first".to_string(),
        });
    }
    let selection = meet_targets(&meets, &options.states);
    let targets = selection.targets;
    if targets.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no timer-published meets matched the requested states".to_string(),
        });
    }
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();

    let mut report = AdapterReport::new("athleticlive_athletes", "athletes");
    let journal = ctx.store.journal_keys("athleticlive_rosters")?;
    let pending: Vec<&MeetTarget> = targets
        .iter()
        .filter(|target| !journal.contains(&target.athleticlive_meet_id.to_string()))
        .collect();
    report.note(format!(
        "meets in store: {}; timer-published after state filter: {}; skipped (implausible date): {}; skipped (no jurisdiction): {}; already journaled: {}; pending: {}",
        meets.len(),
        targets.len(),
        selection.skipped_implausible,
        selection.skipped_unplaced,
        journal.len(),
        pending.len()
    ));

    // FIFO: batches are consumed oldest-meet-first, so a capped run covers settled meets with
    // results rather than the tail of future-dated entries.
    let mut queue: VecDeque<Vec<&MeetTarget>> = pending
        .chunks(MEETS_PER_BATCH)
        .map(|chunk| chunk.to_vec())
        .collect();
    let mut stats = BatchStats::default();
    let (requests_before, cache_before) = stats_of(ctx).await;

    run_batches(ctx, options, &by_id, &mut queue, &mut stats, &mut report).await?;
    finish_run(
        ctx,
        options,
        &stats,
        &mut report,
        requests_before,
        cache_before,
    )
    .await;
    Ok(report)
}

/// Read the fetcher totals after the walk and note what the run produced.
async fn finish_run(
    ctx: &AdapterContext<'_>,
    options: &Options,
    stats: &BatchStats,
    report: &mut AdapterReport,
    requests_before: u64,
    cache_before: u64,
) {
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = u64::try_from(stats.athletes).unwrap_or(u64::MAX);
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.note(format!(
        "meets processed: {}; athlete rows: {}; canonical athletes written: {}",
        stats.meets, stats.rows, stats.athletes
    ));
    report.note(format!(
        "schools written: {}; teams written: {}; batch splits: {}",
        stats.schools, stats.teams, stats.splits
    ));
    report.note(format!(
        "rows with a grade: {}/{}, with an Athletic.net athlete id: {}, with an Athletic.net team id: {}, without a school name: {}",
        stats.rows_with_grade, stats.rows, stats.rows_with_athlete_id, stats.rows_with_team_id, stats.rows_without_school
    ));
    if !options.states.is_empty() {
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!("state filter: {}", codes.join(",")));
    }
}

#[derive(Debug, Default)]
struct BatchStats {
    meets: usize,
    rows: usize,
    athletes: usize,
    schools: usize,
    teams: usize,
    rows_with_grade: usize,
    rows_with_athlete_id: usize,
    rows_with_team_id: usize,
    rows_without_school: usize,
    splits: usize,
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests;
