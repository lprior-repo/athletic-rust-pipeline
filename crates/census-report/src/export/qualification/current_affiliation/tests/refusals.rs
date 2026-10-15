use super::*;

#[test]
fn missing_body_or_immutable_metadata_stays_unknown_despite_mutable_cache() -> TestResult {
    for missing in ["body", "metadata"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
        let path = if missing == "body" {
            &fixture.body_path
        } else {
            &fixture.metadata_path
        };
        let bytes = std::fs::read(path)?;
        std::fs::write(
            fixture.store.http_cache_dir().join("current.meta.json"),
            bytes,
        )?;
        std::fs::remove_file(path)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
        check!(pinned
            .unavailable()
            .get(&fixture.athlete)
            .is_some_and(|gaps| gaps.contains(&UnavailableAffiliation::ArchiveUnavailable)));
        check!(eq; pinned.into_verified().resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
    }
    Ok(())
}

#[test]
fn archive_body_and_metadata_hash_mismatches_refuse_the_bridge() -> TestResult {
    for changed in ["body", "metadata"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
        let path = if changed == "body" {
            &fixture.body_path
        } else {
            &fixture.metadata_path
        };
        let mut bytes = std::fs::read(path)?;
        let first = bytes.first_mut().ok_or("archive bytes")?;
        *first = b'!';
        std::fs::write(path, bytes)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        check!(PinnedCurrentAffiliations::read(&dataset, &snapshot).is_err());
    }
    Ok(())
}

#[test]
fn reader_requires_exact_snapshot_sequence_profile_and_unchanged_dataset() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let snapshot = fixture.store.snapshot();
    let mut dataset = fixture.dataset(&snapshot)?;
    fixture
        .store
        .journal_done("after-pinned-affiliation", "one", &true)?;
    check!(PinnedCurrentAffiliations::read(&dataset, &fixture.store.snapshot()).is_err());
    check!(eq; PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified().resolution(&fixture.athlete).school(), Some(&fixture.school));
    let first = dataset.athletes.first_mut().ok_or("athlete")?;
    first.canonical_name.push_str(" changed");
    check!(PinnedCurrentAffiliations::read(&dataset, &snapshot).is_err());
    let mut wrong_cutoff = fixture.dataset(&snapshot)?;
    wrong_cutoff
        .bind_selection_for_test(fixture.profile.id()?, ProfileDate::parse("2026-10-08")?)?;
    check!(PinnedCurrentAffiliations::read(&wrong_cutoff, &snapshot).is_err());
    Ok(())
}

#[test]
fn row_metadata_cannot_change_provider_person_or_original_acquisition_time() -> TestResult {
    for field in ["provider_athlete_id", "fetched_at"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
        let mut athlete = fixture
            .store
            .scan::<CanonicalAthlete>(Table::Athletes)?
            .into_iter()
            .next()
            .ok_or("athlete")?;
        let evidence = athlete.evidence.first_mut().ok_or("evidence")?;
        let mut note: serde_json::Value =
            serde_json::from_str(evidence.note.as_deref().ok_or("note")?)?;
        *note.get_mut(field).ok_or("field")? = if field == "fetched_at" {
            "2026-10-07T18:00:00Z".into()
        } else {
            "2".into()
        };
        evidence.note = Some(serde_json::to_string(&note)?);
        fixture.store.append(Table::Athletes, &athlete)?;
        fixture::accept(&fixture.store, &fixture.writer, &fixture.athlete)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        check!(PinnedCurrentAffiliations::read(&dataset, &snapshot).is_err());
    }
    Ok(())
}

#[test]
fn missing_same_day_competitor_cannot_be_hidden_by_one_readable_school() -> TestResult {
    let fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    let (other, _, body, _) = fixture::append_roster(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    std::fs::remove_file(body)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let pinned = PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    check!(pinned
        .unavailable()
        .get(&canonical)
        .is_some_and(|gaps| gaps.contains(&UnavailableAffiliation::ArchiveUnavailable)));
    let verified = pinned.into_verified();
    check!(eq; verified.resolution(&canonical), &CurrentAffiliationResolution::Unknown);
    let audit = verified.audit(&canonical).ok_or("unknown audit")?;
    check!(eq; audit.references().len(), 2);
    check!(eq; audit.unavailable(), &["archive_unavailable".to_string()]);
    identity::assert_unselected(&dataset, &verified, &canonical)?;
    Ok(())
}

#[test]
fn mutable_alias_map_cannot_change_selected_membership_without_a_decision() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let snapshot = fixture.store.snapshot();
    let mut dataset = fixture.dataset(&snapshot)?;
    let digest = census_domain::model::serialized_digest(&dataset)?;
    dataset
        .canonical_aliases
        .insert("foreign-subject".into(), fixture.athlete.as_str().into());
    check!(eq; census_domain::model::serialized_digest(&dataset)?, digest);
    check!(PinnedCurrentAffiliations::read(&dataset, &snapshot).is_err());
    Ok(())
}
