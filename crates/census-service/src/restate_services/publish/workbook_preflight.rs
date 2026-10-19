use super::{blocking, job_error, normalize_jurisdictions, resolve_scope, JobError, Jobs};
use census_domain::model::SchoolYear;
use census_report::{export::ExportJobRequest, workbook};
use census_store::Store;
use restate_sdk::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

pub(in crate::restate_services) async fn prepare(
    jobs: &Jobs,
    request: super::WorkbookRequest,
) -> Result<(workbook::Options, ExportJobRequest), HandlerError> {
    request
        .export_job
        .require_selected_publication()
        .map_err(|error| TerminalError::new_with_code(400, error.to_string()))?;
    let store = Arc::clone(&jobs.store);
    let region = Arc::clone(jobs.region());
    let permit = jobs.permit_control().await?;
    blocking(region, move || {
        let _permit = permit;
        Ok::<_, JobError>(
            checked_options(&store, &request).map(|options| (options, request.export_job)),
        )
    })
    .await
    .map_err(job_error)?
}

pub(in crate::restate_services) fn checked_options(
    store: &Store,
    request: &super::WorkbookRequest,
) -> Result<workbook::Options, HandlerError> {
    let selection = request
        .export_job
        .require_selected_publication()
        .map_err(|error| TerminalError::new_with_code(400, error.to_string()))?;
    let requested_year = request.school_year.ok_or_else(|| {
        job_error(JobError::Terminal {
            message: "workbook publication requires an explicit school year".to_owned(),
        })
    })?;
    let school_year = SchoolYear::new(requested_year).ok_or_else(|| {
        job_error(JobError::Terminal {
            message: format!("school year {requested_year} is outside the supported range"),
        })
    })?;
    let options = workbook::Options {
        grad_year: request.grad_year,
        out: request.out.as_deref().map(PathBuf::from),
        limit: request.limit,
        scope: resolve_scope(request.scope.as_deref())?,
        jurisdictions: normalize_jurisdictions(&request.jurisdictions)?,
        school_year,
    };
    workbook::options_for_selected_request(store, selection, &options)
        .map_err(|error| job_error(JobError::from(error)))
}
