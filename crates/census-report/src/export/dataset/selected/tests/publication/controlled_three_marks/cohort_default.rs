use super::flow::{assert_output, reviewed_inputs};
use super::readback::Generation;
use super::{selection, source};
use crate::report::{ReportError, Scope};
use crate::workbook::{self, Censuses};
use census_domain::model::ProfileSelectionV1;

#[test]
fn selected_publication_derives_cohort_without_accepting_an_override() -> source::TestResult {
    let installed = source::install()?;
    let (current, _accepted) = reviewed_inputs(&installed)?;
    let profile = selection::profile()?;
    installed.store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(installed.store.store_identity()?, profile.id()?);
    let output = tempfile::tempdir()?;
    let root = output.path().join("publication");
    let mut requested = workbook::Options {
        address_corpus: None,
        grad_year: Some(2028),
        out: Some(root.clone()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: profile.input().jurisdictions.clone(),
        school_year: profile.input().contact_school_year,
    };
    check!(matches!(
        workbook::options_for_selected_request(&installed.store, &selection, &requested),
        Err(ReportError::Invariant { .. })
    ));
    check!(!root.exists());
    requested.grad_year = None;
    let options = workbook::options_for_selected_request(&installed.store, &selection, &requested)?;
    let objects = tempfile::tempdir()?;
    let dataset = selection::capture(&installed.store, &profile, objects.path())?;
    let censuses = Censuses::of(&dataset, &installed.store.out_dir(), &options.jurisdictions)?;
    let workbook = workbook::build_from(&dataset, &installed.store, &options, &censuses)?;
    assert_output(
        &dataset,
        &workbook,
        &installed.public,
        &current,
        &installed.school,
        Generation::Initial,
    )
}
