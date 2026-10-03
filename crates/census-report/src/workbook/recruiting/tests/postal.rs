mod ordering;

use super::*;
use crate::export::postal::tests::{captured_school, CAPTURE_DAY, SUMMARY, SUMMARY_URL};
use crate::export::ExportDataset;
use crate::workbook::{verify::verify_frozen, Options};
use sha2::{Digest, Sha256};

fn seed(store: &Store) -> TestResult<(CanonicalSchool, CanonicalAthlete)> {
    let school = captured_school()?;
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Captured School Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-runner"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.evidence = evidence("nchsaa", Some(SUMMARY_URL));
    publish_fixture_cohort(&mut athlete, "nchsaa", "captured-postal", CAPTURE_DAY);
    store.append(Table::Athletes, &athlete)?;
    coach(
        store,
        &school.id,
        "Published Contact",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "published@school.test",
    )?;
    Ok((school, athlete))
}

fn published(store: &Store) -> TestResult<(std::path::PathBuf, ExportDataset, Options)> {
    let options = Options {
        school_year: Some(SchoolYear::new(2026).ok_or("invalid fixture season")?),
        ..Options::default()
    };
    let path = crate::workbook::build(store, &options)?;
    let generation = path.parent().ok_or("missing generation directory")?;
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    verify_frozen(&path, &dataset, &options)?;
    Ok((path, dataset, options))
}

fn expected_postal(school: &CanonicalSchool) -> [(&'static str, String); 12] {
    [
        ("Postal School ID", school.id.to_string()),
        ("Postal Street", "1 Rocket Drive".into()),
        ("Postal Second Line", "".into()),
        ("Postal City", "Asheville".into()),
        ("Postal State", "NC".into()),
        ("Postal ZIP", "28803".into()),
        ("Postal Owner Namespace", "association_school:nchsaa".into()),
        ("Postal Owner ID", "ZCUM49".into()),
        ("Postal Source", "athletic-association:NC".into()),
        ("Postal Source URL", SUMMARY_URL.into()),
        ("Postal Observed Date", CAPTURE_DAY.into()),
        (
            "Postal Capture SHA256",
            format!("{:x}", Sha256::digest(SUMMARY)),
        ),
    ]
}

#[test]
fn captured_zcum49_claim_reaches_school_athlete_and_csv_consumers_without_losing_public_email(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, athlete) = seed(&store)?;
    let (path, _, _) = published(&store)?;
    let mut book = open_workbook(&path)?;
    for (name, id) in [
        ("Schools", school.id.as_str()),
        ("Athletes", athlete.id.as_str()),
    ] {
        let range = sheet(&mut book, name)?;
        let row = row_of(&range, id)?;
        for (header, expected) in expected_postal(&school) {
            check!(eq; text(&range, row, column_of(&range, header)?),
            expected,
            "{name} {header}");
        }
    }
    let athletes = sheet(&mut book, "Athletes")?;
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
        ("postal_street", "1 Rocket Drive".to_owned()),
        ("postal_city", "Asheville".into()),
        ("postal_state", "NC".into()),
        ("postal_zip", "28803".into()),
        ("postal_owner_id", "ZCUM49".into()),
        ("postal_source_url", SUMMARY_URL.into()),
        ("postal_observed_date", CAPTURE_DAY.into()),
        (
            "postal_capture_sha256",
            format!("{:x}", Sha256::digest(SUMMARY)),
        ),
        ("head_track_coach_email", "published@school.test".into()),
    ] {
        check!(eq; record.get(
            headers
                .iter()
                .position(|field| field == name)
                .ok_or_else(|| format!("missing CSV column {name}"))?
        ),
        Some(expected.as_str()),
        "{name}");
    }
    Ok(())
}

fn corrupt_workbook(
    source: &std::path::Path,
    target: &std::path::Path,
    name: &str,
    header: &str,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = Workbook::new();
    for sheet_name in reader.sheet_names() {
        let range = reader.worksheet_range(&sheet_name)?;
        let selected = if sheet_name == name {
            Some(column_of(&range, header)?)
        } else {
            None
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&sheet_name)?;
        for (row, cells) in range.rows().enumerate() {
            for (column, value) in cells.iter().enumerate() {
                let r = u32::try_from(row)?;
                let c = u16::try_from(column)?;
                if row == 1 && selected == Some(column) {
                    sheet.write_string(r, c, "forged postal claim")?;
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(r, c, *value)?;
                    }
                    Data::Int(value) => {
                        sheet.write_number(r, c, *value as f64)?;
                    }
                    _ => {
                        sheet.write_string(r, c, value.to_string())?;
                    }
                }
            }
        }
    }
    writer.save(target)?;
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
    for name in ["Schools", "Athletes"] {
        for (header, _) in expected_postal(&school) {
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
    }
    Ok(())
}

