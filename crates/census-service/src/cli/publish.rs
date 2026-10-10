use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation};
use census_report::{bests, workbook};
use census_service::restate_services::{
    reject_offline_generation, BestsReply, ExportGeneration, WorkbookReply, WorkbookRequest,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use super::{cohort_label, contact_school_year, live, school_year, scope_of, Cli, Route};

pub(super) fn run_consolidate(store: &Store) -> Result<()> {
    let counts = census_service::census::consolidate(store)?;
    for (table, count) in counts {
        println!("{table}\t{count}");
    }
    Ok(())
}

fn print_totals(census: &report::Census) {
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
}

pub(super) fn run_report(store: &Store, print: bool, core: bool) -> Result<()> {
    let scope = if core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    let dataset = ExportDataset::load(store)?;
    let derivation = Derivation::of(&dataset, scope, None);
    let census = report::build_census(&derivation, &store.out_dir());
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    println!("wrote {}", json_path.display());
    println!("wrote {}", csv_path.display());
    print_totals(&census);
    if print {
        println!("{}", serde_json::to_string_pretty(&census)?);
    }
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct ReportArgs {
    #[arg(help = "Print the census JSON to stdout as well as writing files")]
    #[arg(long)]
    print: bool,
    #[arg(
        help = "Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are excluded, exactly as they are when those adapters are never registered"
    )]
    #[arg(long)]
    core: bool,
    #[arg(
        help = "Logical export generation selecting a fresh export run; omitted reuses the legacy generation 1 keys"
    )]
    #[arg(long, value_name = "GENERATION")]
    generation: Option<String>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(super) async fn run_census_report(cli: &Cli, args: &ReportArgs) -> Result<()> {
    let scope = if args.core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            run_report(&Store::open(root)?, args.print, args.core)
        }
        Route::Ingress(origin) => {
            let generation = ExportGeneration::resolve(args.generation.as_deref())?;
            let summary = live::report(Some(origin), scope, &generation).await?;
            println!("wrote {}", summary.json_path);
            println!("wrote {}", summary.csv_path);
            println!("scope={} totals={}", summary.scope, summary.totals);
            if args.print {
                println!("{}", serde_json::to_string_pretty(&summary.totals)?);
            }
            Ok(())
        }
    }
}

#[derive(Args, Debug)]
pub(super) struct BestsArgs {
    #[arg(help = "Graduation year the cohort is selected by (2027 = the class of 2027)")]
    #[arg(long, default_value_t = 2027, conflicts_with = "all")]
    grad_year: u16,
    #[arg(help = "Keep only the first N rows of the reduction")]
    #[arg(long)]
    limit: Option<usize>,
    #[arg(help = "Reduce every athlete in the core scope instead of one graduating class")]
    #[arg(long)]
    all: bool,
    #[arg(
        help = "Logical export generation selecting a fresh export run; omitted reuses the legacy generation 1 keys"
    )]
    #[arg(long, value_name = "GENERATION")]
    generation: Option<String>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(super) async fn run_bests(cli: &Cli, args: &BestsArgs) -> Result<()> {
    let grad_year = if args.all {
        None
    } else {
        Some(school_year(args.grad_year)?)
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            run_bests_offline(&Store::open(root)?, args, grad_year)
        }
        Route::Ingress(origin) => run_bests_live(origin, args, grad_year).await,
    }
}

async fn run_bests_live(origin: &str, args: &BestsArgs, grad_year: Option<i16>) -> Result<()> {
    let generation = ExportGeneration::resolve(args.generation.as_deref())?;
    let BestsReply {
        cohort,
        rows,
        jsonl,
        csv,
    } = live::bests(
        Some(origin),
        report::Scope::Core,
        grad_year,
        args.limit,
        &generation,
    )
    .await?;
    println!("cohort={cohort} rows={rows}");
    println!("wrote {jsonl}");
    println!("wrote {csv}");
    Ok(())
}

fn run_bests_offline(store: &Store, args: &BestsArgs, grad_year: Option<i16>) -> Result<()> {
    let options = bests::Options {
        scope: report::Scope::Core,
        grad_year,
        limit: args.limit,
    };
    let dataset = ExportDataset::load(store).context("reading export dataset for best marks")?;
    let rows = bests::build_from_dataset(&dataset, &options);
    let cohort = cohort_label(grad_year);
    let (jsonl, csv) =
        bests::write(&store.out_dir(), &rows, &cohort).context("writing the best-mark sidecars")?;
    println!("cohort={cohort} rows={}", rows.len());
    println!("wrote {}", jsonl.display());
    println!("wrote {}", csv.display());
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct WorkbookArgs {
    #[arg(
        help = "Publication root directory (defaults to <store>/out/publication); atomically publishes a complete manifested generation"
    )]
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(help = "Graduation year used for the cohort sheets (2027 = the class of 2027)")]
    #[arg(long, default_value_t = 2027)]
    grad_year: u16,
    #[arg(
        help = "School year the contact tenure and coach cells are assessed against (2026 = the 2026-27 school year)"
    )]
    #[arg(long)]
    school_year: u16,
    #[arg(help = "Cap the per-athlete best-mark sheet at N rows")]
    #[arg(long)]
    limit: Option<usize>,
    #[arg(
        help = "Reduce the best-results sheet over the core scope instead of every approved source"
    )]
    #[arg(long)]
    core: bool,
    #[arg(
        help = "Logical export generation selecting a fresh export run; omitted reuses the legacy generation 1 keys"
    )]
    #[arg(long, value_name = "GENERATION")]
    generation: Option<String>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(super) async fn run_workbook(cli: &Cli, args: &WorkbookArgs) -> Result<()> {
    let grad_year = school_year(args.grad_year)?;
    let contact_season = contact_school_year(args.school_year)?;
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            let options = workbook::Options {
                grad_year: Some(grad_year),
                out: args.out.clone(),
                limit: args.limit,
                scope: scope_of(args.core),
                school_year: contact_season,
            };
            let store = Store::open(root)?;
            let path = workbook::build(&store, &options).context("building the census workbook")?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Route::Ingress(origin) => run_workbook_live(origin, args, grad_year, contact_season).await,
    }
}

async fn run_workbook_live(
    origin: &str,
    args: &WorkbookArgs,
    grad_year: i16,
    contact_season: SchoolYear,
) -> Result<()> {
    let generation = ExportGeneration::resolve(args.generation.as_deref())?;
    let request = WorkbookRequest {
        grad_year: Some(grad_year),
        limit: args.limit,
        scope: Some(scope_of(args.core).as_str().to_string()),
        out: args
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        school_year: Some(contact_season.get()),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, &generation).await?;
    println!("wrote {path}");
    Ok(())
}

#[derive(Args, Debug)]
pub(super) struct IndexArgs {
    #[arg(
        help = "School year the contact tenure and coach cells are assessed against (2026 = the 2026-27 school year)"
    )]
    #[arg(long)]
    school_year: u16,
}

pub(super) fn run_index(store: &Store, args: &IndexArgs) -> Result<()> {
    let finished_on = census_crawl::net::today_iso();
    let school_year = contact_school_year(args.school_year)?;
    let report = census_reconcile::index::derive(store, "index", &finished_on, school_year)
        .context("deriving the durable indexes")?;
    println!(
        "index\tsource_identities={} conflicts={} reviews={} superseded={} coverage={} snapshots={}",
        report.source_identities,
        report.conflicts,
        report.reviews,
        report.superseded,
        report.coverage,
        report.snapshots
    );
    Ok(())
}
