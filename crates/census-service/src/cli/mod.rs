mod browser_session;
mod census_doc;
mod command;
mod cycle;
mod dispatch;
mod export_data;
mod gather;
mod live;
mod merge_coaches;
mod national;
mod open_work;
mod provider;
mod publish;
mod qa_reports;
mod review;
mod school_names;
mod seal;
mod serve;
mod source;
mod store;
mod verify;
mod verify_coaches;
use anyhow::{Context, Result};
use census_crawl::net::Fetcher;
use census_domain::UsJurisdiction;
use census_report::report;
use census_service::ingress;
use census_store::Store;
use clap::Parser;
use std::path::PathBuf;

use command::Command;

pub(super) const DEFAULT_STORE_ROOT: &str = "var/census-service";

#[derive(Parser, Debug)]
#[command(
    name = "census-service",
    about = "Independent Midwest HS TF/XC recruiting census (MileSplit discovery, no broad Athletic.net crawling)"
)]
pub(super) struct Cli {
    #[arg(
        help = "Store root (HTTP cache, journals, entity logs, output snapshots). Naming it selects the offline path: this command opens the store itself, which requires `census-serve` stopped, because a Fjall store has one writer. Omitted, a pipeline command submits its work to the running service through the Restate ingress instead and opens nothing"
    )]
    #[arg(long, global = true, value_name = "DIR")]
    store: Option<PathBuf>,
    #[arg(help = "Default per-host delay between requests, milliseconds")]
    #[arg(long, global = true, default_value_t = 1000)]
    delay_ms: u64,
    #[arg(help = "Override the User-Agent sent with every request")]
    #[arg(long, global = true)]
    user_agent: Option<String>,
    #[arg(
        help = "Operator-authorized host (repeatable). Its robots.txt rules are recorded on the run and the stats as `robots_authorized` instead of blocking requests, under the 2 rps per-host ceiling. A bare domain authorizes its subdomains. Default: every host's robots rules are enforced"
    )]
    #[arg(long = "authorized-host", global = true, value_name = "HOST")]
    authorized_hosts: Vec<String>,
    #[command(subcommand)]
    command: Command,
}

pub(super) enum Route<'a> {
    Offline(&'a std::path::Path),
    Ingress(&'a str),
}

impl Cli {
    pub(super) fn store_root(&self) -> PathBuf {
        self.store
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_ROOT))
    }

    pub(super) fn route<'a>(&'a self, ingress: Option<&'a str>) -> Result<Route<'a>> {
        match (self.store.as_deref(), ingress) {
            (Some(store), None) => Ok(Route::Offline(store)),
            (None, given) => Ok(Route::Ingress(ingress::origin(given))),
            (Some(_), Some(_)) => anyhow::bail!(
                "--store selects the offline path and --ingress the live one: give one of the two"
            ),
        }
    }

    pub(super) fn service_origin<'a>(
        &self,
        command: &str,
        ingress: Option<&'a str>,
    ) -> Result<&'a str> {
        if self.store.is_some() {
            anyhow::bail!(
                "`{command}` drives the census pipeline, and a pipeline command never opens the \
                 store: drop --store and let it submit through --ingress, which defaults to \
                 {}",
                ingress::DEFAULT_ORIGIN
            );
        }
        Ok(ingress::origin(ingress))
    }
}

pub(super) fn build_fetcher(cli: &Cli, store: &Store) -> Result<Fetcher> {
    build_fetcher_authorizing(cli, store, Vec::new())
}

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
        census_crawl::default_host_delays(),
        hosts,
    )?
    .with_family_budgets(census_crawl::default_family_delays()))
}

pub(super) async fn run() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match &cli.command {
        Command::National(args) => national::run_national(&cli, args).await,
        Command::Jurisdiction(args) => national::run_jurisdiction(&cli, args).await,
        Command::NationalReport(args) => national::run_national_report(&cli, args).await,
        Command::OpenWork(args) => open_work::run_open_work(&cli, args).await,
        Command::BrowserSession(args) => browser_session::run_browser_session(&cli, args).await,
        Command::Seal(args) => seal::run_seal(&cli, args).await,
        Command::Teams(args) => gather::run_teams(&cli, args).await,
        Command::Meets(args) => gather::run_meets(&cli, args).await,
        Command::Collect(args) => gather::run_collect(&cli, args).await,
        Command::Report(args) => publish::run_census_report(&cli, args).await,
        Command::Bests(args) => publish::run_bests(&cli, args).await,
        Command::Workbook(args) => publish::run_workbook(&cli, args).await,
        Command::Run(args) => cycle::run_cycle(&cli, args).await,
        Command::MergeCoaches(args) => merge_coaches::run_merge_coaches(args),
        Command::VerifyCoaches(args) => verify_coaches::run_verify_coaches(args).await,
        Command::CensusDoc(args) => census_doc::run_census_doc(args),
        Command::StoreBackup(args) => store::run_backup(&cli.store_root(), args),
        Command::Serve => serve::run_serve(&cli),
        _ => {
            let store = Store::open(cli.store_root())?;
            dispatch::dispatch(&cli, &store).await
        }
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();
}

pub(super) fn cohort_label(grad_year: Option<i16>) -> String {
    grad_year.map_or_else(|| "all".to_string(), |year| format!("co{year}"))
}

pub(super) fn scope_of(core: bool) -> report::Scope {
    if core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    }
}

pub(super) fn school_year(grad_year: u16) -> Result<i16> {
    i16::try_from(grad_year)
        .with_context(|| format!("--grad-year {grad_year} is not a representable year"))
}

pub(super) fn resolve_states(
    all_states: bool,
    states: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>> {
    match (all_states, states.is_empty()) {
        (true, false) => anyhow::bail!("--all-states cannot be combined with --states"),
        (true, true) => Ok(UsJurisdiction::CENSUS_SCOPE.to_vec()),
        (false, true) => Ok(vec![UsJurisdiction::Wisconsin]),
        (false, false) => within_census_scope(states),
    }
}

pub(super) fn resolve_restriction(
    all_states: bool,
    states: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>> {
    match (all_states, states.is_empty()) {
        (true, false) => anyhow::bail!("--all-states cannot be combined with --states"),
        (true, true) => Ok(UsJurisdiction::CENSUS_SCOPE.to_vec()),
        (false, _) => within_census_scope(states),
    }
}

pub(crate) fn within_census_scope(states: &[UsJurisdiction]) -> Result<Vec<UsJurisdiction>> {
    states
        .iter()
        .copied()
        .map(UsJurisdiction::require_census_scope)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;
