mod common;

use census_domain::model::{
    normalize_name, CanonicalMeet, CanonicalSchool, CollectionSnapshot, CompetitionLevel,
    ReviewCase, ReviewState, SourceIdentity, SourceNamespace, SourceObjectIdentity,
    UNRESOLVED_VENUE_FAMILY,
};
use census_domain::UsJurisdiction;
use census_reconcile::index::derive;
use census_reconcile::verify::{column_index, missing_columns, sample_indices};
use census_store::{Store, Table};
use common::{stored_case, TestResult};

#[test]
fn resolved_decision_survives_rederivation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let meet = CanonicalMeet::new(
        None,
        "Unplaced Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet)?;
    derive(&store, "index", "2026-09-22", common::season()?)?;
    let pending = stored_case(&store, meet.id.as_str())?;
    if pending.state != ReviewState::Pending {
        return Err(format!("pending state: {:?}", pending.state).into());
    }
    let mut decided = pending.clone();
    decided.state = ReviewState::Resolved;
    store.replace(Table::ReviewCases, &decided)?;

    let report = derive(&store, "index", "2026-09-22", common::season()?)?;
    let retained = stored_case(&store, meet.id.as_str())?;
    if retained.state != ReviewState::Resolved {
        return Err(format!("retained state: {:?}", retained.state).into());
    }
    if report.superseded != 0 {
        return Err(format!("superseded cases: {}", report.superseded).into());
    }
    Ok(())
}

#[test]
fn stale_pending_case_is_superseded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let ghost = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:ghost",
        "Ghost Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    store.replace(Table::ReviewCases, &ghost)?;
    let live = CanonicalMeet::new(
        None,
        "Live Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &live)?;

    let report = derive(&store, "index", "2026-09-22", common::season()?)?;
    if report.superseded != 1 {
        return Err(format!("superseded cases: {}", report.superseded).into());
    }
    let ghost_case = stored_case(&store, "meet:ghost")?;
    if ghost_case.state != ReviewState::Superseded {
        return Err(format!("ghost state: {:?}", ghost_case.state).into());
    }
    let live_case = stored_case(&store, live.id.as_str())?;
    if live_case.state != ReviewState::Pending {
        return Err(format!("live state: {:?}", live_case.state).into());
    }
    Ok(())
}

#[test]
fn repeated_pass_reuses_ids_and_single_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (mut school, _id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Abbotsford",
        normalize_name("Abbotsford"),
        None,
    );
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "1234",
    ));
    store.append(Table::Schools, &school)?;

    let first = derive(&store, "index", "2026-09-22", common::season()?)?;
    let second = derive(&store, "index", "2026-09-22", common::season()?)?;
    if first.total() != second.total() {
        return Err(format!("totals: {:?} and {:?}", first.total(), second.total()).into());
    }
    let identities: Vec<SourceObjectIdentity> = store.scan(Table::SourceIdentities)?;
    if identities.len() != 1 {
        return Err(format!("identity rows: {}", identities.len()).into());
    }
    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots)?;
    if snapshots.len() != 1 {
        return Err(format!("snapshot rows: {}", snapshots.len()).into());
    }
    if snapshots[0].id != "index:2026-09-22" {
        return Err(format!("snapshot id: {:?}", snapshots[0].id).into());
    }
    let schools = snapshots[0].observations.get("schools");
    if schools != Some(&1) {
        return Err(format!("school observations: {schools:?}").into());
    }
    Ok(())
}

#[test]
fn verify_helpers_are_exact_and_bounded() {
    assert_eq!(
        column_index(&["Athlete ID".to_string(), "Name".to_string()], "Name"),
        Some(1)
    );
    assert_eq!(column_index(&["Name".to_string()], "name"), None);
    assert_eq!(
        missing_columns(&["A".to_string(), "C".to_string()], &["A", "B", "C"]),
        vec!["B"]
    );
    assert!(missing_columns(&["A".to_string()], &["A"]).is_empty());
    assert_eq!(sample_indices(0, 1), Vec::<usize>::new());
    assert_eq!(sample_indices(1, 1), vec![0]);
    assert_eq!(sample_indices(5, 1), vec![0, 1, 2, 3, 4]);
    assert_eq!(sample_indices(10, 2), vec![0, 2, 4, 6, 8]);
    for total in [2usize, 3, 7, 999, 100_000] {
        for k in [1usize, 2, 3, 20] {
            let indices = sample_indices(total, k);
            assert!(!indices.is_empty());
            assert_eq!(indices[0], 0);
            assert!(indices.len() <= 5000);
            assert!(indices.iter().all(|index| *index < total));
            let mut ordered = indices.clone();
            ordered.dedup();
            assert_eq!(ordered, indices);
        }
    }
}
