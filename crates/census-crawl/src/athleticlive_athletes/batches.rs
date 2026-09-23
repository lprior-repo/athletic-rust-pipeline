//! The batch walk: one query per batch of meet ids, its Elasticsearch pages, and the entities a
//! batch contributes before it is journaled as done.

use super::map::build_entities;
use super::parse::AthleteHit;
use super::targets::MeetTarget;
use super::{batch_query, BatchStats, Options, ENDPOINT, PAGE_SIZE, RESULT_WINDOW};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_store::Table;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

/// Walk the batch queue, storing every batch's entities and journaling each batch as done.
pub(super) async fn run_batches<'t>(
    ctx: &AdapterContext<'_>,
    options: &Options,
    by_id: &HashMap<u64, &'t MeetTarget>,
    queue: &mut VecDeque<Vec<&'t MeetTarget>>,
    stats: &mut BatchStats,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    while let Some(batch) = queue.pop_front() {
        if let Some(limit) = options.limit {
            if stats.meets >= limit {
                break;
            }
        }
        let ids: Vec<u64> = batch.iter().map(|t| t.athleticlive_meet_id).collect();
        let (hits, total) = page_hits(ctx, options, &ids, report).await?;
        // A batch whose total exceeds the Elasticsearch result window must be split: the missing
        // rows are not recoverable by paging past 10,000.
        if split_batch(queue, &batch, total, report, stats) {
            continue;
        }
        emit_batch(ctx, options, &batch, &hits, by_id, stats)?;
    }
    Ok(())
}

/// Page one batch's query to the end of its result window, with the total it reported first.
async fn page_hits(
    ctx: &AdapterContext<'_>,
    options: &Options,
    ids: &[u64],
    report: &mut AdapterReport,
) -> CrawlResult<(Vec<AthleteHit>, usize)> {
    let fetch = crate::net::FetchOptions {
        refresh: options.refresh,
        allow_not_found: false,
        headers: vec![("accept".to_string(), "application/json".to_string())],
    };
    let mut from = 0usize;
    let mut hits: Vec<AthleteHit> = Vec::new();
    let mut total = 0usize;
    loop {
        let body = batch_query(ids, from);
        let outcome = ctx.fetcher.post_json(ENDPOINT, &body, &fetch).await?;
        if outcome.status != 200 {
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "batch of {} meets returned HTTP {} at offset {from}",
                ids.len(),
                outcome.status
            ));
            break;
        }
        let parsed: Value = outcome.json()?;
        if from == 0 {
            total = parsed
                .pointer("/hits/total/value")
                .and_then(Value::as_u64)
                .map_or(0, |value| usize::try_from(value).unwrap_or(usize::MAX));
        }
        let page: Vec<AthleteHit> = serde_json::from_value(Value::Array(page_sources(&parsed)))
            .map_err(|source| CrawlError::Decode {
                url: ENDPOINT.to_string(),
                source,
            })?;
        let fetched = page.len();
        hits.extend(page);
        from = from.saturating_add(fetched);
        if fetched == 0 || from >= total || from.saturating_add(PAGE_SIZE) > RESULT_WINDOW {
            break;
        }
    }
    Ok((hits, total))
}

/// The `_source` documents of one response, in the order the hits arrived.
fn page_sources(parsed: &Value) -> Vec<Value> {
    parsed
        .pointer("/hits/hits")
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter()
                .map(|hit| hit.get("_source").cloned().unwrap_or(Value::Null))
                .collect::<Vec<Value>>()
        })
        .unwrap_or_default()
}

/// Split a batch that exceeds the result window; `true` when the caller must move to the next one.
fn split_batch<'t>(
    queue: &mut VecDeque<Vec<&'t MeetTarget>>,
    batch: &[&'t MeetTarget],
    total: usize,
    report: &mut AdapterReport,
    stats: &mut BatchStats,
) -> bool {
    if total <= RESULT_WINDOW {
        return false;
    }
    if batch.len() > 1 {
        let (left, right) = batch.split_at(batch.len() / 2);
        queue.push_front(right.to_vec());
        queue.push_front(left.to_vec());
        stats.splits = stats.splits.saturating_add(1);
        report.note(format!(
            "split a {}-meet batch ({} rows exceeds the {}-row result window)",
            batch.len(),
            total,
            RESULT_WINDOW
        ));
        return true;
    }
    // Every larger batch was split and re-queued above, so exactly one meet remains here.
    let Some(target) = batch.first() else {
        return true;
    };
    report.note(format!(
        "meet {} alone has {} rows: only {} were retrievable in one result window",
        target.athleticlive_meet_id, total, RESULT_WINDOW
    ));
    false
}

/// Journal a batch as done, store its canonical entities and add it to the run's counters.
fn emit_batch<'t>(
    ctx: &AdapterContext<'_>,
    options: &Options,
    batch: &[&'t MeetTarget],
    hits: &[AthleteHit],
    by_id: &HashMap<u64, &'t MeetTarget>,
    stats: &mut BatchStats,
) -> CrawlResult<()> {
    if hits.is_empty() {
        for target in batch {
            ctx.store.journal_done(
                "athleticlive_rosters",
                &target.athleticlive_meet_id.to_string(),
                &json!({ "meet": target.name, "rows": 0 }),
            )?;
        }
        stats.meets = stats.meets.saturating_add(batch.len());
        return Ok(());
    }

    let entities = build_entities(hits, by_id, &options.observed_on, ctx.school_year);
    ctx.store.append_many(Table::Schools, &entities.schools)?;
    ctx.store.append_many(Table::Teams, &entities.teams)?;
    ctx.store.append_many(Table::Athletes, &entities.athletes)?;
    for target in batch {
        ctx.store.journal_done(
            "athleticlive_rosters",
            &target.athleticlive_meet_id.to_string(),
            &json!({ "meet": target.name, "batch_rows": hits.len() }),
        )?;
    }
    stats.meets = stats.meets.saturating_add(batch.len());
    stats.rows = stats.rows.saturating_add(entities.rows);
    stats.athletes = stats.athletes.saturating_add(entities.athletes.len());
    stats.schools = stats.schools.saturating_add(entities.schools.len());
    stats.teams = stats.teams.saturating_add(entities.teams.len());
    stats.rows_with_grade = stats
        .rows_with_grade
        .saturating_add(entities.rows_with_grade);
    stats.rows_with_athlete_id = stats
        .rows_with_athlete_id
        .saturating_add(entities.rows_with_athlete_id);
    stats.rows_with_team_id = stats
        .rows_with_team_id
        .saturating_add(entities.rows_with_team_id);
    stats.rows_without_school = stats
        .rows_without_school
        .saturating_add(entities.rows_without_school);
    Ok(())
}
