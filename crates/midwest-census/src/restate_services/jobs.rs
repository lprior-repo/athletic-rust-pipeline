use std::path::PathBuf;

use serde_json::Value;

use crate::report::{self, Scope};
use crate::store::{Store, Table};
use crate::{bests, workbook};

use super::wire::{BestsReply, ConsolidatedTable, ReportReply, SweepReport, WorkbookReply};
use super::{cohort_label, MAX_ROWS_PER_REQUEST};

/// Append observations for one table. Every row must carry its canonical `id`; that is what the
/// store keys the observation by.
pub fn append_observations(store: &Store, table: Table, rows: &[Value]) -> anyhow::Result<usize> {
    if rows.len() > MAX_ROWS_PER_REQUEST {
        anyhow::bail!(
            "{} rows exceeds the per-request ceiling of {MAX_ROWS_PER_REQUEST}",
            rows.len()
        );
    }
    store.append_many(table, rows)?;
    Ok(rows.len())
}

pub(super) fn consolidate_tables(
    store: &Store,
    tables: &[Table],
) -> anyhow::Result<Vec<ConsolidatedTable>> {
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

pub(super) fn build_report(store: &Store, scope: Scope) -> anyhow::Result<ReportReply> {
    let census = report::build_census(store, scope)?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    Ok(ReportReply {
        scope: scope.as_str().to_string(),
        generated_on: census.generated_on.clone(),
        totals: serde_json::to_value(&census.totals)?,
        json_path: json_path.display().to_string(),
        csv_path: csv_path.display().to_string(),
    })
}

pub(super) fn build_bests(store: &Store, options: &bests::Options) -> anyhow::Result<BestsReply> {
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
) -> anyhow::Result<WorkbookReply> {
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
) -> anyhow::Result<PathBuf> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("sweep-{today}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(report)?)?;
    Ok(path)
}
