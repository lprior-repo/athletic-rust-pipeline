use super::*;

fn chunk_readback(
    fixture: &TieFixture,
    observations: &[Observation],
    expected: &Observation,
    case: &NumericCase,
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store_path = directory.path().join("store");
    let store = fixture.store_with_replay(&store_path, observations)?;
    let root = crate::export::freeze_v2(&store, "tie-artifacts:chunk-materialization")?;
    let replay = crate::export::freeze_v2(&store, "tie-artifacts:chunk-materialization")?;
    check!(eq; replay.manifest_digest(), root.manifest_digest());
    check!(eq; replay.attempt(), root.attempt());
    let stream = root
        .manifest()
        .streams
        .iter()
        .find(|stream| stream.stream == "performances")
        .ok_or("missing frozen performance stream")?;
    check!(eq; stream.rows, u64::try_from(observations.len())?);
    check!(stream.bytes > 0);
    store.flush()?;
    drop(store);
    let reopened = Store::open(&store_path)?;
    let materialized = crate::export::to_export_dataset(&root, &reopened)?;
    let row = assert_winner(&materialized, expected)?;
    check!(eq; row.result.value, case.value);
    check!(eq; materialized.lineage, root.manifest().lineage);
    let live = ExportDataset::load(&reopened)?;
    check!(eq; materialized.performances, live.performances);
    check!(eq; row, assert_winner(&live, expected)?);
    drop(live);
    let options = crate::workbook::Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: Some(directory.path().join("publication")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("chunk fixture season")?,
    };
    let censuses = crate::workbook::Censuses::of(&materialized, &reopened.out_dir(), &[])?;
    let path = crate::workbook::build_from(&materialized, &reopened, &options, &censuses)?;
    let generation = path.parent().ok_or("chunk publication generation")?;
    artifacts::assert_bests(generation, expected, &row)?;
    let mut book: Xlsx<_> = open_workbook(&path)?;
    assert_prs(&book.worksheet_range("PRs")?, expected, &row)?;
    assert_recruiting(&book.worksheet_range("Athletes")?, case, &row)?;
    drop(materialized);
    let archived = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    check!(eq; assert_winner(&archived, expected)?, row);
    drop(archived);
    check!(eq; crate::workbook::publication::verify_published(&path)?.workbook.mapped_athletes, 1);
    Ok(())
}

#[test]
fn chunk_materialized_tie_winners_keep_explicit_ids_and_provenance_in_every_artifact() -> TestResult
{
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let earlier = fixture.report(case, "workbook-earlier", "2024-02-29")?;
        check!(eq; earlier.performance.id.as_str(), EARLIEST_WORKBOOK_ID);
        let observations = [
            fixture.report(case, "workbook-later", "2026-04-01")?,
            fixture.report(case, "workbook-partial", "2023")?,
            earlier.clone(),
        ];
        orders(&observations)?
            .into_iter()
            .try_for_each(|order| chunk_readback(&fixture, &order, &earlier, case))
    })
}

#[test]
fn chunk_materialized_same_date_ties_keep_the_explicit_smaller_id() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = stable_id_reports(&fixture, case)?;
        let reversed = reverse_meet_order(&observations, case)?;
        [observations, reversed]
            .iter()
            .try_for_each(|observations| -> TestResult {
                let expected = observations.first().ok_or("chunk stable ID fixture")?;
                check!(eq; expected.performance.id.as_str(), SAME_DATE_WORKBOOK_ID);
                orders(observations)?
                    .into_iter()
                    .try_for_each(|order| chunk_readback(&fixture, &order, expected, case))
            })
    })
}
