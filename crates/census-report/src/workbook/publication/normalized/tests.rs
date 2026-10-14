mod fixture;

use crate::report::ReportError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Record = BTreeMap<String, String>;

#[test]
fn given_admitted_athlete_without_mark_when_published_then_directory_preserves_status_and_quotes(
) -> TestResult {
    let given = fixture::Population::new("Ada, \"Quoted\"", &[])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let athletes = read(directory.path(), "athletes.csv")?;
    check!(eq; athletes.len(), 1);
    let athlete = athletes.first().ok_or("missing no-mark athlete")?;
    check!(eq; field(athlete, "display_name")?, "Ada, \"Quoted\"");
    check!(eq; field(athlete, "graduation_year")?, "2027");
    check!(
        eq;
        field(athlete, "performance_availability")?,
        "no_eligible_selected_mark_in_snapshot"
    );
    check!(eq; field(athlete, "record_revision")?, "unavailable");
    check!(eq; read(directory.path(), "best_marks.csv")?.len(), 0);
    check!(eq; read(directory.path(), "comparison_classes.csv")?.len(), 0);
    Ok(())
}

#[test]
fn given_formula_like_name_when_published_then_csv_is_literal_and_authoritative_name_is_unchanged(
) -> TestResult {
    let given = fixture::Population::new("=SUM(1,2)", &[])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let athletes = read(directory.path(), "athletes.csv")?;
    check!(
        eq;
        field(athletes.first().ok_or("missing athlete")?, "display_name")?,
        "'=SUM(1,2)"
    );
    check!(
        eq;
        given
            .dataset
            .athletes
            .first()
            .ok_or("missing input athlete")?
            .canonical_name,
        "=SUM(1,2)"
    );
    Ok(())
}

#[test]
fn given_equal_marks_at_distinct_meets_when_published_then_one_key_has_complete_tie_inventory(
) -> TestResult {
    let given = fixture::Population::new("Ada Runner", &["50.00", "50.000"])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let marks = read(directory.path(), "best_marks.csv")?;
    let classes = read(directory.path(), "comparison_classes.csv")?;
    check!(eq; marks.len(), 1);
    check!(eq; classes.len(), 1);
    let mark = marks.first().ok_or("missing best")?;
    check!(
        eq;
        field(mark, "comparison_class_id")?,
        field(
            classes.first().ok_or("missing class")?,
            "comparison_class_id"
        )?
    );
    check!(eq; field(mark, "normalized_mark")?, "50000000000");
    check!(eq; field(mark, "unit")?, "ns");
    check!(eq; field(mark, "eligible_candidate_count")?, "2");
    check!(eq; field(mark, "tied_performance_count")?, "2");
    let ids: BTreeSet<String> = serde_json::from_str(field(mark, "tied_performance_ids")?)?;
    let expected: BTreeSet<String> = given
        .dataset
        .performances
        .iter()
        .map(|row| row.id.to_string())
        .collect();
    check!(eq; ids, expected);
    check!(eq; field(mark, "school_at_performance")?, given.school.as_str());
    check!(
        eq;
        field(mark, "selection_policy_version")?,
        given.dataset.lineage.policy_revision.to_string()
    );
    Ok(())
}

#[test]
fn given_evidence_without_capture_edges_when_published_then_no_capture_or_parser_version_is_invented(
) -> TestResult {
    let given = fixture::Population::new("Ada Runner", &["50.00"])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let marks = read(directory.path(), "best_marks.csv")?;
    let provenance = read(directory.path(), "provenance.csv")?;
    let mark = marks.first().ok_or("missing best")?;
    check!(eq; field(mark, "capture_reference")?, "unavailable");
    check!(eq; field(mark, "performance_version")?, "unavailable");
    let refs: Vec<String> = serde_json::from_str(field(mark, "evidence_refs")?)?;
    check!(eq; refs.len(), 1);
    check!(eq; field(mark, "source_decimal_precision")?, "2");
    check!(
        eq;
        field(mark, "precision_basis")?,
        "normalized_integer_quantum"
    );
    for id in refs {
        let evidence = provenance
            .iter()
            .find(|row| row.get("evidence_id") == Some(&id))
            .ok_or("dangling evidence reference")?;
        check!(eq; field(evidence, "capture_id")?, "unavailable");
        check!(eq; field(evidence, "capture_time")?, "unavailable");
        check!(eq; field(evidence, "interpretation_version")?, "unavailable");
        check!(eq; field(evidence, "observed_on")?, "2026-05-01");
        check!(
            eq;
            field(evidence, "source_reference")?,
            "captured-fixture-results"
        );
    }
    Ok(())
}

#[test]
fn given_unknown_school_address_and_empty_metrics_when_published_then_absence_is_not_active_or_zero_coverage(
) -> TestResult {
    let given = fixture::Population::new("Ada Runner", &[])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let schools = read(directory.path(), "schools.csv")?;
    let school = schools.first().ok_or("missing school")?;
    check!(eq; field(school, "address_status")?, "not_collected");
    check!(eq; field(school, "school_status")?, "currentness_unavailable");
    check!(eq; field(school, "address_verification_date")?, "unavailable");
    check!(eq; field(school, "address_evidence_refs")?, "[]");
    let coverage = read(directory.path(), "coverage.csv")?;
    check!(
        coverage.is_empty(),
        "empty coverage preserves unknown without a fake derived row: {coverage:?}"
    );
    Ok(())
}

