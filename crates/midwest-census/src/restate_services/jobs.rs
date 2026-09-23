use std::path::PathBuf;
use std::sync::Arc;

use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, RunRetryPolicy, TerminalError};
use serde_json::Value;

use crate::census::{self, CollectOptions, MeetCensus, StateProgress};
use crate::net::Fetcher;
use crate::report::{self, ReportError, ReportResult, Scope};
use crate::sources::CrawlError;
use crate::store::{Store, StoreError, StoreResult, Table};
use crate::{bests, workbook};

use super::wire::ingest::SweepReport;
use super::wire::{BestsReply, ConsolidatedTable, ReportReply, StageOutcome, WorkbookReply};
use super::{cohort_label, JobError, MAX_ROWS_PER_REQUEST};

/// Append observations for one table. Every row must carry its canonical `id`; that is what the
/// store keys the observation by.
///
/// The per-request ceiling is enforced here, where the row count is known. It is a *request*
/// failure, and the store's taxonomy has no admission-bound variant, so it rides
/// [`StoreError::Invariant`] — the slot for a bound the caller cannot satisfy by retrying.
/// [`JobError`](super::JobError)'s conversion classifies that variant as terminal, so a replay does
/// not re-offer a batch the ceiling already refused.
pub fn append_observations(store: &Store, table: Table, rows: &[Value]) -> StoreResult<usize> {
    if rows.len() > MAX_ROWS_PER_REQUEST {
        return Err(StoreError::Invariant {
            detail: format!(
                "{} rows exceeds the per-request ceiling of {MAX_ROWS_PER_REQUEST}",
                rows.len()
            ),
        });
    }
    store.append_many(table, rows)?;
    Ok(rows.len())
}

pub(super) fn consolidate_tables(
    store: &Store,
    tables: &[Table],
) -> StoreResult<Vec<ConsolidatedTable>> {
    let mut out = Vec::with_capacity(tables.len());
    for table in tables {
        let path = store.table_path(*table);
        let consolidated = store.consolidate_table(*table, &path)?;
        // Same rule as the CLI: the count comes out of the merge that wrote the snapshot.
        let emails_withheld = (*table == Table::Coaches).then_some(consolidated.withheld);
        out.push(ConsolidatedTable {
            table: table.file().to_string(),
            rows: consolidated.rows,
            emails_withheld,
        });
    }
    Ok(out)
}

pub(super) fn build_report(store: &Store, scope: Scope) -> ReportResult<ReportReply> {
    let census = report::build_census(store, scope)?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    Ok(ReportReply {
        scope: scope.as_str().to_string(),
        generated_on: census.generated_on.clone(),
        // Encoding a value this model already holds cannot fail, and `Decode` is the reader-shaped
        // slot, so the serialize site rides `Invariant` exactly as `report::write_census` does.
        totals: serde_json::to_value(&census.totals).map_err(|source| ReportError::Invariant {
            detail: format!("the census totals are not valid json: {source}"),
        })?,
        json_path: json_path.display().to_string(),
        csv_path: csv_path.display().to_string(),
    })
}

pub(super) fn build_bests(store: &Store, options: &bests::Options) -> ReportResult<BestsReply> {
    // `bests` reports store failures, which `ReportError` absorbs through its `#[from]`: the reply
    // and the reduction it summarizes travel as one error type.
    let rows = bests::build(store, options)?;
    let cohort = cohort_label(options.grad_year);
    let (jsonl, csv_path) = bests::write(store, &rows, &cohort)?;
    Ok(BestsReply {
        cohort,
        rows: rows.len(),
        jsonl: jsonl.display().to_string(),
        csv: csv_path.display().to_string(),
    })
}

pub(super) fn build_workbook(
    store: &Store,
    options: &workbook::Options,
) -> ReportResult<WorkbookReply> {
    let path = workbook::build(store, options)?;
    Ok(WorkbookReply {
        path: path.display().to_string(),
        grad_year: options.grad_year,
    })
}

pub(super) fn write_sweep_report(
    store: &Store,
    report: &SweepReport,
    today: &str,
) -> ReportResult<PathBuf> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out).map_err(|source| ReportError::Io {
        path: out.clone(),
        source,
    })?;
    let path = out.join(format!("sweep-{today}.json"));
    let encoded = serde_json::to_vec_pretty(report).map_err(|source| ReportError::Invariant {
        detail: format!("the sweep report is not valid json: {source}"),
    })?;
    std::fs::write(&path, encoded).map_err(|source| ReportError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

// ------------------------------------------------------- jurisdiction census stages

/// The team-index stage: enumerate one jurisdiction's teams and report how many the index lists.
///
/// The index itself stays where `collect_state_teams` put it — the store's observations and the
/// fetch cache — because the roster stage re-reads it. Returning the count rather than the refs is
/// what keeps the journal entry small on a state with thousands of teams.
pub(super) async fn teams_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    refresh: bool,
    at: String,
) -> Result<Json<StageOutcome>, HandlerError> {
    let teams = census::collect_state_teams(&fetcher, &store, jurisdiction, refresh)
        .await
        .map_err(collect_error)?;
    Ok(Json(StageOutcome {
        records: teams.len(),
        at,
    }))
}

/// The meet census stage: enumerate the jurisdiction's published meets for one season year and
/// write them as `source_meets` rows. Nothing here walks meet pages — the results index publishes
/// fifty meets per response — so the stage costs a bounded number of index reads, journaled per page.
pub(super) async fn meets_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: String,
) -> Result<Json<MeetCensus>, HandlerError> {
    let census = census::collect_state_meets(&fetcher, &store, jurisdiction, year, &at, refresh)
        .await
        .map_err(collect_error)?;
    Ok(Json(census))
}

/// The roster stage: walk every roster the jurisdiction's index lists, under one set of collection
/// options. This is the stage the cohort counts come from, so its outcome is recorded whole.
pub(super) async fn rosters_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    options: CollectOptions,
    jurisdiction: UsJurisdiction,
) -> Result<Json<StateProgress>, HandlerError> {
    let teams = census::collect_state_teams(&fetcher, &store, jurisdiction, false)
        .await
        .map_err(collect_error)?;
    let progress = census::collect_state_rosters(&fetcher, &store, &teams, &options, jurisdiction)
        .await
        .map_err(collect_error)?;
    Ok(Json(progress))
}

/// Classify a collection failure for retry. The store keeps its own classification, an invariant
/// violation is terminal because replaying it cannot restore one, and everything else — a fetch, a
/// schema mismatch, a poisoned page — is what a bounded retry is for.
fn collect_error(error: CrawlError) -> JobError {
    match error {
        CrawlError::Store(source) => JobError::from(source),
        CrawlError::Invariant { detail } => JobError::Terminal { message: detail },
        other => JobError::Transient {
            message: other.to_string(),
        },
    }
}

/// One attempt per `run` inside a handler: ADR-002 makes Restate the owner of retries, and the
/// retry it owns is the *invocation* retry declared on the handler. A `run`-level retry would be a
/// second, in-process budget the journal cannot account for, so every `run` attempts once and a
/// failure leaves the handler for the invocation policy to replay.
pub(super) fn no_run_retry() -> RunRetryPolicy {
    RunRetryPolicy::new().max_attempts(1)
}

/// A stage that should have completed has no recorded outcome: a bug in the stage sequence, not a
/// source condition, so it is terminal.
pub(super) fn invariant(message: &str) -> HandlerError {
    TerminalError::new(format!("jurisdiction census invariant: {message}")).into()
}
