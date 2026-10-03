use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::Table;

use super::support::{add_school, batch, client, lane, options, Fixture, TestResult};
use crate::run_lanes;

#[test]
fn both_advice_lanes_and_case_states_cross_the_checkpoint_boundary_together() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, first) = Fixture::school()?;
            let mut cases = vec![first];
            for index in 1..=crate::CHECKPOINT_CASES {
                cases.push(add_school(&fixture.store, &format!("School {index}"))?);
            }
            cases.sort_by(|first, second| first.id.cmp(&second.id));
            let replies = cases
                .iter()
                .map(|case| batch(case, "value_proposed", "state", "WI"))
                .collect::<Vec<_>>();
            let (first, server_a) = lane(replies.clone())?;
            let (second, server_b) = lane(replies)?;
            let report = run_lanes(
                &fixture.store,
                &[client(&first)?, client(&second)?],
                &options(),
                "checkpoint",
            )
            .await?;
            server_a.join().map_err(|_| "first lane panicked")??;
            server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; report.requested, 257);
            check!(eq; report.accepted, 257);
            check!(eq; fixture.store.receipt_count()?, 2);
            let rows = fixture
                .store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            let states = fixture.store.scan::<ReviewCase>(Table::ReviewCases)?;
            check!(eq; rows.len(), 257);
            check!(eq; states.len(), 257);
            for case in states {
                check!(eq; case.state, ReviewState::Resolved);
                let row = rows
                    .iter()
                    .find(|row| row.case_id == case.id)
                    .ok_or("atomically bound consensus")?;
                check!(row.accepted);
                let audit: serde_json::Value = serde_json::from_str(&row.rationale)?;
                check!(eq; audit["lanes"][0]["batch"]["verdicts"][0]["case_id"],
    case.id);
                check!(eq; audit["lanes"][1]["batch"]["verdicts"][0]["case_id"],
    case.id);
            }
            Ok(())
        })
}
