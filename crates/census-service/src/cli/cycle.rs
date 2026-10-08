use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report;
use census_report::workbook;
use census_service::census;
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use super::live;

mod publish;
use super::{build_fetcher, contact_school_year, school_year, scope_of, Cli, Route};
use publish::{
    publish_bests_and_workbook_live, publish_bests_and_workbook_with, publish_scope_live,
    publish_scope_with,
};

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
    #[arg(help = "ISO date stamped into evidence (defaults to today)")]
    #[arg(long)]
    observed_on: Option<String>,
    #[arg(
        long,
        help = "Published performance cutoff date, independent of physical acquisition"
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
    #[arg(help = "Workbook path (defaults to the store's own `out/` path)")]
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(super) async fn run_cycle(cli: &Cli, args: &RunArgs) -> Result<()> {
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let root = root.to_path_buf();
            let store = tokio::task::spawn_blocking(move || Store::open(&root))
                .await
                .map_err(|error| anyhow::anyhow!("opening the store did not finish: {error}"))??;
            run_offline(cli, std::sync::Arc::new(store), args).await
        }
        Route::Ingress(origin) => run_live(origin, args).await,
    }
}

async fn run_offline(cli: &Cli, store: std::sync::Arc<Store>, args: &RunArgs) -> Result<()> {
    let observed_on = match args.observed_on.clone() {
        Some(value) => value,
        None => census_crawl::net::today_iso(),
    };
    let grad_year = school_year(args.grad_year)?;
    let contact_season = contact_school_year(args.school_year)?;
    let scope = scope_of(args.core);

    match (&args.input, args.meets.is_empty()) {
        (Some(_), _) | (None, false) => {
            gather_athleticnet(cli, &store, args, observed_on.clone()).await?
        }
        (None, true) => {
            println!("gather\tathleticnet\tskipped (no --input): publishing what the store holds")
        }
    }

    let stages = std::sync::Arc::clone(&store);
    let arguments = args.clone();
    tokio::task::spawn_blocking(move || {
        offline_stages(
            &stages,
            &arguments,
            &observed_on,
            grad_year,
            contact_season,
            scope,
        )
    })
    .await
    .map_err(|error| anyhow::anyhow!("the offline stages did not finish: {error}"))?
}

fn stage_line(name: &str, detail: &str, started: std::time::Instant) {
    println!("{name}\t{detail}\t{}ms", started.elapsed().as_millis());
}

fn offline_stages(
    store: &Store,
    args: &RunArgs,
    observed_on: &str,
    grad_year: i16,
    school_year: SchoolYear,
    scope: report::Scope,
) -> Result<()> {
    let total = std::time::Instant::now();
    derive_index(store, observed_on, school_year)?;
    consolidate_store(store)?;
    let dataset = load_dataset(store)?;

    let started = std::time::Instant::now();
    let all_sources = publish_scope_with(&dataset, store, report::Scope::AllSources)?;
    let core = publish_scope_with(&dataset, store, report::Scope::Core)?;
    stage_line("reports", "all_sources+core", started);
    let censuses = workbook::Censuses { core, all_sources };

    let started = std::time::Instant::now();
    publish_bests_and_workbook_with(
        &dataset,
        store,
        args,
        scope,
        grad_year,
        school_year,
        &censuses,
    )?;
    stage_line("publish", "bests+workbook", started);

    stage_line("stages", "total", total);
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

fn consolidate_store(store: &Store) -> Result<()> {
    let started = std::time::Instant::now();
    let counts = census::consolidate(store).context("consolidating the store")?;
    let detail = counts
        .iter()
        .map(|(table, count)| format!("{table}={count}"))
        .collect::<Vec<_>>()
        .join(" ");
    stage_line("consolidate", &detail, started);
    Ok(())
}

fn load_dataset(store: &Store) -> Result<ExportDataset> {
    let started = std::time::Instant::now();
    let dataset = ExportDataset::load(store).context("loading the export dataset")?;
    let detail = format!(
        "athletes={} performances={} events={} coaches={}",
        dataset.athletes.len(),
        dataset.performances.len(),
        dataset.events.len(),
        dataset.coaches.len()
    );
    stage_line("dataset", &detail, started);
    Ok(dataset)
}

async fn run_live(origin: &str, args: &RunArgs) -> Result<()> {
    if args.input.is_some() || !args.meets.is_empty() {
        anyhow::bail!(
            "the gather stage writes the store the running service holds: --input and --meets need --store"
        );
    }
    println!("gather\tathleticnet\tskipped (no --input): publishing what the store holds");
    let grad_year = school_year(args.grad_year)?;
    let contact_season = contact_school_year(args.school_year)?;
    let scope = scope_of(args.core);
    let tables = live::consolidate(Some(origin)).await?;
    println!(
        "consolidate\t{}",
        tables
            .iter()
            .map(|table| format!("{}={}", table.table, table.rows))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("index\tskipped (offline stage: `index --store <dir>` with census-serve stopped)");

    for scope in [report::Scope::AllSources, report::Scope::Core] {
        publish_scope_live(origin, scope).await?;
    }

    publish_bests_and_workbook_live(origin, args, scope, grad_year, contact_season).await
}

async fn gather_athleticnet(
    cli: &Cli,
    store: &Store,
    args: &RunArgs,
    observed_on: String,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let season =
        SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?;
    let context = census_crawl::AdapterContext {
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: season,
        observed_on: observed_on.clone(),
        performance_as_of: match args.as_of {
            Some(date) => date,
            None => chrono::Utc::now().date_naive(),
        },
        recording: None,
    };
    let report = census_crawl::athleticnet::collect(
        &context,
        &census_crawl::athleticnet::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.states.clone(),
            meets: args.meets.clone(),
            event_metadata: args.event_metadata,
            meet_limit: args.meet_limit,
        },
    )
    .await
    .with_context(|| {
        format!(
            "gathering athletic.net rows: input={:?} meets={:?}",
            args.input, args.meets
        )
    })?;
    println!(
        "gather\tathleticnet\trows={} requests={} errors={}",
        report.rows, report.requests, report.errors
    );
    for note in &report.notes {
        println!("\t{note}");
    }
    if report.errors > 0 {
        anyhow::bail!(
            "Athletic.net gathering remains incomplete: {} acquisition or admission errors",
            report.errors
        );
    }
    Ok(())
}
