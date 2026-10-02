use std::time::Duration;

use census_domain::model::{ReviewCase, ReviewState};
use census_store::Table;

use super::support::{audit, batch, client, lane, options, row, state, Fixture};
use crate::{run_lanes, ModelClient, ModelOptions};

#[tokio::test]
async fn retrying_a_failed_lane_reuses_the_other_lanes_exact_bound_advice() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec!["not JSON".to_string(), good]);
    let clients = [client(&first), client(&second)];
    let initial = run_lanes(&fixture.store, &clients, &options(), "first")
        .await
        .expect("failed advice retained");
    server_a.join().expect("successful endpoint closes");
    let retained = audit(&fixture.store)["lanes"][0].clone();
    assert_eq!(initial.accepted, 0);
    assert_eq!(initial.failed, 1);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    let resumed = run_lanes(&fixture.store, &clients, &options(), "resumed")
        .await
        .expect("retry unresolved lane");
    assert_eq!(server_b.join().expect("only failed lane retried").len(), 2);
    assert_eq!(resumed.requested, 1);
    assert_eq!(resumed.failed, 0);
    assert_eq!(resumed.accepted, 1);
    assert_eq!(audit(&fixture.store)["lanes"][0], retained);
    assert_eq!(row(&fixture.store).value, "WI");
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 2);
}

#[tokio::test]
async fn only_the_changed_model_or_request_lane_is_reasked_after_explicit_reopening() {
    for changed in ["model", "request"] {
        let (fixture, mut case) = Fixture::school();
        let good = batch(&case, "value_proposed", "state", "WI");
        let (first, server_a) = lane(vec![good.clone()]);
        let (second, server_b) = lane(vec![good, batch(&case, "value_proposed", "state", "MN")]);
        let clients = [client(&first), client(&second)];
        assert_eq!(
            run_lanes(&fixture.store, &clients, &options(), "first")
                .await
                .expect("first consensus")
                .accepted,
            1
        );
        server_a.join().expect("unchanged endpoint closes");
        let original = audit(&fixture.store)["lanes"][0].clone();
        case.state = ReviewState::Pending;
        fixture
            .store
            .replace_many(Table::ReviewCases, &[case])
            .expect("operator reopened");
        let model = if changed == "model" {
            "new-model-revision"
        } else {
            "same-model-name"
        };
        let model_options = ModelOptions::local(&second, model)
            .expect("local endpoint")
            .with_timeout(Duration::from_secs(3));
        let model_options = if changed == "request" {
            model_options.with_max_tokens(1024)
        } else {
            model_options
        };
        let clients = [
            client(&first),
            ModelClient::new(model_options).expect("changed request client"),
        ];
        let report = run_lanes(&fixture.store, &clients, &options(), "changed")
            .await
            .expect("reopened review");
        let requests = server_b
            .join()
            .expect("changed lane receives both requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(report.failed, 0);
        assert_eq!(report.accepted, 0);
        assert_eq!(audit(&fixture.store)["lanes"][0], original);
        if changed == "model" {
            assert_eq!(requests[0]["model"], "same-model-name");
            assert_eq!(requests[1]["model"], "new-model-revision");
        } else {
            assert_eq!(requests[0]["max_tokens"], 1536);
            assert_eq!(requests[1]["max_tokens"], 1024);
        }
        assert_eq!(audit(&fixture.store)["outcome"], "disagreement");
        assert_eq!(state(&fixture.store), ReviewState::Retained);
    }
}

#[tokio::test]
async fn reordered_lanes_reuse_unresolved_advice_without_network_or_another_checkpoint() {
    let (fixture, case) = Fixture::school();
    let (first, server_a) = lane(vec![batch(&case, "value_proposed", "state", "WI")]);
    let (second, server_b) = lane(vec![batch(&case, "insufficient_evidence", "", "")]);
    let clients = [client(&first), client(&second)];
    let initial = run_lanes(&fixture.store, &clients, &options(), "initial")
        .await
        .expect("insufficient advice retained");
    server_a.join().expect("first endpoint closes");
    server_b.join().expect("second endpoint closes");
    assert_eq!(initial.accepted, 0);
    let original = row(&fixture.store);
    let clients = [client(&second), client(&first)];
    let replay = run_lanes(&fixture.store, &clients, &options(), "reordered")
        .await
        .expect("bound lane replay");
    assert_eq!(replay.requested, 0);
    assert_eq!(replay.failed, 0);
    assert_eq!(row(&fixture.store), original);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 1);
}

