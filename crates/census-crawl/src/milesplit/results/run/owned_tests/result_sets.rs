use super::*;
#[test]
fn shared_owned_capture_projects_only_matching_result_sets_without_duplicate_subject_effects(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let second =
                ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
                    .ok_or("second set")?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            document["data"][1]["meetResultsId"] = json!("1266815");
            let body = serde_json::to_vec(&document)?;
            seed_owned(&fetcher, &first, &body)?;
            seed_metadata(&fetcher, &first)?;
            seed_metadata(&fetcher, &second)?;
            let mut run = Run::new(ProviderSchools::from_schools(&[
                school("Spann", "38332"),
                school("Charles", "4912"),
            ]));
            let ctx = context(&store, &fetcher)?;
            run.read(&ctx, &first).await?;
            let first_rows: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                first_rows.iter()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782806"
                ])
            );
            run.read(&ctx, &second).await?;
            let all_rows: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                all_rows.iter()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782277",
                    "milesplit_result:201782806",
                ])
            );
            let long = all_rows
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782263")
                .ok_or("long jump")?;
            let triple = all_rows
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782277")
                .ok_or("triple jump")?;
            check!(eq; long.athlete, triple.athlete);
            check!(eq; long.team, triple.team);
            check!(eq; fetcher.stats().await.cache_hits, 3);
            Ok(())
        })
}

#[test]
fn a_later_result_set_cannot_reuse_the_previous_meets_owned_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let foreign = ResultSetRef::parse("https://al.milesplit.com/meets/1/results/2/raw")
                .ok_or("foreign set")?;
            seed_owned(&fetcher, &first, TROY)?;
            seed_metadata(&fetcher, &first)?;
            let mut run = Run::new(ProviderSchools::from_schools(&[
                school("Spann", "38332"),
                school("Charles", "4912"),
            ]));
            let ctx = context(&store, &fetcher)?;
            run.read(&ctx, &first).await?;
            let facts = super::replay::entities(&store)?;
            let physical = super::replay::physical(&store)?;
            match run.read(&ctx, &foreign).await {
                Err(crate::CrawlError::Fetch(crate::net::FetchError::Offline { url })) => {
                    check!(eq; url, crate::milesplit::fetch::owned_meet_url(&foreign)?);
                }
                outcome => {
                    return Err(format!(
                        "later meet must acquire its own source capture: {outcome:?}"
                    )
                    .into())
                }
            }
            check!(eq; super::replay::entities(&store)?, facts);
            check!(eq; super::replay::physical(&store)?, physical);
            Ok(())
        })
}
