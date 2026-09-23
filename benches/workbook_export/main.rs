//! Committed measurement for the root workbook export.
//!
//! ```text
//! cargo bench -p athletic-rust-pipeline --bench workbook_export
//! ```
//!
//! **What is measured.** `WorkbookExport::{new, write, finish}` (`src/workbook_export/`), the root
//! half of the "workbook build" metric:
//!
//! * `new` — per-file cost: the source workbook is re-hashed (`snapshot::verify`) and every
//!   worksheet's export state is constructed.
//! * `write` — per-row throughput: one validated row written into its worksheet, reported as
//!   rows/s over a fresh export (`WRITE_BATCH` rows per iteration, plus a whole-manifest variant).
//! * `finish` — per-file cost: the workbook is saved to a temporary file, flushed and fsynced, the
//!   source is re-hashed, and the export is published without clobbering.
//!
//! **Dataset.** One synthetic worksheet (`Prospects`) carrying every `SOURCE_HEADERS` column and
//! `ROWS` data rows (Excel rows 2..=ROWS+1), written to a real XLSX file and digested for the
//! manifest, with the production `EXPORT_HEADERS` as extra columns. Every value is derived from
//! the row index, so two runs measure the same corpus; nothing here reads the network or a fixture
//! profile.
//!
//! **Self-asserting counts.** `Dataset::build` refuses to hand the benches a dataset it has not
//! round-tripped: it re-reads the synthetic source with calamine (sheet name, headers, data-row
//! count), exports every row once, requires `ExportStats` to equal the manifest cardinality, and
//! re-reads the published XLSX back to the same header/row shape. No rate is reported until that
//! pass succeeded, so the benches cannot measure a dataset that lost rows — the discipline
//! `crates/census-service/examples/bench_store.rs` applies to its own observation counts.

mod dataset;
mod verify;

use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::WorkbookDigest,
    model::{SheetStats, SourceRecord, WorkbookStats, SOURCE_HEADERS},
    runtime::{
        export::EXPORT_HEADERS,
        import::{SourceManifest, INGESTION_REVISION},
    },
    workbook_export::{ExportRow, ExportSheetStats, ExportStats, WorkbookExport},
};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use criterion::{BatchSize, Criterion, Throughput};
use rust_xlsxwriter::Workbook;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

use self::dataset::{written_export, Dataset};

/// Worksheet name of the synthetic source workbook.
const SHEET: &str = "Prospects";
/// Manifest data rows: every reported rate is normalised against this cardinality.
const ROWS: usize = 20_000;
/// Rows written per measured `write` iteration; its throughput is reported per row.
const WRITE_BATCH: usize = 512;
/// First Excel row carrying data; row 1 is the header row.
const FIRST_DATA_ROW: u32 = 2;
/// Extra columns filled on every synthetic row. The remaining `EXPORT_HEADERS` stay empty, exactly
/// as a partially assessed row reaches the export.
const FILLED_EXTRA_COLUMNS: usize = 2;

fn main() {
    let dataset = Dataset::build()
        .unwrap_or_else(|error| panic!("workbook_export dataset is not usable: {error:#}"));
    let mut criterion = Criterion::default().configure_from_args();
    bench_new(&mut criterion, &dataset);
    bench_write(&mut criterion, &dataset);
    bench_finish(&mut criterion, &dataset);
    criterion.final_summary();
}

/// `WorkbookExport::new`: per-file construction cost (source re-hash plus worksheet state).
fn bench_new(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("workbook_export/new");
    group.throughput(Throughput::Elements(1));
    group.bench_function("manifest", |bencher| {
        bencher.iter(|| {
            let export = checked(WorkbookExport::new(
                &dataset.manifest,
                &dataset.extra_headers,
            ));
            std::hint::black_box(&export);
        })
    });
    group.finish();
}

/// `WorkbookExport::write`: per-row throughput, and the whole manifest written per iteration.
fn bench_write(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("workbook_export/write");
    group.throughput(Throughput::Elements(row_count(WRITE_BATCH)));
    group.bench_function("row", |bencher| {
        bencher.iter_batched(
            || {
                let export = checked(WorkbookExport::new(
                    &dataset.manifest,
                    &dataset.extra_headers,
                ));
                let rows = dataset
                    .rows
                    .iter()
                    .take(WRITE_BATCH)
                    .cloned()
                    .collect::<Vec<_>>();
                (export, rows)
            },
            |(mut export, rows)| {
                for row in rows {
                    checked(export.write(row));
                }
                std::hint::black_box(&export);
            },
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Elements(row_count(ROWS)));
    group.bench_function("manifest", |bencher| {
        bencher.iter_batched(
            || {
                let export = checked(WorkbookExport::new(
                    &dataset.manifest,
                    &dataset.extra_headers,
                ));
                (export, dataset.rows.clone())
            },
            |(mut export, rows)| {
                for row in rows {
                    checked(export.write(row));
                }
                std::hint::black_box(&export);
            },
            BatchSize::PerIteration,
        )
    });
    group.finish();
}

/// `WorkbookExport::finish`: per-file publication cost, excluding the row writes that precede it.
///
/// Each iteration publishes to a fresh path (publication refuses to clobber), and the previous
/// iteration's file is removed during setup so the bench directory stays bounded and the timed
/// region stays free of cleanup work.
fn bench_finish(criterion: &mut Criterion, dataset: &Dataset) {
    let mut group = criterion.benchmark_group("workbook_export/finish");
    group.throughput(Throughput::Elements(1));
    group.bench_function("publish", |bencher| {
        let mut published = 0usize;
        let mut previous: Option<PathBuf> = None;
        bencher.iter_batched(
            || {
                if let Some(export) = previous.take() {
                    discard(&export);
                }
                let export = written_export(dataset);
                published = published.saturating_add(1);
                let destination = dataset
                    .directory
                    .path()
                    .join(format!("finish-{published:04}.xlsx"));
                previous = Some(destination.clone());
                (export, destination)
            },
            |(export, destination)| {
                let stats = checked(export.finish(&destination));
                std::hint::black_box(stats);
            },
            BatchSize::PerIteration,
        );
        if let Some(export) = previous.take() {
            discard(&export);
        }
    });
    group.finish();
}

/// A row count as the `u64` a criterion throughput declaration needs.
fn row_count(rows: usize) -> u64 {
    u64::try_from(rows).unwrap_or(u64::MAX)
}

/// Remove a previous iteration's published export, tolerating an already-absent file.
fn discard(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => panic!("removing a previous bench export failed: {error}"),
    }
}

/// Fail the measurement when the workload itself fails rather than reporting a rate for it.
fn checked<T, E>(result: std::result::Result<T, E>) -> T
where
    E: std::fmt::Display,
{
    result.unwrap_or_else(|error| panic!("workbook export workload failed: {error}"))
}
