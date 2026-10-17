use super::*;

pub(super) mod operation;
mod oracle;

#[test]
fn native_genuine_documentary_revocation_replays_without_reactivation_and_preserves_prior_publication(
) -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let archives = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let expected = fixture.expected.clone();
    let before = inspect(&dataset, &expected)?;
    let frozen = archives.path().join("genuine-documentary.json");
    dataset.save_frozen(&frozen)?;
    let digest = dataset.archive_digest()?;
    let lineage = dataset.lineage.clone();
    let root = output.path().join("publication");
    let prior = publish(&fixture, &dataset, &root)?;
    drop(dataset);
    let generation = prior.parent().ok_or("prior generation")?;
    let inventory = fixture::generation_inventory(generation)?;
    let verified = publication::verify_published(&prior)?.generation_digest;
    let pointer = std::fs::read_link(root.join("current"))?;
    let revoked = operation::revoke(&fixture.store)?;
    operation::replay_and_require_revoked(&fixture.store, &revoked)?;
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; fixture::generation_inventory(generation)?, inventory);
    let fixture = fixture.reopen()?;
    operation::replay_and_require_revoked(&fixture.store, &revoked)?;
    let current_objects = tempfile::tempdir()?;
    let current = fixture.capture(current_objects.path())?;
    oracle::empty(&current)?;
    let current_lineage = current.lineage.clone();
    let workbook = publish(&fixture, &current, &root)?;
    drop(current);
    check!(ne; workbook, prior);
    check!(eq; publication::current_workbook(&root)?, workbook);
    oracle::published(&workbook)?;
    check!(eq; fixture::generation_inventory(generation)?, inventory);
    check!(eq; publication::verify_published(&prior)?.generation_digest, verified);
    current_objects.close()?;
    let store_path = fixture.store.root().to_path_buf();
    let objects_path = objects.path().to_path_buf();
    fixture.close()?;
    objects.close()?;
    check!(!store_path.exists());
    check!(!objects_path.exists());
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    check!(eq; reopened.lineage, lineage);
    check!(eq; inspect(&reopened, &expected)?, before);
    drop(reopened);
    let historical =
        publication::open_publication(&prior, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; inspect(&historical.dataset, &expected)?, before);
    csv(generation, &historical.dataset, &expected)?;
    drop(historical);
    check!(eq; fixture::generation_inventory(generation)?, inventory);
    check!(eq; publication::verify_published(&prior)?.generation_digest, verified);
    oracle::published(&workbook)?;
    let current = publication::open_publication(
        &workbook,
        &current_lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    oracle::empty(&current.dataset)?;
    check!(eq; publication::current_workbook(&root)?, workbook);
    Ok(())
}
