use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_report::export::{ExportDataset, ExportJobRequest};
use census_report::workbook;
use census_service::restate_services::{
    reject_offline_generation, ExportGeneration, PublicationContract, WorkbookReply,
    WorkbookRequest,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use crate::cli::{
    contact_school_year, live, publication_preflight, resolve_restriction, restriction_display,
    scope_of, Cli, Route,
};

#[derive(Args, Debug)]
pub(crate) struct WorkbookArgs {
    #[arg(
        help = "Publication root directory (defaults to <store>/out/publication); atomically publishes a complete manifested generation"
    )]
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(
        help = "Graduation cohort; omitted uses the selected profile, or 2027 without a selection"
    )]
    #[arg(long)]
    grad_year: Option<u16>,
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
        help = "Logical publication generation; reuse retains frozen inputs, use a new generation only for a new accepted-input publication"
    )]
    #[arg(long, value_name = "GENERATION")]
    generation: Option<String>,
    #[arg(
        long,
        value_name = "STORE_IDENTITY",
        help = "Store identity for the selected publication; requires --selected-profile-id"
    )]
    selected_store_identity: Option<String>,
    #[arg(
        long,
        value_name = "PROFILE_ID",
        help = "Selected publication profile ID; requires --selected-store-identity"
    )]
    selected_profile_id: Option<String>,
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
}

pub(crate) async fn run_workbook(cli: &Cli, args: &WorkbookArgs) -> Result<()> {
    let grad_year = publication_preflight::publication_cohort(
        args.grad_year,
        args.selected_store_identity.is_some() || args.selected_profile_id.is_some(),
    )?;
    let contact_season = contact_school_year(args.school_year)?;
    let jurisdictions = if args.all_states || !args.states.is_empty() {
        resolve_restriction(args.all_states, &args.states)?
    } else {
        Vec::new()
    };
    println!("jurisdictions={}", restriction_display(&jurisdictions));
    match cli.route(args.ingress.as_deref())? {
        Route::Offline(root) => {
            let options = workbook::Options {
                grad_year,
                out: args.out.clone(),
                limit: args.limit,
                scope: scope_of(args.core),
                jurisdictions,
                school_year: contact_season,
            };
            let path = run_workbook_offline(root, args, &options)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Route::Ingress(origin) => {
            let export_job = publication_preflight::selected_request(
                args.selected_store_identity.as_deref(),
                args.selected_profile_id.as_deref(),
            )?;
            let client = census_service::ingress::client(origin)?;
            publication_preflight::require_publication_contract(
                &client,
                PublicationContract::SelectedPublicationV2,
            )
            .await?;
            run_workbook_live(
                origin,
                args,
                grad_year,
                contact_season,
                &jurisdictions,
                &export_job,
            )
            .await
        }
    }
}

fn run_workbook_offline(
    root: &std::path::Path,
    args: &WorkbookArgs,
    options: &workbook::Options,
) -> Result<PathBuf> {
    match (
        args.selected_store_identity.as_deref(),
        args.selected_profile_id.as_deref(),
    ) {
        (None, None) => {
            reject_offline_generation(args.generation.as_deref())?;
            let store = Store::open(root)?;
            workbook::build(&store, options).context("building the census workbook")
        }
        (store_identity, profile_id) => {
            let request = publication_preflight::selected_request(store_identity, profile_id)?;
            let generation = ExportGeneration::resolve(args.generation.as_deref())?;
            let store = publication_preflight::open_selected_store(root)?;
            build_offline_selected(&store, options, &request, &generation)
        }
    }
}

fn build_offline_selected(
    store: &Store,
    requested: &workbook::Options,
    request: &ExportJobRequest,
    generation: &ExportGeneration,
) -> Result<PathBuf> {
    let selection = request.require_selected_publication();
    let options = workbook::options_for_selected_request(store, selection, requested)
        .context("binding workbook options to the selected profile")?;
    let job = format!(
        "cli-workbook-selected-v1-{}-{}-{}",
        generation.as_str(),
        selection.store_identity().as_str(),
        selection.profile_id().as_str()
    );
    let dataset = ExportDataset::for_job_request(store, &job, request)
        .context("capturing the selected workbook input")?;
    let censuses = workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
    workbook::build_from(&dataset, store, &options, &censuses)
        .context("building the selected census workbook")
}

async fn run_workbook_live(
    origin: &str,
    args: &WorkbookArgs,
    grad_year: Option<i16>,
    contact_season: SchoolYear,
    jurisdictions: &[UsJurisdiction],
    export_job: &ExportJobRequest,
) -> Result<()> {
    let generation = ExportGeneration::resolve(args.generation.as_deref())?;
    let request = WorkbookRequest {
        grad_year,
        limit: args.limit,
        scope: Some(scope_of(args.core).as_str().to_string()),
        out: args
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        school_year: Some(contact_season.get()),
        jurisdictions: jurisdictions.to_vec(),
        export_job: export_job.clone(),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, &generation).await?;
    println!("wrote {path}");
    Ok(())
}
