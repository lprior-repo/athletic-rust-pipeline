use super::*;

fn corrupt_occurrence_refused(fixture: &Fixture) -> TestResult {
    let _dataset = fixture.dataset()?;
    let failure = match fixture.assess() {
        Ok(_) => return Err("corrupt controlled physical occurrence was accepted".into()),
        Err(error) => error,
    };
    check!(
        matches!(failure.downcast_ref::<ReportError>(), Some(ReportError::Store(StoreError::Refused { detail })) if detail == "physical roster row differs from its source-owned school candidate")
    );
    Ok(())
}

#[test]
fn controlled_foreign_school_copied_row_note_is_refused_even_with_accepted_provider_id(
) -> TestResult {
    let fixture = Fixture::new()?;
    let accepted = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&accepted.athlete.id)?;
    let foreign = fixture::install_row(
        &fixture,
        "12440",
        "42",
        Sport::OutdoorTrack,
        true,
        0,
        "Runner, Controlled",
        fixture::PhysicalProjection::Omit,
    )?;
    let mut copied = accepted.athlete.clone();
    copied.evidence = foreign.athlete.evidence.clone();
    fixture.store.append(Table::Athletes, &copied)?;
    issuance::review(&fixture, &accepted, "controlled-copied-origin")?;
    issuance::review(&fixture, &foreign, "controlled-copied-foreign")?;
    corrupt_occurrence_refused(&fixture)
}

#[test]
fn controlled_exact_row_with_changed_candidate_id_is_refused() -> TestResult {
    let fixture = Fixture::new()?;
    let accepted = install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&accepted.athlete.id)?;
    let mut changed = accepted.athlete.clone();
    changed.id = AthleteId::mint(
        "ath",
        &["controlled-invalid-candidate-for-exact-source-row"],
    );
    check!(ne; changed.id, accepted.athlete.id);
    fixture.store.append(Table::Athletes, &changed)?;
    issuance::review(&fixture, &accepted, "controlled-changed-candidate-id")?;
    corrupt_occurrence_refused(&fixture)
}
