//! Store throughput harness: append, merge-scan, and consolidate on the Fjall substrate.
//!
//! ```text
//! cargo run --release --example bench_store -- --rows 200000 --batch 1000 --scan
//! ```
//!
//! **Bound.** The harness appends `--rows` observations twice — once one at a time, once in
//! `--batch`-sized batches — so the dataset is exactly `2 * --rows` observations of the `schools`
//! table and `--rows` distinct schools, whatever the flags say. Nothing else caps the store, so a
//! large `--rows` is a deliberate operator choice: a single append pays one `fdatasync`
//! (`PersistMode::SyncData`) per call, which is the honest cost of the single-append phase.
//!
//! **Output.** `metric=<name> items=<n> unit=<unit>` lines, `metric=<name> seconds=<s>` /
//! `metric=<name> rate=<n> unit=<unit>/s` lines for each phase, and one `json={...}` summary line.
//! Every phase asserts the row counts it produced before reporting a rate, so a harness run can
//! never silently measure a store that lost rows.

use anyhow::{Context, Result};
use clap::Parser;
use midwest_census::model::{normalize_name, CanonicalSchool, Evidence, SourceRef};
use midwest_census::store::{Store, Table};
use serde::Serialize;
use serde_json::json;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DEFAULT_ROWS: usize = 200_000;
const DEFAULT_BATCH: usize = 1_000;
/// Evidence source id for every synthetic row; a core adapter id so `Scope::Core` keeps the rows.
const SOURCE_ID: &str = "bench_store";
const OBSERVED_ON_SINGLE: &str = "2026-05-01";
const OBSERVED_ON_BATCH: &str = "2026-05-02";
/// The only table this harness writes.
const TABLE: Table = Table::Schools;

#[derive(Debug, Parser)]
#[command(
    name = "bench_store",
    about = "Fjall store throughput harness: single append, batched append, scan, consolidate"
)]
struct Options {
    /// Observations per append phase; the dataset is `2 * rows` observations.
    #[arg(long, default_value_t = DEFAULT_ROWS)]
    rows: usize,
    /// Records per `append_many` call in the batched phase.
    #[arg(long, default_value_t = DEFAULT_BATCH)]
    batch: usize,
    /// Run the scan and consolidate phases (both read every observation).
    #[arg(long, default_value_t = false)]
    scan: bool,
}

/// One measured phase: how many items moved, how long it took, and the resulting rate.
#[derive(Debug, Clone, Copy, Serialize)]
struct Phase {
    items: usize,
    seconds: f64,
    rate_per_second: f64,
}

/// Everything the append passes produced, so the summary is emitted from one place.
#[derive(Debug, Clone, Copy)]
struct Measured {
    single: Phase,
    batched: Phase,
    scanned: Option<(Phase, Phase)>,
    /// Observations the store must hold once both passes ran (`2 * --rows`).
    observations: usize,
}

fn main() -> Result<()> {
    let options = Options::parse();
    anyhow::ensure!(options.rows >= 1, "--rows must be at least 1");
    anyhow::ensure!(options.batch >= 1, "--batch must be at least 1");
    let started = Instant::now();

    let dir = tempfile::tempdir().context("creating a temporary store root")?;
    let store = Store::open(dir.path()).context("opening the store")?;
    let measured = append_phases(&store, &dir, &options)?;
    emit_summary(&store, &options, &measured, started.elapsed())?;
    println!(
        "note=store root {} is removed when this process exits",
        dir.path().display()
    );
    Ok(())
}

/// One append per row, then one `append_many` per batch, then the optional read phases. Each phase
/// checks the row counts it produced before the next one starts.
fn append_phases(store: &Store, dir: &TempDir, options: &Options) -> Result<Measured> {
    // Both passes use the same names, so the second pass merges onto the first: the scan phase then
    // proves the substrate kept both observations instead of overwriting one.
    let single = append_one_by_one(store, &build_rows(options.rows, OBSERVED_ON_SINGLE))?;
    let observations = u64::try_from(single.items).context("row count does not fit u64")?;
    expect_observations(store, observations, "single appends")?;

    let batch_rows = build_rows(options.rows, OBSERVED_ON_BATCH);
    let batched = append_in_batches(store, &batch_rows, options.batch)?;
    let doubled = options
        .rows
        .checked_mul(2)
        .context("--rows is too large to double")?;
    let doubled_observations = u64::try_from(doubled).context("row count does not fit u64")?;
    expect_observations(store, doubled_observations, "batched appends")?;

    let scanned = if options.scan {
        Some(scan_and_consolidate(store, dir, options.rows)?)
    } else {
        None
    };
    Ok(Measured {
        single,
        batched,
        scanned,
        observations: doubled,
    })
}

