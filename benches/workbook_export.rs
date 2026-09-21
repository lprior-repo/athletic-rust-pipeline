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
//! `crates/midwest-census/examples/bench_store.rs` applies to its own observation counts.

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

/// The synthetic corpus, its verified source workbook, and the manifest the export is built from.
struct Dataset {
    manifest: SourceManifest,
    rows: Vec<ExportRow>,
    extra_headers: Vec<String>,
    directory: TempDir,
}

impl Dataset {
    fn build() -> Result<Self> {
        let directory = tempfile::tempdir().context("creating the bench directory")?;
        let source = directory.path().join("source.xlsx");
        write_source_workbook(&source)?;
        let digest = digest_of(&source)?;
        let extra_headers = EXPORT_HEADERS
            .iter()
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>();
        let manifest = manifest_for(directory.path(), source, digest)?;
        let rows = build_rows()?;

        // The source workbook on disk must carry the shape the manifest declares.
        verify_source_workbook(&manifest.original)?;
        // One full export must round-trip to the same cardinality before anything is measured.
        let destination = directory.path().join("verified-export.xlsx");
        let stats = export_once(&manifest, &rows, &extra_headers, &destination)?;
        verify_stats(&stats, &manifest, rows.len())?;
        verify_export_workbook(&destination, extra_headers.len())?;

        let source_bytes = fs::metadata(&manifest.original)
            .context("reading the source workbook size")?
            .len();
        let export_bytes = fs::metadata(&destination)
            .context("reading the verified export size")?
            .len();
        println!("metric=bench_dataset_rows value={ROWS} unit=rows");
        println!("metric=bench_source_bytes value={source_bytes} unit=bytes");
        println!("metric=bench_export_bytes value={export_bytes} unit=bytes");
        println!(
            "metric=bench_columns value={} unit=columns",
            SOURCE_HEADERS.len() + extra_headers.len()
        );
        println!(
            "bench dataset verified: {ROWS} rows exported and re-read over one worksheet, \
             {} header columns",
            SOURCE_HEADERS.len() + extra_headers.len()
        );
        Ok(Self {
            manifest,
            rows,
            extra_headers,
            directory,
        })
    }
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

/// A fresh export holding every manifest row: the state `finish` starts from.
fn written_export(dataset: &Dataset) -> WorkbookExport {
    let mut export = checked(WorkbookExport::new(
        &dataset.manifest,
        &dataset.extra_headers,
    ));
    for row in &dataset.rows {
        checked(export.write(row.clone()));
    }
    export
}

/// One complete export pass, used by the dataset gate.
fn export_once(
    manifest: &SourceManifest,
    rows: &[ExportRow],
    extra_headers: &[String],
    destination: &Path,
) -> Result<ExportStats> {
    let mut export = WorkbookExport::new(manifest, extra_headers)?;
    for row in rows {
        export.write(row.clone())?;
    }
    export.finish(destination)
}

/// The export must report exactly the manifest's rows, sheets, and source digest.
fn verify_stats(stats: &ExportStats, manifest: &SourceManifest, rows: usize) -> Result<()> {
    let rows = u64::try_from(rows).context("row count conversion overflow")?;
    ensure!(
        stats.aggregate_rows == rows,
        "export reported {} rows, expected {rows}",
        stats.aggregate_rows
    );
    ensure!(
        stats.sheets
            == [ExportSheetStats {
                name: SHEET.to_owned(),
                rows,
            }],
        "export reported sheets {:?}",
        stats.sheets
    );
    ensure!(
        stats.source_sha256 == manifest.workbook.as_str(),
        "export reported source digest {}",
        stats.source_sha256
    );
    Ok(())
}

/// The published XLSX must re-read to the source headers followed by the extra headers, with the
/// header row plus exactly `ROWS` data rows.
fn verify_export_workbook(path: &Path, extra_columns: usize) -> Result<()> {
    let mut workbook: Xlsx<_> = open_workbook(path).context("opening the published export")?;
    let names = workbook.sheet_names().to_owned();
    ensure!(names == [SHEET.to_owned()], "export sheets are {names:?}");
    let range = workbook
        .worksheet_range(SHEET)
        .context("reading the published export")?;
    let expected = expected_headers(extra_columns);
    ensure!(
        header_row(&range)? == expected,
        "published export headers differ from the expected source plus extra headers"
    );
    ensure!(
        range.width() == expected.len(),
        "published export is {} columns wide, expected {}",
        range.width(),
        expected.len()
    );
    ensure!(
        range.height() == ROWS + 1,
        "published export has {} rows, expected {}",
        range.height(),
        ROWS + 1
    );
    Ok(())
}

/// The synthetic source workbook must carry the headers and row count the manifest declares.
fn verify_source_workbook(path: &Path) -> Result<()> {
    let mut workbook: Xlsx<_> = open_workbook(path).context("opening the source workbook")?;
    let names = workbook.sheet_names().to_owned();
    ensure!(names == [SHEET.to_owned()], "source sheets are {names:?}");
    let range = workbook
        .worksheet_range(SHEET)
        .context("reading the source workbook")?;
    ensure!(
        header_row(&range)? == expected_headers(0),
        "source headers differ from SOURCE_HEADERS"
    );
    ensure!(
        range.height() == ROWS + 1,
        "source workbook has {} rows, expected {}",
        range.height(),
        ROWS + 1
    );
    Ok(())
}

/// The header row of a worksheet range, as text.
fn header_row(range: &Range<Data>) -> Result<Vec<String>> {
    let row = range.rows().next().context("worksheet has no header row")?;
    row.iter()
        .map(|cell| match cell {
            Data::String(text) => Ok(text.clone()),
            other => Err(anyhow::anyhow!("header cell {other:?} is not text")),
        })
        .collect()
}

/// `SOURCE_HEADERS` followed by `count` extra headers, as one worksheet's header row.
fn expected_headers(count: usize) -> Vec<String> {
    SOURCE_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .chain(
            EXPORT_HEADERS
                .iter()
                .take(count)
                .map(|header| (*header).to_owned()),
        )
        .collect()
}

/// Write the synthetic source workbook: one header row and `ROWS` deterministic data rows.
fn write_source_workbook(path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(SHEET)
        .context("naming the source worksheet")?;
    SOURCE_HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column).context("header column conversion overflow")?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .context("writing a source header")
        })?;
    for row in 0..ROWS {
        let worksheet_row = u32::try_from(row).context("row conversion overflow")?;
        SOURCE_HEADERS
            .iter()
            .enumerate()
            .try_for_each(|(column, header)| {
                let column = u16::try_from(column).context("data column conversion overflow")?;
                worksheet
                    .write_string(worksheet_row + 1, column, cell_value(row, header))
                    .map(|_| ())
                    .context("writing a source value")
            })?;
    }
    workbook
        .save(path)
        .context("saving the synthetic source workbook")
}

