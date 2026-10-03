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
mod transitions;

use census_domain::model::{CanonicalSchool, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use crate::run_lanes;
use support::{add_school, audit, batch, client, lane, options, row, state, Fixture, TestResult};

#[test]
fn every_case_requires_both_independent_lanes_and_retains_both_advice_packets() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, first) = Fixture::school()?;
            let second = add_school(&fixture.store, "Madison East")?;
            let mut cases = [first.clone(), second.clone()];
            cases.sort_by(|first, second| first.id.cmp(&second.id));
            let replies = cases
                .iter()
                .map(|case| batch(case, "value_proposed", "state", "WI"))
                .collect::<Vec<_>>();
            let (endpoint_a, server_a) = lane(replies.clone())?;
            let (endpoint_b, server_b) = lane(replies)?;
            let report = run_lanes(
                &fixture.store,
                &[client(&endpoint_a)?, client(&endpoint_b)?],
                &options(),
                "dual",
            )
            .await?;
            let requests_a = server_a.join().map_err(|_| "first lane panicked")??;
            let requests_b = server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.requested, 2);
            check!(eq; report.accepted, 2);
            let rows = fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            for row in rows {
                check!(row.accepted);
                check!(eq; row.value, "WI");
                let audit: serde_json::Value = serde_json::from_str(&row.rationale)?;
                check!(eq; audit["outcome"], "agreement");
                check!(eq; audit["packet"]["cases"][0]["case_id"], row.case_id);
                check!(eq; audit["lanes"][0]["batch"]["verdicts"][0]["case_id"],
    row.case_id);
                check!(eq; audit["lanes"][1]["batch"]["verdicts"][0]["case_id"],
    row.case_id);
                check!(ne; audit["lanes"][0]["endpoint"], audit["lanes"][1]["endpoint"]);
                check!(eq; audit["lanes"][0]["model"], audit["lanes"][1]["model"]);
            }
            for (first, second) in requests_a.iter().zip(requests_b.iter()) {
                check!(eq; first["messages"], second["messages"]);
            }
            let schools = fixture.store.scan::<CanonicalSchool>(Table::Schools)?;
            check!(schools.iter().all(|school| school.state.is_none()));
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}

#[test]
fn disagreement_insufficient_missing_and_malformed_advice_never_resolve_a_case() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
                let (fixture, case) = Fixture::school()?;
                let good = batch(&case, "value_proposed", "state", "WI");
                let bad = match scenario {
                    "disagreement" => batch(&case, "value_proposed", "state", "MN"),
                    "insufficient_evidence" => batch(&case, "insufficient_evidence", "", ""),
                    "missing" => serde_json::json!({"subject_id": case.subject_id, "verdicts": []})
                        .to_string(),
                    "malformed" => "not JSON".to_string(),
                    "invalid_value" => batch(&case, "value_proposed", "state", "imaginary"),
                    "wrong_field" => batch(&case, "value_proposed", "identity", "same_person"),
                    "missing_value" => batch(&case, "value_proposed", "state", ""),
                    "ghost_case"
                    | "confidence_out_of_range"
                    | "confidence_overflow"
                    | "unsupported_kind" => {
                        let mut reply: serde_json::Value = serde_json::from_str(&good)?;
                        match scenario {
                            "ghost_case" => reply["verdicts"][0]["case_id"] = "unasked-case".into(),
                            "confidence_out_of_range" => {
                                reply["verdicts"][0]["confidence"] = 101.into()
                            }
                            "confidence_overflow" => {
                                reply["verdicts"][0]["confidence"] = 256.into()
                            }
                            "unsupported_kind" => {
                                reply["verdicts"][0]["kind"] = "same_person".into()
                            }
                            _ => return Err("unknown verdict mutation".into()),
                        }
                        reply.to_string()
                    }
                    "wrong_subject" => {
                        let mut reply: serde_json::Value = serde_json::from_str(&good)?;
                        reply["subject_id"] = "another-school".into();
                        reply.to_string()
                    }
                    "duplicate" => {
                        let mut reply: serde_json::Value = serde_json::from_str(&good)?;
                        let duplicate = reply["verdicts"][0].clone();
                        reply["verdicts"]
                            .as_array_mut()
                            .ok_or("verdicts")?
                            .push(duplicate);
                        reply.to_string()
                    }
                    _ => return Err("unknown review scenario".into()),
                };
                let (endpoint_a, server_a) = lane(vec![good])?;
                let (endpoint_b, server_b) = lane(vec![bad])?;
                let report = run_lanes(
                    &fixture.store,
                    &[client(&endpoint_a)?, client(&endpoint_b)?],
                    &options(),
                    scenario,
                )
                .await?;
                server_a.join().map_err(|_| "first lane panicked")??;
                server_b.join().map_err(|_| "second lane panicked")??;
                check!(eq; report.accepted, 0, "{scenario}");
                check!(eq; report.resolved(), 0, "{scenario}");
                check!(eq; state(&fixture.store)?, ReviewState::Retained);
                check!(!row(&fixture.store)?.accepted);
                check!(eq; row(&fixture.store)?.kind, "insufficient_evidence");
                check!(eq; audit(&fixture.store)?["lanes"][0]["batch"]["verdicts"][0]["value"],
    "WI");
                check!(eq; fixture.store.receipt_count()?, 1);
            }
            Ok(())
        })
}

#[test]
fn one_failed_lane_retains_the_successful_advice_and_the_failure_without_acceptance() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let (endpoint, server) = lane(vec![batch(&case, "value_proposed", "state", "WI")])?;
            let dead = {
                let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
                format!("http://{}", listener.local_addr()?)
            };
            let report = run_lanes(
                &fixture.store,
                &[client(&endpoint)?, client(&dead)?],
                &options(),
                "failure",
            )
            .await?;
            server.join().map_err(|_| "successful lane panicked")??;
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
            Ok(())
        })
}

#[test]
fn dual_dry_run_reports_consensus_without_persisting_any_advice_or_state() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "Wisconsin");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let mut options = options();
            options.dry_run = true;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options,
                "dry",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.accepted, 1);
            check!(eq; state(&fixture.store)?, ReviewState::Pending);
            check!(fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?
                .is_empty());
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}

#[test]
fn oversized_evidence_is_refused_without_truncation_or_partial_checkpoint_publication() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, _) = Fixture::school()?;
            let original = "x".repeat(4_194_305);
            let mut schools = fixture.store.scan::<CanonicalSchool>(Table::Schools)?;
            schools[0].athletics_website = Some(original.clone());
            fixture.store.append_many(Table::Schools, &schools)?;
            let error = match run_lanes(
                &fixture.store,
                &[
                    client("http://127.0.0.1:18081")?,
                    client("http://127.0.0.1:18082")?,
                ],
                &options(),
                "oversized",
            )
            .await
            {
                Err(error) => error,
                Ok(_) => return Err("bounded audit failure must remain explicit".into()),
            };
            check!(matches!(error, StoreError::Invariant { .. }));
            check!(eq; state(&fixture.store)?, ReviewState::Pending);
            check!(fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?
                .is_empty());
            check!(eq; fixture.store.receipt_count()?, 0);
            check!(eq; fixture.store.scan::<CanonicalSchool>(Table::Schools)?[0]
    .athletics_website
    .as_deref(),
Some(original.as_str()));
            Ok(())
        })
}
