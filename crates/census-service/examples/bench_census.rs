#[path = "bench_census/corpus.rs"]
mod corpus;
#[path = "bench_census/fixtures.rs"]
mod fixtures;
#[path = "bench_census/lcg.rs"]
mod lcg;
#[path = "bench_census/phases.rs"]
mod phases;

use anyhow::{Context, Result};
use census_store::Store;
use clap::Parser;
use serde::Serialize;
use serde_json::json;
use std::time::{Duration, Instant};

use crate::corpus::build_corpus;
use crate::phases::{append_corpus, bests_phase, census_phases, consolidate_phase, workbook_phase};

const DEFAULT_SCHOOLS: usize = 500;

#[derive(Debug, Parser)]
#[command(
    name = "bench_census",
    about = "Synthetic-corpus throughput harness: append, consolidate, census, bests, workbook"
)]
struct Options {
    #[arg(
        help = "Schools in the synthetic corpus; athletes, meets and performances scale from this"
    )]
    #[arg(long, default_value_t = DEFAULT_SCHOOLS)]
    schools: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Phase {
    items: usize,
    seconds: f64,
    rate_per_second: f64,
}

fn main() -> Result<()> {
    let options = Options::parse();
    anyhow::ensure!(options.schools >= 1, "--schools must be at least 1");
    let started = Instant::now();

    let dir = tempfile::tempdir().context("creating a temporary root")?;
    let store = Store::open(dir.path()).context("opening the store")?;
    let corpus = build_corpus(options.schools)?;

    let appended = append_corpus(&store, &corpus)?;
    let consolidated = consolidate_phase(&store, &corpus)?;
    let (core_census, all_sources_census) = census_phases(&store, corpus.athletes.len())?;
    let (bests_measured, best_rows) =
        bests_phase(&store, corpus.athletes.len(), corpus.performances.len())?;
    let workbook_measured = workbook_phase(&store, &dir, corpus.merged_rows())?;

    let wall = started.elapsed().as_secs_f64();
    println!("metric=wall_seconds value={wall:.3} unit=s");
    let summary = json!({
        "harness": "bench_census",
        "schools": options.schools,
        "athletes": corpus.athletes.len(),
        "meets": corpus.meets.len(),
        "distinct_events": corpus.distinct_events.len(),
        "performances": corpus.performances.len(),
        "appended_rows": corpus.appended_rows(),
        "append": appended,
        "consolidate": consolidated,
        "census_core": core_census,
        "census_all_sources": all_sources_census,
        "bests": bests_measured,
        "best_rows": best_rows,
        "workbook": workbook_measured,
        "wall_seconds": wall,
    });
    println!("json={}", serde_json::to_string(&summary)?);
    println!(
        "note=store root {} is removed when this process exits",
        dir.path().display()
    );
    Ok(())
}

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

fn per_second(items: usize, elapsed: Duration) -> Result<f64> {
    let items = u32::try_from(items).context("item count does not fit u32")?;
    let seconds = elapsed.as_secs_f64();
    anyhow::ensure!(seconds > 0.0, "elapsed time is not positive");
    Ok(f64::from(items) / seconds)
}
