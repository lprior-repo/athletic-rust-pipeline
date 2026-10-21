use anyhow::{Context, Result};
use census_report::export::{ExportDataset, ExportJobRequest};
use census_report::workbook;
use census_service::restate_services::{ExportGeneration, WorkbookReply, WorkbookRequest};
use census_store::Store;

use super::RunArgs;
use crate::cli::{contact_school_year, live, publication_preflight, resolve_restriction, scope_of};

pub(super) fn publication_options(args: &RunArgs) -> Result<workbook::Options> {
    let jurisdictions = if args.all_states || !args.states.is_empty() {
        resolve_restriction(args.all_states, &args.states)?
    } else {
        Vec::new()
    };
    Ok(workbook::Options {
        grad_year: publication_preflight::publication_cohort(
            args.grad_year,
            args.selected_store_identity.is_some() || args.selected_profile_id.is_some(),
        )?,
        out: args.out.clone(),
        limit: args.limit,
        scope: scope_of(args.core),
        jurisdictions,
        school_year: contact_school_year(args.school_year)?,
    })
}

pub(super) fn publish_workbook_offline(
    store: &Store,
    options: &workbook::Options,
    publication: Option<&(ExportJobRequest, ExportGeneration)>,
) -> Result<()> {
    let path = match publication {
        Some((request @ ExportJobRequest::SelectedPublicationV1 { selection }, generation)) => {
            let options = workbook::options_for_selected_request(store, selection, options)
                .context("binding workbook options to the selected profile")?;
            let job = format!(
                "cli-cycle-selected-v1-{}-{}-{}",
                generation.as_str(),
                selection.store_identity().as_str(),
                selection.profile_id().as_str()
            );
            let dataset = ExportDataset::for_job_request(store, &job, request)
                .context("capturing the selected workbook input")?;
            let censuses =
                workbook::Censuses::of(&dataset, &store.out_dir(), &options.jurisdictions)?;
            workbook::build_from(&dataset, store, &options, &censuses)
                .context("building the selected census workbook")?
        }
        None => {
            workbook::build(store, options).context("publishing the managed workbook bundle")?
        }
    };
    println!("workbook\t{}", path.display());
    Ok(())
}

pub(super) async fn publish_workbook_live(
    origin: &str,
    options: &workbook::Options,
    generation: &ExportGeneration,
    export_job: &ExportJobRequest,
) -> Result<()> {
    let request = WorkbookRequest {
        grad_year: options.grad_year,
        out: options
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        limit: options.limit,
        scope: Some(options.scope.as_str().to_string()),
        school_year: Some(options.school_year.get()),
        jurisdictions: options.jurisdictions.clone(),
        export_job: export_job.clone(),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, generation).await?;
    println!("workbook\t{path}");
    Ok(())
}
