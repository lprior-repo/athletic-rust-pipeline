use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{add_school, batch, client, lane, options, Fixture};
use crate::run_lanes;

fn historical(case: &ReviewCase, bytes: usize) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        member_ids: case.member_ids.clone(),
        kind: "insufficient_evidence".to_string(),
        field: String::new(),
        value: String::new(),
        accepted: false,
        confidence: 0,
        rationale: "x".repeat(bytes),
        reviewer: "historical-model".to_string(),
        observed_at: "old".to_string(),
    }
}

#[tokio::test]
async fn zero_budget_validates_lanes_without_reading_census_or_advice() {
    let (fixture, _) = Fixture::school();
    fixture
        .store
        .replace_many(
            Table::ReviewCases,
            &[serde_json::json!({"id": "poison-case"})],
        )
        .expect("poison case");
    fixture
        .store
        .replace_many(
            Table::IdentityVerdicts,
            &[serde_json::json!({"id": "poison-advice"})],
        )
        .expect("poison advice");
    let mut options = options();
    options.limit = 0;
    let first = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    let second = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    first.set_nonblocking(true).expect("nonblocking");
    second.set_nonblocking(true).expect("nonblocking");
    let clients = [
        client(&format!("http://{}", first.local_addr().expect("address"))),
        client(&format!("http://{}", second.local_addr().expect("address"))),
    ];
    assert_eq!(
        run_lanes(&fixture.store, &clients, &options, "zero")
            .await
            .expect("zero intake"),
        crate::ReviewReport::default()
    );
    assert!(matches!(
        run_lanes(&fixture.store, &clients[..1], &options, "invalid").await,
        Err(StoreError::Invariant { .. })
    ));
    for listener in [first, second] {
        assert_eq!(
            listener
                .accept()
                .expect_err("no HTTP at zero budget")
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
}

#[tokio::test]
async fn irrelevant_large_advice_history_is_not_part_of_the_selected_checkpoint_budget() {
    let (fixture, case) = Fixture::school();
    let mut resolved = ReviewCase::pending(
        "School jurisdiction unresolved",
        "historical",
        "Old school",
        "old case",
    );
    resolved.state = ReviewState::Resolved;
    let historical = historical(&resolved, 9 * 1024 * 1024);
    fixture
        .store
        .replace_many(Table::ReviewCases, &[resolved])
        .expect("resolved history");
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&historical))
        .expect("large history");
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let mut options = options();
    options.limit = 1;
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options,
        "selected",
    )
    .await
    .expect("selected review");
    assert_eq!(server_a.join().expect("first lane").len(), 1);
    assert_eq!(server_b.join().expect("second lane").len(), 1);
    assert_eq!(
        (report.requested, report.accepted, report.failed),
        (1, 1, 0)
    );
    let rows = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rows");
    assert_eq!(
        rows.iter().find(|row| row.id == historical.id),
        Some(&historical)
    );
    let cases = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("states");
    assert_eq!(
        cases
            .iter()
            .find(|row| row.id == case.id)
            .expect("selected")
            .state,
        ReviewState::Resolved
    );
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
}

#[tokio::test]
async fn aggregate_selected_advice_over_eight_mib_refuses_without_partial_checkpoint() {
    let (fixture, first) = Fixture::school();
    let cases = vec![
        first,
        add_school(&fixture.store, "Madison East"),
        add_school(&fixture.store, "Madison North"),
    ];
    let history: Vec<_> = cases
        .iter()
        .map(|case| historical(case, 3 * 1024 * 1024))
        .collect();
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &history)
        .expect("selected historical advice");
    let mut options = options();
    options.limit = 3;
    let error = run_lanes(
        &fixture.store,
        &[
            client("http://127.0.0.1:18081"),
            client("http://127.0.0.1:18082"),
        ],
        &options,
        "bound",
    )
    .await
    .expect_err("aggregate budget refusal");
    match error {
        StoreError::Invariant { detail } => assert!(
            detail.starts_with("review checkpoint exceeds aggregate byte budget 8388608:"),
            "{detail}"
        ),
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(
        fixture
            .store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .expect("advice"),
        {
            let mut history = history;
            history.sort_by(|a, b| a.id.cmp(&b.id));
            history
        }
    );
    assert!(fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("states")
        .iter()
        .all(|case| case.state == ReviewState::Pending));
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
}
