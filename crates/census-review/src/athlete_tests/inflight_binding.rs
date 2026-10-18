use census_domain::model::{CanonicalAthlete, ReviewState};
use census_store::Table;

use super::binding_support::{assert_preserved, canonical, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{
    audit, batch, client, lane, lane_with, row, state, TestResult,
};
use crate::run_lanes;

#[test]
fn a_source_url_or_retained_conflict_arriving_during_advice_refuses_the_old_binding() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for change in [Change::EvidenceUrl, Change::RetainedConflict] {
                let original_rows = rows()?;
                let fixture = Fixture::new(&original_rows)?;
                let mut incoming = original_rows[0].clone();
                change.apply(&mut incoming)?;
                let store = fixture.store.clone();
                let same = batch(&fixture.case, "value_proposed", "identity", "same_person");
                let different =
                    batch(&fixture.case, "value_proposed", "identity", "different_person");
                let (first, server_a) =
                    lane_with(vec![same.clone(), different.clone()], move |index, _| {
                        if index == 0 {
                            store.append_many(Table::Athletes, std::slice::from_ref(&incoming))?;
                        }
                        Ok(())
                    })?;
                let (second, server_b) = lane(vec![same, different])?;
                let clients = [client(&first)?, client(&second)?];
                let stale = run_lanes(&fixture.store, &clients, &options(), "inflight").await?;
                check!(eq; stale.accepted, 0, "material change: {change:?}");
                check!(eq; &audit(&fixture.store)?["outcome"], "evidence_changed");
                check!(eq; state(&fixture.store)?, ReviewState::Retained);
                check!(!row(&fixture.store)?.accepted, "stale advice must not be accepted");
                let current = fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?;
                check!(current != original_rows,
                    "expected unequal: left={current:?} right={original_rows:?}");
                check!(current.iter().any(|row| row.id == original_rows[0].id
                    && row.observed_grades == original_rows[0].observed_grades
                    && row.source == original_rows[0].source),
                    "merged row lost original grade or owner: current={current:?} original={original_rows:?}");
                let refreshed = run_lanes(&fixture.store, &clients, &options(), "refreshed").await?;
                check!(eq; refreshed.requested, 1);
                check!(eq; refreshed.accepted, 1);
                check!(eq; &row(&fixture.store)?.value, "different_person");
                check!(eq; &row(&fixture.store)?.case_id, &fixture.case.id);
                check!(eq; &row(&fixture.store)?.subject_id, &fixture.case.subject_id);
                check!(eq; &row(&fixture.store)?.member_ids, &fixture.case.member_ids);
                for requests in [
                    server_a.join().map_err(|_| "first lane panicked")??,
                    server_b.join().map_err(|_| "second lane panicked")??,
                ] {
                    check!(eq; requests.len(), 2);
                    check!(eq; canonical_side(&requests[0], "side_a")?, canonical(&original_rows[0])?);
                    let subject = current
                        .iter()
                        .find(|row| row.id == original_rows[0].id)
                        .ok_or("subject")?;
                    check!(eq; canonical_side(&requests[1], "side_a")?, canonical(subject)?);
                    check!(eq; canonical_side(&requests[1], "side_b")?, canonical(&original_rows[1])?);
                }
                check!(eq; &audit(&fixture.store)?["outcome"], "agreement");
                check!(eq; state(&fixture.store)?, ReviewState::Resolved);
                assert_preserved(&fixture.store, &current)?;
            }
            Ok(())
        })
}
