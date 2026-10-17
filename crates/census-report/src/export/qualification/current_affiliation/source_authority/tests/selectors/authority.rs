use super::*;

#[test]
fn controlled_checked_review_and_same_publication_identity_select_school_and_geography(
) -> TestResult {
    let fixture = Fixture::new()?;
    let capture = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&capture.athlete.id)?;
    issuance::review(&fixture, &capture, "controlled-school-selection")?;
    let dataset = fixture.dataset()?;
    let before = serde_json::to_vec(&dataset)?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&capture.athlete.id).school(), Some(&capture.school.id));
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("family inventory")?.len(), 1);
    let selected = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Michigan],
        &verified,
    )?;
    check!(eq; selected.athletes().len(), 1);
    check!(eq; &selected.athletes().first().ok_or("selected athlete")?.school, &capture.school.id);
    let outside = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Wisconsin],
        &verified,
    )?;
    check!(outside.athletes().is_empty());
    check!(eq; serde_json::to_vec(&dataset)?, before);
    Ok(())
}

#[test]
fn controlled_review_cannot_establish_unaccepted_person_identity() -> TestResult {
    let fixture = Fixture::new()?;
    let capture = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    issuance::review(&fixture, &capture, "controlled-unaccepted")?;
    let (verified, _) = fixture.assess()?;
    check!(eq; verified.resolution(&capture.athlete.id), &Resolution::Unknown);
    let dataset = fixture.dataset()?;
    let selected = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Michigan],
        &verified,
    )?;
    check!(selected.athletes().is_empty());
    Ok(())
}

#[test]
fn controlled_connected_missing_owner_withholds_otherwise_resolved_school() -> TestResult {
    let fixture = Fixture::new()?;
    let valid = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&valid.athlete.id)?;
    let missing = install(&fixture, "12440", "42", Sport::OutdoorTrack, false)?;
    crate::export::qualification::current_affiliation::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &valid.athlete.id,
        &missing.athlete.id,
    )?;
    issuance::review(&fixture, &valid, "controlled-valid-owner")?;
    issuance::review(&fixture, &missing, "controlled-missing-owner")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&valid.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("complete family inventory")?.len(), 2);
    check!(verified
        .audit(&valid.athlete.id)
        .ok_or("authority gap audit")?
        .unavailable()
        .contains(&"missing_temporal_authority".to_string()));
    unselected(&fixture, &verified, &valid.athlete.id)
}

#[test]
fn controlled_unrelated_missing_owner_does_not_poison_resolved_subject() -> TestResult {
    let fixture = Fixture::new()?;
    let valid = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&valid.athlete.id)?;
    let missing = install(&fixture, "12440", "43", Sport::OutdoorTrack, false)?;
    fixture.accept(&missing.athlete.id)?;
    issuance::review(&fixture, &valid, "controlled-unrelated-valid")?;
    issuance::review(&fixture, &missing, "controlled-unrelated-gap")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&valid.athlete.id).school(), Some(&valid.school.id));
    check!(eq; verified.resolution(&missing.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("complete unrelated inventory")?.len(), 2);
    Ok(())
}

#[test]
fn controlled_competing_school_claims_preserve_conflict_alongside_connected_owner_gap() -> TestResult
{
    let fixture = Fixture::new()?;
    let first = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    let second = install(&fixture, "12440", "42", Sport::OutdoorTrack, true)?;
    crate::export::qualification::current_affiliation::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &first.athlete.id,
        &second.athlete.id,
    )?;
    let missing = install(&fixture, "12441", "42", Sport::CrossCountry, false)?;
    attach(
        &fixture,
        &first.athlete.id,
        &missing.athlete.id,
        "controlled-third-school",
    )?;
    issuance::review(&fixture, &first, "controlled-first-conflict")?;
    issuance::review(&fixture, &second, "controlled-second-conflict")?;
    issuance::review(&fixture, &missing, "controlled-conflict-owner-gap")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&first.athlete.id), &Resolution::Conflicted);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("all competing claims")?.len(), 3);
    unselected(&fixture, &verified, &first.athlete.id)
}
