use super::*;

#[test]
fn controlled_reviewed_roster_positive_selected_publication_reopens_offline_with_nonempty_athlete(
) -> TestResult {
    roundtrip(Case::Resolved)
}

#[test]
fn controlled_reviewed_roster_conflict_and_gap_publications_preserve_withholding_offline(
) -> TestResult {
    for case in [Case::Conflicted, Case::MissingOwner] {
        roundtrip(case)?;
    }
    Ok(())
}

fn roundtrip(case: Case) -> TestResult {
    let (fixture, expected) = setup::prepare(case)?;
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    let before = witness(&dataset, &expected)?;
    let lineage = dataset.lineage.clone();
    let digest = dataset.archive_digest()?;
    let frozen = archive.path().join("controlled-reviewed-source.json");
    dataset.save_frozen(&frozen)?;
    inspect_wire(&frozen)?;
    let root = output.path().join("publication");
    let workbook = publish(&fixture, &dataset, &root)?;
    let verified = published::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, u64::try_from(expected.selected_count())?);
    check!(eq; published::current_workbook(&root)?, workbook);
    let generation = workbook.parent().ok_or("controlled published generation")?;
    inspect_wire(&generation.join("frozen-input.json"))?;
    let rows = artifacts::readback(&workbook, &dataset, &expected)?;
    check!(eq; witness(&dataset, &expected)?, before);
    drop(dataset);
    remove_sources(fixture, objects)?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    check!(eq; reopened.lineage, lineage);
    check!(eq; witness(&reopened, &expected)?, before);
    check!(eq; reopened.archive_digest()?, digest);
    let publication =
        published::open_publication(&workbook, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; publication.workbook.mapped_athletes, u64::try_from(expected.selected_count())?);
    check!(eq; witness(&publication.dataset, &expected)?, before);
    check!(eq; artifacts::readback(&workbook, &publication.dataset, &expected)?, rows);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified.generation_digest);
    Ok(())
}
