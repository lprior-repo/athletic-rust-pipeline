//! Tests for bounded checkpoints, atomic commits, and recovery guarantees.
//!
//! These tests prove:
//! 1. One checkpoint writes both tables in one commit (one receipt).
//! 2. A replayed checkpoint writes nothing (Application::Repeated).
//! 3. Multiple chunks produce multiple receipts.
//! 4. Resume-after-crash: cases closed in a prior checkpoint are not re-asked.

use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewState,
    ReviewVerdictKind, ReviewVerdictRecord, SourceIdentity, SourceNamespace,
    ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use super::{reconcile_athletes, ReviewFamily, ReviewOptions};

/// Build a test verdict record.
fn verdict_row(id: &str, case_id: &str, subject_id: &str) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: id.to_string(),
        case_id: case_id.to_string(),
        subject_id: subject_id.to_string(),
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        member_ids: vec![],
        kind: ReviewVerdictKind::ValueProposed.slug().to_string(),
        field: "identity".to_string(),
        value: "same_person".to_string(),
        accepted: true,
        confidence: 80,
        rationale: "test".to_string(),
        reviewer: "test".to_string(),
        observed_at: "2026-09-25".to_string(),
    }
}

/// Build a test case row.
fn case_row(id: &str, state: ReviewState) -> ReviewCase {
    ReviewCase {
        id: id.to_string(),
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        subject_id: "a1".to_string(),
        subject: "Athlete".to_string(),
        detail: "test".to_string(),
        state,
        member_ids: vec![],
    }
}

fn receipt_count(store: &Store) -> u64 {
    store.receipt_count().expect("receipt count")
}

#[test]
fn one_checkpoint_writes_both_tables_in_one_commit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");

    let verdict = verdict_row("v1", "c1", "s1");
    let case = case_row("c1", ReviewState::Resolved);

    let mut batch = store.write_batch();
    batch
        .replace_many(Table::IdentityVerdicts, &[verdict])
        .unwrap();
    batch.replace_many(Table::ReviewCases, &[case]).unwrap();
    let app = batch
        .commit_once("review:2026-09-25:0", "digest-0")
        .unwrap();
    assert!(app.written(), "checkpoint wrote");

    assert_eq!(
        store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .unwrap()
            .len(),
        1,
        "verdict row readable"
    );
    assert_eq!(
        store.scan::<ReviewCase>(Table::ReviewCases).unwrap().len(),
        1,
        "case row readable"
    );
    assert_eq!(receipt_count(&store), 1, "one checkpoint, one receipt");
}

#[tokio::test]
async fn replayed_checkpoint_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");

    let verdict = verdict_row("v1", "c1", "s1");
    let case = case_row("c1", ReviewState::Resolved);

    let mut batch = store.write_batch();
    batch
        .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&verdict))
        .unwrap();
    batch
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .unwrap();
    let first = batch
        .commit_once("test-op", "test-digest")
        .expect("first commit");
    assert!(first.written(), "first commit wrote");
    assert_eq!(receipt_count(&store), 1);

    let mut batch2 = store.write_batch();
    batch2
        .replace_many(Table::IdentityVerdicts, &[verdict])
        .unwrap();
    batch2.replace_many(Table::ReviewCases, &[case]).unwrap();
    let second = batch2
        .commit_once("test-op", "test-digest")
        .expect("replay commit");
    assert!(second.repeated(), "replay is rejected");
    assert_eq!(receipt_count(&store), 1, "still one receipt");
}

#[test]
fn multiple_chunks_produce_multiple_receipts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");

    let verdict = verdict_row("v1", "c1", "s1");
    let case = case_row("c1", ReviewState::Pending);

    let mut batch = store.write_batch();
    batch
        .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&verdict))
        .unwrap();
    batch
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .unwrap();
    let app = batch
        .commit_once("review:2026-09-25:0", "digest-0")
        .unwrap();
    assert!(app.written(), "checkpoint 0 wrote");

    let mut batch2 = store.write_batch();
    batch2
        .replace_many(Table::IdentityVerdicts, &[verdict])
        .unwrap();
    batch2.replace_many(Table::ReviewCases, &[case]).unwrap();
    let app2 = batch2
        .commit_once("review:2026-09-25:1", "digest-1")
        .unwrap();
    assert!(app2.written(), "checkpoint 1 wrote");

    assert_eq!(receipt_count(&store), 2, "two checkpoints, two receipts");
}

#[test]
fn resume_after_crash_does_not_reask_closed_cases() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");

    let case = case_row("c1", ReviewState::Resolved);
    store
        .replace_many(Table::ReviewCases, &[case])
        .expect("case written as resolved");

    let options = ReviewOptions {
        families: vec![ReviewFamily::AthleteIdentity],
        limit: 1000,
        dry_run: false,
    };
    let pending = super::packets::pending_cases(&store, &options).expect("pending readable");
    assert!(
        pending.is_empty(),
        "no pending cases remain after a pass closed the case"
    );
}

/// Invariant: every ReviewCases row whose state is not Pending has a matching IdentityVerdicts row
/// keyed by its case id. This must hold after any pass that writes verdicts and case updates.
///
/// reconcile_athletes decides N cases and must write N verdicts and N resolved cases in one
/// atomic commit, so a crash cannot leave a resolved case with no verdict (or vice versa).
#[test]
fn reconcile_athletes_writes_both_tables_in_one_commit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path()).expect("store opens");

    let school_a = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "School A",
        census_domain::model::normalize_name("School A"),
    )
    .0
    .id;
    let school_b = CanonicalSchool::new(
        census_domain::UsJurisdiction::Minnesota,
        "School B",
        census_domain::model::normalize_name("School B"),
    )
    .0
    .id;

    let mut a = CanonicalAthlete::new(&school_a, "Jordan Smith", GradYear::CO2027, Gender::Boys);
    let mut b = CanonicalAthlete::new(&school_b, "Jordan Smith", GradYear::CO2027, Gender::Boys);
    a.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
    ));
    b.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
    ));
    store
        .append_many(Table::Athletes, &[a.clone(), b.clone()])
        .expect("athletes written");

    let report = reconcile_athletes(&store, "2026-09-25", false).expect("reconcile runs");
    assert_eq!(report.decided, 1, "one pair decided by rule");

    let verdicts = store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .unwrap();
    let cases = store.scan::<ReviewCase>(Table::ReviewCases).unwrap();
    assert_eq!(verdicts.len(), 1, "one verdict row written");
    assert_eq!(cases.len(), 1, "one case row written");
    assert_eq!(
        verdicts[0].case_id, cases[0].id,
        "verdict is keyed to the resolved case"
    );
    assert_eq!(
        receipt_count(&store),
        1,
        "one receipt for the reconcile pass"
    );

    let report2 = reconcile_athletes(&store, "2026-09-25", false).expect("replay runs");
    assert_eq!(
        report2.decided, 0,
        "replay decides nothing — case is held by standing decision"
    );
    assert_eq!(
        store
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
            .unwrap()
            .len(),
        1,
        "verdict count unchanged after replay"
    );
    assert_eq!(
        store.scan::<ReviewCase>(Table::ReviewCases).unwrap().len(),
        1,
        "case count unchanged after replay"
    );
    assert_eq!(receipt_count(&store), 1, "still one receipt");
}
