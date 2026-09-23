//! The `provider` subcommand: one association contact adapter per name.
//!
//! Each name has its own `Options` shape, so the dispatch is a plain match over the name with
//! no trait indirection.

use anyhow::{bail, Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use clap::Args;
use midwest_census::store::Store;

use super::{build_fetcher, Cli};

mod arms;

#[derive(Args, Debug)]
pub(super) struct ProviderArgs {
    /// Adapter name, matching its registry slug: ks, wiaa, wiaa_results, ihsa, ihsa_tournament,
    /// ohsaa, mshsl, plain_names, wayzata, athleticlive, athleticlive_athletes,
    /// athleticlive_results, athleticnet, milesplit, milesplit_results, coach_contacts.
    name: String,
    /// Cap the number of schools processed (smoke runs).
    #[arg(long)]
    limit: Option<usize>,
    /// Restrict to these jurisdictions (adapters that span several states). The `milesplit` arm
    /// walks the list and defaults to Wisconsin; every other adapter reads an empty list as its own
    /// coverage, and the list must include that state or the run reports the mismatch.
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    /// Cover the census run scope: the 48 continental states plus DC (ADR-009). Cannot be
    /// combined with `--states`.
    #[arg(long)]
    all_states: bool,
    /// Restrict to these archive years (result-archive adapters only).
    #[arg(long, value_delimiter = ',')]
    seasons: Vec<i16>,
    /// School names to resolve for adapters with no bulk index.
    #[arg(long, value_delimiter = ',')]
    school_names: Vec<String>,
    /// Input artifact for import-style adapters.
    #[arg(long)]
    input: Option<String>,
    /// Athletic.net meet ids to pull whole (`--meets`), comma-separated. Non-empty selects the
    /// whole-meet route (two requests per meet) instead of the per-athlete registry route.
    #[arg(long, value_delimiter = ',')]
    meets: Vec<i64>,
    /// Spend the third request per meet for the per-event type and hurdle metadata.
    #[arg(long)]
    event_metadata: bool,
    /// Cap the number of meets processed on the whole-meet route.
    #[arg(long)]
    meet_limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    #[arg(long)]
    refresh: bool,
    /// ISO date stamped into evidence (defaults to today).
    #[arg(long)]
    observed_on: Option<String>,
}

/// The jurisdictions this run restricts to: `--all-states`, else `--states`, else the adapter's own
/// coverage (an empty list). The `milesplit` arm is a roster walk and uses the Wisconsin default;
/// the `milesplit_results` arm reads the meets the `meets` subcommand discovered.
impl ProviderArgs {
    fn jurisdictions(&self) -> Result<Vec<UsJurisdiction>> {
        super::resolve_restriction(self.all_states, &self.states)
    }
}

/// Run one association contact adapter by name.
pub(super) async fn run_provider(cli: &Cli, store: &Store, args: &ProviderArgs) -> Result<()> {
    // The adapter's own registry slug, so every access condition this fetcher records is attributed
    // to the source that hit it (§69).
    let fetcher = build_fetcher(cli, store)?.with_source(args.name.clone());
    let observed_on = args
        .observed_on
        .clone()
        .unwrap_or_else(midwest_census::net::today_iso);
    let context = midwest_census::sources::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: SchoolYear::new(2026),
        observed_on: observed_on.clone(),
    };
    let outcome = match args.name.as_str() {
        "ks" => arms::ks_report(&context, args, observed_on).await,
        "wiaa_results" => arms::wiaa_results_report(&context, args, observed_on).await,
        "wiaa" => arms::wiaa_report(&context, args, observed_on).await,
        "ihsa" => arms::ihsa_report(&context, args, observed_on).await,
        "ihsa_tournament" => arms::ihsa_tournament_report(&context, args, observed_on).await,
        "ohsaa" => arms::ohsaa_report(&context, args, observed_on).await,
        "mshsl" => arms::mshsl_report(&context, args, observed_on).await,
        "wayzata" | "wayzata_schedule" => arms::wayzata_report(&context, args, observed_on).await,
        "plain_names" => arms::plain_names_report(&context, args, observed_on).await,
        "athleticlive" => arms::athleticlive_report(&context, args, observed_on).await,
        "athleticlive_results" => {
            arms::athleticlive_results_report(&context, args, observed_on).await
        }
        "athleticlive_athletes" => {
            arms::athleticlive_athletes_report(&context, args, observed_on).await
        }
        "athleticnet" => arms::athleticnet_report(&context, args, observed_on).await,
        "milesplit" => arms::milesplit_report(&context, args, observed_on).await,
        "milesplit_results" => arms::milesplit_results_report(&context, args).await,
        "coach_contacts" => arms::coach_contacts_report(store, args, observed_on),
        other => bail!(
            "unknown adapter {other}; expected one of ks, wiaa, wiaa_results, ihsa, ihsa_tournament, ohsaa, mshsl, plain_names, wayzata, athleticlive, athleticlive_results, athleticlive_athletes, athleticnet, milesplit, milesplit_results, coach_contacts"
        ),
    };
    // §69: the blocked hosts are named before the report, so a run that hit a hard block never reads
    // like a clean one — including when the adapter fails after the block.
    super::source::print_blocked_hosts(&fetcher).await;
    let report = outcome.with_context(|| format!("adapter {}", args.name))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
