use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};
use serde_json::Value;

use crate::census::{self, CollectOptions, StateProgress};
use census_crawl::net::Fetcher;
use census_crawl::{AdapterContext, AdapterReport, CrawlError, RecordedJournal, Recording};
use census_report::report::{self, ReportError, ReportResult, Scope};
use census_report::{bests, workbook};
use census_store::{Store, StoreError, StoreResult, Table};

use super::wire::ingest::SweepReport;
use super::wire::{BestsReply, ConsolidatedTable, ReportReply, WorkbookReply};
use super::{cohort_label, job_error, JobError, MAX_ROWS_PER_REQUEST};

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
        // The same location the CLI consolidates to and every reader opens: `<store>/out/<table>.jsonl`.
        // `Store::table_path` is the pre-Fjall journal the one-time import reads, not a snapshot
        // output, so publishing there left the read model this run is supposed to refresh unreachable
        // to `report`, `bests`, the workbook and every adapter.
        let path = store.out_dir().join(format!("{}.jsonl", table.file()));
        let consolidated = store.consolidate_table(*table, &path)?;
        out.push(ConsolidatedTable {
            table: table.file().to_string(),
            rows: consolidated.rows,
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

pub(super) use super::meets_arms::meets_stage;
pub(super) use super::results_arms::results_stage;
pub(super) use super::teams_arms::teams_stage;

/// The context a source walk runs under, built the way the `provider` subcommand builds it: one
/// object's shared fetcher, the run's journaled date and the season the request asked for.
pub(super) fn adapter_context<'a>(
    store: &'a Arc<Store>,
    fetcher: &'a Arc<Fetcher>,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    recording: Option<&'a Recording>,
) -> AdapterContext<'a> {
    AdapterContext {
        fetcher: fetcher.as_ref(),
        store: store.as_ref(),
        refresh,
        school_year: season,
        observed_on: at.to_string(),
        recording,
    }
}

/// Write the journal entries a recorded walk produced, once its rows have been posted.
///
/// The entries are the walk's own markers — "this unit has been read" — and the store's batch writes
/// them beside the rows when one walk writes both. A routed walk writes neither: the caller posts
/// the rows to the source's `Ingest` object first and flushes the markers after, which keeps the
/// store's rule — no unit journaled whose rows are missing — across two writers. A run that stops in
/// between re-reads the unit from cache and posts it again: work, not loss.
pub(super) async fn flush_journal(
    store: Arc<Store>,
    entries: Vec<RecordedJournal>,
) -> Result<Json<u64>, HandlerError> {
    let written = u64::try_from(entries.len()).map_err(|_| {
        TerminalError::new(format!("{} journal entries do not fit u64", entries.len()))
    })?;
    let mut batch = store.write_batch();
    for entry in &entries {
        batch
            .journal_done(&entry.phase, &entry.key, &entry.payload)
            .map_err(|source| job_error(source.into()))?;
    }
    batch.commit().map_err(|source| job_error(source.into()))?;
    Ok(Json(written))
}

/// A walk's row count as a stage reports it.
///
/// The report counts `u64`; every count a stage reports fits a `usize` on the platforms it runs on,
/// and a walk that claimed otherwise would be a bug worth naming rather than a number worth
/// wrapping.
pub(super) fn rows_written(report: &AdapterReport) -> Result<usize, HandlerError> {
    usize::try_from(report.rows).map_err(|_| {
        TerminalError::new(format!(
            "{} reported {} rows, which this platform cannot count",
            report.adapter, report.rows
        ))
        .into()
    })
}

/// Whether some stage in the chain arms a planned slug.
///
/// Each stage runs its own arms, so a planned unit another stage owns is that stage's work rather
/// than a missing arm here. A slug no stage arms is a build bug — the plan's list and the arm
/// tables disagree — and is refused terminally rather than skipped in silence.
pub(super) fn assert_some_stage_arms(slug: &str) -> Result<(), HandlerError> {
    if super::jurisdiction::DISPATCHED.contains(&slug) {
        return Ok(());
    }
    Err(TerminalError::new(format!(
        "the plan calls {slug} sweepable and no stage in the chain arms it"
    ))
    .into())
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
        .map_err(|error| job_error(collect_error(error)))?;
    let progress = census::collect_state_rosters(&fetcher, &store, &teams, &options, jurisdiction)
        .await
        .map_err(|error| job_error(collect_error(error)))?;
    Ok(Json(progress))
}

/// Classify a collection failure for retry. The store keeps its own classification, an invariant
/// violation is terminal because replaying it cannot restore one, and `FetchError::retryable()`
/// lets the transport decide whether a transient network condition is worth retrying — the census
/// does not second-guess the transport's verdict.
///
/// Everything the fetcher did not handle — schema mismatches, JSON decode/encode, domain
/// construction failures, arithmetic overflows, and local I/O — is terminal: reading the same
/// bytes again does not make them less wrong.
///
/// `CrawlError::Io` is terminal here: it is a crawl-side failure reading a local fixture or
/// recording artifact. The store keeps its own `StoreError::Io` which is transient because
/// it wraps a WAL or sidecar write — a different failure surface (the store's durable log versus
/// the crawl's read-only artifact). The different classification is intentional: a crawl fixture
/// read that fails on one machine will fail identically everywhere, while a store WAL flush that
/// fails due to disk pressure may recover once the pressure eases.
pub fn collect_error(error: CrawlError) -> JobError {
    match error {
        CrawlError::Store(source) => JobError::from(source),
        CrawlError::Invariant { detail } => JobError::Terminal { message: detail },
        // Encode a row for the wire once; a replay cannot fix a broken row.
        error @ CrawlError::Encode { .. } => JobError::Terminal {
            message: error.to_string(),
        },
        // Schema, Decode, Domain and Arithmetic are all deterministic: the same input reproduces
        // the same failure, so a retry is pointless.
        CrawlError::Schema { .. }
        | CrawlError::Decode { .. }
        | CrawlError::Domain(..)
        | CrawlError::Arithmetic { .. }
        | CrawlError::Io { .. }
        | CrawlError::RegexInit { .. } => JobError::Terminal {
            message: error.to_string(),
        },
        // Let the transport decide: it classified the failure as retryable or not, and the census
        // honours that verdict. A fetch the transport says can be retried rides Transient; one it
        // says cannot (robots, TooLarge, BrowserLane { retryable: false }) is terminal.
        CrawlError::Fetch(e) if e.retryable() => JobError::Transient {
            message: e.to_string(),
        },
        CrawlError::Fetch(e) => JobError::Terminal {
            message: e.to_string(),
        },
    }
}

/// A closure that is total and cannot fail needs no policy; the `ctx.run` it wraps is pure
/// observation whose result is deterministic.
pub(super) async fn run_once<F, Fut>(closure: F) -> Fut::Output
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    closure().await
}

/// A stage that should have completed has no recorded outcome: a bug in the stage sequence, not a
/// source condition, so it is terminal.
pub(super) fn invariant(message: &str) -> HandlerError {
    TerminalError::new(format!("jurisdiction census invariant: {message}")).into()
}
