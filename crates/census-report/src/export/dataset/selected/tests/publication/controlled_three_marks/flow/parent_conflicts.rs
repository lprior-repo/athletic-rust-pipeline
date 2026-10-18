use super::super::source::{self, InstalledSource, TestResult};
use crate::export::ExportDataset;
use crate::report::ReportError;
use census_domain::model::{CanonicalPerformance, CanonicalTeam, SchoolId};
use census_store::{Entity, StoreError, Table};
use sha2::{Digest, Sha256};

fn may15(dataset: &ExportDataset) -> TestResult<&CanonicalPerformance> {
    let performance = dataset
        .performances
        .iter()
        .find(|row| row.date == "2026-05-15")
        .ok_or("controlled May15 performance missing")?;
    check!(eq; crate::bests::mark_text(&performance.mark), "50.00");
    Ok(performance)
}

fn conflicting_team(
    dataset: &ExportDataset,
    current_school: &SchoolId,
) -> TestResult<CanonicalTeam> {
    let performance = may15(dataset)?;
    let mut team = dataset
        .teams
        .get(&performance.team)
        .ok_or("controlled team missing")?
        .clone();
    check!(&team.school != current_school);
    let mut divergent = team.clone();
    divergent.school = current_school.clone();
    team.merge(divergent);
    check!(!team.retained_conflicts.is_empty());
    Ok(team)
}

#[test]
fn conflicted_parent_refuses_new_capture_and_preserves_frozen_generation() -> TestResult {
    let installed = source::install()?;
    let (current_school, _) = super::reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;
    let objects = tempfile::tempdir()?;
    let dataset = super::super::selection::capture(&installed.store, &profile, objects.path())?;
    let original = may15(&dataset)?.clone();
    let frozen = tempfile::tempdir()?;
    let path = frozen.path().join("controlled-clean.json");
    dataset.save_frozen(&path)?;
    let original_bytes = std::fs::read(&path)?;
    let digest = format!("{:x}", Sha256::digest(&original_bytes));
    let conflict = conflicting_team(&dataset, &current_school)?;
    installed.store.append(Table::Teams, &conflict)?;
    let next_objects = tempfile::tempdir()?;
    let result = super::super::selection::capture(&installed.store, &profile, next_objects.path());
    let error = result.err().ok_or("conflicted parent was admitted")?;
    check!(matches!(
        error.downcast_ref::<ReportError>(),
        Some(ReportError::Store(StoreError::Refused { .. }))
    ));
    source::require_originals(&installed.store, &installed.source_records)?;
    drop(dataset);
    drop(objects);
    let InstalledSource {
        directory,
        store,
        identity_writer,
        ..
    } = installed;
    drop(identity_writer);
    drop(store);
    directory.close()?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &digest)?;
    check!(eq; may15(&reopened)?, &original);
    check!(eq; std::fs::read(&path)?, original_bytes);
    Ok(())
}
