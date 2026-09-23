//! The publishing subcommands: consolidate the append log, build the census, reduce best
//! marks and write the workbook.
//!
//! `report`, `bests` and `workbook` build census artifacts and run either way: with `--store` they
//! build them in-process against the store, and without it they ask the running census service for the
//! same work, which is the only path open while `midwest-serve` holds the store. `consolidate` and
//! `index` merge and derive the store's own files and are offline tools — they require the writer to
//! be stopped.

use anyhow::{Context, Result};
use clap::Args;
use midwest_census::report;
use midwest_census::restate_services::{BestsReply, WorkbookReply, WorkbookRequest};
use midwest_census::store::Store;
use midwest_census::{bests, workbook};
use std::path::PathBuf;

use super::{cohort_label, live, school_year, scope_of, Cli, Route};

/// Merge append observations into `out/*.jsonl` snapshots.
pub(super) fn run_consolidate(store: &Store) -> Result<()> {
    let counts = midwest_census::census::consolidate(store)?;
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
pub(super) struct ReportArgs {
    /// Print the census JSON to stdout as well as writing files.
    #[arg(long)]
    print: bool,
    /// Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are
    /// excluded, exactly as they are when those adapters are never registered.
    #[arg(long)]
    core: bool,
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

/// Compute the measured census: in-process with `--store`, through the service without it.
pub(super) async fn run_census_report(cli: &Cli, args: &ReportArgs) -> Result<()> {
    let scope = if args.core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => run_report(&Store::open(root)?, args.print, args.core),
        Route::Ingress(origin) => {
            let summary = live::report(Some(origin), scope).await?;
            println!("wrote {}", summary.json_path);
            println!("wrote {}", summary.csv_path);
            println!("scope={} totals={}", summary.scope, summary.totals);
            if args.print {
                println!("{}", serde_json::to_string_pretty(&summary.totals)?);
            }
            Ok(())
        }
    }
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
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

/// One best mark per `(athlete, event)` for one cohort, written as `out/best-results-<cohort>.*`.
///
/// The reduction is core-scoped, exactly as the workbook's best-results sheet is: Athletic.net and
/// the AthleticLIVE derivative contribute nothing. `--all` widens the cohort to every athlete in that
/// scope instead of one graduating class, and clap rejects it alongside `--grad-year`.
pub(super) async fn run_bests(cli: &Cli, args: &BestsArgs) -> Result<()> {
    let grad_year = if args.all {
        None
    } else {
        Some(school_year(args.grad_year)?)
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => run_bests_offline(&Store::open(root)?, args, grad_year),
        Route::Ingress(origin) => {
            let BestsReply {
                cohort,
                rows,
                jsonl,
                csv,
            } = live::bests(Some(origin), report::Scope::Core, grad_year, args.limit).await?;
            println!("cohort={cohort} rows={rows}");
            println!("wrote {jsonl}");
            println!("wrote {csv}");
            Ok(())
        }
    }
}

/// The in-process reduction, with the store this command opened itself.
fn run_bests_offline(store: &Store, args: &BestsArgs, grad_year: Option<i16>) -> Result<()> {
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
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

/// The census workbook and its sidecars, written by the crate's own Rust writer.
pub(super) async fn run_workbook(cli: &Cli, args: &WorkbookArgs) -> Result<()> {
    let grad_year = school_year(args.grad_year)?;
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let options = workbook::Options {
                grad_year: Some(grad_year),
                out: args.out.clone(),
                limit: args.limit,
                scope: scope_of(args.all_sources),
            };
            let store = Store::open(root)?;
            let path = workbook::build(&store, &options).context("building the census workbook")?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Route::Ingress(origin) => {
            let request = WorkbookRequest {
                grad_year: Some(grad_year),
                limit: args.limit,
                scope: Some(scope_of(args.all_sources).as_str().to_string()),
                out: args
                    .out
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
            };
            let WorkbookReply { path, .. } = live::workbook(Some(origin), request).await?;
            println!("wrote {path}");
            Ok(())
        }
    }
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
