use crate::school_directory::{
    collapse_entries, CityName, CollapseOutcome, IdentifiedKey, NcesSchoolId, PostalAddress,
    SchoolDirectoryEntry, SchoolName, SourceLabel, StreetLine, WeakKey, ZipCode,
};
use crate::UsJurisdiction;

fn name(raw: &str) -> SchoolName {
    SchoolName::parse(raw).expect("school name parses")
}

fn city(raw: &str) -> CityName {
    CityName::parse(raw).expect("city parses")
}

fn street(raw: &str) -> StreetLine {
    StreetLine::parse(raw).expect("street parses")
}

fn address(line: &str, city_name: &str, zip_code: &str) -> PostalAddress {
    PostalAddress::of(
        Some(street(line)),
        None,
        Some(city(city_name)),
        Some(UsJurisdiction::Alabama),
        Some(ZipCode::parse(zip_code).expect("zip")),
    )
    .expect("address has parts")
}

fn census_entry(id: &str, line: &str, city_name: &str) -> SchoolDirectoryEntry {
    SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id).expect("nces id")),
        SourceLabel::Ccd,
        Some(name("Albertville High School")),
    )
    .with_address(Some(address(line, city_name, "35950")))
}

fn association_entry(city_name: Option<&str>) -> SchoolDirectoryEntry {
    let located = city_name.map(city);
    SchoolDirectoryEntry::weak(
        name("Albertville High School"),
        located,
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )
    .expect("weak entry")
}

#[test]
fn rows_sharing_a_key_collapse_into_one_entry() {
    let first = census_entry("010001000001", "600 E Alabama Ave", "Albertville");
    let second = census_entry("010001000001", "600 East Alabama Avenue", "Albertville");

    let outcome = collapse_entries(vec![first.clone(), second.clone()]);
    assert_eq!(outcome.entries.len(), 1);
    assert!(outcome.notes.is_empty());
    let entry = outcome.entries.first().expect("entry");
    assert_eq!(entry.sources().len(), 1);

    let reversed = collapse_entries(vec![second, first]);
    assert_eq!(reversed, outcome);
    assert_eq!(
        entry
            .address()
            .and_then(PostalAddress::line1)
            .map(StreetLine::as_str),
        Some("600 East Alabama Avenue")
    );
}

#[test]
fn a_uniquely_matching_weak_row_is_absorbed_into_the_identified_school() {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville");
    let outcome = collapse_entries(vec![census.clone(), association_entry(Some("Albertville"))]);

    assert_eq!(outcome.entries.len(), 1);
    assert!(outcome.notes.is_empty());
    let entry = outcome.entries.first().expect("entry");
    assert_eq!(entry.key(), census.key());
    assert!(entry.sources().contains(&SourceLabel::AthleticAssociation {
        state: UsJurisdiction::Alabama
    }));
}

#[test]
fn an_ambiguous_weak_row_survives_with_a_note() {
    let first = census_entry("010001000001", "600 E Alabama Ave", "Albertville");
    let second = census_entry("010001000002", "600 E Alabama Ave", "Albertville");
    let outcome = collapse_entries(vec![
        first.clone(),
        association_entry(Some("Albertville")),
        second.clone(),
    ]);

    assert_eq!(outcome.entries.len(), 3);
    assert_eq!(outcome.notes.len(), 1);
    let note = outcome.notes.first().expect("note");
    assert_eq!(note.weak.label(), "albertville high school|albertville|AL");
    assert_eq!(note.candidates.len(), 2);
    assert!(outcome.entries.iter().any(|entry| entry.key().rank() == 3));
}

#[test]
fn an_unmatched_weak_row_is_never_forced_onto_a_neighbour() {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville");
    let elsewhere = association_entry(Some("Boaz"));
    let outcome = collapse_entries(vec![census.clone(), elsewhere.clone()]);

    assert_eq!(outcome.entries.len(), 2);
    assert!(outcome.notes.is_empty());
    assert!(outcome
        .entries
        .iter()
        .any(|entry| entry.key() == census.key()));
    assert!(outcome
        .entries
        .iter()
        .any(|entry| entry.key() == elsewhere.key()));
}

#[test]
fn a_weak_row_without_a_locality_does_not_guess() {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville");
    let unlocated = SchoolDirectoryEntry::weak(
        name("Albertville High School"),
        None,
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )
    .expect("weak entry");
    let outcome = collapse_entries(vec![census, unlocated]);
    assert_eq!(outcome.entries.len(), 2);
}

#[test]
fn collapse_is_order_independent_and_idempotent() {
    let rows = vec![
        census_entry("010001000002", "600 E Alabama Ave", "Albertville"),
        association_entry(Some("Albertville")),
        census_entry("010001000001", "600 E Alabama Ave", "Albertville"),
    ];
    let forward = collapse_entries(rows.clone());
    let mut reversed = rows;
    reversed.reverse();
    let backward = collapse_entries(reversed);

    assert_eq!(forward, backward);

    let settled: CollapseOutcome = collapse_entries(forward.entries.clone());
    assert_eq!(settled.entries, forward.entries);
    assert_eq!(settled.notes, forward.notes);
}

#[test]
fn a_weak_key_that_no_identified_row_claims_stays_standalone() {
    let outcome = collapse_entries(vec![association_entry(Some("Albertville"))]);
    assert_eq!(outcome.entries.len(), 1);
    assert!(outcome.notes.is_empty());
    let key = outcome.entries.first().expect("entry").key();
    assert_eq!(key.rank(), 3);
    assert_eq!(
        key,
        &crate::school_directory::DirectoryKey::Weak(
            WeakKey::of(
                &name("Albertville High School"),
                Some(&city("Albertville")),
                Some(UsJurisdiction::Alabama),
            )
            .expect("weak key"),
        ),
    );
}
