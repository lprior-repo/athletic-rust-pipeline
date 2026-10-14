use crate::report::{ReportError, ReportResult};
use std::path::Path;

mod files;
mod historical;
mod population;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const FAMILIES: [&str; 8] = [
    "athletes.csv",
    "best_marks.csv",
    "schools.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "provenance.csv",
    "coverage.csv",
    "identity_aliases.csv",
];

#[test]
fn given_hand_authored_eight_families_when_read_then_exact_marks_statuses_and_no_mark_member_are_verified(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = given.directory.path().join("manual");
    std::fs::create_dir(&directory)?;
    files::write(&given, &directory)?;
    check!(eq; value(&directory, "best_marks.csv", 0, "normalized_mark")?, "50000000000");
    check!(eq; value(&directory, "best_marks.csv", 0, "eligible_candidate_count")?, "3");
    check!(eq; value(&directory, "best_marks.csv", 0, "tied_performance_count")?, "2");
    check!(eq; value(&directory, "athletes.csv", 1, "performance_availability")?, "no_eligible_selected_mark_in_snapshot");
    check!(eq; value(&directory, "coach_contacts.csv", 0, "work_email")?, "withheld_not_admitted");
    check!(eq; value(&directory, "coverage.csv", 0, "evidence_state")?, "retained_metrics_scope_not_qualified");
    check!(eq; value(&directory, "coverage.csv", 0, "subject")?, "MI");
    super::verify(&directory, &given.inputs()?)?;
    Ok(())
}

