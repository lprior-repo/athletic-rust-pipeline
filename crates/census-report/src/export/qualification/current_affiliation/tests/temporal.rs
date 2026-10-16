use super::*;

#[test]
fn valid_undated_roster_retains_references_but_cannot_resolve_affiliation() -> TestResult {
    let fixture = Fixture::with_heading("2026-10-07T12:00:00Z", "2026-10-07", true, "Roster")?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    check!(eq; pinned.rows.get(&fixture.athlete), Some(&CurrentAffiliationResolution::Unknown));
    let audit = pinned.into_verified();
    let audit = audit.audit(&fixture.athlete).ok_or("undated audit")?;
    check!(eq; audit.references().len(), 2);
    check!(eq; audit.unavailable(), &["missing_temporal_authority".to_string()]);
    Ok(())
}

#[test]
fn competing_valid_undated_school_withholds_dated_school() -> TestResult {
    let fixture = Fixture::with_heading("2026-10-07T09:00:00Z", "2026-10-07", true, "Roster")?;
    let (other, _, _, _) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; verified.resolution(&canonical), &CurrentAffiliationResolution::Unknown);
    let audit = verified.audit(&canonical).ok_or("competing audit")?;
    check!(eq; audit.references().len(), 4);
    check!(eq; audit.unavailable(), &["missing_temporal_authority".to_string()]);
    identity::assert_unselected(&dataset, &verified, &canonical)?;
    Ok(())
}

#[test]
fn scoped_current_heading_does_not_cover_previous_or_next_day() -> TestResult {
    for cutoff in ["2026-10-06", "2026-10-08"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", cutoff, true)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
        check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
        check!(eq; verified.audit(&fixture.athlete).ok_or("day audit")?.unavailable(), &["outside_capture_day".to_string()]);
    }
    Ok(())
}

#[test]
fn frozen_undated_roster_keeps_unknown_after_live_archive_removal() -> TestResult {
    let fixture = Fixture::with_heading("2026-10-07T12:00:00Z", "2026-10-07", true, "Roster")?;
    let directory = tempfile::tempdir()?;
    let objects = directory.path().join("objects");
    std::fs::create_dir(&objects)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, original) = frozen::capture(&dataset, &snapshot, &objects)?;
    let digest = descriptor.digest()?;
    std::fs::remove_file(&fixture.body_path)?;
    std::fs::remove_file(&fixture.metadata_path)?;
    let replayed =
        frozen::replay(&dataset, &objects, &descriptor, &expected, &digest)?.into_verified();
    let original = original.into_verified();
    check!(eq; replayed.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
    check!(eq; replayed.audit(&fixture.athlete).ok_or("replay audit")?.references(), original.audit(&fixture.athlete).ok_or("original audit")?.references());
    check!(eq; replayed.audit(&fixture.athlete).ok_or("replay audit")?.unavailable(), &["missing_temporal_authority".to_string()]);
    Ok(())
}

#[test]
fn unresolved_extra_row_does_not_clear_existing_school_conflict() -> TestResult {
    let fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    let (other, _, _, _) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    fixture::append_roster_heading(
        &fixture.store,
        "52649",
        "Old High",
        "2026-10-07T18:00:00Z",
        Sport::OutdoorTrack,
        "Roster",
    )?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; verified.resolution(&canonical), &CurrentAffiliationResolution::Conflicted);
    check!(verified
        .audit(&canonical)
        .ok_or("conflict audit")?
        .unavailable()
        .contains(&"missing_temporal_authority".to_string()));
    Ok(())
}

#[test]
fn observed_day_uses_utc_while_exact_source_offset_is_retained() -> TestResult {
    let fixture = Fixture::new("2026-10-07T23:30:00-05:00", "2026-10-08", true)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    let athlete = dataset
        .athletes
        .iter()
        .find(|athlete| athlete.id == fixture.athlete)
        .ok_or("athlete")?;
    let evidence = athlete
        .evidence
        .iter()
        .find(|evidence| note::RowNote::read(evidence).ok().flatten().is_some())
        .ok_or("roster evidence")?;
    let context = pinned.roster_context();
    let dated = context
        .dated_membership(&fixture.athlete, evidence)?
        .ok_or("dated membership")?;
    check!(eq; dated.observed_at, "2026-10-07T23:30:00-05:00");
    check!(eq; dated.observed_on.as_str(), "2026-10-08");
    check!(eq; dated.day_convention, super::super::context::ObservationDayConvention::UtcCaptureDay);
    check!(eq; pinned.into_verified().resolution(&fixture.athlete).school(), Some(&fixture.school));
    Ok(())
}
