mod browser_session;
mod capture_protocol;
mod census_doc;
mod command;
mod compare_reference;
mod cycle;
mod dispatch;
mod export_data;
mod gather;
mod installation;
mod live;
mod local_adjudication;
mod merge_coaches;
mod national;
mod nces_address_accept;
mod open_work;
mod profile;
mod provider;
mod publication_preflight;
mod publish;
mod qa_reports;
mod retained_capture;
mod retained_events;
mod retained_marks;
mod review;
mod run;
pub(super) use run::run;
mod school_address_join;
mod school_address_preflight;
mod school_names;
mod school_sites;
mod seal;
mod selected_publication;
mod serve;
mod source;
mod store;
mod survey;
mod verify;
mod verify_coaches;
use anyhow::{Context, Result};
use census_crawl::net::Fetcher;
use census_domain::model::SchoolYear;
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
    about = "Public-source-discovered Class-of-2027 TF/XC census"
)]
pub(super) struct Cli {
    #[arg(
        help = "Store root (cache, journals, entity logs and output). Offline access requires the endpoint stopped. The serve command hosts the endpoint with this root; verify reads only the immutable bundle and uses it for its default artifact path. Other pipeline commands default to the running Restate service"
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
        help = "Explicit redirect destination host (repeatable). Same-origin redirects need no grant; origin changes require an exact host or dot-bounded subdomain grant. Granted hosts remain paced at no more than 2 rps; private-address and browser-lane checks remain binding"
    )]
    #[arg(long = "authorized-host", global = true, value_name = "HOST")]
    authorized_hosts: Vec<String>,
    #[arg(
        help = "Requests one source family admits at once: at 1 the family keeps one turn across all its hosts, and above it every host of the family holds its own turn while the family still bounds in-flight work"
    )]
    #[arg(long, global = true, value_name = "N", default_value_t = census_crawl::net::DEFAULT_FAMILY_PARALLELISM)]
    source_parallelism: usize,
    #[command(subcommand)]
    command: Command,
}

pub(super) enum Route<'a> {
    Offline(&'a std::path::Path),
    Ingress(&'a str),
}

impl Cli {
    pub(super) fn store_root(&self) -> PathBuf {
        match self.store.clone() {
            Some(value) => value,
            None => PathBuf::from(DEFAULT_STORE_ROOT),
        }
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

pub(super) fn open_offline_store(root: &std::path::Path) -> Result<Store> {
    match Store::open(root) {
        Ok(store) => Ok(store),
        Err(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::OptInRequired,
        )) => {
            let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            Ok(store)
        }
        Err(error) => Err(error.into()),
    }
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
    if cli.source_parallelism == 0
        || cli.source_parallelism > census_service::restate_services::MAX_SOURCE_PARALLELISM
    {
        anyhow::bail!(
            "source_parallelism {} is outside 1..={}",
            cli.source_parallelism,
            census_service::restate_services::MAX_SOURCE_PARALLELISM
        );
    }
    Ok(Fetcher::new(
        store.http_cache_dir(),
        cli.user_agent.clone(),
        std::time::Duration::from_millis(cli.delay_ms),
        census_crawl::default_host_delays(),
        hosts,
    )?
    .with_family_budgets(census_crawl::default_family_delays())
    .with_family_parallelism(cli.source_parallelism)
    .with_origin_locks(census_service::census::DEFAULT_ORIGIN_LOCK_ROOT))
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            match tracing_subscriber::EnvFilter::try_from_default_env() {
                Ok(value) => value,
                Err(_) => tracing_subscriber::EnvFilter::new("info"),
            },
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

pub(super) fn contact_school_year(year: u16) -> Result<SchoolYear> {
    let year = i16::try_from(year)
        .with_context(|| format!("--school-year {year} is not representable"))?;
    SchoolYear::new(year).ok_or_else(|| {
        anyhow::anyhow!(
            "--school-year {year} is outside {}..={}",
            SchoolYear::MIN_START_YEAR,
            SchoolYear::MAX_START_YEAR
        )
    })
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

pub(super) fn restriction_display(jurisdictions: &[UsJurisdiction]) -> String {
    if jurisdictions.is_empty() {
        return "all".to_string();
    }
    jurisdictions
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect::<Vec<_>>()
        .join(",")
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
