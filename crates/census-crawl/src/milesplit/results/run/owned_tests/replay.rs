use super::super::super::group::{group_meets, run_group};
use super::super::super::{EntityCounts, RESULT_SET_PHASE};
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
            let unread =
                ResultSetRef::parse("https://oh.milesplit.com/meets/770621/results/1321880/raw")
                    .ok_or("uncached result set of a later meet")?;
            let mut both = options(&reference);
            both.urls.push(crate::milesplit::ResultSetRequest {
                url: unread.url.clone(),
                jurisdiction: census_domain::UsJurisdiction::Ohio,
            });
            interrupt_on_unread_meet(&store, &fetcher, &both, &unread).await?;
            assert_completed_meet_only(&store, &reference)?;
            let raw = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            check!(raw.iter().any(|value| value["encoding"] == "base64"));
            let owned = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            check!(owned.iter().any(|value| value["result_id"] == 201782263));
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            assert_completed_meet_only(&store, &reference)?;
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
fn a_later_meets_offline_failure_keeps_the_completed_meets_projection_resumable() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let unread =
                ResultSetRef::parse("https://al.milesplit.com/meets/725219/results/1266816/raw")
                    .ok_or("uncached result set of a later meet")?;
            let mut both = options(&reference);
            both.urls.push(crate::milesplit::ResultSetRequest {
                url: unread.url.clone(),
                jurisdiction: census_domain::UsJurisdiction::Alabama,
            });
            interrupt_on_unread_meet(&store, &fetcher, &both, &unread).await?;
            assert_completed_meet_only(&store, &reference)?;
            let before_entities = entities(&store)?;
            let before_physical = physical(&store)?;
            let before_captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            let before_receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            assert_completed_meet_only(&store, &reference)?;
            interrupt_on_unread_meet(&store, &fetcher, &both, &unread).await?;
            check!(eq; entities(&store)?, before_entities);
            check!(eq; physical(&store)?, before_physical);
            check!(eq;
                store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?,
                before_captures
            );
            check!(eq;
                store.journal_payloads(super::super::super::RESULT_SET_PHASE)?,
                before_receipts
            );
            Ok(())
        })
}

#[test]
fn many_distinct_meets_bound_retained_captures_and_keep_facts_identical_when_replayed() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let ctx = context(&store, &fetcher)?;
            let mut run = grouped_run();
            let mut total = EntityCounts::default();
            commit_group(&ctx, &mut run, &mut total, &[request(&reference)], 1).await?;
            check!(eq; run.owned.is_empty(), true, "captures released at meet commit");
            check!(eq; run.rows(), 0);
            check!(eq; run.pending.is_empty(), true);
            assert_projected(&store)?;
            check!(eq; (total.meets, total.performances), (1, 3));
            let facts = entities(&store)?;
            let walk = physical(&store)?;
            let receipts = store.journal_payloads(RESULT_SET_PHASE)?.len();
            let captures = store
                .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?
                .len();
            let manifests = store
                .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?
                .len();
            let mut capture_step = 0usize;
            let mut manifest_step = 0usize;
            let mut processed = 0usize;
            for scale in [10usize, 100, 1000] {
                for _ in 0..scale {
                    processed = processed.saturating_add(1);
                    let meet = 726_000u64.saturating_add(u64::try_from(processed)?);
                    let synthetic = ResultSetRef::parse(&format!(
                        "https://al.milesplit.com/meets/{meet}/results/1266814/raw"
                    ))
                    .ok_or("synthetic reference")?;
                    seed_owned(&fetcher, &synthetic, &distinct_meet_body(meet)?)?;
                    commit_group(&ctx, &mut run, &mut total, &[request(&synthetic)], 1).await?;
                    if processed == 1 {
                        capture_step = store
                            .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?
                            .len()
                            .saturating_sub(captures);
                        manifest_step = store
                            .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?
                            .len()
                            .saturating_sub(manifests);
                    }
                    check!(eq; run.owned.is_empty(), true, "captures released at meet commit");
                    check!(eq; run.rows(), 0);
                    check!(eq; run.pending.is_empty(), true);
                }
            }
            check!(
                capture_step > 0 && manifest_step > 0,
                "every meet acquires its own capture"
            );
            check!(eq; run.stats.result_sets, 1, "no synthetic meet projects facts");
            check!(eq; run.stats.failures.len(), processed, "every retained failure is recorded");
            check!(eq; entities(&store)?, facts);
            check!(eq; physical(&store)?, walk);
            let capture_step = capture_step.saturating_mul(processed);
            let manifest_step = manifest_step.saturating_mul(processed);
            check!(eq;
                store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?.len(),
                captures.saturating_add(capture_step),
                "durable captures grow linearly with the meets processed"
            );
            check!(eq;
                store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?.len(),
                manifests.saturating_add(manifest_step)
            );
            check!(eq;
                store.journal_payloads(RESULT_SET_PHASE)?.len(),
                receipts.saturating_add(processed)
            );
            let mut replayed = grouped_run();
            let mut again = EntityCounts::default();
            commit_group(&ctx, &mut replayed, &mut again, &[request(&reference)], 1).await?;
            check!(eq; replayed.stats.result_sets_resumed, 1);
            check!(eq; replayed.stats.result_sets, 1);
            check!(eq;
                (again.meets, again.teams, again.athletes, again.performances),
                (0, 0, 0, 0),
                "replayed facts are never applied twice"
            );
            check!(eq; entities(&store)?, facts);
            check!(eq; physical(&store)?, walk);
            check!(eq;
                store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?.len(),
                captures.saturating_add(capture_step)
            );
            check!(eq;
                store.journal_payloads(RESULT_SET_PHASE)?.len(),
                receipts.saturating_add(processed)
            );
            Ok(())
        })
}

