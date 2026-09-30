use std::path::PathBuf;
use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};
use serde_json::Value;

use crate::census::{self, CollectOptions, StateProgress};
use census_crawl::net::Fetcher;
use census_crawl::{AdapterContext, AdapterReport, CrawlError, RecordedJournal, Recording};
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, ReportError, ReportResult, Scope};
use census_report::{bests, workbook};
use census_store::{Application, Store, StoreError, StoreResult, Table};

use super::wire::ingest::SweepReport;
use super::wire::{BestsReply, ConsolidatedTable, ReportReply, WorkbookReply};
use super::{cohort_label, job_error, JobError, MAX_ROWS_PER_REQUEST};

pub fn apply_observations(
    store: &Store,
    table: Table,
    rows: &[Value],
    operation: &str,
    digest: &str,
) -> StoreResult<Application> {
    if rows.len() > MAX_ROWS_PER_REQUEST {
        return Err(StoreError::Invariant {
            detail: format!(
                "{} rows exceeds the per-request ceiling of {MAX_ROWS_PER_REQUEST}",
                rows.len()
            ),
        });
    }
    let mut batch = store.write_batch();
    batch.append_many(table, rows)?;
    batch.commit_once(operation, digest)
}

pub(super) fn consolidate_tables(
    store: &Store,
    tables: &[Table],
) -> StoreResult<Vec<ConsolidatedTable>> {
    let mut out = Vec::with_capacity(tables.len());
    for table in tables {
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
    let dataset = ExportDataset::load(store)?;
    let derivation = Derivation::of(&dataset, scope, None);
    let census = report::build_census(&derivation, &store.out_dir());
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    Ok(ReportReply {
        scope: scope.as_str().to_string(),
        generated_on: census.generated_on.clone(),
        totals: serde_json::to_value(&census.totals).map_err(|source| ReportError::Invariant {
            detail: format!("the census totals are not valid json: {source}"),
        })?,
        json_path: json_path.display().to_string(),
        csv_path: csv_path.display().to_string(),
    })
}

pub(super) fn build_bests(store: &Store, options: &bests::Options) -> ReportResult<BestsReply> {
    let dataset = ExportDataset::load(store)?;
    let rows = bests::build_from_dataset(&dataset, options);
    let cohort = cohort_label(options.grad_year);
    let (jsonl, csv_path) = bests::write(&store.out_dir(), &rows, &cohort)?;
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

pub(super) use super::meets_arms::meets_stage;
pub(super) use super::results_arms::results_stage;
pub(super) use super::teams_arms::teams_stage;

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

pub(super) fn rows_written(report: &AdapterReport) -> Result<usize, HandlerError> {
    usize::try_from(report.rows).map_err(|_| {
        TerminalError::new(format!(
            "{} reported {} rows, which this platform cannot count",
            report.adapter, report.rows
        ))
        .into()
    })
}

pub(super) fn assert_some_stage_arms(slug: &str) -> Result<(), HandlerError> {
    if super::jurisdiction::DISPATCHED.contains(&slug) {
        return Ok(());
    }
    Err(TerminalError::new(format!(
        "the plan calls {slug} sweepable and no stage in the chain arms it"
    ))
    .into())
}

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

pub fn collect_error(error: CrawlError) -> JobError {
    match error {
        CrawlError::Store(source) => JobError::from(source),
        CrawlError::Invariant { detail } => JobError::Terminal { message: detail },
        error @ CrawlError::Encode { .. } => JobError::Terminal {
            message: error.to_string(),
        },
        CrawlError::Schema { .. }
        | CrawlError::Decode { .. }
        | CrawlError::Canonical { .. }
        | CrawlError::Domain(..)
        | CrawlError::Arithmetic { .. }
        | CrawlError::Io { .. }
        | CrawlError::RegexInit { .. } => JobError::Terminal {
            message: error.to_string(),
        },
        CrawlError::Fetch(e) if e.retryable() => JobError::Transient {
            message: e.to_string(),
        },
        CrawlError::Fetch(e) => JobError::Terminal {
            message: e.to_string(),
        },
    }
}

pub(super) fn invariant(message: &str) -> HandlerError {
    TerminalError::new(format!("jurisdiction census invariant: {message}")).into()
}
