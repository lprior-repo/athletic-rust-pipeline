//! `midwest-census` CLI. Every subcommand is safe to re-run: HTTP bodies are cached, finished work
//! is journaled in the Fjall store, and observations are append-only, so an interrupted walk
//! continues where it stopped.
//!
//! One process owns the store at a time: the database takes an exclusive lock in
//! [`Store::open`], so a run either holds the store for its whole life or fails with the reason
//! instead of interleaving writes with another process.

#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use midwest_census::bootstrap::ServeOptions;
use midwest_census::model::SchoolYear;
use midwest_census::net::{FetchOptions, Fetcher};
use midwest_census::sources::default_host_delays;
use midwest_census::sources::milesplit::{self, SITES};
use midwest_census::store::{Store, Table};
use midwest_census::{bests, census, report, workbook};
use std::path::{Path, PathBuf};
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

#[derive(Args, Debug)]
struct ProviderArgs {
    /// Adapter name: ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names, wayzata_schedule,
    /// athleticlive, athleticlive_athletes, athleticnet.
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

#[derive(Args, Debug)]
struct BestsArgs {
    /// Graduation year the cohort is selected by (2027 = the class of 2027).
    #[arg(long, default_value_t = 2027, conflicts_with = "all")]
    grad_year: u16,
    /// Keep only the first N rows of the reduction.
    #[arg(long)]
    limit: Option<usize>,
    /// Reduce every athlete in the core scope instead of one graduating class.
    #[arg(long)]
    all: bool,
}

#[derive(Args, Debug)]
struct RunArgs {
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

#[derive(Args, Debug)]
struct WorkbookArgs {
    /// Where to write the `.xlsx` (defaults to `<store>/out/midwest-census-<generated-on>.xlsx`).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Graduation year used for the cohort sheets (2027 = the class of 2027).
    #[arg(long, default_value_t = 2027)]
    grad_year: u16,
    /// Cap the per-athlete best-mark sheet at N rows.
    #[arg(long)]
    limit: Option<usize>,
    /// Reduce the best-results sheet over every source rather than the core scope alone.
    #[arg(long)]
    all_sources: bool,
}

fn build_fetcher(cli: &Cli, store: &Store) -> Result<Fetcher> {
    Fetcher::new(
        store.http_cache_dir(),
        cli.user_agent.clone(),
        Duration::from_millis(cli.delay_ms),
        default_host_delays(),
        cli.authorized_hosts.clone(),
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
            let fetcher = build_fetcher(&cli, &store)?;
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
            let fetcher = build_fetcher(&cli, &store)?;
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
            let fetcher = build_fetcher(&cli, &store)?;
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
            let fetcher = build_fetcher(&cli, &store)?;
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
                "wayzata_schedule" => {
                    providers::wayzata::collect(
                        &context,
                        &providers::wayzata::Options {
                            years: args.seasons.clone(),
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on: Some(observed_on),
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
                "athleticnet" => {
                    providers::athleticnet::collect(
                        &context,
                        &providers::athleticnet::Options {
                            input: args.input.clone(),
                            limit: args.limit,
                            refresh: args.refresh,
                            observed_on,
                            states: args.states.clone(),
                        },
                    )
                    .await
                }
                other => bail!(
                    "unknown adapter {other}; expected one of ks, wiaa, wiaa_results, ihsa, ohsaa, mshsl, plain_names, wayzata_schedule, athleticlive, athleticlive_athletes, athleticnet"
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
                "scope={} totals: schools={} athletes={} co2027={} (boys={} girls={}) profile_url={} multisource={} coaches={}",
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
        Command::Bests(args) => run_bests(&store, args)?,
        Command::Workbook(args) => run_workbook(&store, args)?,
        Command::Run(args) => run_cycle(&cli, &store, args).await?,
        Command::FjallStats => print_store_stats(&store)?,
        Command::ImportLegacy => run_legacy_import(&store)?,
        Command::Serve => print_serve_command(&cli),
    }
    Ok(())
}

/// The whole cycle in one command: gather (when a registry is given), consolidate, publish both
/// census scopes, reduce best marks, and write the workbook.
///
/// Each stage is the same code path its own subcommand uses, and every stage is resumable, so a run
/// that fails half way is continued by re-running it rather than restarted.
async fn run_cycle(cli: &Cli, store: &Store, args: &RunArgs) -> Result<()> {
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let grad_year = school_year(args.grad_year)?;
    let scope = scope_of(args.all_sources);

    match &args.input {
        Some(input) => {
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
                    input: Some(input.clone()),
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
        }
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
    }

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

/// One best mark per `(athlete, event)` for one cohort, written as `out/best-results-<cohort>.*`.
///
/// The reduction is core-scoped, exactly as the workbook's best-results sheet is: Athletic.net and
/// the AthleticLIVE derivative contribute nothing. `--all` widens the cohort to every athlete in that
/// scope instead of one graduating class, and clap rejects it alongside `--grad-year`.
fn run_bests(store: &Store, args: &BestsArgs) -> Result<()> {
    let grad_year = if args.all {
        None
    } else {
        Some(school_year(args.grad_year)?)
    };
    let options = bests::Options {
        scope: report::Scope::Core,
        grad_year,
        limit: args.limit,
    };
    let rows = bests::build(store, &options).context("reducing the best marks")?;
    let cohort = cohort_label(grad_year);
    let (jsonl, csv) =
        bests::write(store, &rows, &cohort).context("writing the best-mark sidecars")?;
    println!("cohort={cohort} rows={}", rows.len());
    println!("wrote {}", jsonl.display());
    println!("wrote {}", csv.display());
    Ok(())
}

/// The census workbook and its sidecars, written by the crate's own Rust writer.
fn run_workbook(store: &Store, args: &WorkbookArgs) -> Result<()> {
    let options = workbook::Options {
        grad_year: Some(school_year(args.grad_year)?),
        out: args.out.clone(),
        limit: args.limit,
        scope: scope_of(args.all_sources),
    };
    let path = workbook::build(store, &options).context("building the census workbook")?;
    println!("wrote {}", path.display());
    Ok(())
}

/// `<table>\t<observations>` for every table, then the store's own totals.
fn print_store_stats(store: &Store) -> Result<()> {
    let stats = store.stats().context("reading the Fjall store stats")?;
    println!("store\t{}", store.root().display());
    for (table, observations) in &stats.tables {
        println!("{table}\t{observations}");
    }
    println!("observations\t{}", stats.observations);
    println!("bytes_on_disk\t{}", stats.bytes_on_disk);
    Ok(())
}

/// [`Store::open`] performs the one-time pre-Fjall JSONL import before a command sees the store, so
/// this reports the import source it read, then the resulting store. A table is marked imported in
/// the store's `meta` keyspace and is never imported twice; the JSONL files are left in place as the
/// record of what the database was built from.
fn run_legacy_import(store: &Store) -> Result<()> {
    for table in Table::ALL {
        let path = store.table_path(table);
        match legacy_journal_bytes(&path)? {
            Some(bytes) => println!(
                "legacy\t{}\t{bytes} bytes\t{}",
                table.file(),
                path.display()
            ),
            None => println!("legacy\t{}\tabsent", table.file()),
        }
    }
    println!(
        "legacy_journal_dir\t{}",
        store.root().join("journal").display()
    );
    print_store_stats(store)
}

/// Size of one legacy entity journal, or `None` when the pre-Fjall file was never written.
fn legacy_journal_bytes(path: &Path) -> Result<Option<u64>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

/// `bests::write` and `workbook` agree on this label: `co2027` for one class, `all` for every cohort.
/// The scope a `--all-sources` flag selects.
fn scope_of(all_sources: bool) -> report::Scope {
    if all_sources {
        report::Scope::AllSources
    } else {
        report::Scope::Core
    }
}

fn cohort_label(grad_year: Option<i16>) -> String {
    grad_year.map_or_else(|| "all".to_string(), |year| format!("co{year}"))
}

/// Cohort fields are `i16`; the CLI takes `u16` so a negative year is a parse error, and rejects the
/// values above `i16::MAX` instead of truncating them.
fn school_year(grad_year: u16) -> Result<i16> {
    i16::try_from(grad_year)
        .with_context(|| format!("--grad-year {grad_year} is not a representable year"))
}

/// `midwest-serve` owns the Restate endpoint; this reports how to start it against this store.
///
/// The flags and their values come from the service's own defaults, so the printed command cannot
/// drift from what `midwest-serve` parses.
fn print_serve_command(cli: &Cli) {
    let defaults = ServeOptions::default();
    let flags = format!(
        "--listen {} --data-dir {} --max-concurrent {} --drain-timeout {}",
        defaults.listen,
        cli.store.display(),
        defaults.max_concurrent,
        defaults.drain_timeout.as_secs()
    );
    println!("midwest-serve {flags}");
    println!("run it with: cargo run --release -p midwest-census --bin midwest-serve -- {flags}");
}
