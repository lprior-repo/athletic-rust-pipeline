use super::*;
use census_domain::model::EventSemantics;

#[test]
fn empty_best_results_use_the_explicit_policy_for_the_complete_csv_header() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (policy, columns) in [
        (EventSemantics::CombinedEvents20, 36),
        (EventSemantics::ResultGroups21, 39),
    ] {
        let output = directory.path().join(policy.revision().to_string());
        let (jsonl, csv) = write(&output, &[], "empty", policy)?;
        check!(eq; std::fs::read_to_string(jsonl)?, "");
        let mut reader = csv::Reader::from_path(csv)?;
        let headers = reader.headers()?.clone();
        check!(eq; headers.len(), columns);
        check!(eq; reader.records().count(), 0);
        check!(eq; headers.iter().any(|name| name == "tied_performance_count"), policy == EventSemantics::CombinedEvents20);
        check!(eq; headers.iter().any(|name| name == "tied_result_group_count"), policy == EventSemantics::ResultGroups21);
    }
    Ok(())
}

#[test]
fn writer_refuses_mixed_or_mislabeled_inventory_before_creating_output() -> TestResult {
    let directory = tempfile::tempdir()?;
    let output = directory.path().join("not-created");
    let mut row = selection(
        ORDINARY_NAME,
        "School",
        "Meet",
        Mark::Raw("11.00".into()),
        None,
    );
    row.inventory = crate::bests::SelectionInventory::from_groups(
        EventSemantics::CombinedEvents20,
        1,
        vec![row.source.performance_id.clone()],
    );
    check!(write(&output, &[row], "co2027", EventSemantics::ResultGroups21).is_err());
    check!(!output.exists());
    Ok(())
}

#[test]
fn current_csv_and_json_use_result_group_keys_and_unknown_physical_cardinality() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut row = selection(
        ORDINARY_NAME,
        "School",
        "Meet",
        Mark::Raw("11.00".into()),
        None,
    );
    row.inventory = crate::bests::SelectionInventory::from_groups(
        EventSemantics::ResultGroups21,
        3,
        vec![row.source.performance_id.clone()],
    );
    let (jsonl, csv) = write(
        directory.path(),
        &[row],
        "co2027",
        EventSemantics::ResultGroups21,
    )?;
    let encoded: serde_json::Value = serde_json::from_str(std::fs::read_to_string(jsonl)?.trim())?;
    check!(eq; encoded.get("eligible_result_group_count"), Some(&serde_json::json!(3)));
    check!(eq; encoded.get("tied_result_group_count"), Some(&serde_json::json!(1)));
    check!(encoded.get("tied_performance_count").is_none());
    let mut reader = csv::Reader::from_path(csv)?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("missing current row")??;
    check!(eq; headers.len(), 39);
    check!(eq; record.len(), 39);
    for (name, expected) in [
        ("eligible_result_group_count", "3"),
        ("tied_result_group_count", "1"),
        ("result_group_basis", "canonical_performance_id"),
        ("physical_performance_count_status", "unknown"),
        ("selection_policy_version", "21"),
    ] {
        let index = headers
            .iter()
            .position(|field| field == name)
            .ok_or("missing result-group field")?;
        check!(eq; record.get(index), Some(expected));
    }
    Ok(())
}
