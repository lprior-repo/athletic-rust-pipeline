use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_service::census;
use census_store::Store;
use clap::Args;

use super::super::live::{self, jurisdiction_request};
use super::super::national::WorkflowFlags;
use super::super::source::{combine_access, print_blocked_hosts};
use super::super::{build_fetcher, resolve_states, Cli, Route};

#[derive(Args, Debug)]
pub(in crate::cli) struct CollectArgs {
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

pub(in crate::cli) async fn run_collect(cli: &Cli, args: &CollectArgs) -> Result<()> {
    let options = collect_options(args)?;
    match cli.route(args.flags.ingress.as_deref())? {
        Route::Offline(root) => {
            let store = Store::open(root)?;
            let fetcher = build_fetcher(cli, &store)?.with_source("milesplit");
            let outcome = census::collect_milesplit(&fetcher, &store, &options).await;
            let access = print_blocked_hosts(&fetcher).await;
            let outcome = combine_access(outcome, access);
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
