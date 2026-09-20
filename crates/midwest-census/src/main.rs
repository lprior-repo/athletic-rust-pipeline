//! `midwest-census` CLI. Every subcommand is safe to re-run: HTTP bodies are cached, finished work
//! is journaled and entity logs are append-only, so an interrupted walk continues where it stopped.

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use midwest_census::model::SchoolYear;
use midwest_census::net::{FetchOptions, Fetcher};
use midwest_census::sources::default_host_delays;
use midwest_census::sources::milesplit::{self, SITES};
use midwest_census::store::Store;
use midwest_census::{census, report};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "midwest-census",
    about = "Independent Midwest HS TF/XC recruiting census (MileSplit discovery, no broad Athletic.net crawling)"
)]
struct Cli {
    /// Store root (HTTP cache, journals, entity logs, output snapshots).
    #[arg(long, global = true, default_value = "var/midwest-census")]
    store: PathBuf,
    /// Default per-host delay between requests, milliseconds.
    #[arg(long, global = true, default_value_t = 1000)]
    delay_ms: u64,
    /// Override the User-Agent sent with every request.
    #[arg(long, global = true)]
    user_agent: Option<String>,
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
    /// Merge append logs into `out/*.jsonl` snapshots.
    Consolidate,
    /// Compute the measured census from consolidated snapshots.
    Report {
        /// Print the census JSON to stdout as well as writing files.
        #[arg(long)]
        print: bool,
        /// Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are
        /// excluded, exactly as they are when those adapters are never registered.
        #[arg(long)]
        core: bool,
    },
}

#[derive(Args, Debug)]
struct ProviderArgs {
    /// Adapter name: ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names.
    name: String,
    /// Cap the number of schools processed (smoke runs).
    #[arg(long)]
    limit: Option<usize>,
    /// Restrict to these state codes (adapters that span several states).
    #[arg(long, value_delimiter = ',')]
    states: Vec<String>,
    /// Restrict to these archive years (result-archive adapters only).
    #[arg(long, value_delimiter = ',')]
    seasons: Vec<i16>,
    /// School names to resolve for adapters with no bulk index.
    #[arg(long, value_delimiter = ',')]
    school_names: Vec<String>,
    /// Input artifact for import-style adapters.
    #[arg(long)]
    input: Option<String>,
    /// Ignore cached HTTP bodies and re-fetch.
    #[arg(long)]
    refresh: bool,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
}

#[derive(Args, Debug)]
struct CollectArgs {
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

fn build_fetcher(cli: &Cli) -> Result<Fetcher> {
    let store = Store::open(&cli.store)?;
    Fetcher::new(
        store.http_cache_dir(),
        cli.user_agent.clone(),
        Duration::from_millis(cli.delay_ms),
        default_host_delays(),
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let store = Store::open(&cli.store)?;

    match &cli.command {
        Command::Sites => {
            for site in SITES {
                println!("{}\t{}", site.state, site.host);
            }
        }
        Command::Fetch { url, refresh } => {
            let fetcher = build_fetcher(&cli)?;
            let outcome = fetcher
                .get(
                    url,
                    &FetchOptions {
                        refresh: *refresh,
                        ..Default::default()
                    },
                )
                .await?;
            println!(
                "status={} bytes={} from_cache={} sha256={} fetched_at={}",
                outcome.status,
                outcome.bytes,
                outcome.from_cache,
                outcome.sha256,
                outcome.fetched_at
            );
            println!("url={}", outcome.url);
        }
        Command::Teams { states, refresh } => {
            let fetcher = build_fetcher(&cli)?;
            for state in states {
                let teams = census::collect_state_teams(&fetcher, &store, state, *refresh).await?;
                println!("{state}\tteams={}", teams.len());
                for team in teams.iter().take(3) {
                    println!("  {}\t{}\t{}", team.id, team.name, team.city_state);
                }
            }
        }
        Command::Collect(args) => {
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
            let fetcher = build_fetcher(&cli)?;
            let options = census::CollectOptions {
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
            };
            let report = census::collect_milesplit(&fetcher, &store, &options)
                .await
                .context("milesplit collection")?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ImportCoaches { csv, observed_on } => {
            let observed_on = observed_on
                .clone()
                .unwrap_or_else(midwest_census::net::today_iso);
            let report =
                midwest_census::sources::coach_contacts::import_csv(&store, csv, &observed_on)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Provider(args) => {
            let fetcher = build_fetcher(&cli)?;
            let observed_on = args
                .observed_on
                .clone()
                .unwrap_or_else(midwest_census::net::today_iso);
            let context = midwest_census::sources::AdapterContext {
                fetcher: &fetcher,
                store: &store,
                refresh: args.refresh,
                school_year: SchoolYear(2026),
                observed_on: observed_on.clone(),
            };
            use midwest_census::sources as providers;
            let report = match args.name.as_str() {
                "ks" => {
                    providers::ks::collect(
                        &context,
                        &providers::ks::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "wiaa_results" => {
                    providers::wiaa_results::collect(
                        &context,
                        &providers::wiaa_results::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            seasons: args.seasons.clone(),
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "wiaa" => {
                    providers::wiaa::collect(
                        &context,
                        &providers::wiaa::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "ihsa" => {
                    providers::ihsa::collect(
                        &context,
                        &providers::ihsa::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "ohsaa" => {
                    providers::ohsaa::collect(
                        &context,
                        &providers::ohsaa::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "mshsl" => {
                    providers::mshsl::collect(
                        &context,
                        &providers::mshsl::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "plain_names" => {
                    providers::plain_names::collect(
                        &context,
                        &providers::plain_names::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "athleticlive" => {
                    providers::athleticlive::collect(
                        &context,
                        &providers::athleticlive::Options {
                            input: args.input.clone(),
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                "athleticlive_athletes" => {
                    providers::athleticlive_athletes::collect(
                        &context,
                        &providers::athleticlive_athletes::Options {
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                            school_names: args.school_names.clone(),
                        },
                    )
                    .await
                }
                other => bail!(
                    "unknown adapter {other}; expected one of ks, wiaa, ihsa, ohsaa, mshsl, plain_names, athleticlive, athleticlive_athletes"
                ),
            }
            .with_context(|| format!("adapter {}", args.name))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Consolidate => {
            let counts = census::consolidate(&store)?;
            for (table, count) in counts {
                println!("{table}\t{count}");
            }
        }
        Command::Report { print, core } => {
            let scope = if *core {
                report::Scope::Core
            } else {
                report::Scope::AllSources
            };
            let census = report::build_census(&store, scope)?;
            let (json_path, csv_path) = report::write_census(&store, &census, scope)?;
            println!("wrote {}", json_path.display());
            println!("wrote {}", csv_path.display());
            println!(
                "scope={} totals: schools={} athletes={} co2027={} (boys={} girls={}) profile_url={} mult_isource={} coaches={}",
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
            if *print {
                println!("{}", serde_json::to_string_pretty(&census)?);
            }
        }
    }
    Ok(())
}
