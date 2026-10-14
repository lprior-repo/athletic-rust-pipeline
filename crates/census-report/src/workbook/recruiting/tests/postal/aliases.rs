use super::*;

#[test]
fn accepted_alias_school_addresses_match_the_representative_in_both_formats() -> TestResult {
    accepted_alias_school_address(Some("2 Alias Way"))
}

#[test]
fn an_alias_address_does_not_fill_the_representative_school_unknown_address() -> TestResult {
    accepted_alias_school_address(None)
}

fn accepted_alias_school_address(selected_address: Option<&str>) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (captured, alternate) = schools(&store, selected_address)?;
    accept_members(&store, &captured, &alternate)?;
    seed_coaches(&store, &captured, &alternate)?;
    let (path, dataset, _) = published(&store)?;
    let canonical = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]);
    let [athlete] = canonical.athletes() else {
        return Err("one accepted athlete required".into());
    };
    check!(eq; athlete.school, alternate.id);
    let primary = selected_address.map_or("", core::convert::identity);
    assert_workbook(&path, athlete, &captured, &alternate, primary)?;
    assert_csv(&path, athlete, &alternate, primary)?;
    crate::workbook::publication::verify_published(&path)?;
    Ok(())
}

fn schools(
    store: &Store,
    selected_address: Option<&str>,
) -> TestResult<(CanonicalSchool, CanonicalSchool)> {
    let captured = captured_school()?;
    store.append(Table::Schools, &captured)?;
    use census_domain::model::SchoolPostalAddress;
    use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};
    let (mut alternate, _) = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Alternate Affiliation",
        "alternate affiliation",
        None,
    );
    if let Some(address) = selected_address {
        let owner = SourceIdentity::new(
            SourceNamespace::association_school("synthetic-alias"),
            "alias-school",
        );
        alternate.source_identities.push(owner.clone());
        alternate.add_postal_address(SchoolPostalAddress::new(
            PostalAddress::line(StreetLine::parse(address)?),
            owner,
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::NorthCarolina,
            },
            Evidence::parsed(
                SourceRef::new("synthetic-alias", Some("https://fixture.test/alias".into())),
                CAPTURE_DAY,
            ),
            "c".repeat(64),
        )?)?;
    }
    store.append(Table::Schools, &alternate)?;
    Ok((captured, alternate))
}

fn accept_members(
    store: &Store,
    captured: &CanonicalSchool,
    alternate: &CanonicalSchool,
) -> TestResult {
    let profile = "https://nc.milesplit.com/athletes/9001002";
    let source =
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001002").with_url(profile);
    let mut first = CanonicalAthlete::new(
        &captured.id,
        "Affiliated Runner 1",
        GradYear::CO2027,
        Gender::Boys,
        source.clone(),
    )?;
    let mut second = CanonicalAthlete::new(
        &alternate.id,
        "Affiliated Runner 1",
        GradYear::CO2027,
        Gender::Boys,
        source,
    )?;
    check!(captured.id < alternate.id);
    check!(second.id < first.id);
    for member in [&mut first, &mut second] {
        member.sports = vec![Sport::OutdoorTrack];
        member.evidence = vec![Evidence::parsed(
            SourceRef::new("milesplit", Some(profile.into())),
            CAPTURE_DAY,
        )];
    }
    publish_fixture_cohort(&mut first, "milesplit", "postal-alias-first", CAPTURE_DAY);
    publish_fixture_cohort(&mut second, "milesplit", "postal-alias-second", CAPTURE_DAY);
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    identity::accept(store, &[first, second])?;
    Ok(())
}

fn seed_coaches(
    store: &Store,
    captured: &CanonicalSchool,
    alternate: &CanonicalSchool,
) -> TestResult {
    for (school, name, email) in [
        (&captured.id, "Alias School Coach", "alias@school.test"),
        (
            &alternate.id,
            "Representative School Coach",
            "representative@school.test",
        ),
    ] {
        coach(
            store,
            school,
            name,
            Some(Sport::OutdoorTrack),
            CoachRole::HeadCoach,
            email,
        )?;
    }
    Ok(())
}

fn assert_workbook(
    path: &std::path::Path,
    athlete: &CanonicalAthlete,
    captured: &CanonicalSchool,
    alternate: &CanonicalSchool,
    primary: &str,
) -> TestResult {
    let mut book = open_workbook(path)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, athlete.id.as_str())?;
    for (header, expected) in [
        ("School ID", alternate.id.as_str()),
        ("School", alternate.name.as_str()),
        ("School Address", primary),
        ("Head TF Coach Email", "representative@school.test"),
    ] {
        check!(eq; text(&range, row, column_of(&range, header)?), expected, "{header}");
    }
    let schools = sheet(&mut book, "Schools")?;
    for (school, expected) in [(captured, "1 Rocket Drive"), (alternate, primary)] {
        let row = row_of(&schools, school.id.as_str())?;
        check!(eq; text(&schools, row, column_of(&schools, "Postal Street")?), expected);
    }
    Ok(())
}

fn assert_csv(
    path: &std::path::Path,
    athlete: &CanonicalAthlete,
    alternate: &CanonicalSchool,
    primary: &str,
) -> TestResult {
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    let [record] = records.as_slice() else {
        return Err("one recruiting CSV athlete required".into());
    };
    check!(eq; record.get(0), Some(athlete.id.as_str()));
    for (header, expected) in [
        ("school", alternate.name.as_str()),
        ("school_address", primary),
        ("head_track_coach_email", "representative@school.test"),
    ] {
        let column = headers
            .iter()
            .position(|field| field == header)
            .ok_or_else(|| format!("missing CSV column {header}"))?;
        check!(eq; record.get(column), Some(expected), "{header}");
    }
    Ok(())
}
