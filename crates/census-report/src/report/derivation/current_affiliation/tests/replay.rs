use super::*;

#[test]
fn historical_frozen_replay_keeps_original_school_and_bytes() -> TestResult {
    let mut fixture = Fixture::retained()?;
    fixture.dataset.bind_historical_policy_for_test(21)?;
    let path = fixture.directory.path().join("frozen.json");
    fixture.dataset.save_frozen(&path)?;
    let original = std::fs::read(&path)?;
    let reopened = ExportDataset::reopen_frozen(&path)?;
    let historical = Derivation::of(
        &reopened,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Wisconsin],
    )?;
    check!(matches!(
        historical.current_affiliations(),
        CurrentAffiliationView::Historical
    ));
    check!(eq; &historical.athletes().first().ok_or("historical athlete")?.school, &fixture.previous);
    check!(historical.current_affiliation(&fixture.athlete).is_none());
    check!(historical.affiliation_exclusions().is_empty());
    check!(eq; std::fs::read(&path)?, original);
    Ok(())
}

#[test]
fn selected_recruiting_csv_joins_current_school() -> TestResult {
    let mut fixture = Fixture::new()?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "dated-transfer")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let derivation = Derivation::of(
        &fixture.dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Michigan],
    )?;
    let path = fixture.directory.path().join("current.csv");
    crate::workbook::write_recruiting_csv(
        &derivation,
        census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
        &path,
    )?;
    let mut reader = csv::Reader::from_path(&path)?;
    let header = reader.headers()?.clone();
    let school = header
        .iter()
        .position(|name| name == "school")
        .ok_or("school column")?;
    let state = header
        .iter()
        .position(|name| name == "state")
        .ok_or("state column")?;
    let mut records = reader.records();
    let record = records.next().ok_or("athlete record")??;
    check!(eq; record.get(school), Some("Current school"));
    check!(eq; record.get(state), Some("MI"));
    check!(records.next().is_none());
    Ok(())
}

#[test]
fn transfer_coverage_reconciles_current_outside_population() -> TestResult {
    let mut fixture = Fixture::new()?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "dated-transfer")),
    )?;
    for (scope, expected) in [
        (UsJurisdiction::Michigan, 1),
        (UsJurisdiction::Wisconsin, 0),
    ] {
        bind_selected(&mut fixture, &affiliations)?;
        let derivation = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[scope])?;
        let report = crate::report::coverage_report_for_derivation(&derivation)?;
        check!(eq; report.read.athletes, expected);
        check!(eq; report.read.performances, expected);
        check!(report
            .notes
            .iter()
            .any(|note| note.starts_with("current affiliation geography:")));
    }
    Ok(())
}
