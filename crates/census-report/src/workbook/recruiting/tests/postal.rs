mod ordering;

use super::*;
use crate::export::postal::tests::{captured_school, CAPTURE_DAY, SUMMARY, SUMMARY_URL};
use crate::export::ExportDataset;
use crate::workbook::{verify::verify_frozen, Options};
use sha2::{Digest, Sha256};

fn seed(store: &Store) -> (CanonicalSchool, CanonicalAthlete) {
    let school = captured_school();
    store.append(Table::Schools, &school).unwrap();
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Captured School Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-runner"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.evidence = evidence("nchsaa", Some(SUMMARY_URL));
    store.append(Table::Athletes, &athlete).unwrap();
    coach(
        store,
        &school.id,
        "Published Contact",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "published@school.test",
    );
    (school, athlete)
}

fn published(store: &Store) -> (std::path::PathBuf, ExportDataset, Options) {
    let options = Options {
        school_year: Some(SchoolYear::new(2026).unwrap()),
        ..Options::default()
    };
    let path = crate::workbook::build(store, &options).unwrap();
    let dataset =
        ExportDataset::reopen_frozen(&path.parent().unwrap().join("frozen-input.json")).unwrap();
    verify_frozen(&path, &dataset, &options).unwrap();
    (path, dataset, options)
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
fn captured_zcum49_claim_reaches_school_athlete_and_csv_consumers_without_losing_public_email() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, athlete) = seed(&store);
    let (path, _, _) = published(&store);
    let mut book = open_workbook(&path).unwrap();
    for (name, id) in [
        ("Schools", school.id.as_str()),
        ("Athletes", athlete.id.as_str()),
    ] {
        let range = sheet(&mut book, name);
        let row = row_of(&range, id);
        for (header, expected) in expected_postal(&school) {
            assert_eq!(
                text(&range, row, column_of(&range, header)),
                expected,
                "{name} {header}"
            );
        }
    }
    let athletes = sheet(&mut book, "Athletes");
    assert_eq!(
        text(
            &athletes,
            row_of(&athletes, athlete.id.as_str()),
            column_of(&athletes, "Head TF Coach Email")
        ),
        "published@school.test"
    );
    let mut reader =
        ::csv::Reader::from_path(path.parent().unwrap().join("recruiting.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let record = reader.records().next().unwrap().unwrap();
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
        assert_eq!(
            record.get(headers.iter().position(|field| field == name).unwrap()),
            Some(expected.as_str()),
            "{name}"
        );
    }
}

fn corrupt_workbook(source: &std::path::Path, target: &std::path::Path, name: &str, header: &str) {
    let mut reader: Xlsx<_> = open_workbook(source).unwrap();
    let mut writer = Workbook::new();
    for sheet_name in reader.sheet_names() {
        let range = reader.worksheet_range(&sheet_name).unwrap();
        let selected = if sheet_name == name {
            Some(column_of(&range, header))
        } else {
            None
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&sheet_name).unwrap();
        for (row, cells) in range.rows().enumerate() {
            for (column, value) in cells.iter().enumerate() {
                let r = u32::try_from(row).unwrap();
                let c = u16::try_from(column).unwrap();
                if row == 1 && selected == Some(column) {
                    sheet.write_string(r, c, "forged postal claim").unwrap();
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(r, c, *value).unwrap();
                    }
                    Data::Int(value) => {
                        sheet.write_number(r, c, *value as f64).unwrap();
                    }
                    _ => {
                        sheet.write_string(r, c, value.to_string()).unwrap();
                    }
                }
            }
        }
    }
    writer.save(target).unwrap();
}

#[test]
fn tampering_any_published_postal_component_or_provenance_cell_fails_workbook_readback() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, _) = seed(&store);
    let (path, dataset, options) = published(&store);
    let target = dir.path().join("tampered.xlsx");
    for name in ["Schools", "Athletes"] {
        for (header, _) in expected_postal(&school) {
            corrupt_workbook(&path, &target, name, header);
            let error = verify_frozen(&target, &dataset, &options)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("forged postal claim"),
                "{name} {header}: {error}"
            );
        }
    }
}

