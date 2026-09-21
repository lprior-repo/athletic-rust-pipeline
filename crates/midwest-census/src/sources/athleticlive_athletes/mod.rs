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

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::model::CanonicalMeet;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;

mod map;
mod parse;
mod tokens;

pub use map::{build_entities, meet_targets, BatchEntities, MeetSelection, MeetTarget};
pub use parse::{AthleteHit, HitTeam};
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
    /// Restrict to meets in these state codes; empty = every state present in the meet log.
    pub states: Vec<String>,
    pub school_names: Vec<String>,
}

impl Options {
    pub fn for_states(states: Vec<String>, observed_on: impl Into<String>) -> Self {
        Self {
            states,
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
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
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let meets: Vec<CanonicalMeet> = ctx.store.scan(Table::Meets)?;
    if meets.is_empty() {
        bail!("no meets in the store: run the `athleticlive` adapter first");
    }
    let selection = meet_targets(&meets, &options.states);
    let targets = selection.targets;
    if targets.is_empty() {
        bail!("no timer-published meets matched the requested states");
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
        "meets in store: {}; timer-published after state filter: {}; skipped (implausible date): {}; already journaled: {}; pending: {}",
        meets.len(),
        targets.len(),
        selection.skipped_implausible,
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

    while let Some(batch) = queue.pop_front() {
        if let Some(limit) = options.limit {
            if stats.meets >= limit {
                break;
            }
        }
        let ids: Vec<u64> = batch.iter().map(|t| t.athleticlive_meet_id).collect();
        let mut from = 0usize;
        let mut hits: Vec<AthleteHit> = Vec::new();
        let mut total = 0usize;
        loop {
            let body = batch_query(&ids, from);
            let outcome = ctx
                .fetcher
                .post_json(
                    ENDPOINT,
                    &body,
                    &crate::net::FetchOptions {
                        refresh: options.refresh,
                        allow_not_found: false,
                        headers: vec![("accept".to_string(), "application/json".to_string())],
                    },
                )
                .await
                .context("querying athleticlive athlete_list")?;
            if outcome.status != 200 {
                report.errors += 1;
                report.note(format!(
                    "batch of {} meets returned HTTP {} at offset {from}",
                    ids.len(),
                    outcome.status
                ));
                break;
            }
            let parsed: Value = outcome.json().context("parsing athlete_list response")?;
            let page_total = parsed
                .pointer("/hits/total/value")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if from == 0 {
                total = page_total;
            }
            let sources: Vec<Value> = parsed
                .pointer("/hits/hits")
                .and_then(Value::as_array)
                .map(|hits| {
                    hits.iter()
                        .map(|hit| hit.get("_source").cloned().unwrap_or(Value::Null))
                        .collect()
                })
                .unwrap_or_default();
            let page: Vec<AthleteHit> = serde_json::from_value(Value::Array(sources))
                .context("decoding athlete_list hits")?;
            let fetched = page.len();
            hits.extend(page);
            from += fetched;
            if fetched == 0 || from >= total || from + PAGE_SIZE > RESULT_WINDOW {
                break;
            }
        }

        // A batch whose total exceeds the Elasticsearch result window must be split: the missing
        // rows are not recoverable by paging past 10,000.
        if total > RESULT_WINDOW {
            if batch.len() > 1 {
                let (left, right) = batch.split_at(batch.len() / 2);
                queue.push_front(right.to_vec());
                queue.push_front(left.to_vec());
                stats.splits += 1;
                report.note(format!(
                    "split a {}-meet batch ({} rows exceeds the {}-row result window)",
                    batch.len(),
                    total,
                    RESULT_WINDOW
                ));
                continue;
            }
            // Every larger batch was split and re-queued above, so exactly one meet remains here.
            let Some(target) = batch.first() else {
                continue;
            };
            report.note(format!(
                "meet {} alone has {} rows: only {} were retrievable in one result window",
                target.athleticlive_meet_id, total, RESULT_WINDOW
            ));
        }

        if hits.is_empty() {
            for target in &batch {
                ctx.store.journal_done(
                    "athleticlive_rosters",
                    &target.athleticlive_meet_id.to_string(),
                    &json!({ "meet": target.name, "rows": 0 }),
                )?;
            }
            stats.meets += batch.len();
            continue;
        }

        let entities = build_entities(&hits, &by_id, &options.observed_on, ctx.school_year);
        ctx.store.append_many(Table::Schools, &entities.schools)?;
        ctx.store.append_many(Table::Teams, &entities.teams)?;
        ctx.store.append_many(Table::Athletes, &entities.athletes)?;
        for target in &batch {
            ctx.store.journal_done(
                "athleticlive_rosters",
                &target.athleticlive_meet_id.to_string(),
                &json!({ "meet": target.name, "batch_rows": hits.len() }),
            )?;
        }
        stats.meets += batch.len();
        stats.rows += entities.rows;
        stats.athletes += entities.athletes.len();
        stats.schools += entities.schools.len();
        stats.teams += entities.teams.len();
        stats.rows_with_grade += entities.rows_with_grade;
        stats.rows_with_athlete_id += entities.rows_with_athlete_id;
        stats.rows_with_team_id += entities.rows_with_team_id;
        stats.rows_without_school += entities.rows_without_school;
    }

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = stats.athletes as u64;
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
        report.note(format!("state filter: {}", options.states.join(",")));
    }
    Ok(report)
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
