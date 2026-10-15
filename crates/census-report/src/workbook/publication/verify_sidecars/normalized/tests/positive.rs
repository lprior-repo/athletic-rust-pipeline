use super::{read, TestResult, FAMILIES};
use std::path::Path;

mod fixture;
mod identity;
mod mutations;
mod provenance;
mod rows;

use fixture::Positive;

#[test]
fn admitted_contact_postal_and_merged_alias_rows_match_independent_values_and_read_back(
) -> TestResult {
    let given = Positive::new()?;
    let directory = tempfile::tempdir()?;
    prepare(&given, directory.path())?;
    super::super::verify(directory.path(), &given.population.inputs()?, 6)?;
    check!(eq; read(directory.path(), "coach_contacts.csv")?.1.len(), 2);
    check!(eq; read(directory.path(), "schools.csv")?.1.len(), 1);
    check!(eq; read(directory.path(), "identity_aliases.csv")?.1.len(), 5);
    check!(eq; read(directory.path(), "athletes.csv")?.1.len(), 3);
    check!(eq; read(directory.path(), "best_marks.csv")?.1.len(), 1);
    let generation = &given.population.dataset.lineage.input_generation;
    for family in FAMILIES {
        let (header, records) = read(directory.path(), family)?;
        let position = column(&header, "input_generation")?;
        for row in records {
            check!(eq; row.get(position), Some(generation));
        }
    }
    Ok(())
}

#[test]
fn merged_aliases_keep_one_no_mark_canonical_member_with_verified_identity() -> TestResult {
    let given = Positive::new()?;
    let directory = tempfile::tempdir()?;
    prepare(&given, directory.path())?;
    let (header, records) = read(directory.path(), "athletes.csv")?;
    let key = column(&header, "athlete_id")?;
    let row = records
        .iter()
        .find(|row| {
            row.get(key)
                .is_some_and(|id| id == given.canonical().as_str())
        })
        .ok_or("missing canonical merged athlete")?;
    for (field, expected) in [
        ("identity_status", "verified"),
        ("graduation_year", "2027"),
        (
            "performance_availability",
            "no_eligible_selected_mark_in_snapshot",
        ),
    ] {
        check!(eq; row.get(column(&header, field)?).map(String::as_str), Some(expected));
    }
    check!(!records
        .iter()
        .any(|row| row.get(key).is_some_and(|id| id == given.alias().as_str())));
    super::super::verify(directory.path(), &given.population.inputs()?, 6)?;
    Ok(())
}

fn prepare(given: &Positive, directory: &Path) -> TestResult {
    given.write(directory)?;
    rows::compare_and_replace(
        directory,
        "coach_contacts.csv",
        &rows::CONTACTS,
        &rows::contacts(given)?,
    )?;
    rows::compare_and_replace(
        directory,
        "schools.csv",
        &rows::SCHOOLS,
        &rows::schools(given)?,
    )?;
    rows::compare_and_replace(
        directory,
        "identity_aliases.csv",
        &rows::ALIASES,
        &rows::aliases(given)?,
    )?;
    provenance::compare_and_replace(given, directory)?;
    super::super::verify(directory, &given.population.inputs()?, 6)?;
    Ok(())
}

fn column(header: &[String], name: &str) -> TestResult<usize> {
    header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing positive column {name}").into())
}
