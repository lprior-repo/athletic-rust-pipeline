//! The `run` subcommand: the whole cycle in one command.

use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use clap::Args;
use midwest_census::census;
use midwest_census::report;
use midwest_census::store::Store;
use std::path::PathBuf;

use super::live;

mod publish;
use publish::{
    publish_bests_and_workbook, publish_bests_and_workbook_live, publish_scope,
    publish_scope_live,
};
use super::{build_fetcher, school_year, scope_of, Cli, Route};

#[derive(Args, Debug)]
pub(super) struct RunArgs {
    /// Athletic.net athlete registry: one `athlete_id` or `athlete_id,ST` per line. Omitted, the
    /// cycle publishes whatever the store already holds.
    #[arg(long)]
    input: Option<String>,
    /// Jurisdictions for the registry, used only for lines that name no state (so exactly one).
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
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
    /// Athletic.net meet ids to pull whole (`--meets`), comma-separated. Non-empty selects the
    /// whole-meet route (two requests per meet) instead of the per-athlete registry route, and
    /// needs no `--input`.
    #[arg(long, value_delimiter = ',')]
    meets: Vec<i64>,
    /// Spend the third request per meet for the per-event type and hurdle metadata.
    #[arg(long)]
    event_metadata: bool,
    /// Cap the number of meets processed on the whole-meet route.
    #[arg(long)]
    meet_limit: Option<usize>,
    /// Workbook path (defaults to the store's own `out/` path).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Ingress origin of the local Restate server. The local census deployment when omitted.
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

/// The whole cycle in one command: gather (when a registry is given), consolidate, publish both
/// census scopes, reduce best marks, and write the workbook.
///
/// Each stage is the same code path its own subcommand uses, and every stage is resumable, so a run
/// that fails half way is continued by re-running it rather than restarted. `--store` runs every
/// stage in-process; without it the publishing stages are submitted to the running service, which is
/// the only way they can run while `midwest-serve` holds the store.
pub(super) async fn run_cycle(cli: &Cli, args: &RunArgs) -> Result<()> {
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            // Opening a store is a read; the census run is one of the paths that migrates, so it
            // imports the pre-Fjall corpus here, before its first stage reads a row.
            store.import_legacy()?;
            run_offline(cli, &store, args).await
        }
        Route::Ingress(origin) => run_live(origin, args).await,
    }
}

/// The offline cycle: every stage against the store this command opened.
async fn run_offline(cli: &Cli, store: &Store, args: &RunArgs) -> Result<()> {
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let grad_year = school_year(args.grad_year)?;
    let scope = scope_of(args.all_sources);

    match (&args.input, args.meets.is_empty()) {
        (Some(_), _) | (None, false) => {
            gather_athleticnet(cli, store, args, observed_on.clone()).await?
        }
        (None, true) => {
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

    let index = midwest_census::index::derive(store, "run", &observed_on)
        .context("deriving the durable indexes")?;
    println!(
        "index\tsource_identities={} conflicts={} reviews={} coverage={}",
        index.source_identities, index.conflicts, index.reviews, index.coverage
    );

    for scope in [report::Scope::AllSources, report::Scope::Core] {
        publish_scope(store, scope)?;
    }

    publish_bests_and_workbook(store, args, scope, grad_year)
}

/// The live cycle: every stage that has a service handler submitted to the running service.
///
/// Two stages cannot run this way, and are named rather than faked: the gather stage writes the
/// store's own cache and journals, and the index pass has no handler on the `Census` service. Both
/// are offline work that requires `--store` and a stopped `midwest-serve`.
async fn run_live(origin: &str, args: &RunArgs) -> Result<()> {
    if args.input.is_some() || !args.meets.is_empty() {
        anyhow::bail!(
            "the gather stage writes the store the running service holds: --input and --meets need --store"
        );
    }
    println!("gather\tathleticnet\tskipped (no --input): publishing what the store holds");
    let grad_year = school_year(args.grad_year)?;
    let scope = scope_of(args.all_sources);
    let tables = live::consolidate(Some(origin)).await?;
    println!(
        "consolidate\t{}",
        tables
            .iter()
            .map(|table| format!("{}={}", table.table, table.rows))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("index\tskipped (offline stage: `index --store <dir>` with midwest-serve stopped)");

    for scope in [report::Scope::AllSources, report::Scope::Core] {
        publish_scope_live(origin, scope).await?;
    }

    publish_bests_and_workbook_live(origin, args, scope, grad_year).await
}

/// Gather the Athletic.net rows the args name - the registry `--input`, whole meets `--meets`, or
/// both - and print the stage's line.
async fn gather_athleticnet(
    cli: &Cli,
    store: &Store,
    args: &RunArgs,
    observed_on: String,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    // A literal year, still passed through the domain's own constructor: the field is private so a
    // value the domain would refuse cannot be constructed anywhere else in the tree.
    let season = SchoolYear::new(2026)
        .ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?;
    let context = midwest_census::sources::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: season,
        observed_on: observed_on.clone(),
    };
    let report = midwest_census::sources::athleticnet::collect(
        &context,
        &midwest_census::sources::athleticnet::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.states.clone(),
            meets: args.meets.clone(),
            event_metadata: args.event_metadata,
            meet_limit: args.meet_limit,
        },
    )
    .await
    .with_context(|| {
        format!(
            "gathering athletic.net rows: input={:?} meets={:?}",
            args.input, args.meets
        )
    })?;
    println!(
        "gather\tathleticnet\trows={} requests={} errors={}",
        report.rows, report.requests, report.errors
    );
    for note in &report.notes {
        println!("\t{note}");
    }
    Ok(())
}

