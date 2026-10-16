use anyhow::Result;
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation};
use census_service::restate_services::{reject_offline_generation, ExportGeneration};
use census_store::Store;
use clap::Args;

use crate::cli::{live, resolve_restriction, restriction_display, Cli, Route};

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

pub(crate) fn run_report(
    store: &Store,
    print: bool,
    core: bool,
    jurisdictions: &[UsJurisdiction],
) -> Result<()> {
    let scope = if core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    let dataset = ExportDataset::load(store)?;
    println!("jurisdictions={}", restriction_display(jurisdictions));
    let derivation = Derivation::of(&dataset, scope, None, jurisdictions)?;
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
pub(crate) struct ReportArgs {
    #[arg(help = "Print the census JSON to stdout as well as writing files")]
    #[arg(long)]
    print: bool,
    #[arg(
        help = "Restrict the census to core evidence: Athletic.net and the AthleticLIVE derivative are excluded, exactly as they are when those adapters are never registered"
    )]
    #[arg(long)]
    core: bool,
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

pub(crate) async fn run_census_report(cli: &Cli, args: &ReportArgs) -> Result<()> {
    let scope = if args.core {
        report::Scope::Core
    } else {
        report::Scope::AllSources
    };
    let jurisdictions = if args.all_states || !args.states.is_empty() {
        resolve_restriction(args.all_states, &args.states)?
    } else {
        Vec::new()
    };
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            run_report(&Store::open(root)?, args.print, args.core, &jurisdictions)
        }
        Route::Ingress(origin) => {
            let generation = ExportGeneration::resolve(args.generation.as_deref())?;
            let summary = live::report(Some(origin), scope, &jurisdictions, &generation).await?;
            println!("wrote {}", summary.json_path);
            println!("wrote {}", summary.csv_path);
            println!("scope={} totals={}", summary.scope, summary.totals);
            println!("jurisdictions={}", restriction_display(&jurisdictions));
            if args.print {
                println!("{}", serde_json::to_string_pretty(&summary.totals)?);
            }
            Ok(())
        }
    }
}
