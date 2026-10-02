mod aggregate_output;
mod cancellation;
mod checkpoints;
mod format_change;
mod lanes;
mod reopened_receipt;
mod replay;
mod resources;
mod reuse;
pub(crate) mod support;

use census_domain::model::{CanonicalSchool, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use crate::run_lanes;
use support::{add_school, audit, batch, client, lane, options, row, state, Fixture};

#[tokio::test]
async fn every_case_requires_both_independent_lanes_and_retains_both_advice_packets() {
    let (fixture, first) = Fixture::school();
    let second = add_school(&fixture.store, "Madison East");
    let mut cases = vec![first.clone(), second.clone()];
    cases.sort_by(|first, second| first.id.cmp(&second.id));
    let replies = cases
        .iter()
        .map(|case| batch(case, "value_proposed", "state", "WI"))
        .collect::<Vec<_>>();
    let (endpoint_a, server_a) = lane(replies.clone());
    let (endpoint_b, server_b) = lane(replies);
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint_a), client(&endpoint_b)],
        &options(),
        "dual",
    )
    .await
    .expect("dual review");
    let requests_a = server_a.join().expect("first lane");
    let requests_b = server_b.join().expect("second lane");
    assert_eq!(report.requested, 2);
    assert_eq!(report.accepted, 2);
    let rows = fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("consensus rows");
    for row in rows {
        assert!(row.accepted);
        assert_eq!(row.value, "WI");
        let audit: serde_json::Value = serde_json::from_str(&row.rationale).expect("audit");
        assert_eq!(audit["outcome"], "agreement");
        assert_eq!(audit["packet"]["cases"][0]["case_id"], row.case_id);
        assert_eq!(
            audit["lanes"][0]["batch"]["verdicts"][0]["case_id"],
            row.case_id
        );
        assert_eq!(
            audit["lanes"][1]["batch"]["verdicts"][0]["case_id"],
            row.case_id
        );
        assert_ne!(audit["lanes"][0]["endpoint"], audit["lanes"][1]["endpoint"]);
        assert_eq!(audit["lanes"][0]["model"], audit["lanes"][1]["model"]);
    }
    for (first, second) in requests_a.iter().zip(requests_b.iter()) {
        assert_eq!(first["messages"], second["messages"]);
    }
    let schools = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("original schools");
    assert!(schools.iter().all(|school| school.state.is_none()));
    assert_eq!(fixture.store.receipt_count().expect("receipt"), 1);
}

#[tokio::test]
async fn disagreement_insufficient_missing_and_malformed_advice_never_resolve_a_case() {
    for scenario in [
        "disagreement",
        "insufficient_evidence",
        "missing",
        "malformed",
        "wrong_subject",
        "duplicate",
        "invalid_value",
        "ghost_case",
        "confidence_out_of_range",
        "confidence_overflow",
        "unsupported_kind",
        "missing_value",
        "wrong_field",
    ] {
        let (fixture, case) = Fixture::school();
        let good = batch(&case, "value_proposed", "state", "WI");
        let bad = match scenario {
            "disagreement" => batch(&case, "value_proposed", "state", "MN"),
            "insufficient_evidence" => batch(&case, "insufficient_evidence", "", ""),
            "missing" => {
                serde_json::json!({"subject_id": case.subject_id, "verdicts": []}).to_string()
            }
            "malformed" => "not JSON".to_string(),
            "invalid_value" => batch(&case, "value_proposed", "state", "imaginary"),
            "wrong_field" => batch(&case, "value_proposed", "identity", "same_person"),
            "missing_value" => batch(&case, "value_proposed", "state", ""),
            "ghost_case"
            | "confidence_out_of_range"
            | "confidence_overflow"
            | "unsupported_kind" => {
                let mut reply: serde_json::Value = serde_json::from_str(&good).expect("batch");
                match scenario {
                    "ghost_case" => reply["verdicts"][0]["case_id"] = "unasked-case".into(),
                    "confidence_out_of_range" => reply["verdicts"][0]["confidence"] = 101.into(),
                    "confidence_overflow" => reply["verdicts"][0]["confidence"] = 256.into(),
                    "unsupported_kind" => reply["verdicts"][0]["kind"] = "same_person".into(),
                    _ => unreachable!(),
                }
                reply.to_string()
            }
            "wrong_subject" => {
                let mut reply: serde_json::Value = serde_json::from_str(&good).expect("batch");
                reply["subject_id"] = "another-school".into();
                reply.to_string()
            }
            "duplicate" => {
                let mut reply: serde_json::Value = serde_json::from_str(&good).expect("batch");
                let duplicate = reply["verdicts"][0].clone();
                reply["verdicts"]
                    .as_array_mut()
                    .expect("verdicts")
                    .push(duplicate);
                reply.to_string()
            }
            _ => unreachable!(),
        };
        let (endpoint_a, server_a) = lane(vec![good]);
        let (endpoint_b, server_b) = lane(vec![bad]);
        let report = run_lanes(
            &fixture.store,
            &[client(&endpoint_a), client(&endpoint_b)],
            &options(),
            scenario,
        )
        .await
        .expect("retained review");
        server_a.join().expect("first lane");
        server_b.join().expect("second lane");
        assert_eq!(report.accepted, 0, "{scenario}");
        assert_eq!(report.resolved(), 0, "{scenario}");
        assert_eq!(state(&fixture.store), ReviewState::Retained);
        assert!(!row(&fixture.store).accepted);
        assert_eq!(row(&fixture.store).kind, "insufficient_evidence");
        assert_eq!(
            audit(&fixture.store)["lanes"][0]["batch"]["verdicts"][0]["value"],
            "WI"
        );
        assert_eq!(fixture.store.receipt_count().expect("receipt"), 1);
    }
}

