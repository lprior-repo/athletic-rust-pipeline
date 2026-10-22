use super::*;
use census_domain::model::{Evidence, SchoolPostalAddress};
use census_domain::school_directory::{CityName, PostalAddress};
use census_domain::UsJurisdiction;

fn add_locality(store: &Store, school: &mut CanonicalSchool) -> TestResult {
    let original = school.postal_addresses.first().ok_or("captured address")?;
    let address = PostalAddress::of(
        None,
        None,
        Some(CityName::parse("Asheville")?),
        Some(UsJurisdiction::NorthCarolina),
        None,
    )
    .ok_or("controlled locality tuple")?;
    school.add_postal_address(SchoolPostalAddress::new(
        address,
        original.owner().clone(),
        original.source_label().clone(),
        Evidence::parsed(
            SourceRef::new("nchsaa", Some(SUMMARY_URL.into())),
            CAPTURE_DAY,
        ),
        original.capture_sha256().to_owned(),
    )?)?;
    store.append(Table::Schools, school)?;
    Ok(())
}

#[test]
fn retained_locality_and_street_reach_workbook_csv_and_store_absent_readback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let store = Store::open(&root)?;
    let (mut school, athlete) = seed(&store)?;
    add_locality(&store, &mut school)?;
    let (path, _, options) = published(&store)?;
    let expected = format!("Asheville, NC\n{CAPTURED_ADDRESS}");
    let mut book = open_workbook(&path)?;
    let athletes = sheet(&mut book, "Athletes")?;
    check!(eq; text(&athletes, row_of(&athletes, athlete.id.as_str())?,
        column_of(&athletes, "School Address")?), expected);
    let schools = sheet(&mut book, "Schools")?;
    let row = row_of(&schools, school.id.as_str())?;
    check!(eq; text(&schools, row, column_of(&schools, "Postal Street")?),
        "\n1 Rocket Drive");
    let generation = path.parent().ok_or("publication directory")?;
    let mut reader = ::csv::Reader::from_path(generation.join("recruiting.csv"))?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("recruiting row")??;
    let column = headers
        .iter()
        .position(|value| value == "school_address")
        .ok_or("CSV school address")?;
    check!(eq; record.get(column), Some(expected.as_str()));
    let generation_name = generation.file_name().ok_or("generation name")?;
    let workbook_name = path.file_name().ok_or("workbook name")?;
    let external = dir.path().join("publication").join("generations");
    std::fs::create_dir_all(&external)?;
    let retained_generation = external.join(generation_name);
    std::fs::rename(generation, &retained_generation)?;
    let retained_path = retained_generation.join(workbook_name);
    drop(store);
    std::fs::rename(&root, dir.path().join("retained-store"))?;
    check!(!root.exists());
    let frozen = ExportDataset::reopen_frozen(&retained_generation.join("frozen-input.json"))?;
    verify_frozen(&retained_path, &frozen, &options)?;
    let tampered = dir.path().join("missing-address-claim.xlsx");
    for incomplete in ["Asheville, NC", CAPTURED_ADDRESS] {
        corrupt_workbook(
            &retained_path,
            &tampered,
            "Athletes",
            "School Address",
            incomplete,
        )?;
        check!(verify_frozen(&tampered, &frozen, &options).is_err());
    }
    Ok(())
}