#[test]
fn many_sets_of_one_meet_commit_as_one_group_and_release_every_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let second =
                ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
                    .ok_or("second set")?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            let row = document.pointer_mut("/data/1").ok_or("second row")?;
            *row.pointer_mut("/meetResultsId").ok_or("result set id")? = json!("1266815");
            seed_owned(&fetcher, &first, &serde_json::to_vec(&document)?)?;
            seed_metadata(&fetcher, &first)?;
            seed_metadata(&fetcher, &second)?;
            let ctx = context(&store, &fetcher)?;
            let mut run = grouped_run();
            let mut total = EntityCounts::default();
            let urls = [request(&first), request(&second)];
            let groups = group_meets(&mut run, &urls);
            check!(eq; groups.len(), 1, "sets of one meet commit as one group");
            check!(eq;
                groups.first().ok_or("group")?.references.len(),
                2
            );
            run_group(
                &ctx,
                &mut run,
                groups.into_iter().next().ok_or("group")?,
                1,
                &mut total,
            )
            .await?;
            check!(eq; run.owned.is_empty(), true, "captures released at meet commit");
            check!(eq; run.rows(), 0);
            check!(eq; run.pending.is_empty(), true);
            check!(eq; run.stats.result_sets, 2);
            check!(eq; total.performances, 3);
            let applied = store
                .journal_payloads(RESULT_SET_PHASE)?
                .iter()
                .filter(|value| value["disposition"] == "projection_applied")
                .count();
            check!(eq; applied, 2);
            Ok(())
        })
}

fn grouped_run() -> Run {
    Run {
        meet_id: String::new(),
        schools: ProviderSchools::from_schools(&[
            school("Spann", "38332"),
            school("Charles", "4912"),
        ]),
        owned: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        seen: HashSet::new(),
        pending: Vec::new(),
    }
}

fn distinct_meet_body(meet_id: u64) -> TestResult<Vec<u8>> {
    let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
    let envelope = document
        .pointer_mut("/_embedded/meet/id")
        .ok_or("owned meet envelope")?;
    *envelope = json!(meet_id.to_string());
    let rows = document
        .pointer_mut("/data")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("owned rows")?;
    for row in rows {
        let slot = row.pointer_mut("/meetId").ok_or("owned row meet id")?;
        *slot = json!(meet_id);
    }
    Ok(serde_json::to_vec(&document)?)
}

fn request(reference: &ResultSetRef) -> crate::milesplit::ResultSetRequest {
    crate::milesplit::ResultSetRequest {
        url: reference.url.clone(),
        jurisdiction: census_domain::UsJurisdiction::Alabama,
    }
}

async fn commit_group(
    ctx: &AdapterContext<'_>,
    run: &mut Run,
    total: &mut EntityCounts,
    urls: &[crate::milesplit::ResultSetRequest],
    window_rows: usize,
) -> TestResult {
    let groups = group_meets(run, urls);
    check!(eq; groups.len(), 1, "one meet is one commit unit");
    run_group(
        ctx,
        run,
        groups.into_iter().next().ok_or("group")?,
        window_rows,
        total,
    )
    .await?;
    Ok(())
}

fn assert_completed_meet_only(store: &Store, reference: &ResultSetRef) -> TestResult {
    assert_projected(store)?;
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let identified: Vec<&str> = meets
        .iter()
        .flat_map(|meet| meet.source_identities.iter())
        .map(|identity| identity.id.as_str())
        .collect();
    check!(
        eq;
        identified,
        vec![reference.meet_id.as_str()],
        "only the completed meet is projected; the interrupted meet leaves no half-visible rows",
    );
    Ok(())
}

async fn interrupt_on_unread_meet(
    store: &Store,
    fetcher: &Fetcher,
    options: &crate::milesplit::ResultSetOptions,
    unread: &ResultSetRef,
) -> TestResult {
    match crate::milesplit::collect_result_sets(&context(store, fetcher)?, options).await {
        Err(crate::CrawlError::Fetch(crate::net::FetchError::Offline { url })) => {
            check!(eq; url, crate::milesplit::fetch::owned_meet_url(unread)?);
            Ok(())
        }
        outcome => Err(format!(
            "expected the later meet to interrupt after the completed meet commits: {outcome:?}"
        )
        .into()),
    }
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
