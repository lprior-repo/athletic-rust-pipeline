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
    options_for_profile(profile, out)
}

fn options_for_profile(
    profile: &census_domain::model::QueryProfileV1,
    out: Option<PathBuf>,
) -> ReportResult<Options> {
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

pub fn options_for_selected_request(
    store: &census_store::Store,
    selection: &census_domain::model::ProfileSelectionV1,
    requested: &Options,
) -> ReportResult<Options> {
    let profile = store.snapshot().selected_profile(selection)?;
    let mut selected = options_for_profile(&profile, requested.out.clone())?;
    if !same_jurisdictions(&requested.jurisdictions, &selected.jurisdictions) {
        return Err(crate::report::ReportError::Invariant {
            detail: "selected jurisdictions differ from the bound profile".into(),
        });
    }
    if let Some(cohort) = requested.grad_year {
        selected.grad_year = Some(cohort);
    }
    selected.school_year = requested.school_year;
    selected.scope = requested.scope;
    selected.limit = requested.limit;
    super::publication::selected::ensure_profile_options(&profile, &selected)?;
    Ok(selected)
}

fn same_jurisdictions(actual: &[UsJurisdiction], expected: &[UsJurisdiction]) -> bool {
    let actual = if actual.is_empty() {
        &UsJurisdiction::CENSUS_SCOPE[..]
    } else {
        actual
    };
    let expected = if expected.is_empty() {
        &UsJurisdiction::CENSUS_SCOPE[..]
    } else {
        expected
    };
    actual.iter().all(|row| expected.contains(row))
        && expected.iter().all(|row| actual.contains(row))
}
