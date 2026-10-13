use super::*;

#[test]
fn missing_capture_url_or_date_produces_no_effect_receipt_or_witness() -> TestResult {
    for (url, date) in [
        (None, Some("2026-08-01")),
        (Some("https://nces.ed.gov/ccd.csv"), None),
    ] {
        let (_dir, store) = store_with_school()?;
        let index = DirectoryIndex::build(&[entry(
            "390000000001",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "1 Main St",
        )?]);
        let (store, before) = physical::checkpoint(store)?;
        let (counts, outcomes) = process(&store, &index, &lanes(url, date), Mode::Apply)?;
        check!(eq; counts.linked, 0);
        check!(eq; counts.evidence_missing, 1);
        check!(eq; outcomes.len(), 1);
        check!(eq; outcomes.first().map(|row| row.outcome.as_str()), Some("evidence_missing"));
        witness::assert_no_application(&store)?;
        let (_store, after) = physical::checkpoint(store)?;
        check!(eq; after, before);
    }
    Ok(())
}

#[test]
fn foreign_capture_membership_cannot_create_an_application_witness() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let foreign = IdentifiedKey::Nces(NcesSchoolId::parse("390000000099")?);
    let lanes = lanes_of(vec![
        lane_of(
            "nces-ccd",
            "capture-one.csv",
            Some("https://nces.ed.gov/one.csv"),
            Some("2026-08-01"),
            vec![foreign.clone()],
        ),
        lane_of(
            "nces-ccd",
            "capture-two.csv",
            Some("https://nces.ed.gov/two.csv"),
            Some("2026-08-01"),
            vec![foreign],
        ),
    ]);
    let (store, before) = physical::checkpoint(store)?;
    let (counts, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; counts.linked, 0);
    check!(eq; counts.evidence_missing, 1);
    check!(outcomes
        .iter()
        .any(|row| row.detail.as_deref().is_some_and(
            |detail| detail.contains("no capture") && detail.contains("390000000001")
        )));
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    Ok(())
}

#[test]
fn a_foreign_provider_key_pair_is_refused_without_receipted_mutations() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let base = entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?;
    let foreign = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?),
        SourceLabel::Pss,
        Some(SchoolName::parse("Springfield High School")?),
    )
    .with_address(base.address().cloned())
    .with_grades(base.grades());
    let index = DirectoryIndex::build(&[foreign]);
    let (store, before) = physical::checkpoint(store)?;
    let (counts, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counts.refused, 1);
    check!(eq; counts.linked, 0);
    check!(outcomes.iter().any(|row| row
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("cannot own a directory identity"))));
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    Ok(())
}
