use super::*;
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalTeam, SourceObservation};

const TABLES: [Table; 6] = [
    Table::Meets,
    Table::Events,
    Table::Teams,
    Table::Athletes,
    Table::Performances,
    Table::SourceObservations,
];

pub(super) fn physical(store: &Store) -> TestResult<Vec<(Table, census_store::TableWalk)>> {
    TABLES
        .into_iter()
        .map(|table| Ok((table, store.walk_table(table)?)))
        .collect()
}

fn assert_absent(store: &Store) -> TestResult {
    for table in TABLES {
        check!(eq;
            store.walk_table(table)?.rows,
            0,
            "{table:?}"
        );
    }
    check!(store
        .journal_payloads(super::super::super::RESULT_SET_PHASE)?
        .is_empty());
    Ok(())
}

pub(super) fn entities(store: &Store) -> TestResult<serde_json::Value> {
    Ok(json!({
        "meets": store.scan::<CanonicalMeet>(Table::Meets)?,
        "events": store.scan::<CanonicalEvent>(Table::Events)?,
        "teams": store.scan::<CanonicalTeam>(Table::Teams)?,
        "athletes": store.scan::<CanonicalAthlete>(Table::Athletes)?,
        "performances": store.scan::<CanonicalPerformance>(Table::Performances)?,
        "observations": store.scan::<SourceObservation>(Table::SourceObservations)?,
    }))
}

fn assert_projected(store: &Store) -> TestResult {
    for (table, count) in [
        (Table::Meets, 1),
        (Table::Events, 3),
        (Table::Teams, 2),
        (Table::Athletes, 2),
        (Table::Performances, 3),
        (Table::SourceObservations, 3),
    ] {
        let walk = store.walk_table(table)?;
        check!(eq; walk.rows, count, "{table:?}");
        if table != Table::SourceObservations {
            check!(eq; walk.repeated_ids, 0, "{table:?}");
        }
    }
    let payloads = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
    let receipt = payloads
        .iter()
        .find(|value| value["disposition"] == "projection_applied")
        .ok_or("complete projection")?;
    check!(eq; receipt["projected_rows"], 3);
    check!(eq; receipt["meet"], "725218");
    check!(eq; receipt["rsid"], "1266814");
    check!(eq;
        receipt["capture"]["content_digest"],
        crate::net::cache::content_digest(TROY)
    );
    check!(eq;
        receipt["raw_metadata_capture"]["content_digest"],
        crate::net::cache::content_digest(FEMALE_RAW)
    );
    check!(eq; receipt["source_completeness"], "unknown");
    check!(eq; receipt["ownership_complete"], false);
    check!(eq; receipt["canonical_identity_accepted"], false);
    check!(eq; receipt["lifetime_pr_claimed"], false);
    check!(eq; receipt["census_sealed"], false);
    let state = entities(store)?;
    check!(eq; state["meets"][0]["date"], "2026-03-27");
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq;
        performances
            .iter()
            .map(|row| row.source_key.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "milesplit_result:201782263",
            "milesplit_result:201782277",
            "milesplit_result:201782806"
        ])
    );
    check!(performances
        .iter()
        .all(|row| row.observed_grade.is_none() && row.date == "2026-03-27"));
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq;
        athletes
            .iter()
            .map(|row| Ok(row.source.as_ref().ok_or("owner")?.id.as_str()))
            .collect::<TestResult<std::collections::BTreeSet<_>>>()?,
        std::collections::BTreeSet::from(["14222592", "11357806"])
    );
    Ok(())
}

#[test]
fn collect_interruption_after_owned_capture_reopens_without_a_half_visible_projection() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let other =
                ResultSetRef::parse("https://oh.milesplit.com/meets/770621/results/1321880/raw")
                    .ok_or("uncached source")?;
            let mut interrupted = options(&reference);
            interrupted.urls.push(crate::milesplit::ResultSetRequest {
                url: other.url.clone(),
                jurisdiction: census_domain::UsJurisdiction::Ohio,
            });
            match crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &interrupted)
                .await
            {
                Err(crate::CrawlError::Fetch(crate::net::FetchError::Offline { url })) => {
                    check!(eq;
                        url,
                        crate::milesplit::fetch::owned_meet_url(&other)?
                    );
                }
                outcome => {
                    return Err(format!(
                        "expected deterministic interruption before projection commit: {outcome:?}"
                    )
                    .into());
                }
            }
            assert_absent(&store)?;
            let raw = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            check!(raw.iter().any(|value| value["encoding"] == "base64"));
            let owned = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            check!(owned.iter().any(|value| value["result_id"] == 201782263));
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            assert_absent(&store)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (report.rows, report.errors, report.requests), (3, 0, 0));
            assert_projected(&store)?;
            check!(eq;
                store
                    .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)
                    ?,
                raw
            );
            check!(eq;
                store
                    .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)
                    ?,
                owned
            );
            let before = entities(&store)?;
            let physical_before = physical(&store)?;
            let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let replay = crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; (replay.rows, replay.errors, replay.requests), (3, 0, 0));
            check!(
                recording.drain().is_empty(),
                "completed v3 effects do not become physical writes"
            );
            check!(eq; entities(&store)?, before);
            check!(eq; physical(&store)?, physical_before);
            check!(eq;
                store
                    .journal_payloads(super::super::super::RESULT_SET_PHASE)
                    ?,
                receipts
            );
            Ok(())
        })
}

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
            let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            let replay = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (replay.rows, replay.errors, replay.requests), (3, 0, 0));
            check!(eq; entities(&store)?, before);
            check!(eq; physical(&store)?, physical_before);
            check!(eq;
                store
                    .journal_payloads(super::super::super::RESULT_SET_PHASE)
                    ?,
                receipts
            );
            Ok(())
        })
}