#[tokio::test]
async fn one_failed_lane_retains_the_successful_advice_and_the_failure_without_acceptance() {
    let (fixture, case) = Fixture::school();
    let (endpoint, server) = lane(vec![batch(&case, "value_proposed", "state", "WI")]);
    let dead = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("unused endpoint");
        format!("http://{}", listener.local_addr().expect("address"))
    };
    let report = run_lanes(
        &fixture.store,
        &[client(&endpoint), client(&dead)],
        &options(),
        "failure",
    )
    .await
    .expect("failure retained");
    server.join().expect("successful lane");
    assert_eq!(report.requested, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.unanswered, 1);
    assert_eq!(report.accepted, 0);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    let audit = audit(&fixture.store);
    assert_eq!(audit["outcome"], "lane_failed");
    assert_eq!(audit["lanes"][0]["status"], "answered");
    assert_eq!(audit["lanes"][0]["batch"]["verdicts"][0]["value"], "WI");
    assert_eq!(audit["lanes"][1]["status"], "failed");
}

#[tokio::test]
async fn dual_dry_run_reports_consensus_without_persisting_any_advice_or_state() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "Wisconsin");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let mut options = options();
    options.dry_run = true;
    let report = run_lanes(
        &fixture.store,
        &[client(&first), client(&second)],
        &options,
        "dry",
    )
    .await
    .expect("dry run");
    server_a.join().expect("first lane");
    server_b.join().expect("second lane");
    assert_eq!(report.accepted, 1);
    assert_eq!(state(&fixture.store), ReviewState::Pending);
    assert!(fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("rows")
        .is_empty());
    assert_eq!(fixture.store.receipt_count().expect("receipt"), 0);
}

#[tokio::test]
async fn oversized_evidence_is_refused_without_truncation_or_partial_checkpoint_publication() {
    let (fixture, _) = Fixture::school();
    let original = "x".repeat(4_194_305);
    let mut schools = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("school evidence");
    schools[0].athletics_website = Some(original.clone());
    fixture
        .store
        .append_many(Table::Schools, &schools)
        .expect("oversized original evidence");
    let error = run_lanes(
        &fixture.store,
        &[
            client("http://127.0.0.1:18081"),
            client("http://127.0.0.1:18082"),
        ],
        &options(),
        "oversized",
    )
    .await
    .expect_err("bounded audit failure must remain explicit");
    assert!(matches!(error, StoreError::Invariant { .. }));
    assert_eq!(state(&fixture.store), ReviewState::Pending);
    assert!(fixture
        .store
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)
        .expect("verdicts")
        .is_empty());
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
    assert_eq!(
        fixture
            .store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("original schools")[0]
            .athletics_website
            .as_deref(),
        Some(original.as_str())
    );
}
