use super::bounded_buffers::document;
use super::*;

#[test]
fn duplicate_delivery_to_one_recording_retains_each_fact_and_receipt_exactly_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(
                &fetcher,
                &reference,
                &serde_json::to_vec(&document(725218, 1, 1)?)?,
            )?;
            seed_metadata(&fetcher, &reference)?;
            let recording = crate::Recording::new();
            let mut ctx = context(&store, &fetcher)?;
            ctx.recording = Some(&recording);
            let first = crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; (first.rows, first.errors), (1, 0));
            let admitted = recording.usage();
            let second = crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; (second.rows, second.errors), (1, 0));
            check!(eq; recording.usage(), admitted);
            let staged = recording.drain();
            apply(&store, &staged)?;
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            check!(eq; store.walk_table(Table::SourceObservations)?.rows, 1);
            check!(eq; store.walk_table(Table::Meets)?.rows, 1);
            Ok(())
        })
}

#[test]
fn newly_resolved_school_changes_projection_without_repeating_pending_source_observations(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup_bare()?;
            let source = document(725218, 1, 1)?;
            let team = &source["data"][0]["teamId"];
            let team = team
                .as_str()
                .map(str::to_string)
                .or_else(|| team.as_u64().map(|id| id.to_string()))
                .ok_or("provider team ID")?;
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&source)?)?;
            seed_metadata(&fetcher, &reference)?;
            let recording = crate::Recording::new();
            let mut ctx = context(&store, &fetcher)?;
            ctx.recording = Some(&recording);
            let unresolved =
                crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; unresolved.rows, 1);
            check!(unresolved.errors > 0);
            store.append(
                Table::Schools,
                &school("Newly resolved provider school", &team),
            )?;
            let resolved =
                crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; (resolved.rows, resolved.errors), (1, 0));
            apply(&store, &recording.drain())?;
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            check!(eq; store.walk_table(Table::SourceObservations)?.rows, 1);
            let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
            check!(receipts
                .iter()
                .any(|receipt| receipt["disposition"] == "partial"));
            check!(receipts
                .iter()
                .any(|receipt| receipt["disposition"] == "projection_applied"));
            Ok(())
        })
}

#[test]
fn invalid_middle_event_specification_retains_valid_neighbors_and_leaves_projection_unfinished(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut source = document(725218, 1, 3)?;
            let rows = source
                .get_mut("data")
                .and_then(serde_json::Value::as_array_mut)
                .ok_or("owned rows")?;
            rows.iter_mut()
                .zip(["Shot Put (4kg)", "Shot Put (0kg)", "Shot Put (3kg)"])
                .for_each(|(row, label)| {
                    row["eventName"] = json!(label);
                    row["eventCode"] = json!("SP");
                    row["mark"] = json!("12.34");
                });
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&source)?)?;
            seed_metadata(&fetcher, &reference)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.rows, 3);
            check!(report.errors > 0);
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 2);
            check!(eq; store.walk_table(Table::SourceObservations)?.rows, 3);
            let events: Vec<census_domain::model::CanonicalEvent> = store.scan(Table::Events)?;
            check!(eq; events.len(), 2);
            check!(performances
                .iter()
                .all(|row| events.iter().any(|event| event.id == row.event)));
            let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
            check!(receipts
                .iter()
                .any(|row| row["locator"] == "data[1]"
                    && row["disposition"] == "retained_unresolved"));
            check!(!receipts
                .iter()
                .any(|row| row["disposition"] == "projection_applied"));
            super::bounded_buffers::replay_once(&store, &fetcher, &options(&reference), 3).await?;
            Ok(())
        })
}
