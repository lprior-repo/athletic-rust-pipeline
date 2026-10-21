use census_report::export::ExportDataset;
use census_report::report::ReportResult;
use census_report::workbook;
use census_store::Store;

use super::super::wire::WorkbookReply;

pub(in super::super) fn build(
    store: &Store,
    options: &workbook::Options,
    job: &str,
    export_job: &census_report::export::ExportJobRequest,
) -> ReportResult<WorkbookReply> {
    let census_report::export::ExportJobRequest::SelectedPublicationV1 { selection } = export_job;
    let selected_options = workbook::options_for_selected_request(store, selection, options)?;
    let dataset = ExportDataset::for_job_request(store, job, export_job)?;
    let censuses =
        workbook::Censuses::of(&dataset, &store.out_dir(), &selected_options.jurisdictions)?;
    let path = workbook::build_from(&dataset, store, &selected_options, &censuses)?;
    Ok(WorkbookReply {
        path: path.display().to_string(),
        grad_year: selected_options.grad_year,
    })
}