#[test]
fn accepted_alias_school_affiliations_preserve_their_source_owned_postal_claims() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let captured = captured_school();
    store.append(Table::Schools, &captured).unwrap();
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
    alternate
        .add_postal_address(
            SchoolPostalAddress::new(
                PostalAddress::line(StreetLine::parse("2 Alias Way").unwrap()),
                owner,
                SourceLabel::AthleticAssociation {
                    state: UsJurisdiction::NorthCarolina,
                },
                Evidence::parsed(
                    SourceRef::new("synthetic-alias", Some("https://fixture.test/alias".into())),
                    CAPTURE_DAY,
                ),
                "c".repeat(64),
            )
            .unwrap(),
        )
        .unwrap();
    store.append(Table::Schools, &alternate).unwrap();
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169")
        .with_url("https://fixture.test/shared-person".to_string());
    let mut first = CanonicalAthlete::new(
        &captured.id,
        "Affiliated Runner",
        GradYear::CO2027,
        Gender::Boys,
        source.clone(),
    );
    let mut second =
        CanonicalAthlete::new(&other, "A. Runner", GradYear::CO2027, Gender::Boys, source);
    first.evidence = evidence(
        "milesplit_roster",
        Some("https://fixture.test/shared-person"),
    );
    second.evidence = first.evidence.clone();
    store
        .append_many(Table::Athletes, &[first.clone(), second.clone()])
        .unwrap();
    identity::accept(&store, &[first, second]);
    let (path, dataset, _) = published(&store);
    let canonical = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let [athlete] = canonical.athletes() else {
        panic!("one accepted athlete required");
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
    let mut book = open_workbook(&path).unwrap();
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, athlete.id.as_str());
    let mut reader =
        ::csv::Reader::from_path(path.parent().unwrap().join("recruiting.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get(0), Some(athlete.id.as_str()));
    for (column, (header, _)) in captured_fields.iter().enumerate() {
        assert_eq!(
            text(&range, row, column_of(&range, header)),
            expected[column],
            "{header}"
        );
        let csv_header = crate::export::postal::POSTAL_CSV_HEADERS[column];
        let csv_column = headers
            .iter()
            .position(|field| field == csv_header)
            .unwrap();
        assert_eq!(
            records[0].get(csv_column),
            Some(expected[column].as_str()),
            "{csv_header}"
        );
    }
}

#[test]
fn a_same_named_school_without_a_claim_does_not_inherit_another_schools_postal_address() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let captured = captured_school();
    store.append(Table::Schools, &captured).unwrap();
    let other = school(&store, UsJurisdiction::Wisconsin, &captured.name);
    let athlete = CanonicalAthlete::new(
        &other,
        "Unrelated Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "unrelated"),
    );
    store.append(Table::Athletes, &athlete).unwrap();
    let (path, _, _) = published(&store);
    let mut book = open_workbook(&path).unwrap();
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, athlete.id.as_str());
    for (header, _) in expected_postal(&captured) {
        assert_eq!(text(&range, row, column_of(&range, header)), "", "{header}");
    }
}

#[test]
fn contradictory_postal_addresses_keep_aligned_provenance_on_both_workbook_sheets_and_csv() {
    use census_domain::model::SchoolPostalAddress;
    use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (mut school, athlete) = seed(&store);
    let owner = SourceIdentity::new(
        SourceNamespace::association_school("synthetic-conflict"),
        "other-claim",
    );
    school.source_identities.push(owner.clone());
    let claim = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Review Road").unwrap()),
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
    )
    .unwrap();
    school.add_postal_address(claim).unwrap();
    store.append(Table::Schools, &school).unwrap();
    let (path, _, _) = published(&store);
    let mut book = open_workbook(&path).unwrap();
    for (name, id) in [
        ("Schools", school.id.as_str()),
        ("Athletes", athlete.id.as_str()),
    ] {
        let range = sheet(&mut book, name);
        let row = row_of(&range, id);
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
            assert_eq!(
                text(&range, row, column_of(&range, header)),
                expected,
                "{name} {header}"
            );
        }
    }
    let mut reader =
        ::csv::Reader::from_path(path.parent().unwrap().join("recruiting.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let record = reader.records().next().unwrap().unwrap();
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
        assert_eq!(
            record.get(headers.iter().position(|value| value == header).unwrap()),
            Some(expected.as_str())
        );
    }
}

#[test]
fn unresolved_same_named_athletes_keep_postal_affiliations_distinct_in_workbook_and_csv() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (captured, first) = seed(&store);
    let other = school(&store, UsJurisdiction::Wisconsin, &captured.name);
    let second = CanonicalAthlete::new(
        &other,
        &first.canonical_name,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "distinct-postal-homonym"),
    );
    store.append(Table::Athletes, &second).unwrap();
    let (path, dataset, _) = published(&store);
    assert!(dataset.canonical_aliases.is_empty());
    let mut book = open_workbook(&path).unwrap();
    let range = sheet(&mut book, "Athletes");
    assert_eq!(range.height(), 3);
    let mut reader =
        ::csv::Reader::from_path(path.parent().unwrap().join("recruiting.csv")).unwrap();
    let headers = reader.headers().unwrap().clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 2);
    for (athlete, has_claim) in [(&first, true), (&second, false)] {
        let row = row_of(&range, athlete.id.as_str());
        let record = records
            .iter()
            .find(|record| record.get(0) == Some(athlete.id.as_str()))
            .unwrap();
        for ((header, value), csv_header) in expected_postal(&captured)
            .into_iter()
            .zip(crate::export::postal::POSTAL_CSV_HEADERS)
        {
            let expected = if has_claim { value } else { String::new() };
            assert_eq!(
                text(&range, row, column_of(&range, header)),
                expected,
                "{header}"
            );
            let column = headers
                .iter()
                .position(|field| field == csv_header)
                .unwrap();
            assert_eq!(record.get(column), Some(expected.as_str()), "{csv_header}");
        }
    }
}
