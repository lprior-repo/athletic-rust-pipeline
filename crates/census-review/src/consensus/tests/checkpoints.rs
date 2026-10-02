use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;

use super::support::{add_school, batch, client, lane, options, Fixture};
use crate::run_lanes;

#[tokio::test]
async fn both_advice_lanes_and_case_states_cross_the_checkpoint_boundary_together() {
    let (fixture, first) = Fixture::school();
    let mut cases = vec![first];
    cases.extend(
        (1..=crate::CHECKPOINT_CASES)
            .map(|index| add_school(&fixture.store, &format!("School {index}"))),
    );
    cases.sort_by(|first, second| first.id.cmp(&second.id));
    let replies = cases
        .iter()
        .map(|case| batch(case, "value_proposed", "state", "WI"))
        .collect::<Vec<_>>();
    let (first, server_a) = lane(replies.clone());
    let (second, server_b) = lane(replies);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "checkpoint",
    )
    .await
    .expect("dual review");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.requested, 257);
    assert_eq!(report.accepted, 257);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 2);
    let rows = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("verdicts");
    let states = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("cases");
    assert_eq!(rows.len(), 257);
    assert_eq!(states.len(), 257);
    for case in states {
        assert_eq!(case.state, ReviewState::Resolved);
        let row = rows
            .iter()
            .find(|row| row.case_id == case.id)
            .expect("atomically bound consensus");
        assert!(row.accepted);
        let audit: serde_json::Value = serde_json::from_str(&row.rationale).expect("audit");
        assert_eq!(
            audit["lanes"][0]["batch"]["verdicts"][0]["case_id"],
            case.id
        );
        assert_eq!(
            audit["lanes"][1]["batch"]["verdicts"][0]["case_id"],
            case.id
        );
    }
}