#[test]
fn given_invalid_best_reference_when_published_then_existing_artifact_is_unchanged() -> TestResult {
    let mut given = fixture::Population::new("Ada Runner", &["50.00"])?;
    let directory = tempfile::tempdir()?;
    let sentinel = directory.path().join("athletes.csv");
    std::fs::write(&sentinel, b"prior accepted data")?;
    given
        .bests
        .first_mut()
        .ok_or("missing selection")?
        .source
        .performance_id = census_domain::model::PerformanceId::mint("perf", &["absent"]);
    let failure = super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    );
    match failure {
        Err(ReportError::Invariant { detail }) => check!(
            eq;
            detail,
            format!(
                "best references missing performance {}",
                given
                    .bests
                    .first()
                    .ok_or("missing selection")?
                    .source
                    .performance_id
            )
        ),
        other => return Err(format!("unexpected publication result: {other:?}").into()),
    }
    check!(eq; std::fs::read(sentinel)?, b"prior accepted data");
    Ok(())
}

#[test]
fn given_same_generation_when_written_twice_then_every_family_is_byte_identical() -> TestResult {
    let given = fixture::Population::new("Ada Runner", &["50.00", "51.00"])?;
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    super::write(first.path(), &given.dataset, &given.options, &given.bests)?;
    super::write(second.path(), &given.dataset, &given.options, &given.bests)?;
    for family in [
        "athletes.csv",
        "best_marks.csv",
        "schools.csv",
        "coach_contacts.csv",
        "comparison_classes.csv",
        "provenance.csv",
        "coverage.csv",
        "identity_aliases.csv",
    ] {
        check!(
            eq;
            std::fs::read(first.path().join(family))?,
            std::fs::read(second.path().join(family))?,
            "{family}"
        );
    }
    let aliases = read(first.path(), "identity_aliases.csv")?;
    let alias = aliases.first().ok_or("missing identity disposition")?;
    check!(
        eq;
        field(alias, "canonical_id")?,
        given
            .dataset
            .athletes
            .first()
            .ok_or("missing input athlete")?
            .id
            .as_str()
    );
    check!(eq; field(alias, "effective_revision")?, "unavailable");
    check!(eq; field(alias, "successor_ids")?, "unavailable");
    Ok(())
}

#[test]
fn given_unverified_coach_channels_when_published_then_roles_do_not_multiply_marks_or_claim_current_contacts(
) -> TestResult {
    let given = fixture::Population::new("Ada Runner", &["50.00"])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let coaches = read(directory.path(), "coach_contacts.csv")?;
    check!(eq; coaches.len(), 2);
    for coach in &coaches {
        check!(eq; field(coach, "status")?, "no_current_claim");
        check!(eq; field(coach, "tenure_status")?, "unknown");
        check!(eq; field(coach, "work_email")?, "withheld_not_admitted");
        check!(eq; field(coach, "work_phone")?, "withheld_not_admitted");
        check!(eq; field(coach, "last_verification")?, "unavailable");
        check!(eq; field(coach, "school_id")?, given.school.as_str());
    }
    check!(eq; read(directory.path(), "best_marks.csv")?.len(), 1);
    Ok(())
}

#[test]
fn given_transfer_history_outside_current_scope_when_published_then_historical_school_reference_resolves(
) -> TestResult {
    let (given, historical) =
        fixture::Population::new_with_historical_school("Ada Transfer", &["50.00"])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let athletes = read(directory.path(), "athletes.csv")?;
    check!(
        eq;
        field(
            athletes.first().ok_or("missing transfer athlete")?,
            "current_school_id"
        )?,
        given.school.as_str()
    );
    let marks = read(directory.path(), "best_marks.csv")?;
    check!(eq; marks.len(), 1);
    check!(
        eq;
        field(
            marks.first().ok_or("missing historical best")?,
            "school_at_performance"
        )?,
        historical.as_str()
    );
    let schools = read(directory.path(), "schools.csv")?;
    check!(eq; schools.len(), 2);
    let historical_row = schools
        .iter()
        .find(|row| row.get("school_id").map(String::as_str) == Some(historical.as_str()))
        .ok_or("dangling historical school reference")?;
    check!(eq; field(historical_row, "canonical_name")?, "Prior High");
    Ok(())
}

#[test]
fn given_same_event_indoors_and_outdoors_when_published_then_comparison_keys_and_units_remain_distinct(
) -> TestResult {
    let given =
        fixture::Population::new_with_indoor_second_meet("Ada Runner", &["50.00", "51.00"])?;
    let directory = tempfile::tempdir()?;
    super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
    )?;
    let classes = read(directory.path(), "comparison_classes.csv")?;
    let marks = read(directory.path(), "best_marks.csv")?;
    check!(eq; classes.len(), 2);
    check!(eq; marks.len(), 2);
    let surfaces: BTreeSet<_> = classes
        .iter()
        .map(|row| field(row, "surface"))
        .collect::<TestResult<_>>()?;
    check!(eq; surfaces, BTreeSet::from(["indoor", "outdoor"]));
    let ids: BTreeSet<_> = classes
        .iter()
        .map(|row| field(row, "comparison_class_id"))
        .collect::<TestResult<_>>()?;
    check!(eq; ids.len(), 2);
    let values: BTreeSet<_> = marks
        .iter()
        .map(|row| field(row, "normalized_mark"))
        .collect::<TestResult<_>>()?;
    check!(eq; values, BTreeSet::from(["50000000000", "51000000000"]));
    for mark in &marks {
        check!(eq; field(mark, "unit")?, "ns");
        check!(ids.contains(field(mark, "comparison_class_id")?));
    }
    Ok(())
}

fn read(directory: &Path, name: &str) -> TestResult<Vec<Record>> {
    let mut reader = csv::Reader::from_path(directory.join(name))?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            if record.len() != headers.len() {
                return Err("wrong row width".into());
            }
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}

fn field<'a>(record: &'a Record, name: &str) -> TestResult<&'a str> {
    record
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("missing field {name}").into())
}
