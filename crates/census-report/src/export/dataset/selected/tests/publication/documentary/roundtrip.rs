use super::*;

#[test]
fn native_genuine_documentary_selected_publication_reopens_without_live_store_or_source_objects(
) -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let archives = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let expected = fixture.expected.clone();
    let before = inspect(&dataset, &expected)?;
    let attachment = attachment_object(&dataset, objects.path())?;
    check!(objects.path().join(&attachment).is_file());
    let frozen = archives.path().join("genuine-documentary.json");
    dataset.save_frozen(&frozen)?;
    let digest = dataset.archive_digest()?;
    let lineage = dataset.lineage.clone();
    let root = output.path().join("publication");
    let workbook = publish(&fixture, &dataset, &root)?;
    let verified = publication::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    let generation = workbook.parent().ok_or("published generation")?;
    csv(generation, &dataset, &expected)?;
    check!(eq; publication::current_workbook(&root)?, workbook);
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/participation_exclusions.csv").and_then(Value::as_u64), Some(3));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/athletes.csv").and_then(Value::as_u64), Some(0));
    fixture.require_original_unchanged()?;
    let store_path = fixture.store.root().to_path_buf();
    let object_path = objects.path().to_path_buf();
    fixture.close()?;
    objects.close()?;
    drop(dataset);
    check!(!store_path.exists());
    check!(!object_path.exists());
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    check!(eq; reopened.lineage, lineage);
    check!(eq; inspect(&reopened, &expected)?, before);
    let publication =
        publication::open_publication(&workbook, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; publication.workbook.mapped_athletes, 0);
    check!(eq; inspect(&publication.dataset, &expected)?, before);
    csv(generation, &publication.dataset, &expected)?;
    check!(eq; publication::verify_published(&workbook)?.generation_digest, verified.generation_digest);
    check!(historical_retained::require_v3(&reopened, &frozen).is_err());
    fixture::retain_v3_archive(
        &reopened,
        archives.path(),
        &frozen,
        &digest,
        &lineage,
        &before,
    )?;
    Ok(())
}

#[test]
fn native_genuine_documentary_v4_co2026_stays_unresolved_with_current_frozen_parity() -> TestResult
{
    let fixture = Fixture::open_for(GradYear::CO2026)?;
    let objects = tempfile::tempdir()?;
    let archives = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let expected = fixture.expected.clone();
    let before = inspect(&dataset, &expected)?;
    attachment_object(&dataset, objects.path())?;
    let frozen = archives.path().join("genuine-co2026.json");
    dataset.save_frozen(&frozen)?;
    let digest = dataset.archive_digest()?;
    fixture.require_original_unchanged()?;
    fixture.close()?;
    objects.close()?;
    drop(dataset);
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    check!(eq; inspect(&reopened, &expected)?, before);
    Ok(())
}
