//! The `run` subcommand: the whole cycle in one command.

use anyhow::{Context, Result};
use clap::Args;
use census_domain::model::SchoolYear;
use midwest_census::store::Store;
use midwest_census::{bests, census, report, workbook};
use std::path::PathBuf;

use super::{build_fetcher, cohort_label, school_year, scope_of, Cli};

#[derive(Args, Debug)]
pub(super) struct RunArgs {
    /// Athletic.net athlete registry: one `athlete_id` or `athlete_id,ST` per line. Omitted, the
    /// cycle publishes whatever the store already holds.
    #[arg(long)]
    input: Option<String>,
    /// State codes for the registry, used only for lines that name no state (so exactly one).
    #[arg(long, value_delimiter = ',')]
    states: Vec<String>,
    /// Cap the athletes read from the registry, and the rows each later stage writes.
    #[arg(long)]
    limit: Option<usize>,
    /// Graduation year the best-mark reduction and the workbook are built for (2027 = class of 2027).
    #[arg(long, default_value_t = 2027)]
    grad_year: u16,
    /// Reduce best marks over every source rather than the core scope alone. The core scope excludes
    /// the Athletic.net source by design, so a registry-only store reduces nothing without this.
    #[arg(long)]
    all_sources: bool,
    /// Ignore cached HTTP bodies and re-fetch.
    #[arg(long)]
    refresh: bool,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
    /// Workbook path (defaults to the store's own `out/` path).
    #[arg(long)]
    out: Option<PathBuf>,
}

/// The whole cycle in one command: gather (when a registry is given), consolidate, publish both
/// census scopes, reduce best marks, and write the workbook.
///
/// Each stage is the same code path its own subcommand uses, and every stage is resumable, so a run
/// that fails half way is continued by re-running it rather than restarted.
pub(super) async fn run_cycle(cli: &Cli, store: &Store, args: &RunArgs) -> Result<()> {
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let grad_year = school_year(args.grad_year)?;
    let scope = scope_of(args.all_sources);

    match &args.input {
        Some(input) => gather_registry(cli, store, args, input, observed_on).await?,
        None => {
            println!("gather\tathleticnet\tskipped (no --input): publishing what the store holds")
        }
    }

    let counts = census::consolidate(store).context("consolidating the store")?;
    println!(
        "consolidate\t{}",
        counts
            .iter()
            .map(|(table, count)| format!("{table}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );

    for scope in [report::Scope::AllSources, report::Scope::Core] {
        publish_scope(store, scope)?;
    }

    publish_bests_and_workbook(store, args, scope, grad_year)
}

/// Gather the Athletic.net registry `--input` names, and print the stage's line.
async fn gather_registry(
    cli: &Cli,
    store: &Store,
    args: &RunArgs,
    input: &str,
    observed_on: String,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let context = midwest_census::sources::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: SchoolYear(2026),
        observed_on: observed_on.clone(),
    };
    let report = midwest_census::sources::athleticnet::collect(
        &context,
        &midwest_census::sources::athleticnet::Options {
            input: Some(input.to_string()),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.states.clone(),
        },
    )
    .await
    .with_context(|| format!("gathering the athletic.net registry {input}"))?;
    println!(
        "gather\tathleticnet\tathletes={} requests={} errors={}",
        report.rows, report.requests, report.errors
    );
    for note in &report.notes {
        println!("\t{note}");
    }
    Ok(())
}

/// Build one scope's census, write its JSON and CSV, and print the stage's lines.
fn publish_scope(store: &Store, scope: report::Scope) -> Result<()> {
    let census = report::build_census(store, scope).context("building the census")?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    println!(
        "report\t{}\tscope={} schools={} athletes={} profile_url={} multisource={}",
        json_path.display(),
        census.scope,
        census.totals.schools,
        census.totals.athletes,
        census.totals.class_of_2027_with_profile_url,
        census.totals.class_of_2027_multisource
    );
    println!("\t{}", csv_path.display());
    Ok(())
}

/// Reduce the best marks, write them with their workbook, and print the stage's lines.
fn publish_bests_and_workbook(
    store: &Store,
    args: &RunArgs,
    scope: report::Scope,
    grad_year: i16,
) -> Result<()> {
    let bests = bests::Options {
        scope,
        grad_year: Some(grad_year),
        limit: args.limit,
    };
    let rows = bests::build(store, &bests).context("reducing the best marks")?;
    let cohort = cohort_label(Some(grad_year));
    let (jsonl, csv) = bests::write(store, &rows, &cohort).context("writing the best marks")?;
    println!(
        "bests\tcohort={cohort} rows={} scope={}\t{}",
        rows.len(),
        if args.all_sources { "all" } else { "core" },
        jsonl.display()
    );
    println!("\t{}", csv.display());

    let workbook = workbook::Options {
        grad_year: Some(grad_year),
        out: args.out.clone(),
        limit: args.limit,
        scope,
    };
    let path = workbook::build(store, &workbook).context("building the census workbook")?;
    println!("workbook\t{}", path.display());
    Ok(())
}
