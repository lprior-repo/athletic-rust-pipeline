use anyhow::{Context, Result};
use census_report::bests;
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report;
use census_service::restate_services::{
    reject_offline_generation, BestsReply, ExportGeneration,
};
use census_store::Store;
use clap::Args;

use crate::cli::{cohort_label, live, resolve_restriction, restriction_display, school_year, Cli, Route};

#[derive(Args, Debug)]
pub(crate) struct BestsArgs {
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
        help = "Restrict the publication to these jurisdictions (default: every census jurisdiction)"
    )]
    #[arg(long, value_delimiter = ',')]
    states: Vec<UsJurisdiction>,
    #[arg(help = "Publish every census jurisdiction explicitly")]
    #[arg(long)]
    all_states: bool,
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

pub(crate) async fn run_bests(cli: &Cli, args: &BestsArgs) -> Result<()> {
    let grad_year = if args.all {
        None
    } else {
        Some(school_year(args.grad_year)?)
    };
    let jurisdictions = if args.all_states || !args.states.is_empty() {
        resolve_restriction(args.all_states, &args.states)?
    } else {
        Vec::new()
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            run_bests_offline(&Store::open(root)?, args, grad_year, &jurisdictions)
        }
        Route::Ingress(origin) => run_bests_live(origin, args, grad_year, &jurisdictions).await,
    }
}

async fn run_bests_live(
    origin: &str,
    args: &BestsArgs,
    grad_year: Option<i16>,
    jurisdictions: &[UsJurisdiction],
) -> Result<()> {
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
        jurisdictions,
        &generation,
    )
    .await?;
    println!("cohort={cohort} rows={rows}");
    println!("jurisdictions={}", restriction_display(jurisdictions));
    println!("wrote {jsonl}");
    println!("wrote {csv}");
    Ok(())
}

fn run_bests_offline(
    store: &Store,
    args: &BestsArgs,
    grad_year: Option<i16>,
    jurisdictions: &[UsJurisdiction],
) -> Result<()> {
    let options = bests::Options {
        scope: report::Scope::Core,
        grad_year,
        limit: args.limit,
        jurisdictions: jurisdictions.to_vec(),
    };
    let dataset = ExportDataset::load(store).context("reading export dataset for best marks")?;
    let rows = bests::build_from_dataset(&dataset, &options)?;
    let cohort = cohort_label(grad_year);
    let (jsonl, csv) =
        bests::write(&store.out_dir(), &rows, &cohort).context("writing the best-mark sidecars")?;
    println!("cohort={cohort} rows={}", rows.len());
    println!("jurisdictions={}", restriction_display(jurisdictions));
    println!("wrote {}", jsonl.display());
    println!("wrote {}", csv.display());
    Ok(())
}
