use std::time::Duration;

use census_domain::model::{ReviewCase, ReviewState};
use census_store::Table;

use super::support::{audit, batch, client, lane, options, row, state, Fixture, TestResult};
use crate::{run_lanes, ModelClient, ModelOptions};

#[test]
fn retrying_a_failed_lane_reuses_the_other_lanes_exact_bound_advice() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec!["not JSON".to_string(), good])?;
            let clients = [client(&first)?, client(&second)?];
            let initial = run_lanes(&fixture.store, &clients, &options(), "first").await?;
            server_a
                .join()
                .map_err(|_| "successful endpoint panicked")??;
            let retained = audit(&fixture.store)?["lanes"][0].clone();
            check!(eq; initial.accepted, 0);
            check!(eq; initial.failed, 1);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            let resumed = run_lanes(&fixture.store, &clients, &options(), "resumed").await?;
            check!(eq; server_b.join().map_err(|_| "retried lane panicked")??.len(), 2);
            check!(eq; resumed.requested, 1);
            check!(eq; resumed.failed, 0);
            check!(eq; resumed.accepted, 1);
            check!(eq; audit(&fixture.store)?["lanes"][0], retained);
            check!(eq; row(&fixture.store)?.value, "WI");
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            check!(eq; fixture.store.receipt_count()?, 2);
            Ok(())
        })
}

#[test]
fn only_the_changed_model_or_request_lane_is_reasked_after_explicit_reopening() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { for changed in ["model", "request"] {
    let (fixture, mut case) = Fixture::school()?;
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = lane(vec![good.clone()])?;
    let (second, server_b) = lane(vec![good, batch(&case, "value_proposed", "state", "MN")])?;
    let clients = [client(&first)?, client(&second)?];
    check!(eq; run_lanes(&fixture.store, &clients, &options(), "first").await?.accepted, 1);
    server_a.join().map_err(|_| "unchanged endpoint panicked")??;
    let original = audit(&fixture.store)?["lanes"][0].clone();
    case.state = ReviewState::Pending;
    fixture.store.replace_many(Table::ReviewCases, &[case])?;
    let model = if changed == "model" {
        "new-model-revision"
    } else {
        "same-model-name"
    };
    let model_options = ModelOptions::local(&second, model)?.with_timeout(Duration::from_secs(3));
    let model_options = if changed == "request" {
        model_options.with_max_tokens(1024)
    } else {
        model_options
    };
    let clients = [client(&first)?, ModelClient::new(model_options)?];
    let report = run_lanes(&fixture.store, &clients, &options(), "changed").await?;
    let requests = server_b.join().map_err(|_| "changed lane panicked")??;
    check!(eq; requests.len(), 2);
    check!(eq; report.failed, 0);
    check!(eq; report.accepted, 0);
    check!(eq; audit(&fixture.store)?["lanes"][0], original);
    if changed == "model" {
        check!(eq; requests[0]["model"], "same-model-name");
        check!(eq; requests[1]["model"], "new-model-revision");
    } else {
        check!(eq; requests[0]["max_tokens"], 1536);
        check!(eq; requests[1]["max_tokens"], 1024);
    }
    check!(eq; audit(&fixture.store)?["outcome"], "disagreement");
    check!(eq; state(&fixture.store)?, ReviewState::Retained);
}
Ok(()) })
}

#[test]
fn reordered_lanes_reuse_unresolved_advice_without_network_or_another_checkpoint() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let (first, server_a) = lane(vec![batch(&case, "value_proposed", "state", "WI")])?;
            let (second, server_b) = lane(vec![batch(&case, "insufficient_evidence", "", "")])?;
            let clients = [client(&first)?, client(&second)?];
            let initial = run_lanes(&fixture.store, &clients, &options(), "initial").await?;
            server_a.join().map_err(|_| "first endpoint panicked")??;
            server_b.join().map_err(|_| "second endpoint panicked")??;
            check!(eq; initial.accepted, 0);
            let original = row(&fixture.store)?;
            let clients = [client(&second)?, client(&first)?];
            let replay = run_lanes(&fixture.store, &clients, &options(), "reordered").await?;
            check!(eq; replay.requested, 0);
            check!(eq; replay.failed, 0);
            check!(eq; row(&fixture.store)?, original);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}

#[test]
fn malformed_advice_is_never_reused_even_when_its_stored_dropped_count_is_forged() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let mut bad: serde_json::Value = serde_json::from_str(&good)?;
            bad["verdicts"][0]["case_id"] = "ghost-case".into();
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![bad.to_string(), good])?;
            let clients = [client(&first)?, client(&second)?];
            let initial = run_lanes(&fixture.store, &clients, &options(), "initial").await?;
            server_a.join().map_err(|_| "valid endpoint panicked")??;
            check!(eq; initial.accepted, 0);
            let mut recorded = row(&fixture.store)?;
            let mut retained: serde_json::Value = serde_json::from_str(&recorded.rationale)?;
            retained["lanes"][1]["dropped"] = 0.into();
            recorded.rationale = retained.to_string();
            fixture
                .store
                .replace_many(Table::IdentityVerdicts, &[recorded])?;
            let resumed = run_lanes(&fixture.store, &clients, &options(), "retry").await?;
            check!(eq; server_b.join().map_err(|_| "ghost lane panicked")??.len(), 2);
            check!(eq; resumed.failed, 0);
            check!(eq; resumed.accepted, 1);
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            Ok(())
        })
}

#[test]
fn superseded_cases_and_legacy_resolved_cases_do_not_consume_new_review_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, mut case) = Fixture::school()?;
            case.state = ReviewState::Superseded;
            fixture.store.replace_many(Table::ReviewCases, &[case])?;
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
                .replace_many(Table::ReviewCases, &[resolved.clone()])?;
            let report = run_lanes(
                &fixture.store,
                &[
                    client("http://127.0.0.1:18081")?,
                    client("http://127.0.0.1:18082")?,
                ],
                &options(),
                "preserved",
            )
            .await?;
            check!(eq; report.requested, 0);
            check!(eq; fixture.store.scan::<ReviewCase>(Table::ReviewCases)?
    .into_iter()
    .find(|case| case.id == resolved.id)
    .ok_or("historical case")?,
resolved);
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}

#[test]
fn explicitly_reopened_cases_finalize_from_exact_advice_without_repeating_model_calls() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { let (fixture, mut case) = Fixture::school()?;
let good = batch(&case, "value_proposed", "state", "WI");
let (first, server_a) = lane(vec![good.clone()])?;
let (second, server_b) = lane(vec![good])?;
let clients = [client(&first)?, client(&second)?];
check!(eq; run_lanes(&fixture.store, &clients, &options(), "initial").await?.accepted, 1);
server_a.join().map_err(|_| "first endpoint panicked")??;
server_b.join().map_err(|_| "second endpoint panicked")??;
let original = audit(&fixture.store)?;
case.state = ReviewState::Pending;
fixture.store.replace_many(Table::ReviewCases, &[case])?;
let replay = run_lanes(&fixture.store, &clients, &options(), "reopened").await?;
check!(eq; replay.requested, 1);
check!(eq; replay.accepted, 1);
check!(eq; replay.failed, 0);
check!(eq; audit(&fixture.store)?, original);
check!(eq; state(&fixture.store)?, ReviewState::Resolved);
check!(eq; fixture.store.receipt_count()?, 2);
Ok(()) })
}
