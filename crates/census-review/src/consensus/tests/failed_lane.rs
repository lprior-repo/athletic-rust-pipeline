use census_domain::model::{ReviewCase, ReviewState};
use serde_json::Value;

use super::support::{audit, batch, client, options, state, Fixture, TestResult};
use crate::run_lanes;

#[path = "failed_lane_fixture.rs"]
mod fixture;

use fixture::{bind, run, Reply};

#[test]
fn one_failed_lane_retains_the_successful_advice_and_the_failure_without_acceptance() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, first_address, first_ready) = bind(Reply::Verdict(good)).await?;
            let (second, second_address, second_ready) = bind(Reply::CloseAfterRequest).await?;
            check!(ne; first_address.port(), second_address.port());
            let clients = [
                client(&format!("http://{first_address}"))?,
                client(&format!("http://{second_address}"))?,
            ];
            let (report, [first_requests, second_requests]) = run([first, second], async {
                futures::try_join!(first_ready, second_ready)?;
                Ok(run_lanes(&fixture.store, &clients, &options(), "failure").await?)
            })
            .await?;
            check!(eq; first_requests.len(), 1);
            check!(eq; second_requests.len(), 1);
            let first_request = first_requests
                .first()
                .ok_or("first complete request receipt")?;
            let second_request = second_requests
                .first()
                .ok_or("second complete request receipt")?;
            check!(eq; first_request["model"], "same-model-name");
            check!(eq; second_request["model"], "same-model-name");
            check!(eq; first_request["messages"], second_request["messages"]);
            assert_request(first_request, &case)?;
            assert_request(second_request, &case)?;
            check!(eq; report.requested, 1);
            check!(eq; report.failed, 1);
            check!(eq; report.unanswered, 1);
            check!(eq; report.accepted, 0);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            let audit = audit(&fixture.store)?;
            check!(eq; audit["outcome"], "lane_failed");
            check!(eq; audit["lanes"][0]["status"], "answered");
            check!(eq; audit["lanes"][0]["batch"]["verdicts"][0]["value"], "WI");
            check!(eq; audit["lanes"][1]["status"], "failed");
            check!(eq; audit["lanes"][1]["error"], "model request failed");
            Ok(())
        })
}

fn assert_request(request: &Value, case: &ReviewCase) -> TestResult {
    let messages = request["messages"].as_array().ok_or("request messages")?;
    let user = messages
        .iter()
        .find(|message| message["role"] == "user")
        .and_then(|message| message["content"].as_str())
        .ok_or("request user packet")?;
    let subjects = user
        .lines()
        .filter_map(|line| line.strip_prefix("subject_id: "))
        .collect::<Vec<_>>();
    let cases = user
        .lines()
        .filter_map(|line| line.strip_prefix("- case_id: "))
        .collect::<Vec<_>>();
    check!(eq; subjects, vec![case.subject_id.as_str()]);
    check!(eq; cases, vec![case.id.as_str()]);
    Ok(())
}

#[test]
fn early_operation_error_cancels_both_pending_accepts_and_releases_bound_listeners() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (first, first_address, first_ready) = bind(Reply::Verdict(String::new())).await?;
            let (second, second_address, second_ready) = bind(Reply::CloseAfterRequest).await?;
            let result = run([first, second], async {
                futures::try_join!(first_ready, second_ready)?;
                Err::<(), _>("intentional operation error".into())
            })
            .await;
            let error = match result {
                Err(error) => error,
                Ok(_) => return Err("the early operation error was lost".into()),
            };
            check!(eq; error.to_string(), "intentional operation error");
            let first_rebound = std::net::TcpListener::bind(first_address)?;
            let second_rebound = std::net::TcpListener::bind(second_address)?;
            check!(eq; first_rebound.local_addr()?, first_address);
            check!(eq; second_rebound.local_addr()?, second_address);
            Ok(())
        })
}
