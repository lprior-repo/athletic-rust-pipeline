use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_report::workbook;
use census_service::restate_services::{
    reject_offline_generation, ExportGeneration, WorkbookReply, WorkbookRequest,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use crate::cli::{
    contact_school_year, live, resolve_restriction, restriction_display, school_year, scope_of,
    Cli, Route,
};

#[derive(Args, Debug)]
pub(crate) struct WorkbookArgs {
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

pub(crate) async fn run_workbook(cli: &Cli, args: &WorkbookArgs) -> Result<()> {
    let grad_year = school_year(args.grad_year)?;
    let contact_season = contact_school_year(args.school_year)?;
    let jurisdictions = if args.all_states || !args.states.is_empty() {
        resolve_restriction(args.all_states, &args.states)?
    } else {
        Vec::new()
    };
    println!("jurisdictions={}", restriction_display(&jurisdictions));
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            reject_offline_generation(args.generation.as_deref())?;
            let options = workbook::Options {
                grad_year: Some(grad_year),
                out: args.out.clone(),
                limit: args.limit,
                scope: scope_of(args.core),
                jurisdictions,
                school_year: contact_season,
            };
            let store = Store::open(root)?;
            let path = workbook::build(&store, &options).context("building the census workbook")?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Route::Ingress(origin) => {
            run_workbook_live(origin, args, grad_year, contact_season, &jurisdictions).await
        }
    }
}

async fn run_workbook_live(
    origin: &str,
    args: &WorkbookArgs,
    grad_year: i16,
    contact_season: SchoolYear,
    jurisdictions: &[UsJurisdiction],
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
        jurisdictions: jurisdictions.to_vec(),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, &generation).await?;
    println!("wrote {path}");
    Ok(())
}
