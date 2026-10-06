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
                {
                    let left = &original.case;
                    let right = &changed.case;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
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
                {
                    let left = initial.accepted;
                    if left != 0 {
                        return Err(format!("left={left:?} right=0").into());
                    }
                }
                {
                    let left = state(&original.store)?;
                    let right = ReviewState::Retained;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
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
                {
                    let left = report.requested;
                    if left != 1 {
                        return Err(
                            format!("material change: {change:?}; left={left:?} right=1").into(),
                        );
                    }
                }
                {
                    let left = report.accepted;
                    if left != 1 {
                        return Err(format!(
                            "fresh different-person decision: {change:?}; left={left:?} right=1"
                        )
                        .into());
                    }
                }
                {
                    let left = report.failed;
                    if left != 0 {
                        return Err(format!("left={left:?} right=0").into());
                    }
                }
                let requests_a = server_a.join().map_err(|_| "first lane panicked")??;
                let requests_b = server_b.join().map_err(|_| "second lane panicked")??;
                for requests in [requests_a, requests_b] {
                    {
                        let left = requests.len();
                        if left != 2 {
                            return Err(format!("left={left:?} right=2").into());
                        }
                    }
                    for (request, side, expected) in [
                        (&requests[0], "side_a", &original_rows[0]),
                        (&requests[1], "side_a", &changed_rows[0]),
                        (&requests[1], "side_b", &changed_rows[1]),
                    ] {
                        let actual = canonical_side(request, side)?;
                        let expected = canonical(expected)?;
                        if actual != expected {
                            return Err(format!("left={actual:?} right={expected:?}").into());
                        }
                    }
                }
                {
                    let left = &audit(&changed.store)?["evidence_digest"];
                    let right = &old_digest;
                    if left == right {
                        return Err(
                            format!("expected unequal: left={left:?} right={right:?}").into()
                        );
                    }
                }
                {
                    let left = &audit(&changed.store)?["outcome"];
                    let right = "agreement";
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                {
                    let left = &row(&changed.store)?.case_id;
                    let right = &changed.case.id;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                {
                    let left = &row(&changed.store)?.subject_id;
                    let right = &changed.case.subject_id;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                {
                    let left = &row(&changed.store)?.member_ids;
                    let right = &changed.case.member_ids;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                {
                    let left = &row(&changed.store)?.value;
                    let right = "different_person";
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                {
                    let left = state(&changed.store)?;
                    let right = ReviewState::Resolved;
                    if left != right {
                        return Err(format!("left={left:?} right={right:?}").into());
                    }
                }
                assert_preserved(&original.store, &original_rows)?;
                assert_preserved(&changed.store, &changed_rows)?;
            }
            Ok(())
        })
}
