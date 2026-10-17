use super::*;

#[test]
fn controlled_unresolved_connected_person_in_another_sport_withholds_resolved_school() -> TestResult
{
    let fixture = Fixture::new()?;
    let valid = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&valid.athlete.id)?;
    let unresolved = install(&fixture, "12440", "43", Sport::OutdoorTrack, true)?;
    let mut connected = valid.athlete.clone();
    connected.add_identity(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "43"));
    fixture.store.append(Table::Athletes, &connected)?;
    issuance::review(&fixture, &valid, "controlled-cross-sport-valid")?;
    issuance::review(&fixture, &unresolved, "controlled-cross-sport-unresolved")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&valid.athlete.id), &Resolution::Unknown);
    check!(eq; verified.resolution(&unresolved.athlete.id), &Resolution::Unknown);
    let captures = inventory
        .get(1)
        .and_then(Value::as_array)
        .ok_or("complete cross-sport inventory")?;
    check!(eq; captures.len(), 2);
    let joins = captures
        .iter()
        .filter_map(|capture| capture.get(4).and_then(Value::as_array))
        .flatten()
        .filter_map(|member| member.get(2))
        .collect::<Vec<_>>();
    check!(eq; joins.iter().filter(|join| join.get("Accepted").is_some()).count(), 1);
    check!(eq; joins.iter().filter(|join| join.get("Unresolved").is_some()).count(), 1);
    check!(verified
        .audit(&valid.athlete.id)
        .ok_or("cross-sport unresolved audit")?
        .references()
        .iter()
        .any(|reference| reference == "roster_selector_outcome:OutdoorTrack:SubjectUnresolved"));
    unselected(&fixture, &verified, &valid.athlete.id)
}

#[test]
fn controlled_duplicate_physical_provider_alias_prevents_accepted_identity_selection() -> TestResult
{
    let fixture = Fixture::new()?;
    let accepted = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&accepted.athlete.id)?;
    let other = install(&fixture, "12440", "42", Sport::IndoorTrack, true)?;
    issuance::review(&fixture, &accepted, "controlled-duplicate-first")?;
    issuance::review(&fixture, &other, "controlled-duplicate-second")?;
    let (verified, _) = fixture.assess()?;
    check!(eq; verified.resolution(&accepted.athlete.id), &Resolution::Unknown);
    check!(eq; verified.resolution(&other.athlete.id), &Resolution::Unknown);
    unselected(&fixture, &verified, &accepted.athlete.id)
}

#[test]
fn controlled_capture_exact_utc_day_cannot_be_reinterpreted_as_continuing_tenure() -> TestResult {
    for day in ["2026-10-15", "2026-10-17"] {
        let mut fixture = Fixture::new()?;
        fixture.profile = fixture::profile(day)?;
        fixture.store.register_profile(&fixture.profile)?;
        let capture = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
        fixture.accept(&capture.athlete.id)?;
        issuance::review(&fixture, &capture, "controlled-exact-day")?;
        let (verified, _) = fixture.assess()?;
        check!(eq; verified.resolution(&capture.athlete.id), &Resolution::Unknown);
        unselected(&fixture, &verified, &capture.athlete.id)?;
    }
    Ok(())
}

#[test]
fn controlled_current_statement_without_bounded_source_spans_withholds_resolution() -> TestResult {
    let fixture = Fixture::new()?;
    let valid = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&valid.athlete.id)?;
    let oversized =
        fixture::install_padded(&fixture, "12440", "42", Sport::OutdoorTrack, true, 4097)?;
    crate::export::qualification::current_affiliation::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &valid.athlete.id,
        &oversized.athlete.id,
    )?;
    let packet = oversized.packet.read()?;
    let assertion = packet
        .roster_verdict()
        .roster()
        .and_then(|roster| roster.athletes.first())
        .and_then(|row| row.current_roster())
        .ok_or("retained current statement")?;
    check!(matches!(
        assertion.source_statements_v1(),
        Err(census_crawl::milesplit::RosterWitnessUnavailableV1::SourceSpanLimit)
    ));
    issuance::review(&fixture, &valid, "controlled-bounded-first")?;
    issuance::review(&fixture, &oversized, "controlled-oversized-membership")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&valid.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("retained span gap family")?.len(), 2);
    unselected(&fixture, &verified, &valid.athlete.id)
}

#[test]
fn controlled_connected_physical_capture_without_review_withholds_resolved_school() -> TestResult {
    let fixture = Fixture::new()?;
    let valid = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&valid.athlete.id)?;
    let unreviewed = install(&fixture, "12440", "42", Sport::OutdoorTrack, true)?;
    crate::export::qualification::current_affiliation::tests::identity::accept_pair(
        &fixture.store,
        &fixture.writer,
        &valid.athlete.id,
        &unreviewed.athlete.id,
    )?;
    issuance::review(&fixture, &valid, "controlled-missing-reviewed-capture")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&valid.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("reviewed capture inventory")?.len(), 1);
    check!(eq; inventory.get(2).and_then(|physical| physical.get("gaps")).and_then(Value::as_array).ok_or("unreviewed physical capture inventory")?.len(), 1);
    check!(verified
        .audit(&valid.athlete.id)
        .ok_or("physical gap audit")?
        .unavailable()
        .contains(&"archive_unavailable".to_string()));
    unselected(&fixture, &verified, &valid.athlete.id)
}