#[test]
fn accepted_alias_school_affiliations_preserve_their_source_owned_postal_claims() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let captured = captured_school()?;
    store.append(Table::Schools, &captured)?;
    use census_domain::model::SchoolPostalAddress;
    use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};
    let (mut alternate, other) = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Alternate Affiliation",
        "alternate affiliation",
    );
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("synthetic-alias"),
        "alias-school",
    );
    alternate.source_identities.push(owner.clone());
    alternate.add_postal_address(SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Alias Way")?),
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
    store.append(Table::Schools, &alternate)?;
    let profile = "https://nc.milesplit.com/athletes/9001002";
    let source =
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001002").with_url(profile);
    let mut first = CanonicalAthlete::new(
        &captured.id,
        "Affiliated Runner",
        GradYear::CO2027,
        Gender::Boys,
        source.clone(),
    );
    let mut second =
        CanonicalAthlete::new(&other, "A. Runner", GradYear::CO2027, Gender::Boys, source);
    for member in [&mut first, &mut second] {
        member.evidence = vec![Evidence::parsed(
            SourceRef::new("milesplit", Some(profile.into())),
            CAPTURE_DAY,
        )];
    }
    publish_fixture_cohort(&mut first, "milesplit", "postal-alias-first", CAPTURE_DAY);
    publish_fixture_cohort(&mut second, "milesplit", "postal-alias-second", CAPTURE_DAY);
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    identity::accept(&store, &[first, second])?;
    let (path, dataset, _) = published(&store)?;
    let canonical = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let [athlete] = canonical.athletes() else {
        return Err("one accepted athlete required".into());
    };
    let alternate_fields = [
        other.to_string(),
        "2 Alias Way".into(),
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        "association_school:synthetic-alias".into(),
        "alias-school".into(),
        "athletic-association:NC".into(),
        "https://fixture.test/alias".into(),
        CAPTURE_DAY.into(),
        "c".repeat(64),
    ];
    let captured_fields = expected_postal(&captured);
    let expected: [String; 12] = std::array::from_fn(|column| {
        let captured_value = &captured_fields[column].1;
        let alternate_value = &alternate_fields[column];
        if captured.id < other {
            format!("{captured_value}\n{alternate_value}")
        } else {
            format!("{alternate_value}\n{captured_value}")
        }
    });
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, athlete.id.as_str())?;
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    check!(eq; records[0].get(0), Some(athlete.id.as_str()));
    for (column, (header, _)) in captured_fields.iter().enumerate() {
        check!(eq; text(&range, row, column_of(&range, header)?),
        expected[column],
        "{header}");
        let csv_header = crate::export::postal::POSTAL_CSV_HEADERS[column];
        let csv_column = headers
            .iter()
            .position(|field| field == csv_header)
            .ok_or_else(|| format!("missing CSV column {csv_header}"))?;
        check!(eq; records[0].get(csv_column),
        Some(expected[column].as_str()),
        "{csv_header}");
    }
    Ok(())
}

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
    );
    publish_fixture_cohort(&mut athlete, "wiaa_results", "unrelated-school-postal", DAY);
    store.append(Table::Athletes, &athlete)?;
    let (path, _, _) = published(&store)?;
    let mut book = open_workbook(&path)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, athlete.id.as_str())?;
    for (header, _) in expected_postal(&captured) {
        check!(eq; text(&range, row, column_of(&range, header)?),
        "",
        "{header}");
    }
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
    for (name, id) in [
        ("Schools", school.id.as_str()),
        ("Athletes", athlete.id.as_str()),
    ] {
        let range = sheet(&mut book, name)?;
        let row = row_of(&range, id)?;
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
            check!(eq; text(&range, row, column_of(&range, header)?),
            expected,
            "{name} {header}");
        }
    }
    let mut reader = ::csv::Reader::from_path(
        path.parent()
            .ok_or("missing generation directory")?
            .join("recruiting.csv"),
    )?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("missing recruiting row")??;
    for (header, expected) in [
        ("postal_street", "1 Rocket Drive\n2 Review Road".to_owned()),
        ("postal_owner_id", "ZCUM49\nother-claim".into()),
        (
            "postal_source_url",
            format!("{SUMMARY_URL}\nhttps://fixture.test/conflict"),
        ),
        ("postal_observed_date", "2026-09-27\n2026-09-28".into()),
        (
            "postal_capture_sha256",
            format!("{:x}\n{}", Sha256::digest(SUMMARY), "b".repeat(64)),
        ),
    ] {
        check!(eq; record.get(
            headers
                .iter()
                .position(|value| value == header)
                .ok_or_else(|| format!("missing CSV column {header}"))?
        ),
        Some(expected.as_str()));
    }
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
    );
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
        for ((header, value), csv_header) in expected_postal(&captured)
            .into_iter()
            .zip(crate::export::postal::POSTAL_CSV_HEADERS)
        {
            let expected = if has_claim { value } else { String::new() };
            check!(eq; text(&range, row, column_of(&range, header)?),
            expected,
            "{header}");
            let column = headers
                .iter()
                .position(|field| field == csv_header)
                .ok_or_else(|| format!("missing CSV column {csv_header}"))?;
            check!(eq; record.get(column), Some(expected.as_str()), "{csv_header}");
        }
    }
    Ok(())
}
