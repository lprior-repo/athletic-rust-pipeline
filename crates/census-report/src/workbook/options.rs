use crate::report::{ReportResult, Scope};
use census_domain::UsJurisdiction;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
    pub scope: Scope,
    pub jurisdictions: Vec<UsJurisdiction>,
    pub school_year: census_domain::model::SchoolYear,
}

pub fn options_for_selected_dataset(
    dataset: &crate::export::ExportDataset,
    out: Option<PathBuf>,
) -> ReportResult<Options> {
    let profile = dataset.selected_authority()?.profile();
    let input = profile.input();
    let [cohort] = input.graduation_years.as_slice() else {
        return Err(crate::report::ReportError::Invariant {
            detail: "selected workbook profile must contain exactly one graduation year".into(),
        });
    };
    Ok(Options {
        grad_year: Some(cohort.get()),
        out,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: input.jurisdictions.clone(),
        school_year: input.contact_school_year,
    })
}
