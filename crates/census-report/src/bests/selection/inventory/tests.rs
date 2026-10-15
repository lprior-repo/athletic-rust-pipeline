use super::*;
use census_domain::model::Id;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn historical_count_serialization_keeps_the_literal_three_field_wire_contract() -> TestResult {
    let id: PerformanceId = Id::mint("perf", &["literal-historical-inventory"]);
    let expected = format!(
        "{{\"eligible_candidate_count\":3,\"tied_performance_count\":1,\"tied_performance_ids\":[\"{id}\"]}}"
    );
    for policy in [
        EventSemantics::Legacy18,
        EventSemantics::FlatEvents19,
        EventSemantics::CombinedEvents20,
    ] {
        let inventory = SelectionInventory::from_groups(policy, 3, vec![id.clone(), id.clone()]);
        check!(eq; serde_json::to_string(&inventory)?, expected);
        check!(eq; inventory.policy(), policy);
    }
    Ok(())
}

#[test]
fn result_group_inventory_has_no_numeric_physical_count_or_legacy_alias() -> TestResult {
    let id: PerformanceId = Id::mint("perf", &["literal-current-inventory"]);
    let inventory =
        SelectionInventory::from_groups(EventSemantics::ResultGroups21, 3, vec![id.clone()]);
    let encoded = serde_json::to_value(&inventory)?;
    check!(eq; encoded, serde_json::json!({
        "eligible_result_group_count":3,
        "tied_result_group_count":1,
        "tied_result_group_representative_ids":[id],
        "result_group_basis":"canonical_performance_id",
        "physical_performance_count_status":"unknown",
        "selection_policy_version":21,
    }));
    Ok(())
}
