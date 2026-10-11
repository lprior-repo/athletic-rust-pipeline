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
use common::stored_case;

#[test]
fn resolved_decision_survives_rederivation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let meet = CanonicalMeet::new(
        None,
        "Unplaced Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet).expect("meet appends");
    derive(&store, "index", "2026-09-22").expect("first derive");
    let pending = stored_case(&store, meet.id.as_str());
    assert_eq!(pending.state, ReviewState::Pending);
    let mut decided = pending.clone();
    decided.state = ReviewState::Resolved;
    store
        .replace(Table::ReviewCases, &decided)
        .expect("resolve");

    let report = derive(&store, "index", "2026-09-22").expect("second derive");
    assert_eq!(
        stored_case(&store, meet.id.as_str()).state,
        ReviewState::Resolved
    );
    assert_eq!(report.superseded, 0);
}

#[test]
fn stale_pending_case_is_superseded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let ghost = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:ghost",
        "Ghost Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    store
        .replace(Table::ReviewCases, &ghost)
        .expect("ghost case");
    let live = CanonicalMeet::new(
        None,
        "Live Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &live).expect("meet appends");

    let report = derive(&store, "index", "2026-09-22").expect("derive");
    assert_eq!(report.superseded, 1);
    assert_eq!(
        stored_case(&store, "meet:ghost").state,
        ReviewState::Superseded
    );
    assert_eq!(
        stored_case(&store, live.id.as_str()).state,
        ReviewState::Pending
    );
}

#[test]
fn repeated_pass_reuses_ids_and_single_snapshot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(dir.path()).expect("store");
    let (mut school, _id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Abbotsford",
        normalize_name("Abbotsford"),
    );
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "1234",
    ));
    store
        .append(Table::Schools, &school)
        .expect("school appends");

    let first = derive(&store, "index", "2026-09-22").expect("first derive");
    let second = derive(&store, "index", "2026-09-22").expect("second derive");
    assert_eq!(first.total(), second.total());
    let identities: Vec<SourceObjectIdentity> =
        store.scan(Table::SourceIdentities).expect("identities");
    assert_eq!(identities.len(), 1);
    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots).expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].id, "index:2026-09-22");
    assert_eq!(snapshots[0].observations.get("schools"), Some(&1));
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
