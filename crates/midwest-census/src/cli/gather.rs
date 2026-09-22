//! Commands that talk to a source host: the polite-fetcher probe, the state site list,
//! cached team indexes, the MileSplit roster walk and the coach-contact CSV import.

use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use clap::Args;
use midwest_census::census;
use midwest_census::net::{FetchOptions, Fetcher};
use midwest_census::sources::milesplit::Site;
use midwest_census::store::Store;
use std::path::Path;

use super::{build_fetcher, Cli};

/// List the registered MileSplit state sites.
pub(super) fn run_sites() -> Result<()> {
    for jurisdiction in UsJurisdiction::ALL {
        let site = Site::for_jurisdiction(jurisdiction);
        println!("{}\t{}", site.code(), site.host());
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

/// Fetch (and cache) team indexes for the given jurisdictions.
pub(super) async fn run_teams(
    cli: &Cli,
    store: &Store,
    states: &[UsJurisdiction],
    refresh: bool,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    for jurisdiction in states {
        let teams = census::collect_state_teams(&fetcher, store, *jurisdiction, refresh).await?;
        println!("{}\tteams={}", jurisdiction.code(), teams.len());
        for team in teams.iter().take(3) {
            println!("  {}\t{}\t{}", team.id, team.name, team.city_state);
        }
    }
    Ok(())
}

/// Enumerate every published meet in each state's results index and store the rows.
pub(super) async fn run_meets(
    cli: &Cli,
    store: &Store,
    states: &[UsJurisdiction],
    year: u16,
    refresh: bool,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let observed_on = midwest_census::net::today_iso();
    for jurisdiction in states {
        let census = census::collect_state_meets(
            &fetcher,
            store,
            *jurisdiction,
            year,
            &observed_on,
            refresh,
        )
        .await?;
        println!(
            "{}\tpages={}\tfetched={}\tseen={}\trows={}\tseasons={}\trepeated={}\ttruncated={}",
            jurisdiction.code(),
            census.pages,
            census.fetched,
            census.seen,
            census.rows,
            census.seasons,
            census.repeated,
            census.truncated
        );
    }
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct CollectArgs {
    /// Comma-separated state codes (WI,MN,IA,IL,MI,IN,OH,MO,KS,NE,ND,SD, or any other USPS code).
    /// Default: WI.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Walk every jurisdiction (50 states + DC). Cannot be combined with `--states`.
    #[arg(long)]
    all_states: bool,
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

/// Build the census options: the codes are already validated by clap's parser.
fn collect_options(args: &CollectArgs) -> Result<census::CollectOptions> {
    Ok(census::CollectOptions {
        jurisdictions: super::resolve_states(args.all_states, &args.states)?,
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
    let fetcher = build_fetcher(cli, store)?.with_source("milesplit");
    let outcome = census::collect_milesplit(&fetcher, store, &options).await;
    // §69: the blocked hosts are named before the report, so a run that hit a hard block never reads
    // like a complete one — whatever the walk itself returned.
    print_blocked_hosts(&fetcher).await;
    let report = outcome.context("milesplit collection")?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Print one `\t`-separated line per blocked host, named with the kind of condition that stopped the
/// walk (§69).
///
/// Plain text on purpose — the machine-readable copy is the report's own `access_conditions` — and
/// read off the fetcher rather than the report so a walk that failed *after* the block still names
/// the host that refused it. A blocked host is printed even when no kind is attached to it.
pub(super) async fn print_blocked_hosts(fetcher: &Fetcher) {
    let conditions = fetcher.access_conditions().await;
    let now = midwest_census::net::now_iso8601();
    for host in fetcher.blocked_hosts(now.as_str()).await {
        match conditions.iter().find(|condition| condition.host == host) {
            Some(condition) => println!("blocked\t{host}\t{}", condition.kind.slug()),
            None => println!("blocked\t{host}"),
        }
    }
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
