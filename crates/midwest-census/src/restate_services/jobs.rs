use std::path::PathBuf;

use serde_json::Value;

use crate::report::{self, ReportError, ReportResult, Scope};
use crate::store::{Store, StoreError, StoreResult, Table};
use crate::{bests, workbook};

use super::wire::{BestsReply, ConsolidatedTable, ReportReply, SweepReport, WorkbookReply};
use super::{cohort_label, MAX_ROWS_PER_REQUEST};

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
