use super::*;

#[test]
fn controlled_established_school_conflict_survives_cross_sport_unresolved_person() -> TestResult {
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
    let unresolved = install(&fixture, "12441", "43", Sport::CrossCountry, true)?;
    let mut connected = first.athlete.clone();
    connected.add_identity(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "43"));
    fixture.store.append(Table::Athletes, &connected)?;
    issuance::review(&fixture, &first, "controlled-unresolved-conflict-first")?;
    issuance::review(&fixture, &second, "controlled-unresolved-conflict-second")?;
    issuance::review(
        &fixture,
        &unresolved,
        "controlled-unresolved-conflict-third",
    )?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&first.athlete.id), &Resolution::Conflicted);
    check!(eq; verified.resolution(&unresolved.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("complete unresolved conflict inventory")?.len(), 3);
    check!(verified
        .audit(&first.athlete.id)
        .ok_or("conflict unresolved audit")?
        .references()
        .iter()
        .any(|reference| reference == "roster_selector_outcome:CrossCountry:SubjectUnresolved"));
    unselected(&fixture, &verified, &first.athlete.id)
}

#[test]
fn controlled_recycled_provider_id_without_exact_physical_row_cannot_inherit_accepted_person(
) -> TestResult {
    for name in ["Runner, Controlled", "Recycled, Different Person"] {
        let fixture = Fixture::new()?;
        let accepted = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
        fixture.accept(&accepted.athlete.id)?;
        let recycled = fixture::install_row(
            &fixture,
            "12440",
            "42",
            Sport::OutdoorTrack,
            true,
            0,
            name,
            fixture::PhysicalProjection::Omit,
        )?;
        check!(ne; recycled.packet.roster().body()?, accepted.packet.roster().body()?);
        check!(fixture
            .store
            .scan::<CanonicalAthlete>(Table::Athletes)?
            .iter()
            .all(|row| row.id != recycled.athlete.id));
        issuance::review(&fixture, &accepted, "controlled-reuse-accepted")?;
        issuance::review(&fixture, &recycled, "controlled-reuse-unprojected")?;
        let (verified, inventory) = fixture.assess()?;
        check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("complete recycled provider inventory")?.len(), 2);
        check!(eq; verified.resolution(&accepted.athlete.id), &Resolution::Unknown);
        unselected(&fixture, &verified, &accepted.athlete.id)?;
    }
    Ok(())
}

#[test]
fn controlled_provider_alias_connected_to_two_accepted_people_is_not_selected() -> TestResult {
    let fixture = Fixture::new()?;
    let first = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    let second = install(&fixture, "12440", "43", Sport::OutdoorTrack, true)?;
    fixture.accept(&second.athlete.id)?;
    let mut conflicting = second.athlete.clone();
    conflicting.add_identity(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "42"));
    fixture.store.append(Table::Athletes, &conflicting)?;
    issuance::review(&fixture, &first, "controlled-accepted-alias-first")?;
    issuance::review(&fixture, &second, "controlled-accepted-alias-second")?;
    let (verified, inventory) = fixture.assess()?;
    check!(eq; verified.resolution(&first.athlete.id), &Resolution::Unknown);
    check!(eq; verified.resolution(&second.athlete.id), &Resolution::Unknown);
    check!(eq; inventory.get(1).and_then(Value::as_array).ok_or("accepted conflict inventory")?.len(), 2);
    unselected(&fixture, &verified, &first.athlete.id)?;
    unselected(&fixture, &verified, &second.athlete.id)
}
