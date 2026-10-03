use super::TestResult;
use crate::school_directory::{
    collapse_entries, CityName, CollapseOutcome, IdentifiedKey, NcesSchoolId, PostalAddress,
    SchoolDirectoryEntry, SchoolName, SourceLabel, StreetLine, WeakKey, ZipCode,
};
use crate::UsJurisdiction;

fn name(raw: &str) -> Result<SchoolName, Box<dyn std::error::Error>> {
    Ok(SchoolName::parse(raw)?)
}

fn city(raw: &str) -> Result<CityName, Box<dyn std::error::Error>> {
    Ok(CityName::parse(raw)?)
}

fn street(raw: &str) -> Result<StreetLine, Box<dyn std::error::Error>> {
    Ok(StreetLine::parse(raw)?)
}

fn address(
    line: &str,
    city_name: &str,
    zip_code: &str,
) -> Result<PostalAddress, Box<dyn std::error::Error>> {
    Ok(PostalAddress::of(
        Some(street(line)?),
        None,
        Some(city(city_name)?),
        Some(UsJurisdiction::Alabama),
        Some(ZipCode::parse(zip_code)?),
    )
    .ok_or("address has parts")?)
}

fn census_entry(
    id: &str,
    line: &str,
    city_name: &str,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id)?),
        SourceLabel::Ccd,
        Some(name("Albertville High School")?),
    )
    .with_address(Some(address(line, city_name, "35950")?)))
}

fn association_entry(
    city_name: Option<&str>,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    let located = city_name.map(city).transpose()?;
    Ok(SchoolDirectoryEntry::weak(
        name("Albertville High School")?,
        located,
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )?)
}

#[test]
fn rows_sharing_a_key_collapse_into_one_entry() -> TestResult {
    let first = census_entry("010001000001", "600 E Alabama Ave", "Albertville")?;
    let second = census_entry("010001000001", "600 East Alabama Avenue", "Albertville")?;

    let outcome = collapse_entries(vec![first.clone(), second.clone()]);
    check!(eq; outcome.entries.len(), 1);
    check!(outcome.notes.is_empty());
    let entry = outcome.entries.first().ok_or("entry")?;
    check!(eq; entry.sources().len(), 1);

    let reversed = collapse_entries(vec![second, first]);
    check!(eq; reversed, outcome);
    check!(eq; entry
        .address()
        .and_then(PostalAddress::line1)
        .map(StreetLine::as_str),
    Some("600 East Alabama Avenue"));
    Ok(())
}

#[test]
fn a_uniquely_matching_weak_row_is_absorbed_into_the_identified_school() -> TestResult {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville")?;
    let outcome = collapse_entries(vec![
        census.clone(),
        association_entry(Some("Albertville"))?,
    ]);

    check!(eq; outcome.entries.len(), 1);
    check!(outcome.notes.is_empty());
    let entry = outcome.entries.first().ok_or("entry")?;
    check!(eq; entry.key(), census.key());
    check!(entry.sources().contains(&SourceLabel::AthleticAssociation {
        state: UsJurisdiction::Alabama
    }));
    Ok(())
}

#[test]
fn an_ambiguous_weak_row_survives_with_a_note() -> TestResult {
    let first = census_entry("010001000001", "600 E Alabama Ave", "Albertville")?;
    let second = census_entry("010001000002", "600 E Alabama Ave", "Albertville")?;
    let outcome = collapse_entries(vec![
        first.clone(),
        association_entry(Some("Albertville"))?,
        second.clone(),
    ]);

    check!(eq; outcome.entries.len(), 3);
    check!(eq; outcome.notes.len(), 1);
    let note = outcome.notes.first().ok_or("note")?;
    check!(eq; note.weak.label(), "albertville high school|albertville|AL");
    check!(eq; note.candidates.len(), 2);
    check!(outcome.entries.iter().any(|entry| entry.key().rank() == 3));
    Ok(())
}

#[test]
fn an_unmatched_weak_row_is_never_forced_onto_a_neighbour() -> TestResult {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville")?;
    let elsewhere = association_entry(Some("Boaz"))?;
    let outcome = collapse_entries(vec![census.clone(), elsewhere.clone()]);

    check!(eq; outcome.entries.len(), 2);
    check!(outcome.notes.is_empty());
    check!(outcome
        .entries
        .iter()
        .any(|entry| entry.key() == census.key()));
    check!(outcome
        .entries
        .iter()
        .any(|entry| entry.key() == elsewhere.key()));
    Ok(())
}

#[test]
fn a_weak_row_without_a_locality_does_not_guess() -> TestResult {
    let census = census_entry("010001000001", "600 E Alabama Ave", "Albertville")?;
    let unlocated = SchoolDirectoryEntry::weak(
        name("Albertville High School")?,
        None,
        Some(UsJurisdiction::Alabama),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Alabama,
        },
    )?;
    let outcome = collapse_entries(vec![census, unlocated]);
    check!(eq; outcome.entries.len(), 2);
    Ok(())
}

#[test]
fn collapse_is_order_independent_and_idempotent() -> TestResult {
    let rows = vec![
        census_entry("010001000002", "600 E Alabama Ave", "Albertville")?,
        association_entry(Some("Albertville"))?,
        census_entry("010001000001", "600 E Alabama Ave", "Albertville")?,
    ];
    let forward = collapse_entries(rows.clone());
    let mut reversed = rows;
    reversed.reverse();
    let backward = collapse_entries(reversed);

    check!(eq; forward, backward);

    let settled: CollapseOutcome = collapse_entries(forward.entries.clone());
    check!(eq; settled.entries, forward.entries);
    check!(eq; settled.notes, forward.notes);
    Ok(())
}

#[test]
fn a_weak_key_that_no_identified_row_claims_stays_standalone() -> TestResult {
    let outcome = collapse_entries(vec![association_entry(Some("Albertville"))?]);
    check!(eq; outcome.entries.len(), 1);
    check!(outcome.notes.is_empty());
    let key = outcome.entries.first().ok_or("entry")?.key();
    check!(eq; key.rank(), 3);
    check!(eq; key,
    &crate::school_directory::DirectoryKey::Weak(WeakKey::of(
        &name("Albertville High School")?,
        Some(&city("Albertville")?),
        Some(UsJurisdiction::Alabama),
    )?,),);
    Ok(())
}
