use census_domain::model::ReviewState;
use census_store::Table;

use super::binding_support::{assert_preserved, canonical, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{audit, batch, client, lane, row, state, TestResult};
use crate::run_lanes;

#[test]
fn source_url_ownership_and_conflict_changes_cannot_reuse_another_snapshots_advice() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for change in [
                Change::GradeUrl,
                Change::PrimaryOwnership,
                Change::EvidenceUrl,
                Change::RetainedConflict,
            ] {
                let original_rows = rows()?;
                let original = Fixture::new(&original_rows)?;
                let mut changed_rows = original_rows.clone();
                change.apply(&mut changed_rows[0])?;
                let changed = Fixture::new(&changed_rows)?;
                check!(eq; &original.case, &changed.case);
                let declined = batch(&original.case, "insufficient_evidence", "", "");
                let decided = batch(
                    &changed.case,
                    "value_proposed",
                    "identity",
                    "different_person",
                );
                let (first, server_a) = lane(vec![declined.clone(), decided.clone()])?;
                let (second, server_b) = lane(vec![declined, decided])?;
                let clients = [client(&first)?, client(&second)?];
                let initial = run_lanes(&original.store, &clients, &options(), "original").await?;
                check!(eq; initial.accepted, 0);
                check!(eq; state(&original.store)?, ReviewState::Retained);
                let recorded = row(&original.store)?;
                let old_digest = audit(&original.store)?["evidence_digest"].clone();
                changed
                    .store
                    .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&recorded))?;
                let mut retained = changed.case.clone();
                retained.state = ReviewState::Retained;
                changed
                    .store
                    .replace_many(Table::ReviewCases, &[retained])?;
                let report = run_lanes(&changed.store, &clients, &options(), "changed").await?;
                check!(eq; report.requested, 1, "material change: {change:?}");
                check!(eq; report.accepted, 1, "fresh different-person decision: {change:?}");
                check!(eq; report.failed, 0);
                let requests_a = server_a.join().map_err(|_| "first lane panicked")??;
                let requests_b = server_b.join().map_err(|_| "second lane panicked")??;
                for requests in [requests_a, requests_b] {
                    check!(eq; requests.len(), 2);
                    for (request, side, expected) in [
                        (&requests[0], "side_a", &original_rows[0]),
                        (&requests[1], "side_a", &changed_rows[0]),
                        (&requests[1], "side_b", &changed_rows[1]),
                    ] {
                        let actual = canonical_side(request, side)?;
                        let expected = canonical(expected)?;
                        check!(eq; actual, expected);
                    }
                }
                let new_digest = &audit(&changed.store)?["evidence_digest"];
                check!(
                    new_digest != &old_digest,
                    "expected unequal: left={new_digest:?} right={old_digest:?}"
                );
                check!(eq; &audit(&changed.store)?["outcome"], "agreement");
                check!(eq; &row(&changed.store)?.case_id, &changed.case.id);
                check!(eq; &row(&changed.store)?.subject_id, &changed.case.subject_id);
                check!(eq; &row(&changed.store)?.member_ids, &changed.case.member_ids);
                check!(eq; &row(&changed.store)?.value, "different_person");
                check!(eq; state(&changed.store)?, ReviewState::Resolved);
                assert_preserved(&original.store, &original_rows)?;
                assert_preserved(&changed.store, &changed_rows)?;
            }
            Ok(())
        })
}
