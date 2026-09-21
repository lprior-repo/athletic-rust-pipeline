//! Argument definitions and per-subcommand bodies for the `midwest-census` binary.
//!
//! [`Cli`] and [`Command`] are the clap surface; [`run`] parses the arguments, opens the
//! store and dispatches to the module that owns each subcommand. The global flags stay here
//! because every subcommand reads them.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use midwest_census::net::Fetcher;
use midwest_census::report;
use midwest_census::sources::default_host_delays;
use midwest_census::store::Store;
use std::path::PathBuf;
use std::time::Duration;

mod cycle;
mod gather;
mod provider;
mod publish;
mod serve;
mod store;

use cycle::RunArgs;
use gather::CollectArgs;
use provider::ProviderArgs;
use publish::{BestsArgs, WorkbookArgs};

#[derive(Parser, Debug)]
#[command(
    name = "midwest-census",
    about = "Independent Midwest HS TF/XC recruiting census (MileSplit discovery, no broad Athletic.net crawling)"
)]
pub(super) struct Cli {
    /// Store root (HTTP cache, journals, entity logs, output snapshots).
    #[arg(long, global = true, default_value = "var/midwest-census")]
    store: PathBuf,
    /// Default per-host delay between requests, milliseconds.
    #[arg(long, global = true, default_value_t = 1000)]
    delay_ms: u64,
    /// Override the User-Agent sent with every request.
    #[arg(long, global = true)]
    user_agent: Option<String>,
    /// Operator-authorized host (repeatable). Its robots.txt rules are recorded on the run and the
    /// stats as `robots_authorized` instead of blocking requests, under the 2 rps per-host ceiling.
    /// A bare domain authorizes its subdomains. Default: every host's robots rules are enforced.
    #[arg(long = "authorized-host", global = true, value_name = "HOST")]
    authorized_hosts: Vec<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Fetch a single URL through the polite fetcher (robots-enforced, cached).
    Fetch {
        url: String,
        /// Ignore the cache and hit the network.
        #[arg(long)]
        refresh: bool,
    },
    /// List the registered MileSplit state sites.
    Sites,
    /// Fetch (and cache) team indexes for the given states.
    Teams {
        #[arg(long, value_delimiter = ',', default_value = "WI")]
        states: Vec<String>,
        #[arg(long)]
        refresh: bool,
    },
    /// Walk rosters and emit canonical entities for the given states.
    Collect(CollectArgs),
    /// Import the researched official coach-contact CSV into canonical entities.
    ImportCoaches {
        /// Path to `data/coach-contacts.csv` (research workspace artifact).
        csv: PathBuf,
        /// ISO date used when a row carries no `last_observed`.
        #[arg(long)]
        observed_on: Option<String>,
    },
    /// Run one association contact adapter by name.
    Provider(ProviderArgs),
    /// Merge append observations into `out/*.jsonl` snapshots.
    Consolidate,
    /// Compute the measured census from the store.
    Report {
        /// Print the census JSON to stdout as well as writing files.
        #[arg(long)]
        print: bool,
        /// Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are
        /// excluded, exactly as they are when those adapters are never registered.
        #[arg(long)]
        core: bool,
    },
    /// Reduce the consolidated tables to one best mark per athlete and event.
    Bests(BestsArgs),
    /// Build the census workbook (`.xlsx`) and its text sidecars.
    Workbook(WorkbookArgs),
    /// Run the whole cycle in one command: gather the authorized registry, consolidate, publish both
    /// census scopes, reduce best marks, and write the workbook.
    Run(RunArgs),
    /// Print the Fjall store's per-table observation counts and on-disk footprint.
    FjallStats,
    /// Import pre-Fjall JSONL journals into the store (one-time), then print the store stats.
    ImportLegacy,
    /// Print the command that runs the `midwest-serve` Restate endpoint.
    Serve,
}

pub(super) fn build_fetcher(cli: &Cli, store: &Store) -> Result<Fetcher> {
    Fetcher::new(
        store.http_cache_dir(),
        cli.user_agent.clone(),
        Duration::from_millis(cli.delay_ms),
        default_host_delays(),
        cli.authorized_hosts.clone(),
    )
}

/// Parse the arguments, open the store and dispatch the subcommand.
pub(super) async fn run() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let store = Store::open(&cli.store)?;

    dispatch(&cli, &store).await
}

/// Install the tracing subscriber: `RUST_LOG` when it is set, `info` otherwise.
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
}

/// Run the subcommand `cli` selected.
async fn dispatch(cli: &Cli, store: &Store) -> Result<()> {
    match &cli.command {
        Command::Sites => gather::run_sites()?,
        Command::Fetch { url, refresh } => gather::run_fetch(cli, store, url, *refresh).await?,
        Command::Teams { states, refresh } => {
            gather::run_teams(cli, store, states, *refresh).await?
        }
        Command::Collect(args) => gather::run_collect(cli, store, args).await?,
        Command::ImportCoaches { csv, observed_on } => {
            gather::run_import_coaches(store, csv, observed_on)?
        }
        Command::Provider(args) => provider::run_provider(cli, store, args).await?,
        Command::Consolidate => publish::run_consolidate(store)?,
        Command::Report { print, core } => publish::run_report(store, *print, *core)?,
        Command::Bests(args) => publish::run_bests(store, args)?,
        Command::Workbook(args) => publish::run_workbook(store, args)?,
        Command::Run(args) => cycle::run_cycle(cli, store, args).await?,
        Command::FjallStats => store::print_store_stats(store)?,
        Command::ImportLegacy => store::run_legacy_import(store)?,
        Command::Serve => serve::run_serve(cli)?,
    }
    Ok(())
}

/// `bests::write` and `workbook` agree on this label: `co2027` for one class, `all` for every cohort.
/// The scope a `--all-sources` flag selects.
pub(super) fn scope_of(all_sources: bool) -> report::Scope {
    if all_sources {
        report::Scope::AllSources
    } else {
        report::Scope::Core
    }
}

pub(super) fn cohort_label(grad_year: Option<i16>) -> String {
    grad_year.map_or_else(|| "all".to_string(), |year| format!("co{year}"))
}

/// Cohort fields are `i16`; the CLI takes `u16` so a negative year is a parse error, and rejects the
/// values above `i16::MAX` instead of truncating them.
pub(super) fn school_year(grad_year: u16) -> Result<i16> {
    i16::try_from(grad_year)
        .with_context(|| format!("--grad-year {grad_year} is not a representable year"))
}
