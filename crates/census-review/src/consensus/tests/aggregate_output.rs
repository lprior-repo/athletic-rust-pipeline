use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{StoreError, Table};

use super::support::{add_school, batch, client, lane, options, Fixture, TestResult};
use crate::run_lanes;

#[test]
fn aggregate_joined_advice_refuses_before_any_checkpoint_state_is_published() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, first) = Fixture::school()?;
            let mut cases = vec![first];
            for index in 1..8 {
                cases.push(add_school(
                    &fixture.store,
                    &format!("Madison Aggregate {index}"),
                )?);
            }
            cases.sort_by(|a, b| a.id.cmp(&b.id));
            let replies: Vec<_> = cases
                .iter()
                .map(|case| -> TestResult<String> {
                    let mut reply: serde_json::Value =
                        serde_json::from_str(&batch(case, "value_proposed", "state", "WI"))?;
                    reply["verdicts"][0]["rationale"] = "x".repeat(200 * 1024).into();
                    Ok(reply.to_string())
                })
                .collect::<TestResult<_>>()?;
            let (first, server_a) = lane(replies.clone())?;
            let (second, server_b) = lane(replies)?;
            let error = match run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "aggregate-output",
            )
            .await
            {
                Err(error) => error,
                Ok(_) => return Err("aggregate output refusal".into()),
            };
            check!(eq; server_a.join().map_err(|_| "first lane panicked")??.len(), cases.len());
            check!(eq; server_b.join().map_err(|_| "second lane panicked")??.len(), cases.len());
            check!(matches!(error, StoreError::Invariant { .. }));
            check!(eq; fixture.store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
            check!(fixture
                .store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .iter()
                .all(|case| case.state == ReviewState::Pending));
            check!(eq; fixture.store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
Vec::<ReviewVerdictRecord>::new());
            check!(eq; fixture.store.receipt_count()?, 0);
            Ok(())
        })
}