/// The manifest the export is built from, matching the workbook just written.
fn manifest_for(
    directory: &Path,
    source: PathBuf,
    workbook: WorkbookDigest,
) -> Result<SourceManifest> {
    let last_row = u32::try_from(ROWS)
        .context("row conversion overflow")?
        .checked_add(FIRST_DATA_ROW - 1)
        .context("last row overflow")?;
    let rows = u64::try_from(ROWS).context("row count conversion overflow")?;
    Ok(SourceManifest {
        original: source,
        frozen: directory.join("frozen.xlsx"),
        workbook,
        ingestion_revision: INGESTION_REVISION.to_owned(),
        stats: WorkbookStats {
            sheets: vec![SheetStats {
                name: SHEET.to_owned(),
                declared_dimension: Some(format!(
                    "A1:{}{last_row}",
                    column_letters(SOURCE_HEADERS.len())?
                )),
                xml_rows: u64::from(last_row),
                actual_data_rows: rows,
                last_actual_row: last_row,
                headers: SOURCE_HEADERS
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect(),
            }],
            actual_data_rows: rows,
            selected_prospects: rows,
        },
    })
}

/// The synthetic rows, in manifest order: Excel row `index + FIRST_DATA_ROW`.
fn build_rows() -> Result<Vec<ExportRow>> {
    (0..ROWS)
        .map(|row| {
            let worksheet_row = u32::try_from(row).context("row conversion overflow")?;
            let excel_row = worksheet_row
                .checked_add(FIRST_DATA_ROW)
                .context("row overflow")?;
            let fields = SOURCE_HEADERS
                .iter()
                .map(|header| ((*header).to_owned(), cell_value(row, header)))
                .collect::<BTreeMap<_, _>>();
            let extra_fields = EXPORT_HEADERS
                .iter()
                .take(FILLED_EXTRA_COLUMNS)
                .enumerate()
                .map(|(slot, header)| ((*header).to_owned(), format!("bench-{slot}-{row}")))
                .collect::<BTreeMap<_, _>>();
            Ok(ExportRow {
                source: SourceRecord {
                    source_key: format!("{SHEET}:{excel_row}"),
                    sheet: SHEET.to_owned(),
                    excel_row,
                    fields,
                },
                extra_fields,
            })
        })
        .collect()
}

/// A deterministic value for one header of one row: the same corpus on every machine and run.
fn cell_value(row: usize, header: &str) -> String {
    match header {
        "Person First" => format!("Bench{row}"),
        "Person Last" => "Runner".to_owned(),
        "Person Email" => format!("bench{row}@example.invalid"),
        "Address Mailing / Permanent City" => "Madison".to_owned(),
        "Address Mailing / Permanent Region" => "WI".to_owned(),
        "Sports Sport" => "Track & Field".to_owned(),
        "Origin Source" => "bench-harness".to_owned(),
        "Schools Name" => format!("Bench High School {}", row % 32),
        _ => format!("bench-{row}"),
    }
}

/// SHA-256 of a file, as the workbook digest the manifest carries.
fn digest_of(path: &Path) -> Result<WorkbookDigest> {
    let bytes = fs::read(path).context("reading the source workbook for its digest")?;
    let digest = Sha256::digest(&bytes);
    WorkbookDigest::parse(&format!("{digest:x}")).context("parsing the source workbook digest")
}

/// Excel column letters for a one-based column count, for the declared dimension only.
fn column_letters(count: usize) -> Result<String> {
    let offset = u8::try_from(count)
        .ok()
        .and_then(|count| count.checked_sub(1))
        .filter(|offset| *offset < 26)
        .context("declared dimension needs column letters beyond A..Z")?;
    let letter = b'A'.checked_add(offset).context("column letter overflow")?;
    Ok(char::from(letter).to_string())
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
