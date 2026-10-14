use super::{files, population::Population, TestResult};
use census_domain::model::SchoolId;
use std::path::Path;

fn expected(given: &Population, historical: &SchoolId) -> Vec<String> {
    [
        historical.as_str(),
        "unavailable",
        "[]",
        "unavailable",
        "unresolved_historical_reference",
        "unavailable",
        "unavailable",
        "[]",
        &given.dataset.lineage.input_generation,
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "unavailable",
        "[]",
        "[]",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn write(given: &Population, historical: &SchoolId, directory: &Path) -> TestResult {
    files::write(given, directory)?;
    super::change(
        directory,
        "best_marks.csv",
        "school_at_performance",
        historical.as_str(),
    )?;
    let (header, mut schools) = super::read(directory, "schools.csv")?;
    schools.push(expected(given, historical));
    files::put(
        directory,
        "schools.csv",
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        &schools,
    )
}

#[test]
fn given_unretained_historical_school_when_manual_families_are_read_then_exact_reference_is_verified(
) -> TestResult {
    let (given, historical) = Population::with_unknown_historical_school()?;
    check!(!given.dataset.schools.contains_key(&historical));
    let directory = tempfile::tempdir()?;
    write(&given, &historical, directory.path())?;
    check!(eq; super::value(directory.path(), "best_marks.csv", 0, "school_at_performance")?, historical.as_str());
    check!(eq; super::value(directory.path(), "best_marks.csv", 0, "school_at_performance_status")?, "performance_team_school");
    super::super::verify(directory.path(), &given.inputs()?)?;
    Ok(())
}

#[test]
fn given_manual_unresolved_reference_when_status_name_evidence_or_id_is_tampered_then_validation_refuses(
) -> TestResult {
    let (given, historical) = Population::with_unknown_historical_school()?;
    let directory = tempfile::tempdir()?;
    for (column, replacement) in [
        ("school_status", "currentness_unavailable"),
        ("canonical_name", "Invented Prior High"),
        ("evidence_refs", "[\"invented-school-evidence\"]"),
        ("school_id", "dangling-historical-school"),
        ("address_evidence_refs", "[\"invented-address-evidence\"]"),
    ] {
        write(&given, &historical, directory.path())?;
        super::super::verify(directory.path(), &given.inputs()?)?;
        tamper(directory.path(), &historical, column, replacement)?;
        match super::super::verify(directory.path(), &given.inputs()?) {
            Err(crate::report::ReportError::Invariant { .. }) => {}
            other => {
                return Err(
                    format!("historical {column} tamper was not refused: {other:?}").into(),
                );
            }
        }
    }
    Ok(())
}

fn tamper(directory: &Path, historical: &SchoolId, column: &str, replacement: &str) -> TestResult {
    let (header, mut records) = super::read(directory, "schools.csv")?;
    let column = header
        .iter()
        .position(|name| name == column)
        .ok_or("missing manual school column")?;
    let row = records
        .iter_mut()
        .find(|row| row.first().map(String::as_str) == Some(historical.as_str()))
        .ok_or("missing manual historical row")?;
    *row.get_mut(column).ok_or("missing manual school cell")? = replacement.into();
    files::put(
        directory,
        "schools.csv",
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        &records,
    )
}