#[tokio::test]
async fn malformed_advice_is_never_reused_even_when_its_stored_dropped_count_is_forged() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let mut bad: serde_json::Value = serde_json::from_str(&good).expect("batch");
    bad["verdicts"][0]["case_id"] = "ghost-case".into();
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![bad.to_string(), good]);
    let clients = [client(&first), client(&second)];
    let initial = run_lanes(&fixture.store, &clients, &options(), "initial")
        .await
        .expect("ghost advice retained");
    server_a.join().expect("valid endpoint closes");
    assert_eq!(initial.accepted, 0);
    let mut recorded = row(&fixture.store);
    let mut retained: serde_json::Value = serde_json::from_str(&recorded.rationale).expect("audit");
    retained["lanes"][1]["dropped"] = 0.into();
    recorded.rationale = retained.to_string();
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &[recorded])
        .expect("forged dropped count");
    let resumed = run_lanes(&fixture.store, &clients, &options(), "retry")
        .await
        .expect("malformed lane reasked");
    assert_eq!(server_b.join().expect("ghost lane retried").len(), 2);
    assert_eq!(resumed.failed, 0);
    assert_eq!(resumed.accepted, 1);
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
}

#[tokio::test]
async fn superseded_cases_and_legacy_resolved_cases_do_not_consume_new_review_budget() {
    let (fixture, mut case) = Fixture::school();
    case.state = ReviewState::Superseded;
    fixture
        .store
        .replace_many(Table::ReviewCases, &[case])
        .expect("superseded case");
    let resolved = ReviewCase::pending(
        "School jurisdiction unresolved",
        "historical",
        "Historical",
        "resolved by prior operator",
    );
    let mut resolved = resolved;
    resolved.state = ReviewState::Resolved;
    fixture
        .store
        .replace_many(Table::ReviewCases, &[resolved.clone()])
        .expect("historical resolved state");
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
    .expect("history is not model intake");
    assert_eq!(report.requested, 0);
    assert_eq!(
        fixture
            .store
            .scan::<ReviewCase>(Table::ReviewCases)
            .expect("cases")
            .into_iter()
            .find(|case| case.id == resolved.id)
            .expect("historical case"),
        resolved
    );
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 0);
}

#[tokio::test]
async fn explicitly_reopened_cases_finalize_from_exact_advice_without_repeating_model_calls() {
    let (fixture, mut case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()]);
    let (second, server_b) = lane(vec![good]);
    let clients = [client(&first), client(&second)];
    assert_eq!(
        run_lanes(&fixture.store, &clients, &options(), "initial")
            .await
            .expect("accepted advice")
            .accepted,
        1
    );
    server_a.join().expect("first endpoint closes");
    server_b.join().expect("second endpoint closes");
    let original = audit(&fixture.store);
    case.state = ReviewState::Pending;
    fixture
        .store
        .replace_many(Table::ReviewCases, &[case])
        .expect("operator reopened case");
    let replay = run_lanes(&fixture.store, &clients, &options(), "reopened")
        .await
        .expect("cached review finalized");
    assert_eq!(replay.requested, 1);
    assert_eq!(replay.accepted, 1);
    assert_eq!(replay.failed, 0);
    assert_eq!(audit(&fixture.store), original);
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
    assert_eq!(fixture.store.receipt_count().expect("receipts"), 2);
}
