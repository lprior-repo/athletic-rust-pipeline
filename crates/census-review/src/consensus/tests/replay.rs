use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;

use super::support::{
    add_school, audit, batch, client, lane, lane_with, options, row, state, Fixture,
};
use crate::run_lanes;

#[tokio::test]
async fn exact_dual_advice_replays_without_reasking_or_writing_another_checkpoint() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let clients = [client(&first), client(&second)];
    let report = run_lanes(&fixture.store, &clients, &options(), "first")
        .await
        .expect("initial consensus");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.accepted, 1);
    let original = row(&fixture.store);
    let replay = run_lanes(&fixture.store, &clients, &options(), "second")
        .await
        .expect("durable replay");
    assert_eq!(replay.requested, 0);
    assert_eq!(replay.failed, 0);
    assert_eq!(row(&fixture.store), original);
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(fixture.store.receipt_count().expect("receipt count"), 1);
}

#[tokio::test]
async fn historical_acceptance_is_preserved_until_an_operator_explicitly_reopens_the_case() {
    let (fixture, mut case) = Fixture::school();
    case.state = ReviewState::Resolved;
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("historical resolved case");
    let historical = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        member_ids: case.member_ids.clone(),
        kind: "value_proposed".to_string(),
        field: "state".to_string(),
        value: "WI".to_string(),
        accepted: true,
        confidence: 99,
        rationale: "one model guessed".to_string(),
        reviewer: "historical-model".to_string(),
        observed_at: "old".to_string(),
    };
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&historical))
        .expect("historical verdict");
    let report = run_lanes(
        &fixture.store,
        &[
            client("http://127.0.0.1:18081"),
            client("http://127.0.0.1:18082"),
        ],
        &options(),
        "preserved",
    )
    .await
    .expect("preserved history");
    assert_eq!(report.requested, 0);
    assert_eq!(row(&fixture.store), historical);
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
    case.state = ReviewState::Pending;
    fixture
        .store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("operator reopened case");
    let (first, server_a) = lane(vec![batch(&case, "value_proposed", "state", "WI")]);
    let (second, server_b) = lane(vec![batch(&case, "value_proposed", "state", "MN")]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "cutover",
    )
    .await
    .expect("dual review");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.requested, 1);
    assert_eq!(report.accepted, 0);
    assert!(!row(&fixture.store).accepted);
    assert_eq!(audit(&fixture.store)["outcome"], "disagreement");
    assert_eq!(state(&fixture.store), ReviewState::Retained);
}

#[tokio::test]
async fn changed_evidence_invalidates_standing_advice_and_binds_the_new_packet() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone(), good.clone()]);
    let (second, server_b) = lane(vec![good, batch(&case, "value_proposed", "state", "MN")]);
    let clients = [client(&first), client(&second)];
    assert_eq!(
        run_lanes(&fixture.store, &clients, &options(), "first")
            .await
            .expect("first pass")
            .accepted,
        1
    );
    let old_digest = audit(&fixture.store)["evidence_digest"].clone();
    let mut schools = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("school evidence");
    schools[0].athletics_website = Some("https://new-source.example/school".to_string());
    fixture
        .store
        .append_many(Table::Schools, &schools)
        .expect("changed evidence");
    let unchanged = row(&fixture.store);
    let untouched = run_lanes(&fixture.store, &clients, &options(), "preserve")
        .await
        .expect("resolved history");
    assert_eq!(untouched.requested, 0);
    assert_eq!(row(&fixture.store), unchanged);
    let mut reopened = case.clone();
    reopened.state = ReviewState::Pending;
    fixture
        .store
        .replace_many(Table::ReviewCases, &[reopened])
        .expect("operator reopened case");
    let report = run_lanes(&fixture.store, &clients, &options(), "second")
        .await
        .expect("reopened review");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.requested, 1);
    assert_eq!(report.accepted, 0);
    assert_ne!(audit(&fixture.store)["evidence_digest"], old_digest);
    assert_eq!(audit(&fixture.store)["outcome"], "disagreement");
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    assert_eq!(fixture.store.receipt_count().expect("receipt count"), 2);
}

