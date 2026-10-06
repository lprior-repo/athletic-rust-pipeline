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
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { for change in [Change::EvidenceUrl, Change::RetainedConflict] {
    let original_rows = rows()?;
    let fixture = Fixture::new(&original_rows)?;
    let mut incoming = original_rows[0].clone();
    change.apply(&mut incoming)?;
    let store = fixture.store.clone();
    let same = batch(&fixture.case, "value_proposed", "identity", "same_person");
    let different = batch(&fixture.case, "value_proposed", "identity", "different_person");
    let (first, server_a) = lane_with(vec![same.clone(), different.clone()], move |index, _| {
        if index == 0 {
            store.append_many(Table::Athletes, std::slice::from_ref(&incoming))?;
        }
        Ok(())
    })?;
    let (second, server_b) = lane(vec![same, different])?;
    let clients = [client(&first)?, client(&second)?];
    let stale = run_lanes(&fixture.store, &clients, &options(), "inflight").await?;
    {
        let left = stale.accepted;
        if left != 0 { return Err(format!("material change: {change:?}; left={left:?} right=0").into()); }
    }
    {
        let left = &audit(&fixture.store)?["outcome"];
        let right = "evidence_changed";
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    {
        let left = state(&fixture.store)?;
        let right = ReviewState::Retained;
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    if row(&fixture.store)?.accepted { return Err("stale advice must not be accepted".into()); }
    let current = fixture.store.scan::<CanonicalAthlete>(Table::Athletes)?;
    if current == original_rows {
        return Err(format!("expected unequal: left={current:?} right={original_rows:?}").into());
    }
    if !current.iter().any(|row| row.id == original_rows[0].id
        && row.observed_grades == original_rows[0].observed_grades
        && row.source == original_rows[0].source) {
        return Err(format!("merged row lost original grade or owner: current={current:?} original={original_rows:?}").into());
    }
    let refreshed = run_lanes(&fixture.store, &clients, &options(), "refreshed").await?;
    {
        let left = refreshed.requested;
        if left != 1 { return Err(format!("left={left:?} right=1").into()); }
    }
    {
        let left = refreshed.accepted;
        if left != 1 { return Err(format!("left={left:?} right=1").into()); }
    }
    {
        let left = &row(&fixture.store)?.value;
        let right = "different_person";
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    {
        let left = &row(&fixture.store)?.case_id;
        let right = &fixture.case.id;
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    {
        let left = &row(&fixture.store)?.subject_id;
        let right = &fixture.case.subject_id;
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    {
        let left = &row(&fixture.store)?.member_ids;
        let right = &fixture.case.member_ids;
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    for requests in [
        server_a.join().map_err(|_| "first lane panicked")??,
        server_b.join().map_err(|_| "second lane panicked")??,
    ] {
        {
            let left = requests.len();
            if left != 2 { return Err(format!("left={left:?} right=2").into()); }
        }
        {
            let left = canonical_side(&requests[0], "side_a")?;
            let right = canonical(&original_rows[0])?;
            if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
        }
        let subject = current.iter().find(|row| row.id == original_rows[0].id).ok_or("subject")?;
        {
            let left = canonical_side(&requests[1], "side_a")?;
            let expected = canonical(subject)?;
            if left != expected { return Err(format!("left={left:?} right={expected:?}").into()); }
        }
        {
            let left = canonical_side(&requests[1], "side_b")?;
            let right = canonical(&original_rows[1])?;
            if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
        }
    }
    {
        let left = &audit(&fixture.store)?["outcome"];
        let right = "agreement";
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    {
        let left = state(&fixture.store)?;
        let right = ReviewState::Resolved;
        if left != right { return Err(format!("left={left:?} right={right:?}").into()); }
    }
    assert_preserved(&fixture.store, &current)?;
}
Ok(()) })
}
