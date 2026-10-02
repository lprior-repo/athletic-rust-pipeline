use census_domain::model::ReviewState;

use super::binding_support::{assert_preserved, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{audit, batch, client, lane, row, state};
use crate::run_lanes;

#[tokio::test]
async fn a_retained_source_conflict_refuses_same_person_but_preserves_different_person() {
    for (answer, accepted, outcome, expected_state) in [
        (
            "same_person",
            0,
            "hard_contradiction",
            ReviewState::Retained,
        ),
        ("different_person", 1, "agreement", ReviewState::Resolved),
    ] {
        let mut source_rows = rows();
        Change::RetainedConflict.apply(&mut source_rows[0]);
        let fixture = Fixture::new(&source_rows);
        let reply = batch(&fixture.case, "value_proposed", "identity", answer);
        let (first, server_a) = lane(vec![reply.clone()]);
        let (second, server_b) = lane(vec![reply]);
        let report = run_lanes(
            &fixture.store,
            &[client(&first), client(&second)],
            &options(),
            answer,
        )
        .await
        .expect("source conflict adjudication");
        assert_eq!(report.accepted, accepted);
        assert_eq!(report.rejected, 1 - accepted);
        assert_eq!(audit(&fixture.store)["outcome"], outcome);
        assert_eq!(state(&fixture.store), expected_state);
        let verdict = row(&fixture.store);
        assert_eq!(verdict.accepted, accepted == 1);
        assert_eq!(verdict.case_id, fixture.case.id);
        assert_eq!(verdict.subject_id, fixture.case.subject_id);
        assert_eq!(verdict.member_ids, fixture.case.member_ids);
        let retained = audit(&fixture.store);
        for advice in retained["lanes"].as_array().expect("both advice lanes") {
            assert_eq!(advice["batch"]["verdicts"][0]["value"], answer);
        }
        for requests in [
            server_a.join().expect("first lane"),
            server_b.join().expect("second lane"),
        ] {
            assert_eq!(canonical_side(&requests[0], "side_a"), source_rows[0]);
            assert_eq!(canonical_side(&requests[0], "side_b"), source_rows[1]);
            let content = requests[0]["messages"][1]["content"]
                .as_str()
                .expect("attributed packet");
            assert!(content.contains("retained_source_conflict:"));
            assert!(content.contains("https://source.example/conflict"));
        }
        assert_preserved(&fixture.store, &source_rows);
    }
}