#[test]
fn given_each_mandatory_family_when_missing_then_readback_names_the_missing_file() -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    for family in FAMILIES {
        files::write(&given, directory.path())?;
        super::verify(directory.path(), &given.inputs()?)?;
        std::fs::remove_file(directory.path().join(family))?;
        match super::verify(directory.path(), &given.inputs()?) {
            Err(ReportError::Io { path, source }) => {
                check!(eq; path, directory.path().join(family));
                check!(eq; source.kind(), std::io::ErrorKind::NotFound);
            }
            other => return Err(format!("missing {family} was not refused: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn given_each_family_when_a_row_is_removed_or_duplicated_then_inventory_is_refused() -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    for family in FAMILIES {
        files::write(&given, directory.path())?;
        super::verify(directory.path(), &given.inputs()?)?;
        let (header, records) = read(directory.path(), family)?;
        files::put(
            directory.path(),
            family,
            &header.iter().map(String::as_str).collect::<Vec<_>>(),
            &records.iter().skip(1).cloned().collect::<Vec<_>>(),
        )?;
        rejected(
            super::verify(directory.path(), &given.inputs()?),
            family,
            "expected",
        )?;
        let mut doubled = records.clone();
        doubled.push(records.first().ok_or("missing manual record")?.clone());
        files::put(
            directory.path(),
            family,
            &header.iter().map(String::as_str).collect::<Vec<_>>(),
            &doubled,
        )?;
        rejected(
            super::verify(directory.path(), &given.inputs()?),
            family,
            "duplicate",
        )?;
    }
    Ok(())
}

#[test]
fn given_frozen_records_when_a_mark_revision_unit_status_or_reference_changes_then_readback_refuses(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    for (family, column, replacement, fragment) in [
        (
            "best_marks.csv",
            "normalized_mark",
            "49999999999",
            "50000000000",
        ),
        ("best_marks.csv", "selection_policy_version", "0", "column"),
        ("best_marks.csv", "unit", "seconds", "ns"),
        ("best_marks.csv", "eligible_candidate_count", "2", "column"),
        ("best_marks.csv", "tied_performance_count", "3", "column"),
        ("best_marks.csv", "tied_performance_ids", "[]", "column"),
        (
            "best_marks.csv",
            "evidence_refs",
            "[\"dangling\"]",
            "column",
        ),
        (
            "athletes.csv",
            "current_school_id",
            "dangling-school",
            "column",
        ),
        (
            "athletes.csv",
            "graduation_status",
            "unknown-good",
            "column",
        ),
        ("athletes.csv", "record_revision", "12345", "unavailable"),
        ("comparison_classes.csv", "policy_version", "0", "column"),
        ("comparison_classes.csv", "precision", "NaN", "integer"),
        (
            "coach_contacts.csv",
            "school_id",
            "dangling-school",
            "column",
        ),
        (
            "coach_contacts.csv",
            "status",
            "current_claim",
            "no_current_claim",
        ),
        (
            "coach_contacts.csv",
            "work_email",
            "unadmitted@example.test",
            "withheld_not_admitted",
        ),
        ("schools.csv", "address_status", "verified", "not_collected"),
        (
            "coverage.csv",
            "evidence_state",
            "complete",
            "retained_metrics_scope_not_qualified",
        ),
        (
            "identity_aliases.csv",
            "canonical_id",
            "dangling-athlete",
            "column",
        ),
    ] {
        files::write(&given, directory.path())?;
        super::verify(directory.path(), &given.inputs()?)?;
        change(directory.path(), family, column, replacement)?;
        rejected(
            super::verify(directory.path(), &given.inputs()?),
            family,
            fragment,
        )?;
    }
    Ok(())
}

#[test]
fn given_best_key_when_extra_out_of_scope_excluded_or_missing_performance_is_injected_then_refused(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    for (column, replacement, fragment) in [
        ("athlete_id", given.outside.as_str(), "best key"),
        ("comparison_class_id", "excluded-racewalk-class", "best key"),
        ("performance_id", given.excluded.as_str(), "column"),
        ("performance_id", "dangling-performance", "performance"),
    ] {
        files::write(&given, directory.path())?;
        super::verify(directory.path(), &given.inputs()?)?;
        change(directory.path(), "best_marks.csv", column, replacement)?;
        match super::verify(directory.path(), &given.inputs()?) {
            Err(ReportError::Invariant { detail }) => check!(detail.contains(fragment)),
            other => return Err(format!("unexpected key mutation result: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn given_retained_evidence_when_its_fact_is_replaced_then_source_record_comparison_refuses(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    files::write(&given, directory.path())?;
    super::verify(directory.path(), &given.inputs()?)?;
    change(directory.path(), "provenance.csv", "retained_fact", "{}")?;
    match super::verify(directory.path(), &given.inputs()?) {
        Err(ReportError::Invariant { detail }) => {
            check!(eq; detail, "normalized retained evidence fact differs from frozen input")
        }
        other => return Err(format!("retained fact mutation was not refused: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn given_complete_inventory_when_an_extra_unique_row_is_appended_then_it_is_not_ignored(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    for family in FAMILIES {
        files::write(&given, directory.path())?;
        super::verify(directory.path(), &given.inputs()?)?;
        let (header, mut records) = read(directory.path(), family)?;
        let mut extra = records.first().ok_or("missing manual record")?.clone();
        *extra.first_mut().ok_or("missing key")? = "unexpected-extra".into();
        records.push(extra);
        files::put(
            directory.path(),
            family,
            &header.iter().map(String::as_str).collect::<Vec<_>>(),
            &records,
        )?;
        match super::verify(directory.path(), &given.inputs()?) {
            Err(ReportError::Invariant { detail }) => {
                check!(detail.contains("missing or out-of-scope key"))
            }
            other => return Err(format!("extra {family} was not refused: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn given_oversized_family_when_readback_starts_then_declared_byte_budget_refuses_before_parsing(
) -> TestResult {
    let given = population::Population::new()?;
    let directory = tempfile::tempdir()?;
    files::write(&given, directory.path())?;
    super::verify(directory.path(), &given.inputs()?)?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(directory.path().join("athletes.csv"))?
        .set_len(268_435_457)?;
    rejected(
        super::verify(directory.path(), &given.inputs()?),
        "athletes.csv",
        "byte readback budget",
    )
}

fn rejected(result: ReportResult<()>, family: &str, fragment: &str) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => {
            check!(detail.contains(family));
            check!(detail.contains(fragment));
            Ok(())
        }
        other => Err(format!("expected {family} {fragment} refusal, found {other:?}").into()),
    }
}

fn read(directory: &Path, family: &str) -> TestResult<(Vec<String>, Vec<Vec<String>>)> {
    let mut reader = csv::Reader::from_path(directory.join(family))?;
    let header = reader.headers()?.iter().map(str::to_owned).collect();
    let records = reader
        .records()
        .map(|record| record.map(|record| record.iter().map(str::to_owned).collect()))
        .collect::<Result<_, _>>()?;
    Ok((header, records))
}

fn value(directory: &Path, family: &str, index: usize, column: &str) -> TestResult<String> {
    let (header, records) = read(directory, family)?;
    let position = header
        .iter()
        .position(|field| field == column)
        .ok_or("missing manual column")?;
    Ok(records
        .get(index)
        .and_then(|record| record.get(position))
        .ok_or("missing manual value")?
        .clone())
}

fn change(directory: &Path, family: &str, column: &str, replacement: &str) -> TestResult {
    let (header, mut records) = read(directory, family)?;
    let position = header
        .iter()
        .position(|field| field == column)
        .ok_or("missing mutation column")?;
    *records
        .first_mut()
        .and_then(|record| record.get_mut(position))
        .ok_or("missing mutation cell")? = replacement.into();
    files::put(
        directory,
        family,
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        &records,
    )
}
