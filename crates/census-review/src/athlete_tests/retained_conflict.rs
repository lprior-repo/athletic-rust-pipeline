use census_domain::model::ReviewState;

use super::binding_support::{assert_preserved, canonical_side, rows, Change, Fixture};
use super::options;
use crate::consensus::tests::support::{audit, batch, client, lane, row, state, TestResult};
use crate::run_lanes;

#[test]
fn a_retained_source_conflict_refuses_same_person_but_preserves_different_person() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (answer, accepted, outcome, expected_state) in [
                (
                    "same_person",
                    0,
                    "hard_contradiction",
                    ReviewState::Retained,
                ),
                ("different_person", 1, "agreement", ReviewState::Resolved),
            ] {
                let mut source_rows = rows()?;
                Change::RetainedConflict.apply(&mut source_rows[0])?;
                let fixture = Fixture::new(&source_rows)?;
                let reply = batch(&fixture.case, "value_proposed", "identity", answer);
                let (first, server_a) = lane(vec![reply.clone()])?;
                let (second, server_b) = lane(vec![reply])?;
                let report = run_lanes(
                    &fixture.store,
                    &[client(&first)?, client(&second)?],
                    &options(),
                    answer,
                )
                .await?;
                check!(eq; report.accepted, accepted);
                check!(eq; report.rejected, 1 - accepted);
                check!(eq; audit(&fixture.store)?["outcome"], outcome);
                check!(eq; state(&fixture.store)?, expected_state);
                let verdict = row(&fixture.store)?;
                check!(eq; verdict.accepted, accepted == 1);
                check!(eq; verdict.case_id, fixture.case.id);
                check!(eq; verdict.subject_id, fixture.case.subject_id);
                check!(eq; verdict.member_ids, fixture.case.member_ids);
                let retained = audit(&fixture.store)?;
                for advice in retained["lanes"].as_array().ok_or("both advice lanes")? {
                    check!(eq; advice["batch"]["verdicts"][0]["value"], answer);
                }
                for requests in [
                    server_a.join().map_err(|_| "first lane panicked")??,
                    server_b.join().map_err(|_| "second lane panicked")??,
                ] {
                    check!(eq; canonical_side(&requests[0], "side_a")?, source_rows[0]);
                    check!(eq; canonical_side(&requests[0], "side_b")?, source_rows[1]);
                    let content = requests[0]["messages"][1]["content"]
                        .as_str()
                        .ok_or("attributed packet")?;
                    check!(content.contains("retained_source_conflict:"));
                    check!(content.contains("https://source.example/conflict"));
                }
                assert_preserved(&fixture.store, &source_rows)?;
            }
            Ok(())
        })
}
