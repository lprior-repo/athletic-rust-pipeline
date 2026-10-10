use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_service::census;
use census_service::restate_services::JurisdictionReport;
use census_store::Store;
use clap::Args;
use std::path::Path;

use super::live::{self, jurisdiction_request};
use super::national::WorkflowFlags;
use super::source::print_blocked_hosts;
use super::{build_fetcher, resolve_states, school_year, Cli, Route};

#[derive(Args, Debug)]
pub(super) struct TeamsArgs {
    #[arg(
        help = "Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted). Default: WI"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(
        help = "Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be combined with `--states`"
    )]
    #[arg(long)]
    all_states: bool,
    #[arg(
        help = "Season start year: 2026 is the 2026-27 season, the same convention as `--school-year`"
    )]
    #[arg(long, default_value_t = 2026)]
    season: i16,
    #[arg(help = "Ignore caches and re-read every page (paced at the registry rate)")]
    #[arg(long)]
    refresh: bool,
    #[command(flatten)]
    flags: WorkflowFlags,
}

#[derive(Args, Debug)]
pub(super) struct MeetsArgs {
    #[arg(
        help = "Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted). Default: WI"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(
        help = "Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be combined with `--states`"
    )]
    #[arg(long)]
    all_states: bool,
    #[arg(
        help = "Season start year, the same convention as `--school-year` (2026 = the 2026-27 season)"
    )]
    #[arg(long, default_value_t = 2026)]
    year: u16,
    #[arg(help = "Ignore caches and re-read every page (paced at the registry rate)")]
    #[arg(long)]
    refresh: bool,
    #[command(flatten)]
    flags: WorkflowFlags,
}

fn teams_line(report: &JurisdictionReport) -> Result<String> {
    Ok(format!(
        "{}\tteams={} rosters={} meets={} co2027={}",
        report.jurisdiction.code(),
        report.teams,
        report.rosters.athletes,
        serde_json::to_string(&report.history.meets)?,
        report.rosters.class_of_2027
    ))
}

fn meets_line(report: &JurisdictionReport) -> Result<String> {
    serde_json::to_string(&report.history.meets).context("encoding historical meet frontiers")
}

pub(super) async fn run_teams(cli: &Cli, args: &TeamsArgs) -> Result<()> {
    let jurisdictions = resolve_states(args.all_states, &args.states)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?;
            for jurisdiction in &jurisdictions {
                match census::collect_state_teams(&fetcher, &store, *jurisdiction, args.refresh).await {
                    Ok(teams) => {
                        println!("{}\tteams={}", jurisdiction.code(), teams.len());
                        for team in teams.iter().take(3) {
                            println!("  {}\t{}\t{}", team.id, team.name, team.city_state);
                        }
                    }
                    Err(error) => {
                        eprintln!("{}\tteams=error\t{}", jurisdiction.code(), error);
                    }
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
                    jurisdiction_request(
                        *jurisdiction,
                        season,
                        &args.flags,
                        live::JurisdictionRun {
                            refresh: args.refresh,
                            limit_per_state: None,
                            concurrency: 4,
                            authorized_hosts: cli.authorized_hosts.clone(),
                            source_parallelism: cli.source_parallelism,
                        },
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            live::drive_states(origin, requests, args.flags.rounds(), teams_line).await
        }
    }
}

pub(super) async fn run_meets(cli: &Cli, args: &MeetsArgs) -> Result<()> {
    let jurisdictions = resolve_states(args.all_states, &args.states)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?;
            let observed_on = census_crawl::net::today_iso();
            for jurisdiction in &jurisdictions {
                let options = census_crawl::net::FetchOptions {
                    refresh: args.refresh,
                    ..Default::default()
                };
                let request =
                    census::MeetWalkRequest::new(*jurisdiction, args.year, &observed_on, &options);
                let census = census::collect_state_meets(&fetcher, &store, &request, None).await?;
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
                    jurisdiction_request(
                        *jurisdiction,
                        season,
                        &args.flags,
                        live::JurisdictionRun {
                            refresh: args.refresh,
                            limit_per_state: None,
                            concurrency: 4,
                            authorized_hosts: cli.authorized_hosts.clone(),
                            source_parallelism: cli.source_parallelism,
                        },
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            live::drive_states(origin, requests, args.flags.rounds(), meets_line).await
        }
    }
}

#[derive(Args, Debug)]
pub(super) struct CollectArgs {
    #[arg(
        help = "Comma-separated state codes (`WI,MN` is one example; every USPS code is accepted). Default: WI"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(
        help = "Walk the census run scope: the 48 continental states plus DC (ADR-009). Cannot be combined with `--states`"
    )]
    #[arg(long)]
    all_states: bool,
    #[arg(help = "Cap the number of rosters fetched per state (for smoke runs)")]
    #[arg(long)]
    limit_per_state: Option<usize>,
    #[arg(help = "Concurrent in-flight requests per state (per-host politeness still applies)")]
    #[arg(long, default_value_t = 4)]
    concurrency: usize,
    #[arg(help = "How many state hosts to walk simultaneously")]
    #[arg(long, default_value_t = 4)]
    state_concurrency: usize,
    #[arg(help = "Ignore caches and re-fetch (paced at the registry rate)")]
    #[arg(long)]
    refresh: bool,
    #[arg(
        help = "School year the rosters belong to, as its starting calendar year (2026 = 2026-27)"
    )]
    #[arg(long, default_value_t = 2026)]
    school_year: i16,
    #[arg(help = "ISO date stamped into evidence (defaults to today)")]
    #[arg(long)]
    observed_on: Option<String>,
    #[command(flatten)]
    flags: WorkflowFlags,
}

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
        observed_on: match args.observed_on.clone() {
            Some(value) => value,
            None => census_crawl::net::today_iso(),
        },
        revision: std::num::NonZeroU32::new(args.flags.revision)
            .ok_or_else(|| anyhow::anyhow!("--revision must be greater than zero"))?,
    })
}

pub(super) async fn run_collect(cli: &Cli, args: &CollectArgs) -> Result<()> {
    let options = collect_options(args)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?.with_source("milesplit");
            let outcome = census::collect_milesplit(&fetcher, &store, &options).await;
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
                        live::JurisdictionRun {
                            refresh: options.refresh,
                            limit_per_state: options.limit_per_state,
                            concurrency: options.concurrency,
                            authorized_hosts: cli.authorized_hosts.clone(),
                            source_parallelism: cli.source_parallelism,
                        },
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            live::drive_states(origin, requests, args.flags.rounds(), |report| {
                serde_json::to_string_pretty(report).context("encoding one state's report")
            })
            .await
        }
    }
}

pub(super) fn run_import_coaches(
    store: &Store,
    csv: &Path,
    observed_on: &Option<String>,
) -> Result<()> {
    let observed_on = match observed_on.clone() {
        Some(value) => value,
        None => census_crawl::net::today_iso(),
    };
    let report = census_crawl::coach_contacts::import_csv(store, csv, &observed_on)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
