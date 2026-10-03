use super::*;
#[test]
fn partial_owned_capture_keeps_valid_triple_jump_and_reports_original_individual_failure(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut source: serde_json::Value = serde_json::from_slice(TROY)?;
            source["data"][0]["athleteId"] = serde_json::Value::Null;
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&source)?)?;
            seed_metadata(&fetcher, &reference)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 1);
            let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                marks
                    .iter()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782277",
                    "milesplit_result:201782806"
                ])
            );
            let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            check!(!receipts
                .iter()
                .any(|row| row["disposition"] == "projection_applied"));
            check!(receipts.iter().any(|row| row["disposition"] == "partial"
                && row["meet"] == "725218"
                && row["rsid"] == "1266814"));
            Ok(())
        })
}

#[test]
fn missing_cohort_retains_original_result_without_completing_its_projection_effect() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut source: serde_json::Value = serde_json::from_slice(TROY)?;
            source["data"][0]["gradYear"] = serde_json::Value::Null;
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&source)?)?;
            seed_metadata(&fetcher, &reference)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 1);
            let mut observations = Vec::new();
            store
                .snapshot()
                .for_each_observation::<census_domain::model::SourceObservation>(
                    Table::SourceObservations,
                    |row| {
                        observations.push(row);
                        Ok(())
                    },
                )?;
            let observation = observations
                .iter()
                .find_map(|row| match row {
                    census_domain::model::SourceObservation::Athlete(row)
                        if row.source_row_key == "milesplit_result:201782263" =>
                    {
                        Some(row)
                    }
                    _ => None,
                })
                .ok_or("original owner without inferred grade")?;
            check!(eq; observation.source_athlete_id, "14222592");
            check!(eq; observation.observed_grade, None);
            let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(!marks
                .iter()
                .any(|row| row.source_key == "milesplit_result:201782263"));
            check!(marks
                .iter()
                .any(|row| row.source_key == "milesplit_result:201782277"));
            let payloads = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            let retained = payloads
                .iter()
                .find(|row| row["result_id"] == 201782263)
                .ok_or("retained unresolved original result")?;
            check!(eq; retained["cohort"], "missing");
            check!(eq; retained["disposition"], "retained_unresolved");
            check!(!payloads
                .iter()
                .any(|row| row["disposition"] == "projection_applied"));
            Ok(())
        })
}
