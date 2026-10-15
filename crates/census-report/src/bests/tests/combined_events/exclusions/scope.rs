use super::*;
use census_domain::model::{Evidence, SourceRef};

#[test]
fn combined_exclusions_obey_core_performance_evidence_filter() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "core-scope")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let mut dataset = ExportDataset::load(&store)?;
    let options = Options::default();
    check!(eq; build_with_exclusions(&dataset, &options)?.combined_exclusions.len(), 1);
    let performance = dataset
        .performances
        .first_mut()
        .ok_or("missing scoped performance")?;
    performance.evidence = vec![Evidence::parsed(
        SourceRef::new("athleticnet", Some("https://example.test/non-core".into())),
        "2026-10-14",
    )];
    let core = build_with_exclusions(&dataset, &options)?;
    check!(core.selections.is_empty());
    check!(core.combined_exclusions.is_empty());
    check!(eq; selection_outcome(&dataset)?.combined_exclusions.len(), 1);
    Ok(())
}

#[test]
fn combined_exclusions_obey_selected_jurisdictions() -> TestResult {
    let fixture = TieFixture::new()?;
    let observation = missing_completion(&fixture, "jurisdiction-scope")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&observation))?;
    let dataset = ExportDataset::load(&store)?;
    for (jurisdiction, expected) in [
        (UsJurisdiction::Wisconsin, 1),
        (UsJurisdiction::Illinois, 0),
    ] {
        let outcome = build_with_exclusions(
            &dataset,
            &Options {
                jurisdictions: vec![jurisdiction],
                ..Options::default()
            },
        )?;
        check!(outcome.selections.is_empty());
        check!(eq; outcome.combined_exclusions.len(), expected);
    }
    Ok(())
}

#[test]
fn combined_exclusions_are_not_truncated_by_winner_row_limit() -> TestResult {
    let fixture = TieFixture::new()?;
    let excluded = missing_completion(&fixture, "limited-exclusion")?;
    let eligible = pentathlon_report(
        &fixture,
        "limited-winner",
        400000,
        Some(pentathlon_specification(1)?),
    )?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[excluded, eligible])?;
    let dataset = ExportDataset::load(&store)?;
    let expected = selection_outcome(&dataset)?;
    check!(eq; expected.selections.len(), 1);
    check!(eq; expected.combined_exclusions.len(), 1);
    for limit in [0, 1] {
        let outcome = build_with_exclusions(
            &dataset,
            &Options {
                scope: Scope::AllSources,
                limit: Some(limit),
                ..Options::default()
            },
        )?;
        check!(eq; outcome.selections.len(), limit);
        check!(eq; outcome.combined_exclusions, expected.combined_exclusions);
    }
    Ok(())
}