/// Store counters, the wall time, and the machine-readable summary line.
fn emit_summary(
    store: &Store,
    options: &Options,
    measured: &Measured,
    elapsed: Duration,
) -> Result<()> {
    let stats = store.stats().context("reading store stats")?;
    println!(
        "metric=store_observations value={} unit=observations",
        stats.observations
    );
    println!(
        "metric=store_bytes_on_disk value={} unit=bytes",
        stats.bytes_on_disk
    );
    let wall = elapsed.as_secs_f64();
    println!("metric=wall_seconds value={wall:.3} unit=s");
    let summary = json!({
        "harness": "bench_store",
        "rows_per_phase": options.rows,
        "batch": options.batch,
        "scan_phase": options.scan,
        "observations": measured.observations,
        "distinct_schools": options.rows,
        "single_append": measured.single,
        "batched_append": measured.batched,
        "scan": measured.scanned.map(|(scan, _)| scan),
        "consolidate": measured.scanned.map(|(_, consolidate)| consolidate),
        "store_observations": stats.observations,
        "store_bytes_on_disk": stats.bytes_on_disk,
        "wall_seconds": wall,
    });
    println!("json={}", serde_json::to_string(&summary)?);
    Ok(())
}

/// `count` schools with distinct deterministic names. `observed_on` differs between the two passes
/// so a merged row must show two evidence entries to prove both observations survived.
fn build_rows(count: usize, observed_on: &str) -> Vec<CanonicalSchool> {
    let mut rows = Vec::with_capacity(count);
    for index in 0..count {
        let state = match index % 3 {
            0 => "WI",
            1 => "MN",
            _ => "IA",
        };
        let name = format!("Bench School {index}");
        let (mut school, _) = CanonicalSchool::new(state, name.clone(), normalize_name(&name));
        school
            .evidence
            .push(Evidence::parsed(SourceRef::id(SOURCE_ID), observed_on));
        rows.push(school);
    }
    rows
}

/// One `append` per row: the durability floor of the substrate.
fn append_one_by_one(store: &Store, rows: &[CanonicalSchool]) -> Result<Phase> {
    let started = Instant::now();
    for row in rows {
        store.append(TABLE, row).context("appending one school")?;
    }
    measure(
        "single_append",
        rows.len(),
        "observations",
        started.elapsed(),
    )
}

/// One `append_many` per `batch` rows; the loop is bounded by `rows / batch + 1`.
fn append_in_batches(store: &Store, rows: &[CanonicalSchool], batch: usize) -> Result<Phase> {
    let started = Instant::now();
    for chunk in rows.chunks(batch) {
        store
            .append_many(TABLE, chunk)
            .context("appending a batch of schools")?;
    }
    measure(
        "batched_append",
        rows.len(),
        "observations",
        started.elapsed(),
    )
}

/// Merge-scan every observation, then write the snapshot export. Both phases read all
/// `2 * distinct` observations.
fn scan_and_consolidate(store: &Store, dir: &TempDir, distinct: usize) -> Result<(Phase, Phase)> {
    let observations = distinct
        .checked_mul(2)
        .context("row count is too large to double")?;
    let started = Instant::now();
    let merged = store
        .scan::<CanonicalSchool>(TABLE)
        .context("scanning schools")?;
    let scan = measure("scan", observations, "observations", started.elapsed())?;
    anyhow::ensure!(
        merged.len() == distinct,
        "scan merged {} rows, expected {distinct}",
        merged.len()
    );
    for row in &merged {
        anyhow::ensure!(
            row.evidence.len() == 2,
            "row {} kept {} evidence entries, expected 2",
            row.id.as_str(),
            row.evidence.len()
        );
    }

    let out = dir.path().join("out").join("schools.jsonl");
    let started = Instant::now();
    let written = store
        .consolidate::<CanonicalSchool>(TABLE, &out)
        .context("consolidating schools")?
        .rows;
    let consolidate = measure("consolidate", written, "entities", started.elapsed())?;
    anyhow::ensure!(
        written == distinct,
        "consolidate wrote {written} rows, expected {distinct}"
    );
    let bytes = std::fs::metadata(&out)
        .context("reading the consolidated snapshot size")?
        .len();
    println!("metric=consolidate_bytes value={bytes} unit=bytes");
    Ok((scan, consolidate))
}

/// The store's own count for this table must be the number of observations appended so far.
fn expect_observations(store: &Store, expected: u64, phase: &str) -> Result<()> {
    let stats = store.stats().context("reading store stats")?;
    let found = stats
        .tables
        .iter()
        .find(|(table, _)| table == TABLE.file())
        .map(|(_, count)| *count)
        .unwrap_or(0);
    anyhow::ensure!(
        found == expected,
        "{phase}: store holds {found} {} observations, expected {expected}",
        TABLE.file()
    );
    Ok(())
}

/// Time a phase, print its machine-readable lines, and return the measurement.
fn measure(name: &str, items: usize, unit: &str, elapsed: Duration) -> Result<Phase> {
    let rate_per_second = per_second(items, elapsed)?;
    let seconds = elapsed.as_secs_f64();
    println!("metric={name}_items value={items} unit={unit}");
    println!("metric={name}_seconds value={seconds:.3} unit=s");
    println!("metric={name}_rate value={rate_per_second:.1} unit={unit}/s");
    Ok(Phase {
        items,
        seconds,
        rate_per_second,
    })
}

/// Items per second. Item counts are converted with `try_from` rather than a lossy cast.
fn per_second(items: usize, elapsed: Duration) -> Result<f64> {
    let items = u32::try_from(items).context("item count does not fit u32")?;
    let seconds = elapsed.as_secs_f64();
    anyhow::ensure!(seconds > 0.0, "elapsed time is not positive");
    Ok(f64::from(items) / seconds)
}
