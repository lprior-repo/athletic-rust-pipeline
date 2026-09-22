//! The publishing subcommands: consolidate the append log, build the census, reduce best
//! marks and write the workbook.

use anyhow::{Context, Result};
use clap::Args;
use midwest_census::store::Store;
use midwest_census::{bests, census, report, workbook};
use std::path::PathBuf;

use super::{cohort_label, school_year, scope_of};

/// Merge append observations into `out/*.jsonl` snapshots.
pub(super) fn run_consolidate(store: &Store) -> Result<()> {
    let counts = census::consolidate(store)?;
    for (table, count) in counts {
        println!("{table}\t{count}");
    }
    Ok(())
}

/// Print the one-line census totals for `census`.
fn print_totals(census: &report::Census) {
    println!(
        "scope={} totals: schools={} athletes={} co2027={} (boys={} girls={}) profile_url={} multisource={} coaches={}",
        census.scope,
        census.totals.schools,
        census.totals.athletes,
        census.totals.class_of_2027,
        census.totals.class_of_2027_boys,
        census.totals.class_of_2027_girls,
        census.totals.class_of_2027_with_profile_url,
        census.totals.class_of_2027_multisource,
        census.totals.coaches
    );
}

/// Compute the measured census from the store.
pub(super) fn run_report(store: &Store, print: bool, core: bool) -> Result<()> {
    let scope = if core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    let census = report::build_census(store, scope)?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    println!("wrote {}", json_path.display());
    println!("wrote {}", csv_path.display());
    print_totals(&census);
    if print {
        println!("{}", serde_json::to_string_pretty(&census)?);
    }
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct BestsArgs {
    /// Graduation year the cohort is selected by (2027 = the class of 2027).
    #[arg(long, default_value_t = 2027, conflicts_with = "all")]
    grad_year: u16,
    /// Keep only the first N rows of the reduction.
    #[arg(long)]
    limit: Option<usize>,
    /// Reduce every athlete in the core scope instead of one graduating class.
    #[arg(long)]
    all: bool,
}

/// One best mark per `(athlete, event)` for one cohort, written as `out/best-results-<cohort>.*`.
///
/// The reduction is core-scoped, exactly as the workbook's best-results sheet is: Athletic.net and
/// the AthleticLIVE derivative contribute nothing. `--all` widens the cohort to every athlete in that
/// scope instead of one graduating class, and clap rejects it alongside `--grad-year`.
pub(super) fn run_bests(store: &Store, args: &BestsArgs) -> Result<()> {
    let grad_year = if args.all {
        None
    } else {
        Some(school_year(args.grad_year)?)
    };
    let options = bests::Options {
        scope: report::Scope::Core,
        grad_year,
        limit: args.limit,
    };
    let rows = bests::build(store, &options).context("reducing the best marks")?;
    let cohort = cohort_label(grad_year);
    let (jsonl, csv) =
        bests::write(store, &rows, &cohort).context("writing the best-mark sidecars")?;
    println!("cohort={cohort} rows={}", rows.len());
    println!("wrote {}", jsonl.display());
    println!("wrote {}", csv.display());
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct WorkbookArgs {
    /// Where to write the `.xlsx` (defaults to `<store>/out/midwest-census-<generated-on>.xlsx`).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Graduation year used for the cohort sheets (2027 = the class of 2027).
    #[arg(long, default_value_t = 2027)]
    grad_year: u16,
    /// Cap the per-athlete best-mark sheet at N rows.
    #[arg(long)]
    limit: Option<usize>,
    /// Reduce the best-results sheet over every source rather than the core scope alone.
    #[arg(long)]
    all_sources: bool,
}

/// The census workbook and its sidecars, written by the crate's own Rust writer.
pub(super) fn run_workbook(store: &Store, args: &WorkbookArgs) -> Result<()> {
    let options = workbook::Options {
        grad_year: Some(school_year(args.grad_year)?),
        out: args.out.clone(),
        limit: args.limit,
        scope: scope_of(args.all_sources),
    };
    let path = workbook::build(store, &options).context("building the census workbook")?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Derive the durable indexes and report what the pass appended.
pub(super) fn run_index(store: &Store) -> Result<()> {
    let finished_on = midwest_census::net::today_iso();
    let report = midwest_census::index::derive(store, "index", &finished_on)
        .context("deriving the durable indexes")?;
    println!(
        "index\tsource_identities={} conflicts={} reviews={} coverage={} snapshots={}",
        report.source_identities,
        report.conflicts,
        report.reviews,
        report.coverage,
        report.snapshots
    );
    Ok(())
}
