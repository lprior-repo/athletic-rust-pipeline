use super::*;

#[path = "order_replay.rs"]
mod order_replay;

#[test]
fn completed_result_set_rebuilds_beside_new_set_without_reappending_its_marks_or_observations(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let second =
                ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
                    .ok_or("second source set")?;
            let mut controlled = serde_json::from_slice::<serde_json::Value>(TROY)?;
            controlled["data"][1]["meetResultsId"] = json!("1266815");
            seed_owned(&fetcher, &first, &serde_json::to_vec(&controlled)?)?;
            seed_metadata(&fetcher, &first)?;
            seed_metadata(&fetcher, &second)?;
            let initial = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&first),
            )
            .await?;
            check!(eq; (initial.rows, initial.errors), (2, 0));
            let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let mut both = options(&first);
            both.urls.extend(options(&second).urls);
            let resumed =
                crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &both).await?;
            check!(eq; (resumed.rows, resumed.errors, resumed.requests), (3, 0, 0));
            let after: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                after
                    .iter()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782277",
                    "milesplit_result:201782806"
                ])
            );
            for mark in before {
                check!(eq;
                    after.iter().find(|row| row.source_key == mark.source_key),
                    Some(&mark)
                );
            }
            for table in [Table::Performances, Table::SourceObservations] {
                let physical = store.walk_table(table)?;
                check!(eq; physical.rows, 3, "{table:?}");
                if table != Table::SourceObservations {
                    check!(eq; physical.repeated_ids, 0, "{table:?}");
                }
            }
            let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            let completed: std::collections::BTreeSet<_> = receipts
                .iter()
                .filter(|row| row["disposition"] == "projection_applied")
                .map(|row| {
                    Ok((
                        row["meet"].as_str().ok_or("meet")?,
                        row["rsid"].as_str().ok_or("result set")?,
                    ))
                })
                .collect::<TestResult<_>>()?;
            check!(eq;
                completed,
                std::collections::BTreeSet::from([("725218", "1266814"), ("725218", "1266815"),])
            );
            Ok(())
        })
}
