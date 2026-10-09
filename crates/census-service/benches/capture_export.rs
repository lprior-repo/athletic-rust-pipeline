#[path = "capture_export/corpus.rs"]
mod corpus;
#[path = "capture_export/csv.rs"]
mod csv;
#[path = "capture_export/ingest.rs"]
mod ingest;
#[path = "capture_export/oracle.rs"]
mod oracle;
#[path = "capture_export/records.rs"]
mod records;
#[path = "capture_export/xlsx.rs"]
mod xlsx;

use anyhow::{Context, Result};
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::{self, publication};
use census_store::Store;
use criterion::{Criterion, Throughput};
use oracle::Oracle;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};

fn main() {
    or_fatal(run());
}

fn run() -> Result<()> {
    corpus::report()?;
    let oracle = Oracle::load()?;
    let runtime = Builder::new_current_thread().enable_all().build()?;
    operation(&runtime, &oracle)?;
    println!("capture_export oracle verified: 185 canonical rows, 60 cohort members; XLSX/CSV/JSONL authoritative readback passed");
    let mut criterion = Criterion::default().configure_from_args();
    benchmark(&mut criterion, &runtime, &oracle);
    criterion.final_summary();
    Ok(())
}

fn benchmark(criterion: &mut Criterion, runtime: &Runtime, oracle: &Oracle) {
    let mut group = criterion.benchmark_group("pipeline/capture_export");
    group.throughput(Throughput::Elements(196));
    group.bench_function("captured_live_wiaa_co2027", |bencher| {
        bencher.iter_custom(|iterations| {
            or_fatal((0..iterations).try_fold(Duration::ZERO, |total, _| {
                total
                    .checked_add(operation(runtime, oracle)?)
                    .context("benchmark duration overflow")
            }))
        })
    });
    group.finish();
}

fn operation(runtime: &Runtime, oracle: &Oracle) -> Result<Duration> {
    let directory = tempfile::tempdir()?;
    let started = Instant::now();
    let store = Store::open(directory.path())?;
    ingest::run(&store, runtime)?;
    let dataset = ExportDataset::load(&store)?;
    let options = options(directory.path())?;
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir());
    let path = workbook::build_from(&dataset, &store, &options, &censuses)?;
    oracle.verify(&dataset)?;
    publication::verify_published(&path)?;
    xlsx::verify(&path, oracle)?;
    csv::verify(
        path.parent()
            .context("published workbook has no generation directory")?,
        oracle,
    )?;
    let elapsed = started.elapsed();
    drop(dataset);
    drop(censuses);
    drop(store);
    directory.close()?;
    Ok(elapsed)
}

fn options(root: &Path) -> Result<workbook::Options> {
    Ok(workbook::Options {
        grad_year: Some(2027),
        out: Some(root.join("publication")),
        limit: None,
        scope: Scope::AllSources,
        school_year: ingest::school_year()?,
    })
}

fn or_fatal<T>(result: Result<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("capture_export benchmark refused: {error:#}");
            std::process::exit(1);
        }
    }
}
