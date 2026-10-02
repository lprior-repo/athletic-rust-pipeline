use census_domain::model::ReviewState;
use census_store::Table;

use super::binding_support::{assert_preserved, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{audit, batch, client, lane, row, state};
use crate::run_lanes;

#[tokio::test]
async fn source_url_ownership_and_conflict_changes_cannot_reuse_another_snapshots_advice() {
    for change in [
        Change::GradeUrl,
        Change::PrimaryOwnership,
        Change::EvidenceUrl,
        Change::RetainedConflict,
    ] {
        let original_rows = rows();
        let original = Fixture::new(&original_rows);
        let mut changed_rows = original_rows.clone();
        change.apply(&mut changed_rows[0]);
        let changed = Fixture::new(&changed_rows);
        assert_eq!(original.case, changed.case);
        let declined = batch(&original.case, "insufficient_evidence", "", "");
        let decided = batch(
            &changed.case,
            "value_proposed",
            "identity",
            "different_person",
        );
        let (first, server_a) = lane(vec![declined.clone(), decided.clone()]);
        let (second, server_b) = lane(vec![declined, decided]);
        let clients = [client(&first), client(&second)];
        let initial = run_lanes(&original.store, &clients, &options(), "original")
            .await
            .expect("original unresolved advice");
        assert_eq!(initial.accepted, 0);
        assert_eq!(state(&original.store), ReviewState::Retained);
        let recorded = row(&original.store);
        let old_digest = audit(&original.store)["evidence_digest"].clone();
        changed
            .store
            .replace_many(Table::IdentityVerdicts, &[recorded.clone()])
            .expect("previous exact-bound advice");
        let mut retained = changed.case.clone();
        retained.state = ReviewState::Retained;
        changed
            .store
            .replace_many(Table::ReviewCases, &[retained])
            .expect("unresolved case");
        let report = run_lanes(&changed.store, &clients, &options(), "changed")
            .await
            .expect("changed identity evidence");
        assert_eq!(report.requested, 1, "material change: {change:?}");
        assert_eq!(
            report.accepted, 1,
            "fresh different-person decision: {change:?}"
        );
        assert_eq!(report.failed, 0);
        let requests_a = server_a
            .join()
            .expect("first lane receives changed snapshot");
        let requests_b = server_b
            .join()
            .expect("second lane receives changed snapshot");
        for requests in [requests_a, requests_b] {
            assert_eq!(requests.len(), 2);
            assert_eq!(canonical_side(&requests[0], "side_a"), original_rows[0]);
            assert_eq!(canonical_side(&requests[1], "side_a"), changed_rows[0]);
            assert_eq!(canonical_side(&requests[1], "side_b"), changed_rows[1]);
        }
        assert_ne!(audit(&changed.store)["evidence_digest"], old_digest);
        assert_eq!(audit(&changed.store)["outcome"], "agreement");
        assert_eq!(row(&changed.store).case_id, changed.case.id);
        assert_eq!(row(&changed.store).subject_id, changed.case.subject_id);
        assert_eq!(row(&changed.store).member_ids, changed.case.member_ids);
        assert_eq!(row(&changed.store).value, "different_person");
        assert_eq!(state(&changed.store), ReviewState::Resolved);
        assert_preserved(&original.store, &original_rows);
        assert_preserved(&changed.store, &changed_rows);
    }
}
