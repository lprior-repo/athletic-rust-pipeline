use super::map::{build_entities, BatchEntities};
use super::parse::AthleteHit;
use super::targets::MeetTarget;
use super::{
    batch_query, BatchStats, Options, ENDPOINT, MEETS_PER_BATCH, PAGE_SIZE, RESULT_WINDOW,
};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_store::{StoreBatch, Table};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

const MAX_ROWS_PER_MEET: usize = 8_000;
const MAX_PLAUSIBLE_TOTAL: usize = MAX_ROWS_PER_MEET * MEETS_PER_BATCH;

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
        if split_batch(queue, &batch, total, report, stats) {
            continue;
        }
        emit_batch(ctx, options, &batch, &hits, by_id, stats)?;
    }
    Ok(())
}

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
    let mut declared: Option<usize> = None;
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
        let page: Vec<AthleteHit> = serde_json::from_value(Value::Array(page_sources(&parsed)))
            .map_err(|source| CrawlError::Decode {
                url: ENDPOINT.to_string(),
                source,
            })?;
        let fetched = page.len();
        if from == 0 {
            declared = declared_rows(&parsed);
            total = usable_total(declared, fetched, ids.len(), report);
        }
        hits.extend(page);
        from = from.saturating_add(fetched);
        let window_reached = from.saturating_add(PAGE_SIZE) > RESULT_WINDOW;
        if fetched == 0 || from >= total || window_reached {
            if declared.is_none() && window_reached && fetched > 0 {
                report.note(format!(
                    "batch of {} meets reached the {RESULT_WINDOW}-row result window with no usable result total: any rows beyond it were not retrieved",
                    ids.len()
                ));
            }
            break;
        }
    }
    Ok((hits, total))
}

fn declared_rows(parsed: &Value) -> Option<usize> {
    parsed
        .pointer("/hits/total/value")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn usable_total(
    declared: Option<usize>,
    fetched: usize,
    meets: usize,
    report: &mut AdapterReport,
) -> usize {
    if fetched == 0 {
        if let Some(claimed) = declared.filter(|total| *total > 0) {
            report.note(format!(
                "batch of {meets} meets returned no rows though it declared {claimed}: nothing was written for it"
            ));
        }
        return 0;
    }
    match declared.filter(|total| (fetched..=MAX_PLAUSIBLE_TOTAL).contains(total)) {
        Some(total) => total,
        None => {
            let claim = match declared {
                Some(total) => format!("declared {total} rows against {fetched} on the first page"),
                None => format!("returned {fetched} rows with no result total"),
            };
            tracing::warn!(
                meets,
                fetched,
                declared = ?declared,
                window = RESULT_WINDOW,
                "athleticlive result total is missing or implausible; paginating to the result window"
            );
            report.note(format!(
                "batch of {meets} meets {claim}; paginating to the {RESULT_WINDOW}-row result window"
            ));
            RESULT_WINDOW
        }
    }
}

fn page_sources(parsed: &Value) -> Vec<Value> {
    match parsed.pointer("/hits/hits").and_then(Value::as_array) {
        Some(hits) => hits
            .iter()
            .map(|hit| {
                hit.get("_source")
                    .cloned()
                    .map_or(Value::Null, |value| value)
            })
            .collect::<Vec<Value>>(),
        None => Default::default(),
    }
}

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
    let Some(target) = batch.first() else {
        return true;
    };
    report.note(format!(
        "meet {} alone has {} rows: only {} were retrievable in one result window",
        target.athleticlive_meet_id, total, RESULT_WINDOW
    ));
    false
}

fn emit_batch<'t>(
    ctx: &AdapterContext<'_>,
    options: &Options,
    batch: &[&'t MeetTarget],
    hits: &[AthleteHit],
    by_id: &HashMap<u64, &'t MeetTarget>,
    stats: &mut BatchStats,
) -> CrawlResult<()> {
    let mut page = ctx.store.write_batch();
    if hits.is_empty() {
        for target in batch {
            page.journal_done(
                "athleticlive_rosters",
                &target.athleticlive_meet_id.to_string(),
                &json!({ "meet": target.name, "rows": 0 }),
            )?;
        }
        page.commit()?;
        stats.meets = stats.meets.saturating_add(batch.len());
        return Ok(());
    }

    let entities = fill_page(ctx, options, batch, hits, by_id, &mut page)?;
    page.commit()?;
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

fn fill_page<'t>(
    ctx: &AdapterContext<'_>,
    options: &Options,
    batch: &[&'t MeetTarget],
    hits: &[AthleteHit],
    by_id: &HashMap<u64, &'t MeetTarget>,
    page: &mut StoreBatch<'_>,
) -> CrawlResult<BatchEntities> {
    let entities = build_entities(hits, by_id, &options.observed_on, ctx.school_year);
    page.append_many(Table::Schools, &entities.schools)?;
    page.append_many(Table::Teams, &entities.teams)?;
    page.append_many(Table::Athletes, &entities.athletes)?;
    page.append_many(
        Table::SourceObservations,
        &ctx.athlete_observations(&entities.athletes, &entities.schools),
    )?;
    for target in batch {
        page.journal_done(
            "athleticlive_rosters",
            &target.athleticlive_meet_id.to_string(),
            &json!({ "meet": target.name, "batch_rows": hits.len() }),
        )?;
    }
    Ok(entities)
}
