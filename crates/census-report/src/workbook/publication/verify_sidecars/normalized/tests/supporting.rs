use super::{change, files, read, value, TestResult, FAMILIES};
use crate::report::ReportError;

mod fixture;
mod publication;

#[test]
fn artifact_v3_keeps_exact_headers_and_representative_provenance_with_extra_support() -> TestResult
{
    let (given, _) = fixture::population(true)?;
    let directory = tempfile::tempdir()?;
    files::write(&given, directory.path())?;
    let before = FAMILIES
        .iter()
        .map(|name| std::fs::read(directory.path().join(name)))
        .collect::<Result<Vec<_>, _>>()?;
    super::super::verify(directory.path(), &given.inputs()?, 3)?;
    check!(super::super::verify(directory.path(), &given.inputs()?, 4).is_err());
    let after = FAMILIES
        .iter()
        .map(|name| std::fs::read(directory.path().join(name)))
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; before, after);
    check!(!read(directory.path(), "best_marks.csv")?
        .0
        .iter()
        .any(|name| name == "supporting_tied_candidate_ids"));
    Ok(())
}

#[test]
fn artifact_v4_independent_inventory_retains_three_source_ids_for_two_tied_contexts() -> TestResult
{
    let (given, supporting) = fixture::population(true)?;
    let directory = tempfile::tempdir()?;
    fixture::write(&given, &supporting, directory.path())?;
    super::super::verify(directory.path(), &given.inputs()?, 4)?;
    check!(super::super::verify(directory.path(), &given.inputs()?, 3).is_err());
    check!(eq; value(directory.path(), "best_marks.csv", 0, "tied_performance_count")?, "2");
    check!(eq; value(directory.path(), "best_marks.csv", 0, "eligible_candidate_count")?, "3");
    check!(eq; value(directory.path(), "best_marks.csv", 0, "supporting_tied_candidate_ids")?, serde_json::to_string(&fixture::ids(&given, &supporting))?);
    Ok(())
}

#[test]
fn artifact_v4_support_column_refuses_missing_extra_duplicate_and_wrong_candidate_ids() -> TestResult
{
    let (given, supporting) = fixture::population(true)?;
    let directory = tempfile::tempdir()?;
    let ids = fixture::ids(&given, &supporting);
    let mut representatives_only = ids.clone();
    representatives_only.retain(|id| id != &supporting.id);
    let mut duplicate = ids.clone();
    duplicate.push(supporting.id.clone());
    let mut wrong = ids.clone();
    wrong.push(
        given
            .performances
            .last()
            .ok_or("missing worse candidate")?
            .id
            .clone(),
    );
    for replacement in [
        "[]".into(),
        "[\"absent\"]".into(),
        serde_json::to_string(&representatives_only)?,
        serde_json::to_string(&duplicate)?,
        serde_json::to_string(&wrong)?,
    ] {
        fixture::write(&given, &supporting, directory.path())?;
        change(
            directory.path(),
            "best_marks.csv",
            "supporting_tied_candidate_ids",
            &replacement,
        )?;
        check!(matches!(
            super::super::verify(directory.path(), &given.inputs()?, 4),
            Err(ReportError::Invariant { .. })
        ));
    }
    Ok(())
}

#[test]
fn artifact_v4_support_reference_refuses_missing_extra_duplicate_or_tampered_facts() -> TestResult {
    let (given, supporting) = fixture::population(true)?;
    let directory = tempfile::tempdir()?;
    for mutation in [
        "missing",
        "extra",
        "duplicate",
        "fact",
        "generation",
        "source_key",
    ] {
        fixture::write(&given, &supporting, directory.path())?;
        let (header, mut records) = read(directory.path(), "provenance.csv")?;
        let owner = fixture::column(&header, "entity_id")?;
        let kind = fixture::column(&header, "evidence_type")?;
        let index = records
            .iter()
            .position(|row| {
                row.get(owner).map(String::as_str) == Some(supporting.id.as_str())
                    && row.get(kind).map(String::as_str) == Some("retained_performance_reference")
            })
            .ok_or("missing supporting reference")?;
        let original = records.get(index).ok_or("missing supporting row")?.clone();
        match mutation {
            "missing" => {
                records.remove(index);
            }
            "duplicate" => records.push(original),
            "extra" => {
                let mut extra = original;
                *extra.first_mut().ok_or("missing evidence id")? = "unexpected-support".into();
                records.push(extra);
            }
            "fact" | "generation" | "source_key" => {
                let (column, replacement) = match mutation {
                    "fact" => ("retained_fact", "{}"),
                    "generation" => ("input_generation", "foreign-generation"),
                    _ => ("source_reference", "unproven-source-result"),
                };
                let column = fixture::column(&header, column)?;
                *records
                    .get_mut(index)
                    .and_then(|row| row.get_mut(column))
                    .ok_or("missing mutation cell")? = replacement.into();
            }
            _ => return Err("unknown finite mutation".into()),
        }
        fixture::put(directory.path(), "provenance.csv", &header, &records)?;
        check!(matches!(
            super::super::verify(directory.path(), &given.inputs()?, 4),
            Err(ReportError::Invariant { .. })
        ));
    }
    Ok(())
}

#[test]
fn artifact_v4_source_reference_survives_absent_evidence_without_inventing_capture_links(
) -> TestResult {
    let (given, supporting) = fixture::population(false)?;
    let directory = tempfile::tempdir()?;
    fixture::write(&given, &supporting, directory.path())?;
    super::super::verify(directory.path(), &given.inputs()?, 4)?;
    let (header, records) = read(directory.path(), "provenance.csv")?;
    let owner = fixture::column(&header, "entity_id")?;
    let found = records
        .iter()
        .filter(|row| row.get(owner).map(String::as_str) == Some(supporting.id.as_str()))
        .collect::<Vec<_>>();
    check!(eq; found.len(), 1);
    let reference = found.first().ok_or("missing reference without evidence")?;
    for column in [
        "capture_id",
        "capture_time",
        "interpretation_version",
        "source_url",
    ] {
        check!(eq; reference.get(fixture::column(&header, column)?).map(String::as_str), Some("unavailable"));
    }
    Ok(())
}
