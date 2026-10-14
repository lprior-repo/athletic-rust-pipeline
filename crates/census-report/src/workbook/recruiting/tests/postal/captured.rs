use super::*;

#[test]
fn captured_zcum49_claim_reaches_school_athlete_and_csv_consumers_without_losing_public_email(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, athlete) = seed(&store)?;
    let (path, _, _) = published(&store)?;
    let mut book = open_workbook(&path)?;
    let schools = sheet(&mut book, "Schools")?;
    let school_row = row_of(&schools, school.id.as_str())?;
    for (header, expected) in expected_postal(&school) {
        check!(eq; text(&schools, school_row, column_of(&schools, header)?),
        expected,
        "Schools {header}");
    }
    let athletes = sheet(&mut book, "Athletes")?;
    check!(eq; text(
        &athletes,
        row_of(&athletes, athlete.id.as_str())?,
        column_of(&athletes, "School Address")?
    ),
    CAPTURED_ADDRESS);
    check!(eq; text(
        &athletes,
        row_of(&athletes, athlete.id.as_str())?,
        column_of(&athletes, "Head TF Coach Email")?
    ),
    "published@school.test");
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("missing recruiting row")??;
    for (name, expected) in [
        ("school_address", CAPTURED_ADDRESS),
        ("head_track_coach_email", "published@school.test"),
    ] {
        check!(eq; record.get(
            headers
                .iter()
                .position(|field| field == name)
                .ok_or_else(|| format!("missing CSV column {name}"))?
        ),
        Some(expected),
        "{name}");
    }
    Ok(())
}

#[test]
fn a_director_only_published_email_counts_as_a_cohort_contact_email() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = captured_school()?;
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Captured Director Contact Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "director-contact-runner"),
    )?;
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.evidence = evidence("nchsaa", Some(SUMMARY_URL));
    publish_fixture_cohort(&mut athlete, "nchsaa", "captured-director", CAPTURE_DAY);
    store.append(Table::Athletes, &athlete)?;
    coach(
        &store,
        &school.id,
        "Published Director",
        Some(Sport::OutdoorTrack),
        CoachRole::AthleticDirector,
        "director@school.test",
    )?;
    let (path, _, _) = published(&store)?;
    let summary: serde_json::Value = serde_json::from_reader(std::fs::File::open(
        path.parent()
            .ok_or("missing generation directory")?
            .join("census-all-sources.json"),
    )?)?;
    let totals = summary.get("totals").ok_or("missing totals")?;
    let counter = |name: &str| {
        totals
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .map_or(u64::MAX, core::convert::identity)
    };
    check!(eq; counter("class_of_2027_with_coach"), 1);
    check!(eq; counter("class_of_2027_with_coach_email"), 1);
    Ok(())
}

#[test]
fn tampering_any_published_postal_component_or_provenance_cell_fails_workbook_readback(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, _) = seed(&store)?;
    let (path, dataset, options) = published(&store)?;
    let target = dir.path().join("tampered.xlsx");
    for (name, header) in expected_postal(&school)
        .into_iter()
        .map(|(header, _)| ("Schools", header))
        .chain([("Athletes", "School Address")])
    {
        corrupt_workbook(&path, &target, name, header)?;
        let error = match verify_frozen(&target, &dataset, &options) {
            Err(error) => error.to_string(),
            Ok(_) => return Err(format!("forged {name} {header} accepted").into()),
        };
        check!(
            error.contains("forged postal claim"),
            "{name} {header}: {error}"
        );
    }
    Ok(())
}