#[tokio::test]
async fn evidence_changed_during_advice_prevents_acceptance_of_the_old_snapshot() {
    let (fixture, case) = Fixture::school();
    let store = fixture.store.clone();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane_with(vec![good.clone()], move |_, _| {
        let mut schools = store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("school evidence");
        schools[0].athletics_website = Some("https://new-source.example/school".to_string());
        store
            .append_many(Table::Schools, &schools)
            .expect("new source evidence");
    });
    let (second, server_b) = lane(vec![good]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "stale",
    )
    .await
    .expect("stale advice retained");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.accepted, 0);
    assert_eq!(audit(&fixture.store)["outcome"], "evidence_changed");
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    assert!(!row(&fixture.store).accepted);
    assert!(audit(&fixture.store)["packet"]["evidence"]
        .as_array()
        .expect("original packet")
        .iter()
        .any(|fact| fact["field"] == "association" && fact["value"] == "WIAA"));
}

#[tokio::test]
async fn cached_cases_do_not_consume_the_budget_for_new_ambiguous_cases() {
    let (fixture, first_case) = Fixture::school();
    let second_case = add_school(&fixture.store, "Madison East");
    let mut cases = [first_case, second_case];
    cases.sort_by(|first, second| first.id.cmp(&second.id));
    let replies = cases
        .iter()
        .map(|case| batch(case, "value_proposed", "state", "WI"))
        .collect::<Vec<_>>();
    let (first, server_a) = lane(replies.clone());
    let (second, server_b) = lane(replies);
    let clients = [client(&first), client(&second)];
    let mut options = options();
    options.limit = 1;
    let first = run_lanes(&fixture.store, &clients, &options, "first")
        .await
        .expect("first limited pass");
    let second = run_lanes(&fixture.store, &clients, &options, "second")
        .await
        .expect("second limited pass");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(first.accepted, 1);
    assert_eq!(second.requested, 1);
    assert_eq!(second.accepted, 1);
    assert!(fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("cases")
        .iter()
        .all(|case| case.state == ReviewState::Resolved));
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 2);
}

#[tokio::test]
async fn missing_subjects_are_durable_unresolved_review_not_silent_success() {
    let (fixture, case) = Fixture::school();
    let missing = ReviewCase::pending(
        "School jurisdiction unresolved",
        "missing-school",
        "Unknown",
        "missing source subject",
    );
    fixture
        .store
        .replace_many(Table::ReviewCases, &[missing])
        .expect("missing subject case");
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "missing",
    )
    .await
    .expect("review");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.requested, 2);
    assert_eq!(report.accepted, 1);
    assert_eq!(report.unanswered, 1);
    let rows = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rows");
    let missing = rows
        .iter()
        .find(|row| row.subject_id == "missing-school")
        .expect("missing finding");
    assert!(!missing.accepted);
    let audit: serde_json::Value = serde_json::from_str(&missing.rationale).expect("audit");
    assert_eq!(audit["outcome"], "missing_subject");
    assert_eq!(audit["packet"], serde_json::Value::Null);
}

#[tokio::test]
async fn a_case_superseded_during_advice_is_not_resurrected_by_the_checkpoint() {
    let (fixture, case) = Fixture::school();
    let store = fixture.store.clone();
    let mut superseded = case.clone();
    superseded.state = ReviewState::Superseded;
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane_with(vec![good.clone()], move |_, _| {
        store
            .replace_many(Table::ReviewCases, std::slice::from_ref(&superseded))
            .expect("superseded case");
    });
    let (second, server_b) = lane(vec![good]);
    let error = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options(),
        "superseded",
    )
    .await
    .expect_err("stale case checkpoint rejected");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert!(
        matches!(error, census_store::StoreError::Invariant { detail } if detail.contains("changed before checkpoint"))
    );
    assert_eq!(state(&fixture.store), ReviewState::Superseded);
    assert!(fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rows")
        .is_empty());
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
}
