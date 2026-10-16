use super::*;

#[test]
fn transfer_into_scope_changes_directory_but_keeps_performance_school() -> TestResult {
    let mut fixture = Fixture::new()?;
    let before = serde_json::to_vec(&fixture.dataset)?;
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
    let athlete = derivation
        .athletes()
        .first()
        .ok_or("missing transfer-in athlete")?;
    check!(eq; &athlete.school, &fixture.current);
    check!(eq; derivation.performances().len(), 1);
    let performance = derivation
        .performances()
        .first()
        .ok_or("missing historical result")?;
    let historical = fixture
        .dataset
        .teams
        .get(&performance.team)
        .ok_or("missing historical team")?;
    check!(eq; &historical.school, &fixture.previous);
    check!(eq; serde_json::to_vec(&fixture.dataset)?, before);
    check!(derivation.affiliation_exclusions().is_empty());
    Ok(())
}

#[test]
fn transfer_out_is_excluded_and_records_current_geography_reason() -> TestResult {
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
        &[UsJurisdiction::Wisconsin],
    )?;
    check!(derivation.athletes().is_empty());
    check!(derivation.performances().is_empty());
    check!(eq; derivation.outside_athletes().len(), 1);
    check!(eq; derivation.affiliation_exclusions().get(&fixture.athlete),
        Some(&CurrentAffiliationExclusion::OutsideRequestedJurisdictions));
    Ok(())
}

#[test]
fn unresolved_capture_day_affiliations_never_inherit_historical_school() -> TestResult {
    let mut fixture = Fixture::new()?;
    for resolution in [
        None,
        Some(CurrentAffiliationResolution::Unknown),
        Some(CurrentAffiliationResolution::Conflicted),
        Some(CurrentAffiliationResolution::CoopCampusUnresolved),
    ] {
        let expected = resolution
            .as_ref()
            .map_or("current_affiliation_unknown", |row| row.status());
        let affiliations = view(&fixture, "2026-10-01", resolution)?;
        bind_selected(&mut fixture, &affiliations)?;
        let derivation = Derivation::of(
            &fixture.dataset,
            Scope::AllSources,
            None,
            &[UsJurisdiction::Wisconsin],
        )?;
        check!(derivation.athletes().is_empty());
        check!(derivation.performances().is_empty());
        check!(eq; derivation.affiliation_exclusions().get(&fixture.athlete),
            Some(&CurrentAffiliationExclusion::Unresolved));
        check!(eq; derivation.current_affiliation(&fixture.athlete).map(|row| row.status()), Some(expected));
    }
    Ok(())
}

#[test]
fn current_school_with_unknown_jurisdiction_stays_excluded() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture
        .dataset
        .schools
        .get_mut(&fixture.current)
        .ok_or("current school")?
        .state = None;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "claim")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let derivation = Derivation::of(
        &fixture.dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Michigan],
    )?;
    check!(derivation.athletes().is_empty());
    check!(eq; derivation.affiliation_exclusions().get(&fixture.athlete),
        Some(&CurrentAffiliationExclusion::SchoolJurisdictionUnknown));
    Ok(())
}

#[test]
fn selected_cutoff_uses_its_bound_resolution_not_newest_canonical_field() -> TestResult {
    for cutoff in ["2026-09-30", "2026-10-01", "2026-09-30"] {
        let mut fixture = Fixture::at_cutoff(cutoff)?;
        let expected = match cutoff {
            "2026-09-30" => fixture.previous.clone(),
            _ => fixture.current.clone(),
        };
        let bound = view(
            &fixture,
            cutoff,
            Some(resolved(&expected, "bound-dated-claim")),
        )?;
        bind_selected(&mut fixture, &bound)?;
        let derivation = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
        check!(eq; &derivation.athletes().first().ok_or("selected athlete")?.school, &expected);
        check!(eq; &fixture.dataset.athletes.first().ok_or("frozen athlete")?.school, &fixture.previous);
    }
    Ok(())
}
