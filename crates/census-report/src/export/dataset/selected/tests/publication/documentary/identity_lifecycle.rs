use super::*;
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution};
use census_store::{PublicIdentityWriterV1, Store, Table};

mod attachment_retirement;
mod controlled;
mod correction;
mod oracle;
mod prior;
mod review;

#[derive(Clone, Copy)]
enum Branch {
    Split,
    RevokeRedundantMerge,
}

#[test]
fn native_genuine_attachment_controlled_identity_split_refuses_current_target_and_preserves_g0(
) -> TestResult {
    run(Branch::Split)
}

#[test]
fn native_genuine_attachment_controlled_identity_revocation_keeps_surviving_alias_and_preserves_g0(
) -> TestResult {
    run(Branch::RevokeRedundantMerge)
}

fn run(branch: Branch) -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let archives = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let root = output.path().join("publication");
    let prior = prior::Prior::publish(&fixture, objects.path(), archives.path(), &root)?;
    let (fixture, writer) = fixture.reopen_with_writer()?;
    let controlled = controlled::install(&fixture.store, &writer)?;
    let merged = correction::merge(&fixture.store, &writer, &controlled)?;
    oracle::moved(&fixture)?;
    oracle::merged(&fixture.store, &controlled, merged)?;
    prior.preserved(&root)?;
    let corrected = correction::correct(&fixture.store, &writer, &controlled, merged, branch)?;
    oracle::moved(&fixture)?;
    let expected = oracle::corrected(&fixture.store, &controlled, corrected, merged, branch)?;
    prior.preserved(&root)?;
    let _revoked = super::lifecycle::operation::revoke(&fixture.store)?;
    oracle::historical_status_target_moved(&fixture)?;
    prior.preserved(&root)?;
    drop(writer);
    let fixture = fixture.reopen()?;
    check!(eq; oracle::corrected(&fixture.store, &controlled, corrected, merged, branch)?, expected);
    oracle::historical_status_target_moved(&fixture)?;
    let retired = attachment_retirement::retire_status(&fixture.store)?;
    retired.replay_without_reactivation(&fixture.store)?;
    prior.preserved(&root)?;
    let fixture = fixture.reopen()?;
    retired.replay_without_reactivation(&fixture.store)?;
    check!(eq; oracle::corrected(&fixture.store, &controlled, corrected, merged, branch)?, expected);
    let current_objects = tempfile::tempdir()?;
    let dataset = fixture.capture(current_objects.path())?;
    oracle::dataset(&dataset, &controlled, &expected)?;
    let lineage = dataset.lineage.clone();
    let workbook = publish(&fixture, &dataset, &root)?;
    drop(dataset);
    check!(ne; workbook, prior.workbook);
    oracle::publication(&workbook, &controlled, &expected)?;
    prior.generation_preserved()?;
    fixture.require_original_unchanged()?;
    let store_path = fixture.store.root().to_path_buf();
    fixture.close()?;
    objects.close()?;
    current_objects.close()?;
    check!(!store_path.exists());
    prior.replay()?;
    let reopened =
        publication::open_publication(&workbook, &lineage, Scope::AllSources, GradYear::CO2027)?;
    oracle::dataset(&reopened.dataset, &controlled, &expected)?;
    drop(reopened);
    oracle::publication(&workbook, &controlled, &expected)?;
    check!(eq; publication::current_workbook(&root)?, workbook);
    Ok(())
}
