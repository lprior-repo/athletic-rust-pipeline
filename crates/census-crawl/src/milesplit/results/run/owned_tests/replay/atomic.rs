use super::*;

#[test]
fn recorded_collect_entities_and_v3_receipt_become_visible_only_at_one_batch_commit() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let report = crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; (report.rows, report.errors), (3, 0));
            let recorded = recording.drain();
            assert_absent(&store)?;
            let mut staged = store.write_batch();
            for rows in &recorded.rows {
                staged.append_many(rows.table, &rows.rows)?;
            }
            for receipt in &recorded.journal {
                staged.journal_done(&receipt.phase, &receipt.key, &receipt.payload)?;
            }
            assert_absent(&store)?;
            drop(staged);
            assert_absent(&store)?;
            apply(&store, &recorded)?;
            assert_projected(&store)?;
            let before = entities(&store)?;
            let physical_before = physical(&store)?;
            let receipts = store.journal_payloads(RESULT_SET_PHASE)?;
            let replay = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (replay.rows, replay.errors, replay.requests), (3, 0, 0));
            check!(eq; entities(&store)?, before);
            check!(eq; physical(&store)?, physical_before);
            check!(eq; store.journal_payloads(RESULT_SET_PHASE)?, receipts);
            Ok(())
        })
}
