use std::collections::{HashMap, VecDeque};

use serde_json::{json, Value};

use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;
use census_store::Table;

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

const RESULT_WINDOW: usize = 10_000;
const PAGE_SIZE: usize = 2_000;
const MEETS_PER_BATCH: usize = 40;

const ENDPOINT: &str = "https://search.athletic.live/athlete_list/_search";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

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

async fn finish_run(
    ctx: &AdapterContext<'_>,
    options: &Options,
    stats: &BatchStats,
    report: &mut AdapterReport,
    requests_before: u64,
    cache_before: u64,
) {
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = u64::try_from(stats.athletes).map_or(u64::MAX, |value| value);
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
    (stats.physical_requests(), stats.cache_hits)
}

#[cfg(test)]
mod tests;
