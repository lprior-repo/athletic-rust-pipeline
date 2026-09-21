//! Commands that talk to a source host: the polite-fetcher probe, the state site list,
//! cached team indexes, the MileSplit roster walk and the coach-contact CSV import.

use anyhow::{bail, Context, Result};
use clap::Args;
use midwest_census::census;
use census_domain::model::SchoolYear;
use midwest_census::net::FetchOptions;
use midwest_census::sources::milesplit::{self, SITES};
use midwest_census::store::Store;
use std::path::Path;

use super::{build_fetcher, Cli};

/// List the registered MileSplit state sites.
pub(super) fn run_sites() -> Result<()> {
    for site in SITES {
        println!("{}\t{}", site.state, site.host);
    }
    Ok(())
}

/// Fetch a single URL through the polite fetcher (robots-enforced, cached).
pub(super) async fn run_fetch(cli: &Cli, store: &Store, url: &str, refresh: bool) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let outcome = fetcher
        .get(
            url,
            &FetchOptions {
                refresh,
                ..Default::default()
            },
        )
        .await?;
    println!(
        "status={} bytes={} from_cache={} sha256={} fetched_at={}",
        outcome.status, outcome.bytes, outcome.from_cache, outcome.sha256, outcome.fetched_at
    );
    println!("url={}", outcome.url);
    Ok(())
}

/// Fetch (and cache) team indexes for the given states.
pub(super) async fn run_teams(
    cli: &Cli,
    store: &Store,
    states: &[String],
    refresh: bool,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    for state in states {
        let teams = census::collect_state_teams(&fetcher, store, state, refresh).await?;
        println!("{state}\tteams={}", teams.len());
        for team in teams.iter().take(3) {
            println!("  {}\t{}\t{}", team.id, team.name, team.city_state);
        }
    }
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct CollectArgs {
    /// Comma-separated state codes (WI,MN,IA,IL,MI,IN,OH,MO,KS,NE,ND,SD).
    #[arg(long, value_delimiter = ',', default_value = "WI")]
    states: Vec<String>,
    /// Cap the number of rosters fetched per state (for smoke runs).
    #[arg(long)]
    limit_per_state: Option<usize>,
    /// Concurrent in-flight requests per state (per-host politeness still applies).
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    /// How many state hosts to walk simultaneously.
    #[arg(long, default_value_t = 4)]
    state_concurrency: usize,
    /// Ignore caches and re-fetch (robots still enforced).
    #[arg(long)]
    refresh: bool,
    /// School year the rosters belong to, as its starting calendar year (2026 = 2026-27).
    #[arg(long, default_value_t = 2026)]
    school_year: i16,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
}

/// Upper-case the requested state codes, reject the unknown ones and build the options.
fn collect_options(args: &CollectArgs) -> Result<census::CollectOptions> {
    let states: Vec<String> = args
        .states
        .iter()
        .map(|state| state.to_ascii_uppercase())
        .collect();
    for state in &states {
        if milesplit::site_for_state(state).is_err() {
            bail!("unknown state code {state}");
        }
    }
    Ok(census::CollectOptions {
        states,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        state_concurrency: args.state_concurrency,
        refresh: args.refresh,
        school_year: SchoolYear(args.school_year),
        observed_on: args
            .observed_on
            .clone()
            .unwrap_or_else(midwest_census::net::today_iso),
    })
}

/// Walk rosters and emit canonical entities for the given states.
pub(super) async fn run_collect(cli: &Cli, store: &Store, args: &CollectArgs) -> Result<()> {
    let options = collect_options(args)?;
    let fetcher = build_fetcher(cli, store)?;
    let report = census::collect_milesplit(&fetcher, store, &options)
        .await
        .context("milesplit collection")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Import the researched official coach-contact CSV into canonical entities.
pub(super) fn run_import_coaches(
    store: &Store,
    csv: &Path,
    observed_on: &Option<String>,
) -> Result<()> {
    let observed_on = observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let report = midwest_census::sources::coach_contacts::import_csv(store, csv, &observed_on)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
