use super::super::documentary_lifecycle;
use super::super::source::{self, InstalledSource, TestResult};
use crate::report::{ReportError, Scope};
use crate::workbook::publication;
use census_domain::model::GradYear;
use census_store::StoreError;

fn open_for_seal(
    path: &std::path::Path,
    store: &census_store::Store,
    profile: &census_domain::model::QueryProfileV1,
) -> crate::report::ReportResult<publication::OpenedPublication> {
    publication::open_publication_for_seal(
        path,
        &crate::export::store_lineage(store)?,
        Scope::AllSources,
        GradYear::CO2027,
        &profile.input().jurisdictions,
    )
}

fn initial_publication(
    installed: &InstalledSource,
    profile: &census_domain::model::QueryProfileV1,
    objects: &std::path::Path,
    output: &std::path::Path,
) -> TestResult<(crate::export::ExportDataset, std::path::PathBuf)> {
    installed.store.register_profile(profile)?;
    let (dataset, path) = super::publish(
        &installed.store,
        profile,
        objects,
        &output.join("publication"),
    )?;
    Ok((dataset, path))
}

#[test]
fn seal_authority_withdrawal_blocks_current_seal_but_preserves_historical_publication() -> TestResult
{
    let installed = source::install()?;
    let (current_school, accepted50) = super::reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    let public = installed.public.clone();
    let historical_school = installed.school.clone();

    let objects0 = tempfile::tempdir()?;
    let output0 = tempfile::tempdir()?;
    let (g0, workbook0) =
        initial_publication(&installed, &profile, objects0.path(), output0.path())?;
    super::assert_output(
        &g0,
        &workbook0,
        &public,
        &current_school,
        &historical_school,
        false,
    )?;

    let verified = publication::verify_published(&workbook0)?;
    let original_digest = verified.generation_digest;
    let original_bytes = std::fs::read(&workbook0)?;

    let opened = open_for_seal(&workbook0, &installed.store, &profile)?;
    check!(eq; opened.generation_digest, original_digest);

    let before_lineage = crate::export::store_lineage(&installed.store)?;

    installed
        .store
        .journal_done("controlled-seal-maintenance", "progress", &true)?;

    let after_maintenance = crate::export::store_lineage(&installed.store)?;
    check!(eq; after_maintenance.source_digest, before_lineage.source_digest);
    check!(eq; after_maintenance.input_generation, before_lineage.input_generation);
    check!(after_maintenance.snapshot_sequence > before_lineage.snapshot_sequence);

    let maintenance_opened = open_for_seal(&workbook0, &installed.store, &profile)?;
    check!(eq; maintenance_opened.generation_digest, original_digest);

    documentary_lifecycle::revoke(&installed, &accepted50)?;

    let after_revoke_lineage = crate::export::store_lineage(&installed.store)?;
    check!(eq; after_revoke_lineage.source_digest, before_lineage.source_digest);

    match open_for_seal(&workbook0, &installed.store, &profile) {
        Err(ReportError::Store(StoreError::Refused { .. })) => {}
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("withdrawn authority admitted for current sealing".into()),
    }

    let g0_lineage = g0.lineage.clone();

    let objects1 = tempfile::tempdir()?;
    let output1 = tempfile::tempdir()?;
    let (g1, workbook1) =
        initial_publication(&installed, &profile, objects1.path(), output1.path())?;
    check!(g1.lineage.snapshot_sequence > g0_lineage.snapshot_sequence);
    check!(publication::verify_published(&workbook1)?.generation_digest != original_digest);
    super::assert_output(
        &g1,
        &workbook1,
        &public,
        &current_school,
        &historical_school,
        true,
    )?;

    let g1_opened = open_for_seal(&workbook1, &installed.store, &profile)?;
    check!(eq;
        g1_opened.generation_digest,
        publication::verify_published(&workbook1)?.generation_digest
    );

    installed
        .store
        .journal_done("controlled-g1-maintenance", "progress", &true)?;
    let g1_after_maintenance = open_for_seal(&workbook1, &installed.store, &profile)?;
    check!(eq;
        g1_after_maintenance.generation_digest,
        publication::verify_published(&workbook1)?.generation_digest
    );

    source::require_originals(&installed.store, &installed.source_records)?;

    drop(g0);
    drop(g1);
    drop(opened);
    drop(maintenance_opened);
    drop(g1_opened);
    drop(g1_after_maintenance);

    let InstalledSource {
        directory, store, ..
    } = installed;
    drop(store);
    directory.close()?;
    objects0.close()?;
    objects1.close()?;
    let reopened = publication::open_publication(
        &workbook0,
        &g0_lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    super::assert_output(
        &reopened.dataset,
        &workbook0,
        &public,
        &current_school,
        &historical_school,
        false,
    )?;
    check!(eq; reopened.generation_digest, original_digest);
    check!(eq; std::fs::read(&workbook0)?, original_bytes);
    check!(eq; publication::verify_published(&workbook0)?.generation_digest, original_digest);
    drop(reopened);
    super::retain_outputs(output0, output1)?;

    Ok(())
}
