//! Argument definitions and per-subcommand bodies for the `midwest-census` binary.
//!
//! [`Cli`] and [`Command`] are the clap surface; [`run`] parses the arguments, opens the
//! store and dispatches to the module that owns each subcommand. The global flags stay here
//! because every subcommand reads them.

mod census_doc;
mod command;
mod cycle;
mod dispatch;
mod export_data;
mod gather;
mod ingress;
mod merge_coaches;
mod national;
mod provider;
mod publish;
mod qa_reports;
mod review;
mod school_names;
mod seal;
mod serve;
mod store;
mod verify;
mod verify_coaches;
use anyhow::{Context, Result};
use census_domain::UsJurisdiction;
use clap::Parser;
use midwest_census::net::Fetcher;
use midwest_census::report;
use midwest_census::store::Store;
use std::path::PathBuf;

use command::Command;

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

pub(super) fn build_fetcher(cli: &Cli, store: &Store) -> Result<Fetcher> {
    build_fetcher_authorizing(cli, store, Vec::new())
}

/// The same fetcher with additional hosts named as authorized: the caller has stated that the
/// collection was commissioned for those hosts, so robots refusals there are counted as
/// `robots_authorized` and the requests proceed under the fetcher's per-host ceiling instead.
pub(super) fn build_fetcher_authorizing(
    cli: &Cli,
    store: &Store,
    mut hosts: Vec<String>,
) -> Result<Fetcher> {
    hosts.extend(cli.authorized_hosts.iter().cloned());
    hosts.retain(|host| !host.trim().is_empty());
    hosts.sort();
    hosts.dedup();
    Ok(Fetcher::new(
        store.http_cache_dir(),
        cli.user_agent.clone(),
        std::time::Duration::from_millis(cli.delay_ms),
        midwest_census::sources::default_host_delays(),
        hosts,
    )?
    .with_family_budgets(midwest_census::sources::default_family_delays()))
}

/// Parse the arguments, open the store and dispatch the subcommand.
pub(super) async fn run() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match &cli.command {
        Command::National(args) => return national::run_national(args).await,
        Command::Jurisdiction(args) => return national::run_jurisdiction(args).await,
        Command::MergeCoaches(args) => return merge_coaches::run_merge_coaches(args),
        Command::VerifyCoaches(args) => return verify_coaches::run_verify_coaches(args).await,
        Command::CensusDoc(args) => return census_doc::run_census_doc(args),
        _ => {}
    }
    let store = Store::open(&cli.store)?;
    dispatch::dispatch(&cli, &store).await
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

/// The jurisdictions a gather command covers.
pub(super) fn resolve_states(
    all_states: bool,
    states: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>> {
    match (all_states, states.is_empty()) {
        (true, false) => anyhow::bail!("--all-states cannot be combined with --states"),
        (true, true) => Ok(UsJurisdiction::ALL.to_vec()),
        (false, true) => Ok(vec![UsJurisdiction::Wisconsin]),
        (false, false) => Ok(states.to_vec()),
    }
}

/// The jurisdictions a *restriction* flag covers, for adapters that have their own home coverage.
pub(super) fn resolve_restriction(
    all_states: bool,
    states: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>> {
    match (all_states, states.is_empty()) {
        (true, false) => anyhow::bail!("--all-states cannot be combined with --states"),
        (true, true) => Ok(UsJurisdiction::ALL.to_vec()),
        (false, _) => Ok(states.to_vec()),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;
