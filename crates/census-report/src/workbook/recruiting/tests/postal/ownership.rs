use super::*;

#[test]
fn a_same_named_school_without_a_claim_does_not_inherit_another_schools_postal_address(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let captured = captured_school()?;
    store.append(Table::Schools, &captured)?;
    let other = school(&store, UsJurisdiction::Wisconsin, &captured.name)?;
    let mut athlete = CanonicalAthlete::new(
        &other,
        "Unrelated Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "unrelated"),
    )?;
    publish_fixture_cohort(&mut athlete, "wiaa_results", "unrelated-school-postal", DAY);
    store.append(Table::Athletes, &athlete)?;
    let (path, _, _) = published(&store)?;
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, athlete.id.as_str())?;
    check!(eq; text(&range, row, column_of(&range, "School Address")?), "");
    Ok(())
}

#[test]
fn contradictory_postal_addresses_keep_aligned_provenance_on_both_workbook_sheets_and_csv(
) -> TestResult {
    use census_domain::model::SchoolPostalAddress;
    use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (mut school, athlete) = seed(&store)?;
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("synthetic-conflict"),
        "other-claim",
    );
    school.source_identities.push(owner.clone());
    let claim = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Review Road")?),
        owner,
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::NorthCarolina,
        },
        Evidence::parsed(
            SourceRef::new(
                "synthetic-conflict",
                Some("https://fixture.test/conflict".into()),
            ),
            "2026-09-28",
        ),
        "b".repeat(64),
    )?;
    school.add_postal_address(claim)?;
    store.append(Table::Schools, &school)?;
    let (path, _, _) = published(&store)?;
    let mut book = open_workbook(&path)?;
    let schools = sheet(&mut book, "Schools")?;
    let school_row = row_of(&schools, school.id.as_str())?;
    for (header, expected) in [
        ("Postal Street", "1 Rocket Drive\n2 Review Road".to_owned()),
        ("Postal City", "Asheville\n".into()),
        ("Postal State", "NC\n".into()),
        ("Postal ZIP", "28803\n".into()),
        ("Postal Owner ID", "ZCUM49\nother-claim".into()),
        (
            "Postal Source URL",
            format!("{SUMMARY_URL}\nhttps://fixture.test/conflict"),
        ),
        ("Postal Observed Date", "2026-09-27\n2026-09-28".into()),
        (
            "Postal Capture SHA256",
            format!("{:x}\n{}", Sha256::digest(SUMMARY), "b".repeat(64)),
        ),
    ] {
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
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("missing recruiting row")??;
    let address_column = headers
        .iter()
        .position(|value| value == "school_address")
        .ok_or("missing CSV column school_address")?;
    check!(eq; record.get(address_column), Some(CAPTURED_ADDRESS));
    Ok(())
}

#[test]
fn unresolved_same_named_athletes_keep_postal_affiliations_distinct_in_workbook_and_csv(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (captured, first) = seed(&store)?;
    let other = school(&store, UsJurisdiction::Wisconsin, &captured.name)?;
    let mut second = CanonicalAthlete::new(
        &other,
        &first.canonical_name,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "distinct-postal-homonym"),
    )?;
    publish_fixture_cohort(&mut second, "wiaa_results", "postal-homonym", DAY);
    store.append(Table::Athletes, &second)?;
    let (path, dataset, _) = published(&store)?;
    check!(dataset.canonical_aliases.is_empty());
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    check!(eq; range.height(), 3);
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 2);
    for (athlete, has_claim) in [(&first, true), (&second, false)] {
        let row = row_of(&range, athlete.id.as_str())?;
        let record = records
            .iter()
            .find(|record| record.get(0) == Some(athlete.id.as_str()))
            .ok_or("missing recruiting row for homonym")?;
        let expected = if has_claim { CAPTURED_ADDRESS } else { "" };
        check!(eq; text(&range, row, column_of(&range, "School Address")?),
        expected,
        "School Address");
        let column = headers
            .iter()
            .position(|field| field == "school_address")
            .ok_or("missing CSV column school_address")?;
        check!(eq; record.get(column), Some(expected), "school_address");
    }
    Ok(())
}
