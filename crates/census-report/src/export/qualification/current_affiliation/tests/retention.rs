use super::*;

#[test]
fn unknown_old_and_unaccepted_observations_keep_fixed_unavailability_reasons() -> TestResult {
    for (cutoff, accepted, reason) in [
        ("2026-10-08", true, "outside_capture_day"),
        ("2026-10-07", false, "identity_unaccepted"),
    ] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", cutoff, accepted)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
        check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
        let audit = verified.audit(&fixture.athlete).ok_or("unknown audit")?;
        check!(audit.references().is_empty());
        check!(eq; audit.unavailable(), &[reason.to_string()]);
    }
    Ok(())
}

#[test]
fn source_verified_row_outside_selected_sport_keeps_references_and_reason() -> TestResult {
    let mut fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let mut input = fixture.profile.input().clone();
    input.sports = vec![Sport::CrossCountry];
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&fixture.profile)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
    let audit = verified
        .audit(&fixture.athlete)
        .ok_or("unselected sport audit")?;
    check!(eq; audit.references().len(), 2);
    check!(eq; audit.unavailable(), &["outside_selected_sport".to_string()]);
    Ok(())
}

#[test]
fn source_verified_cooperative_program_keeps_references_without_inventing_campus() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let mut school = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .next()
        .ok_or("school")?;
    school.co_op = true;
    fixture.store.append(Table::Schools, &school)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::CoopCampusUnresolved);
    let audit = verified
        .audit(&fixture.athlete)
        .ok_or("cooperative program audit")?;
    check!(eq; audit.references().len(), 2);
    check!(audit.unavailable().is_empty());
    Ok(())
}

#[test]
fn unrelated_large_historical_notes_do_not_change_current_affiliation() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let mut athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .next()
        .ok_or("athlete")?;
    let mut evidence = athlete.evidence.first().ok_or("evidence")?.clone();
    evidence.note = Some(serde_json::to_string(&serde_json::json!({
        "role": "historical_source_diagnostic", "retained": "x".repeat(17 * 1024),
    }))?);
    check!(note::RowNote::read(&evidence)?.is_none());
    athlete.evidence.push(evidence.clone());
    fixture.store.append(Table::Athletes, &athlete)?;
    fixture::accept(&fixture.store, &fixture.writer, &fixture.athlete)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let before = serde_json::to_vec(&dataset)?;
    let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; verified.resolution(&fixture.athlete).school(), Some(&fixture.school));
    check!(eq; serde_json::to_vec(&dataset)?, before);
    evidence.note = Some(serde_json::to_string(&serde_json::json!({
        "role": "roster_positive_row_metadata_v1", "retained": "x".repeat(17 * 1024),
    }))?);
    check!(note::RowNote::read(&evidence).is_err());
    Ok(())
}
