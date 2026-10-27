use super::source::TestResult;
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use crate::workbook::{self, Censuses, Options};
use census_domain::model::QueryProfileV1;
use census_store::Store;
use std::path::{Path, PathBuf};

pub(super) fn publish(
    dataset: &ExportDataset,
    store: &Store,
    profile: &QueryProfileV1,
    root: &Path,
) -> TestResult<PathBuf> {
    dataset.verified_public_identities()?;
    dataset.verified_participation()?;
    let affiliations = dataset.verified_current_affiliations()?;
    let authority = dataset.selected_authority()?;
    if authority.profile().id()? != profile.id()? {
        return Err("controlled publication profile differs from frozen authority".into());
    }
    let selected_profile = authority.profile();
    let [cohort] = selected_profile.input().graduation_years.as_slice() else {
        return Err("controlled publication requires one selected cohort".into());
    };
    let options = Options {
        address_corpus: None,
        grad_year: Some(cohort.get()),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: selected_profile.input().jurisdictions.clone(),
        school_year: selected_profile.input().contact_school_year,
    };
    Derivation::selected(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
        affiliations,
    )?;
    let censuses = Censuses::of(dataset, &store.out_dir(), &options.jurisdictions)?;
    let path = workbook::build_from(dataset, store, &options, &censuses)?;
    workbook::publication::verify_published(&path)?;
    Ok(path)
}
