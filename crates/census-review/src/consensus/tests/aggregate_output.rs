use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{add_school, batch, client, lane, options, Fixture};
use crate::run_lanes;

#[tokio::test]
async fn aggregate_joined_advice_refuses_before_any_checkpoint_state_is_published() {
    let (fixture, first) = Fixture::school();
    let mut cases = vec![first];
    for index in 1..8 {
        cases.push(add_school(
            &fixture.store,
            &format!("Madison Aggregate {index}"),
        ));
    }
    cases.sort_by(|a, b| a.id.cmp(&b.id));
    let replies: Vec<_> = cases
        .iter()
        .map(|case| {
            let mut reply: serde_json::Value =
                serde_json::from_str(&batch(case, "value_proposed", "state", "WI")).expect("reply");
            reply["verdicts"][0]["rationale"] = "x".repeat(200 * 1024).into();
            reply.to_string()
        })
        .collect();
    let (first, server_a) = lane(replies.clone());
    let (second, server_b) = lane(replies);
    let error = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "aggregate-output",
    )
    .await
    .expect_err("aggregate output refusal");
    assert_eq!(server_a.join().expect("first lane").len(), cases.len());
    assert_eq!(server_b.join().expect("second lane").len(), cases.len());
    assert!(matches!(error, StoreError::Invariant { .. }));
    assert_eq!(
        fixture
            .store
            .scan::<ReviewCase>(Table::ReviewCases)
            .expect("original cases"),
        cases
    );
    assert!(fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("pending cases")
        .iter()
        .all(|case| case.state == ReviewState::Pending));
    assert_eq!(
        fixture
            .store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .expect("verdicts"),
        Vec::<ReviewVerdictRecord>::new()
    );
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
}
