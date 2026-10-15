use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_report::workbook;
use census_service::restate_services::{reject_offline_generation, ExportGeneration};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use census_service::ingress;

mod gather;
mod publish;
use super::{Cli, Route};
use gather::gather_athleticnet;
use publish::{publication_options, publish_workbook_live, publish_workbook_offline};

#[derive(Args, Debug, Clone)]
pub(super) struct RunArgs {
    #[arg(
        help = "Athletic.net athlete registry: one `athlete_id` or `athlete_id,ST` per line. Omitted, the cycle publishes whatever the store already holds"
    )]
    #[arg(long)]
    input: Option<String>,
    #[arg(
        help = "Jurisdictions for the registry, used only for lines that name no state (so exactly one)"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(help = "Cap the athletes read from the registry, and the rows each later stage writes")]
    #[arg(long)]
    limit: Option<usize>,
    #[arg(
        help = "Graduation year the best-mark reduction and the workbook are built for (2027 = class of 2027)"
    )]
    #[arg(long, default_value_t = 2027)]
    grad_year: u16,
    #[arg(
        help = "School year the workbook contact tenure and coach cells are assessed against (2026 = the 2026-27 school year)"
    )]
    #[arg(long)]
    school_year: u16,
    #[arg(
        help = "Restrict the best-mark reduction to the core scope, which excludes the Athletic.net source by design. Every approved source is what a plain run reduces"
    )]
    #[arg(long)]
    core: bool,
    #[arg(help = "Ignore cached HTTP bodies and re-fetch")]
    #[arg(long)]
    refresh: bool,
    #[arg(help = "ISO date stamped into gathered evidence (defaults to today)")]
    #[arg(long)]
    observed_on: Option<String>,
    #[arg(
        long,
        help = "Unsupported for standalone run: retained-data publication has no performance cutoff"
    )]
    as_of: Option<chrono::NaiveDate>,
    #[arg(
        help = "Athletic.net meet ids to pull whole (`--meets`), comma-separated. Non-empty selects the whole-meet route (two requests per meet) instead of the per-athlete registry route, and needs no `--input`"
    )]
    #[arg(long, value_delimiter = ',')]
    meets: Vec<i64>,
    #[arg(help = "Spend the third request per meet for the per-event type and hurdle metadata")]
    #[arg(long)]
    event_metadata: bool,
    #[arg(help = "Cap the number of meets processed on the whole-meet route")]
    #[arg(long)]
    meet_limit: Option<usize>,
    #[arg(
        help = "Publication root directory (defaults to <store>/out/publication); publishes one verified managed bundle"
    )]
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(
        help = "Logical export generation selecting a managed workbook publication; omitted reuses the legacy generation 1 key"
    )]
    #[arg(long, value_name = "GENERATION")]
    generation: Option<String>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(super) async fn run_cycle(cli: &Cli, args: &RunArgs) -> Result<()> {
    if args.as_of.is_some() {
        anyhow::bail!(
            "standalone run does not support --as-of: retained-data publication has no performance cutoff"
        );
    }
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            let root = root.to_path_buf();
            let store = tokio::task::spawn_blocking(move || Store::open(&root))
                .await
                .map_err(|error| anyhow::anyhow!("opening the store did not finish: {error}"))??;
            run_offline(cli, std::sync::Arc::new(store), args).await
        }
        Route::Ingress(origin) => run_live(origin, args).await,
    }
}

enum PublicationPreparation {
    Retained,
    Gathered { observed_on: String },
}

async fn run_offline(cli: &Cli, store: std::sync::Arc<Store>, args: &RunArgs) -> Result<()> {
    let options = publication_options(args)?;
    let preparation = match (&args.input, args.meets.is_empty()) {
        (Some(_), _) | (None, false) => {
            let observed_on = match args.observed_on.clone() {
                Some(value) => value,
                None => census_crawl::net::today_iso(),
            };
            gather_athleticnet(cli, &store, args, observed_on.clone()).await?;
            PublicationPreparation::Gathered { observed_on }
        }
        (None, true) => PublicationPreparation::Retained,
    };
    tokio::task::spawn_blocking(move || offline_stages(&store, &options, preparation))
        .await
        .map_err(|error| anyhow::anyhow!("the offline stages did not finish: {error}"))?
}

fn stage_line(name: &str, detail: &str, started: std::time::Instant) {
    println!("{name}\t{detail}\t{}ms", started.elapsed().as_millis());
}

fn offline_stages(
    store: &Store,
    options: &workbook::Options,
    preparation: PublicationPreparation,
) -> Result<()> {
    if let PublicationPreparation::Gathered { observed_on } = preparation {
        derive_index(store, &observed_on, options.school_year)?;
    }
    let started = std::time::Instant::now();
    publish_workbook_offline(store, options)?;
    stage_line("publish", "managed_bundle", started);
    Ok(())
}

fn derive_index(store: &Store, observed_on: &str, school_year: SchoolYear) -> Result<()> {
    let started = std::time::Instant::now();
    let index = census_reconcile::index::derive(store, "run", observed_on, school_year)
        .context("deriving the durable indexes")?;
    let detail = format!(
        "source_identities={} conflicts={} reviews={} superseded={} coverage={}",
        index.source_identities, index.conflicts, index.reviews, index.superseded, index.coverage
    );
    stage_line("index", &detail, started);
    Ok(())
}

async fn run_live(origin: &str, args: &RunArgs) -> Result<()> {
    if args.input.is_some() || !args.meets.is_empty() {
        anyhow::bail!(
            "the gather stage writes the store the running service holds: --input and --meets need --store"
        );
    }
    let options = publication_options(args)?;
    let generation = ExportGeneration::resolve(args.generation.as_deref())?;
    let client = ingress::client(origin)?;
    super::publication_preflight::require_managed_publication(&client).await?;
    publish_workbook_live(origin, &options, &generation).await
}

#[cfg(test)]
mod tests;
