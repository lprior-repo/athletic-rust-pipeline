use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_report::export::{ExportDataset, ExportJobRequest};
use census_report::workbook;
use census_service::restate_services::{
    reject_offline_generation, workbook_request_key, ExportGeneration, PublicationContract,
    WorkbookReply, WorkbookRequest,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use crate::cli::{
    contact_school_year, live, open_offline_store, publication_preflight, resolve_restriction,
    restriction_display, scope_of, Cli, Route,
};

mod derived;

enum OfflineJob<'a> {
    Recover(&'a str),
    Generation(ExportGeneration),
}

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
        value_name = "JOB",
        value_parser = recovery_job,
        requires_all = ["out", "selected_store_identity", "selected_profile_id"],
        conflicts_with_all = ["resume_frozen", "derive_out", "generation", "ingress"],
        help = "Recover an existing native workbook job offline; supply its original selection and output options. Empty staging has no retained request binding. Requires --store and a stopped owner"
    )]
    recover_job: Option<String>,
    #[arg(
        long,
        requires_all = ["generation", "selected_store_identity", "selected_profile_id"],
        help = "Resume an existing native workbook frozen input with the same request and generation; requires --store and a stopped owner, and never captures new input"
    )]
    resume_frozen: bool,
    #[arg(
        long,
        requires = "resume_frozen",
        value_name = "ROOT",
        help = "Derive a current workbook in a different sibling publication root from the original --out request's frozen input; preserves the original publication"
    )]
    derive_out: Option<PathBuf>,
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
    #[arg(
        help = "Address corpus directory (school_directory.json) used to fill school addresses missing from store claims; unlinked schools land in address-quarantine.csv"
    )]
    #[arg(long, value_name = "DIR")]
    address_corpus: Option<PathBuf>,
}

pub(crate) async fn run_workbook(cli: &Cli, args: &WorkbookArgs) -> Result<()> {
    anyhow::ensure!(
        args.recover_job.is_none() || cli.store.is_some(),
        "--recover-job requires explicit --store and a stopped owner"
    );
    anyhow::ensure!(
        !args.resume_frozen || cli.store.is_some(),
        "--resume-frozen requires explicit --store and a stopped owner"
    );
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
                address_corpus: args.address_corpus.clone(),
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
            let store = open_offline_store(root)?;
            workbook::build(&store, options).context("building the census workbook")
        }
        (store_identity, profile_id) => {
            let request = publication_preflight::selected_request(store_identity, profile_id)?;
            let job = match args.recover_job.as_deref() {
                Some(job) => OfflineJob::Recover(job),
                None => {
                    OfflineJob::Generation(ExportGeneration::resolve(args.generation.as_deref())?)
                }
            };
            let store = publication_preflight::open_selected_store(root)?;
            build_offline_selected(&store, args, options, &request, job)
        }
    }
}

fn build_offline_selected(
    store: &Store,
    args: &WorkbookArgs,
    requested: &workbook::Options,
    request: &ExportJobRequest,
    job: OfflineJob<'_>,
) -> Result<PathBuf> {
    let selection = request.require_selected_publication();
    let mut options = workbook::options_for_selected_request(store, selection, requested)
        .context("binding workbook options to the selected profile")?;
    let dataset = match job {
        OfflineJob::Recover(job) => ExportDataset::recover_job_request(store, job, request)
            .context("recovering the existing native workbook input")?,
        OfflineJob::Generation(generation) => {
            if args.resume_frozen {
                let native = workbook_request(requested, request);
                let job = format!("workbook:{}", workbook_request_key(&native, &generation)?);
                ExportDataset::resume_job_request(store, &job, request)
                    .context("resuming the existing native workbook input")?
            } else {
                let job = format!(
                    "cli-workbook-selected-v1-{}-{}-{}",
                    generation.as_str(),
                    selection.store_identity().as_str(),
                    selection.profile_id().as_str()
                );
                ExportDataset::for_job_request(store, &job, request)
                    .context("capturing the selected workbook input")?
            }
        }
    };
    if let Some(destination) = &args.derive_out {
        let original = match &options.out {
            Some(root) => root.clone(),
            None => store.out_dir().join("publication"),
        };
        options.out = Some(derived::destination(&original, destination)?);
    }
    let source_out = std::path::Path::new(&dataset.lineage.store_root).join("out");
    let censuses = workbook::Censuses::of(&dataset, &source_out, &options.jurisdictions)?;
    workbook::build_from(&dataset, store, &options, &censuses)
        .context("building the selected census workbook")
}

#[cfg(test)]
mod tests;

async fn run_workbook_live(
    origin: &str,
    args: &WorkbookArgs,
    grad_year: Option<i16>,
    contact_season: SchoolYear,
    jurisdictions: &[UsJurisdiction],
    export_job: &ExportJobRequest,
) -> Result<()> {
    let generation = ExportGeneration::resolve(args.generation.as_deref())?;
    let options = workbook::Options {
        grad_year,
        limit: args.limit,
        scope: scope_of(args.core),
        out: args.out.clone(),
        school_year: contact_season,
        jurisdictions: jurisdictions.to_vec(),
        address_corpus: None,
    };
    let request = workbook_request(&options, export_job);
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, &generation).await?;
    println!("wrote {path}");
    Ok(())
}

fn workbook_request(options: &workbook::Options, export_job: &ExportJobRequest) -> WorkbookRequest {
    WorkbookRequest {
        grad_year: options.grad_year,
        limit: options.limit,
        scope: Some(options.scope.as_str().to_string()),
        out: options
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        school_year: Some(options.school_year.get()),
        jurisdictions: options.jurisdictions.clone(),
        export_job: export_job.clone(),
    }
}

fn recovery_job(value: &str) -> Result<String, String> {
    if value.len() > 4096 || value.strip_prefix("workbook:").is_none_or(str::is_empty) {
        return Err(
            "recovery requires an existing full workbook:<key> job of at most 4096 bytes"
                .to_string(),
        );
    }
    Ok(value.to_string())
}
