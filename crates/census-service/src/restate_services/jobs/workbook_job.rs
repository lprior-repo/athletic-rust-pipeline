use census_report::export::ExportDataset;
use census_report::report::{ReportError, ReportResult};
use census_report::workbook;
use census_store::Store;

use super::super::wire::WorkbookReply;

pub(in super::super) fn build(
    store: &Store,
    options: &workbook::Options,
    job: &str,
    export_job: &census_report::export::ExportJobRequest,
) -> ReportResult<WorkbookReply> {
    let dataset = ExportDataset::for_job_request(store, job, export_job)?;
    let selected_options = match export_job {
        census_report::export::ExportJobRequest::LegacyV2 => options.clone(),
        census_report::export::ExportJobRequest::SelectedPublicationV1 { .. } => {
            let mut selected =
                workbook::options_for_selected_dataset(&dataset, options.out.clone())?;
            if selected.grad_year != options.grad_year
                || selected.school_year != options.school_year
                || jurisdiction_codes(&selected.jurisdictions)
                    != jurisdiction_codes(&options.jurisdictions)
            {
                return Err(ReportError::Invariant {
                    detail: "selected workbook profile disagrees with the bound national cohort, school year or jurisdictions".into(),
                });
            }
            selected.limit = options.limit;
            selected.scope = options.scope;
            selected
        }
    };
    let censuses =
        workbook::Censuses::of(&dataset, &store.out_dir(), &selected_options.jurisdictions)?;
    let path = workbook::build_from(&dataset, store, &selected_options, &censuses)?;
    Ok(WorkbookReply {
        path: path.display().to_string(),
        grad_year: selected_options.grad_year,
    })
}

fn jurisdiction_codes(jurisdictions: &[census_domain::UsJurisdiction]) -> Vec<&'static str> {
    let jurisdictions = if jurisdictions.is_empty() {
        &census_domain::UsJurisdiction::CENSUS_SCOPE[..]
    } else {
        jurisdictions
    };
    let mut codes: Vec<_> = jurisdictions.iter().map(|row| row.code()).collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}
