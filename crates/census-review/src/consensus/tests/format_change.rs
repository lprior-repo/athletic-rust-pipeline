use super::*;

#[tokio::test]
async fn reopened_case_reasks_changed_format_and_reuses_exact_unchanged_peer_advice() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![
        good.clone(),
        batch(&case, "value_proposed", "state", "MN"),
    ]);
    let (second, server_b) = lane(vec![good]);
    let original_clients = [client(&first), client(&second)];
    let original = run_lanes(&fixture.store, &original_clients, &options(), "schema")
        .await
        .expect("initial agreement");
    assert_eq!(original.accepted, 1);
    let old_audit = audit(&fixture.store);
    let mut reopened = case.clone();
    reopened.state = ReviewState::Pending;
    fixture
        .store
        .replace(Table::ReviewCases, &reopened)
        .expect("explicitly reopened case");
    let changed = crate::ModelClient::new(
        original_clients[0]
            .options()
            .clone()
            .with_response_format(crate::ModelResponseFormat::PromptJson),
    )
    .expect("explicit prompt-json client");
    let clients = [changed, original_clients[1].clone()];
    let report = run_lanes(&fixture.store, &clients, &options(), "format-change")
        .await
        .expect("format change reopens advice");
    let requests_a = server_a.join().expect("first lane");
    let requests_b = server_b.join().expect("second lane");
    assert_eq!(requests_a.len(), 2);
    assert_eq!(requests_b.len(), 1);
    assert_eq!(report.requested, 1);
    assert_eq!(report.failed, 0);
    assert_eq!(report.accepted, 0);
    let new_audit = audit(&fixture.store);
    assert_eq!(new_audit["packet"], old_audit["packet"]);
    assert_eq!(new_audit["evidence_digest"], old_audit["evidence_digest"]);
    assert_eq!(new_audit["outcome"], "disagreement");
    assert_eq!(new_audit["lanes"][0]["response_format"], "prompt_json");
    assert_eq!(new_audit["lanes"][1]["response_format"], "json_schema");
    assert!(!row(&fixture.store).accepted);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 2);
}
