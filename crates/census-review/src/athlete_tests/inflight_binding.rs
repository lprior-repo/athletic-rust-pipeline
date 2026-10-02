use census_domain::model::{CanonicalAthlete, ReviewState};
use census_store::Table;

use super::binding_support::{assert_preserved, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{audit, batch, client, lane, lane_with, row, state};
use crate::run_lanes;

#[tokio::test]
async fn a_source_url_or_retained_conflict_arriving_during_advice_refuses_the_old_binding() {
    for change in [Change::EvidenceUrl, Change::RetainedConflict] {
        let original_rows = rows();
        let fixture = Fixture::new(&original_rows);
        let mut incoming = original_rows[0].clone();
        change.apply(&mut incoming);
        let store = fixture.store.clone();
        let same = batch(&fixture.case, "value_proposed", "identity", "same_person");
        let different = batch(
            &fixture.case,
            "value_proposed",
            "identity",
            "different_person",
        );
        let (first, server_a) =
            lane_with(vec![same.clone(), different.clone()], move |index, _| {
                if index == 0 {
                    store
                        .append_many(Table::Athletes, std::slice::from_ref(&incoming))
                        .expect("new source evidence before reply");
                }
            });
        let (second, server_b) = lane(vec![same, different]);
        let clients = [client(&first), client(&second)];
        let stale = run_lanes(&fixture.store, &clients, &options(), "inflight")
            .await
            .expect("stale advice retained");
        assert_eq!(stale.accepted, 0, "material change: {change:?}");
        assert_eq!(audit(&fixture.store)["outcome"], "evidence_changed");
        assert_eq!(state(&fixture.store), ReviewState::Retained);
        assert!(!row(&fixture.store).accepted);
        let current = fixture
            .store
            .scan::<CanonicalAthlete>(Table::Athletes)
            .expect("merged source evidence");
        assert_ne!(current, original_rows);
        assert!(current.iter().any(|row| row.id == original_rows[0].id
            && row.observed_grades == original_rows[0].observed_grades
            && row.source == original_rows[0].source));
        let refreshed = run_lanes(&fixture.store, &clients, &options(), "refreshed")
            .await
            .expect("fresh evidence reviewed");
        assert_eq!(refreshed.requested, 1);
        assert_eq!(refreshed.accepted, 1);
        assert_eq!(row(&fixture.store).value, "different_person");
        assert_eq!(row(&fixture.store).case_id, fixture.case.id);
        assert_eq!(row(&fixture.store).subject_id, fixture.case.subject_id);
        assert_eq!(row(&fixture.store).member_ids, fixture.case.member_ids);
        for requests in [
            server_a.join().expect("first lane"),
            server_b.join().expect("second lane"),
        ] {
            assert_eq!(requests.len(), 2);
            assert_eq!(canonical_side(&requests[0], "side_a"), original_rows[0]);
            let subject = current
                .iter()
                .find(|row| row.id == original_rows[0].id)
                .expect("subject");
            assert_eq!(canonical_side(&requests[1], "side_a"), *subject);
            assert_eq!(canonical_side(&requests[1], "side_b"), original_rows[1]);
        }
        assert_eq!(audit(&fixture.store)["outcome"], "agreement");
        assert_eq!(state(&fixture.store), ReviewState::Resolved);
        assert_preserved(&fixture.store, &current);
    }
}
