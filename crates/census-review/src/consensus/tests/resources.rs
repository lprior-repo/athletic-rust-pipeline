use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{add_school, batch, client, lane, options, Fixture, TestResult};
use crate::run_lanes;

fn historical(case: &ReviewCase, bytes: usize) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        member_ids: case.member_ids.clone(),
        kind: "insufficient_evidence".to_string(),
        field: String::new(),
        value: String::new(),
        accepted: false,
        confidence: 0,
        rationale: "x".repeat(bytes),
        reviewer: "historical-model".to_string(),
        observed_at: "old".to_string(),
    }
}

#[test]
fn zero_budget_refuses_corrupt_revocation_inputs_without_model_requests() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, _) = Fixture::school()?;
            fixture.store.replace_many(
                Table::ReviewCases,
                &[serde_json::json!({"id": "poison-case", "family": 42})],
            )?;
            fixture.store.replace_many(
                Table::IdentityVerdicts,
                &[serde_json::json!({"id": "poison-advice", "accepted": "not-a-boolean"})],
            )?;
            let mut options = options();
            options.limit = 0;
            let first = std::net::TcpListener::bind("127.0.0.1:0")?;
            let second = std::net::TcpListener::bind("127.0.0.1:0")?;
            first.set_nonblocking(true)?;
            second.set_nonblocking(true)?;
            let clients = [
                client(&format!("http://{}", first.local_addr()?))?,
                client(&format!("http://{}", second.local_addr()?))?,
            ];
            let result = run_lanes(&fixture.store, &clients, &options, "zero").await;
            check!(
                matches!(result, Err(StoreError::Decode { .. })),
                "{result:?}"
            );
            check!(matches!(
                run_lanes(&fixture.store, &clients[..1], &options, "invalid").await,
                Err(StoreError::Invariant { .. })
            ));
            for listener in [first, second] {
                let error = match listener.accept() {
                    Err(error) => error,
                    Ok(_) => return Err("no HTTP at zero budget".into()),
                };
                check!(eq; error.kind(), std::io::ErrorKind::WouldBlock);
            }
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}

#[test]
fn irrelevant_large_advice_history_is_not_part_of_the_selected_checkpoint_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let mut resolved = ReviewCase::pending(
                "School jurisdiction unresolved",
                "historical",
                "Old school",
                "old case",
            );
            resolved.state = ReviewState::Resolved;
            let historical = historical(&resolved, 9 * 1024 * 1024);
            fixture
                .store
                .replace_many(Table::ReviewCases, &[resolved])?;
            fixture
                .store
                .replace_many(Table::IdentityVerdicts, std::slice::from_ref(&historical))?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![good.clone()])?;
            let (second, server_b) = lane(vec![good])?;
            let mut options = options();
            options.limit = 1;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options,
                "selected",
            )
            .await?;
            check!(eq; server_a.join().map_err(|_| "first lane panicked")??.len(), 1);
            check!(eq; server_b.join().map_err(|_| "second lane panicked")??.len(), 1);
            check!(eq; (report.requested, report.accepted, report.failed), (1, 1, 0));
            let rows = fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            check!(eq; rows.iter().find(|row| row.id == historical.id), Some(&historical));
            let cases = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
            check!(eq; cases.iter().find(|row| row.id == case.id).ok_or("selected")?.state,
ReviewState::Resolved);
            check!(eq; fixture.store.receipt_count()?, 1);
            Ok(())
        })
}

#[test]
fn aggregate_selected_advice_over_eight_mib_refuses_without_partial_checkpoint() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, first) = Fixture::school()?;
            let cases = [
                first,
                add_school(&fixture.store, "Madison East")?,
                add_school(&fixture.store, "Madison North")?,
            ];
            let history: Vec<_> = cases
                .iter()
                .map(|case| historical(case, 3 * 1024 * 1024))
                .collect();
            fixture
                .store
                .replace_many(Table::IdentityVerdicts, &history)?;
            let mut options = options();
            options.limit = 3;
            let error = match run_lanes(
                &fixture.store,
                &[
                    client("http://127.0.0.1:18081")?,
                    client("http://127.0.0.1:18082")?,
                ],
                &options,
                "bound",
            )
            .await
            {
                Err(error) => error,
                Ok(_) => return Err("aggregate budget refusal".into()),
            };
            match error {
                StoreError::Invariant { detail } => check!(
                    detail.starts_with("review checkpoint exceeds aggregate byte budget 8388608:"),
                    "{detail}"
                ),
                other => return Err(format!("unexpected error: {other}").into()),
            }
            check!(eq; fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, {
                let mut history = history;
                history.sort_by(|a, b| a.id.cmp(&b.id));
                history
            });
            check!(fixture
                .store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .iter()
                .all(|case| case.state == ReviewState::Pending));
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}
