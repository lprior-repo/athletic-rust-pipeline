//! The census stages that walk a source host: cached team indexes, the MileSplit meet index and the
//! roster walk, plus the coach-contact CSV import.
//!
//! Each stage runs two ways. Offline it walks the host in-process and owns the store; live it submits
//! the state's own jurisdiction object through the ingress, so the walk runs against the store the
//! running service already holds.

use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use clap::Args;
use census_service::census;
use census_service::restate_services::JurisdictionReport;
use std::path::Path;

use super::live::{self, jurisdiction_request};
use super::national::WorkflowFlags;
use super::source::print_blocked_hosts;
use super::{build_fetcher, resolve_states, school_year, Cli, Route};

#[derive(Args, Debug)]
pub(super) struct TeamsArgs {
    /// Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted).
    /// Default: WI.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be
    /// combined with `--states`.
    #[arg(long)]
    all_states: bool,
    /// Season start year: 2026 is the 2026-27 season, the same convention as `--school-year`.
    #[arg(long, default_value_t = 2026)]
    season: i16,
    /// Ignore caches and re-read every page (robots still enforced).
    #[arg(long)]
    refresh: bool,
    #[command(flatten)]
    flags: WorkflowFlags,
}

#[derive(Args, Debug)]
pub(super) struct MeetsArgs {
    /// Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted).
    /// Default: WI.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be
    /// combined with `--states`.
    #[arg(long)]
    all_states: bool,
    /// Season start year, the same convention as `--school-year` (2026 = the 2026-27 season).
    #[arg(long, default_value_t = 2026)]
    year: u16,
    /// Ignore caches and re-read every page (robots still enforced).
    #[arg(long)]
    refresh: bool,
    #[command(flatten)]
    flags: WorkflowFlags,
}

/// One state's line for `teams`: the counts its own stage wrote.
fn teams_line(report: &JurisdictionReport) -> String {
    format!(
        "{}\tteams={} rosters={} meets={} co2027={}",
        report.jurisdiction.code(),
        report.teams,
        report.rosters.athletes,
        report.meets.rows,
        report.rosters.class_of_2027
    )
}

/// One state's line for `meets`: the counts its own stage wrote.
fn meets_line(report: &JurisdictionReport) -> String {
    let meets = &report.meets;
    format!(
        "{}\tpages={}\tfetched={}\tseen={}\trows={}\tseasons={}\trepeated={}\ttruncated={}",
        report.jurisdiction.code(),
        meets.pages,
        meets.fetched,
        meets.seen,
        meets.rows,
        meets.seasons,
        meets.repeated,
        meets.truncated
    )
}

/// Fetch (and cache) team indexes for the given states.
///
/// Offline it walks each state's index in-process; live it asks that state's own jurisdiction object,
/// which runs the same stage through the store the service already holds.
pub(super) async fn run_teams(cli: &Cli, args: &TeamsArgs) -> Result<()> {
    let jurisdictions = resolve_states(args.all_states, &args.states)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?;
            for jurisdiction in &jurisdictions {
                let teams =
                    census::collect_state_teams(&fetcher, &store, *jurisdiction, args.refresh)
                        .await?;
                println!("{}\tteams={}", jurisdiction.code(), teams.len());
                for team in teams.iter().take(3) {
                    println!("  {}\t{}\t{}", team.id, team.name, team.city_state);
                }
            }
            Ok(())
        }
        Route::Ingress(origin) => {
            let season = SchoolYear::new(args.season)
                .ok_or_else(|| anyhow::anyhow!("--season {} is not a school year", args.season))?;
            let requests = jurisdictions
                .iter()
                .map(|jurisdiction| {
                    jurisdiction_request(*jurisdiction, season, &args.flags, args.refresh, None, 4)
                })
                .collect();
            live::drive_states(origin, requests, args.flags.rounds(), |report| {
                Ok(teams_line(report))
            })
            .await
        }
    }
}

/// Enumerate every published meet in each state's results index and store the rows.
pub(super) async fn run_meets(cli: &Cli, args: &MeetsArgs) -> Result<()> {
    let jurisdictions = resolve_states(args.all_states, &args.states)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?;
            let observed_on = census_crawl::net::today_iso();
            for jurisdiction in &jurisdictions {
                let census = census::collect_state_meets(
                    &fetcher,
                    &store,
                    *jurisdiction,
                    args.year,
                    &observed_on,
                    args.refresh,
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
        Route::Ingress(origin) => {
            let season = SchoolYear::new(school_year(args.year)?)
                .ok_or_else(|| anyhow::anyhow!("--year {} is not a school year", args.year))?;
            let requests = jurisdictions
                .iter()
                .map(|jurisdiction| {
                    jurisdiction_request(*jurisdiction, season, &args.flags, args.refresh, None, 4)
                })
                .collect();
            live::drive_states(origin, requests, args.flags.rounds(), |report| {
                Ok(meets_line(report))
            })
            .await
        }
    }
}

#[derive(Args, Debug)]
pub(super) struct CollectArgs {
    /// Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted).
    /// Default: WI.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Walk the census run scope: the 48 continental states plus DC (ADR-009). Cannot be
    /// combined with `--states`.
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
    #[command(flatten)]
    flags: WorkflowFlags,
}

/// Build the census options: the codes are already validated by clap's parser.
fn collect_options(args: &CollectArgs) -> Result<census::CollectOptions> {
    Ok(census::CollectOptions {
        jurisdictions: resolve_states(args.all_states, &args.states)?,
        limit_per_state: args.limit_per_state,
        concurrency: args.concurrency,
        state_concurrency: args.state_concurrency,
        refresh: args.refresh,
        school_year: SchoolYear::new(args.school_year).ok_or_else(|| {
            anyhow::anyhow!("--school-year {} is not a school year", args.school_year)
        })?,
        observed_on: args
            .observed_on
            .clone()
            .unwrap_or_else(census_crawl::net::today_iso),
    })
}

/// Walk rosters and emit canonical entities for the given states.
///
/// Offline it drives the walk in-process; live it asks each state's own jurisdiction object, which
/// runs the same roster stage through the store the service already holds. Both paths read the same
/// [`census::CollectOptions`], so the two cannot disagree about which states a run covers.
pub(super) async fn run_collect(cli: &Cli, args: &CollectArgs) -> Result<()> {
    let options = collect_options(args)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?.with_source("milesplit");
            let outcome = census::collect_milesplit(&fetcher, &store, &options).await;
            // §69: the blocked hosts are named before the report, so a run that hit a hard block never
            // reads like a complete one — whatever the walk itself returned.
            print_blocked_hosts(&fetcher).await;
            let report = outcome.context("milesplit collection")?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Route::Ingress(origin) => {
            let requests = options
                .jurisdictions
                .iter()
                .map(|jurisdiction| {
                    jurisdiction_request(
                        *jurisdiction,
                        options.school_year,
                        &args.flags,
                        options.refresh,
                        options.limit_per_state,
                        options.concurrency,
                    )
                })
                .collect();
            live::drive_states(origin, requests, args.flags.rounds(), |report| {
                serde_json::to_string_pretty(report).context("encoding one state's report")
            })
            .await
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
        .unwrap_or_else(census_crawl::net::today_iso);
    let report = census_crawl::coach_contacts::import_csv(store, csv, &observed_on)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
