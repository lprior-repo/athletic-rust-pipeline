use super::*;

#[test]
fn reopened_case_reasks_changed_format_and_reuses_exact_unchanged_peer_advice() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = lane(vec![
                good.clone(),
                batch(&case, "value_proposed", "state", "MN"),
            ])?;
            let (second, server_b) = lane(vec![good])?;
            let original_clients = [client(&first)?, client(&second)?];
            let original =
                run_lanes(&fixture.store, &original_clients, &options(), "schema").await?;
            check!(eq; original.accepted, 1);
            let old_audit = audit(&fixture.store)?;
            let mut reopened = case.clone();
            reopened.state = ReviewState::Pending;
            fixture.store.replace(Table::ReviewCases, &reopened)?;
            let changed = crate::ModelClient::new(
                original_clients[0]
                    .options()
                    .clone()
                    .with_response_format(crate::ModelResponseFormat::PromptJson),
            )?;
            let clients = [changed, original_clients[1].clone()];
            let report = run_lanes(&fixture.store, &clients, &options(), "format-change").await?;
            let requests_a = server_a.join().map_err(|_| "first lane panicked")??;
            let requests_b = server_b.join().map_err(|_| "second lane panicked")??;
            check!(eq; requests_a.len(), 2);
            check!(eq; requests_b.len(), 1);
            check!(eq; report.requested, 1);
            check!(eq; report.failed, 0);
            check!(eq; report.accepted, 0);
            let new_audit = audit(&fixture.store)?;
            check!(eq; new_audit["packet"], old_audit["packet"]);
            check!(eq; new_audit["evidence_digest"], old_audit["evidence_digest"]);
            check!(eq; new_audit["outcome"], "disagreement");
            check!(eq; new_audit["lanes"][0]["response_format"], "prompt_json");
            check!(eq; new_audit["lanes"][1]["response_format"], "json_schema");
            check!(!row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            check!(eq; fixture.store.receipt_count()?, 3);
            Ok(())
        })
}
