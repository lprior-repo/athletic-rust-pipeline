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
            let mut run = Run {
                schools: ProviderSchools::from_schools(&[
                    school("Spann", "38332"),
                    school("Charles", "4912"),
                ]),
                owned: HashMap::new(),
                stats: Stats::default(),
                accumulated: Accumulator::default(),
                seen: HashSet::new(),
                pending: Vec::new(),
            };
            let ctx = context(&store, &fetcher)?;
            run.read(&ctx, &first).await?;
            check!(eq;
                run.accumulated
                    .performances
                    .values()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782806"
                ])
            );
            run.read(&ctx, &second).await?;
            check!(eq;
                run.accumulated
                    .performances
                    .values()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782277",
                    "milesplit_result:201782806",
                ])
            );
            let long = run
                .accumulated
                .performances
                .values()
                .find(|row| row.source_key == "milesplit_result:201782263")
                .ok_or("long jump")?;
            let triple = run
                .accumulated
                .performances
                .values()
                .find(|row| row.source_key == "milesplit_result:201782277")
                .ok_or("triple jump")?;
            check!(eq; long.athlete, triple.athlete);
            check!(eq; long.team, triple.team);
            check!(eq; fetcher.stats().await.cache_hits, 3);
            Ok(())
        })
}
