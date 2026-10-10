use anyhow::{bail, Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use clap::Args;

use super::{build_fetcher, Cli};

mod arms;
mod capture_metadata;

#[derive(Args, Debug)]
pub(super) struct ProviderArgs {
    #[arg(
        help = "Adapter name, matching its registry slug: ks, wiaa, wiaa_results, ihsa, ihsa_tournament, ohsaa, mshsl, plain_names, aia, ciac, uhsaa, mpa, riil, pa_piaa, chsaa, tssaa, wayzata, athleticlive, athleticlive_results, athleticlive_athletes, athleticnet, milesplit, milesplit_results, coach_contacts, coach_directories, home_campus, sidearm_staff, bound, arbiter_orgs, tfrrs"
    )]
    name: String,
    #[arg(help = "Cap the number of schools processed (smoke runs)")]
    #[arg(long)]
    limit: Option<usize>,
    #[arg(
        help = "Restrict meet selection to this season year. Without it, all stored seasons are selected"
    )]
    #[arg(long)]
    season_year: Option<u16>,
    #[arg(
        help = "Restrict to these jurisdictions (adapters that span several states). The `milesplit` arm walks the list and defaults to Wisconsin; every other adapter reads an empty list as its own coverage, and the list must include that state or the run reports the mismatch"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(
        help = "Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be combined with `--states`"
    )]
    #[arg(long)]
    all_states: bool,
    #[arg(help = "Restrict to these archive years (result-archive adapters only)")]
    #[arg(long, value_delimiter = ',')]
    seasons: Vec<i16>,
    #[arg(help = "School names to resolve for adapters with no bulk index")]
    #[arg(long, value_delimiter = ',')]
    school_names: Vec<String>,
    #[arg(help = "Input artifact for import-style adapters")]
    #[arg(long)]
    input: Option<String>,
    #[arg(
        long,
        help = "Producer CacheMeta JSON for a retained public LIVE CSV capture"
    )]
    input_metadata: Option<String>,
    #[arg(
        help = "Athletic.net meet ids to pull whole (`--meets`), comma-separated. Non-empty selects the whole-meet route (two requests per meet) instead of the per-athlete registry route"
    )]
    #[arg(long, value_delimiter = ',')]
    meets: Vec<i64>,
    #[arg(help = "Spend the third request per meet for the per-event type and hurdle metadata")]
    #[arg(long)]
    event_metadata: bool,
    #[arg(help = "Cap the number of meets processed on the whole-meet route")]
    #[arg(long)]
    meet_limit: Option<usize>,
    #[arg(help = "Ignore cached HTTP bodies and re-fetch")]
    #[arg(long)]
    refresh: bool,
    #[arg(help = "ISO date stamped into evidence (defaults to today)")]
    #[arg(long)]
    observed_on: Option<String>,
    #[arg(
        long,
        help = "Published performance cutoff date, independent of physical acquisition"
    )]
    as_of: Option<chrono::NaiveDate>,
}

impl ProviderArgs {
    fn jurisdictions(&self) -> Result<Vec<UsJurisdiction>> {
        super::resolve_restriction(self.all_states, &self.states)
    }
}

pub(super) async fn run_provider(cli: &Cli, store: &Store, args: &ProviderArgs) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?.with_source(args.name.clone());
    let observed_on = match args.observed_on.clone() {
        Some(value) => value,
        None => census_crawl::net::today_iso(),
    };
    let context = census_crawl::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: SchoolYear::new(2026)
            .ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?,
        observed_on: observed_on.clone(),
        performance_as_of: match args.as_of {
            Some(date) => date,
            None => chrono::Utc::now().date_naive(),
        },
        recording: None,
    };
    let outcome = dispatch(&context, args, observed_on).await;
    super::source::print_blocked_hosts(&fetcher).await;
    let report = outcome.with_context(|| format!("adapter {}", args.name))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

async fn dispatch(
    context: &census_crawl::AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<census_crawl::AdapterReport> {
    match args.name.as_str() {
        "ks" => arms::ks_report(context, args, observed_on).await,
        "wiaa_results" => arms::wiaa_results_report(context, args, observed_on).await,
        "wiaa" => arms::wiaa_report(context, args, observed_on).await,
        "ihsa" => arms::ihsa_report(context, args, observed_on).await,
        "ihsa_tournament" => arms::ihsa_tournament_report(context, args, observed_on).await,
        "ohsaa" => arms::ohsaa_report(context, args, observed_on).await,
        "mshsl" => arms::mshsl_report(context, args, observed_on).await,
        "wayzata" | "wayzata_schedule" => arms::wayzata_report(context, args, observed_on).await,
        "plain_names" => arms::plain_names_report(context, args, observed_on).await,
        "aia" => arms::aia_report(context, args, observed_on).await,
        "ciac" => arms::ciac_report(context, args, observed_on).await,
        "uhsaa" => arms::uhsaa_report(context, args, observed_on).await,
        "mpa" => arms::mpa_report(context, args, observed_on).await,
        "riil" => arms::riil_report(context, args, observed_on).await,
        "pa_piaa" => arms::pa_piaa_report(context, args, observed_on).await,
        "chsaa" => arms::chsaa_report(context, args, observed_on).await,
        "tssaa" => arms::tssaa_report(context, args, observed_on).await,
        "athleticlive" => arms::athleticlive_report(context, args, observed_on).await,
        "athleticlive_results" => {
            arms::athleticlive_results_report(context, args, observed_on).await
        }
        "athleticlive_athletes" => {
            arms::athleticlive_athletes_report(context, args, observed_on).await
        }
        "athleticnet" => arms::athleticnet_report(context, args, observed_on).await,
        "milesplit" => arms::milesplit_report(context, args, observed_on).await,
        "milesplit_results" => arms::milesplit_results_report(context, args).await,
        "coach_contacts" => arms::coach_contacts_report(context.store, args, observed_on),
        "coach_directories" => {
            arms::coach_directories_report(context, args, observed_on).await
        }
        "home_campus" => arms::home_campus_report(context, args, observed_on).await,
        "sidearm_staff" => arms::sidearm_staff_report(context, args, observed_on).await,
        "bound" => arms::bound_report(context, args, observed_on).await,
        "arbiter_orgs" => arms::arbiter_orgs_report(context, args, observed_on).await,
        "tfrrs" => arms::tfrrs_report(context, args, observed_on).await,
        other => bail!(
            "unknown adapter {other}; expected one of ks, wiaa, wiaa_results, ihsa, ihsa_tournament, ohsaa, mshsl, plain_names, aia, ciac, uhsaa, mpa, riil, pa_piaa, chsaa, tssaa, wayzata, athleticlive, athleticlive_results, athleticlive_athletes, athleticnet, milesplit, milesplit_results, coach_contacts, coach_directories, home_campus, sidearm_staff, bound, arbiter_orgs, tfrrs"
        ),
    }
}
