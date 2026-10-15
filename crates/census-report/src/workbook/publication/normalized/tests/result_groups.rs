use super::{field, fixture::Population, read, TestResult};

#[test]
fn normalized_current_inventory_uses_thirty_eight_truthfully_named_columns() -> TestResult {
    let given = Population::new("Result Group Runner", &["50.00", "50.00"])?;
    let directory = tempfile::tempdir()?;
    super::super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
        &[],
    )?;
    let mut reader = csv::Reader::from_path(directory.path().join("best_marks.csv"))?;
    let headers = reader.headers()?.clone();
    check!(eq; headers.len(), 38);
    for old in [
        "eligible_candidate_count",
        "tied_performance_count",
        "tied_performance_ids",
    ] {
        check!(!headers.iter().any(|name| name == old));
    }
    let records = read(directory.path(), "best_marks.csv")?;
    let row = records.first().ok_or("missing normalized best")?;
    for (name, expected) in [
        ("eligible_result_group_count", "2"),
        ("tied_result_group_count", "2"),
        ("result_group_basis", "canonical_performance_id"),
        ("physical_performance_count_status", "unknown"),
        ("selection_policy_version", "21"),
    ] {
        check!(eq; field(row, name)?, expected);
    }
    Ok(())
}
