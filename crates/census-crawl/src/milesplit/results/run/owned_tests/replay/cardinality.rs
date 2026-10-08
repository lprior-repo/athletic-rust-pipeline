use super::*;

#[test]
fn many_distinct_meets_bound_retained_captures_and_keep_facts_identical_when_replayed() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        seed_metadata(&fetcher, &reference)?;
        let ctx = context(&store, &fetcher)?;
        let mut run = new_run();
        commit_requests(&ctx, &mut run, &[request(&reference)]).await?;
        assert_projected(&store)?;
        check!(eq; (run.counts.meets, run.counts.performances), (1, 3));
        let facts = entities(&store)?;
        let walk = physical(&store)?;
        let before = durable_counts(&store)?;
        let mut step = [0; 3];
        let mut processed = 0usize;
        for scale in [10usize, 100, 1000] {
            for _ in 0..scale {
                processed = processed.saturating_add(1);
                acquire_unprojectable(&ctx, &mut run, &fetcher, processed).await?;
                if processed == 1 { step = growth(durable_counts(&store)?, before)?; }
            }
            check!(eq; run.stats.failure_count, processed, "every unfinished source has a retained failure");
            check!(run.stats.failures.len() <= 5, "failure samples remain bounded independently of source count");
            check!(run.stats.peak_capture_bytes <= crate::net::MAX_BODY_BYTES);
        }
        let [captures, manifests, receipts] = step;
        check!(captures > 0 && manifests > 0);
        check!(eq; receipts, 1, "one retained unfinished receipt for each unprojectable meet");
        check!(eq; run.stats.result_sets, 1);
        check!(eq; entities(&store)?, facts);
        check!(eq; physical(&store)?, walk);
        assert_growth(&store, before, step, processed)?;
        let mut replayed = new_run();
        commit_requests(&ctx, &mut replayed, &[request(&reference)]).await?;
        check!(eq; (replayed.stats.result_sets_resumed, replayed.stats.result_sets), (1, 1));
        check!(eq; (replayed.counts.meets, replayed.counts.teams, replayed.counts.athletes, replayed.counts.performances), (0, 0, 0, 0));
        check!(eq; entities(&store)?, facts);
        check!(eq; physical(&store)?, walk);
        assert_growth(&store, before, step, processed)?;
        Ok(())
    })
}

#[test]
fn many_sets_of_one_meet_commit_atomic_windows_and_release_every_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, first) = setup()?;
        let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
            .ok_or("second set")?;
        let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
        let row = document.pointer_mut("/data/1").ok_or("second row")?;
        *row.pointer_mut("/meetResultsId").ok_or("result set id")? = json!("1266815");
        seed_owned(&fetcher, &first, &serde_json::to_vec(&document)?)?;
        seed_metadata(&fetcher, &first)?;
        seed_metadata(&fetcher, &second)?;
        let mut run = new_run();
        commit_requests(&context(&store, &fetcher)?, &mut run, &[request(&first), request(&second)]).await?;
        check!(eq; run.stats.result_sets, 2);
        check!(eq; run.counts.performances, 3);
        let rows: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        check!(eq; rows.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["milesplit_result:201782263", "milesplit_result:201782277", "milesplit_result:201782806"]));
        let applied = store.journal_payloads(RESULT_SET_PHASE)?.iter()
            .filter(|value| value["disposition"] == "projection_applied").count();
        check!(eq; applied, 2);
        check!(eq; fetcher.stats().await.cache_hits, 3, "one owned acquisition and two metadata acquisitions");
        Ok(())
    })
}

fn new_run() -> Run {
    Run::new(ProviderSchools::from_schools(&[
        school("Spann", "38332"),
        school("Charles", "4912"),
    ]))
}

fn request(reference: &ResultSetRef) -> crate::milesplit::ResultSetRequest {
    crate::milesplit::ResultSetRequest {
        url: reference.url.clone(),
        jurisdiction: census_domain::UsJurisdiction::Alabama,
    }
}

async fn commit_requests(
    ctx: &AdapterContext<'_>,
    run: &mut Run,
    urls: &[crate::milesplit::ResultSetRequest],
) -> TestResult {
    let first = run
        .stats
        .result_sets
        .saturating_add(run.stats.failure_count);
    for (offset, request) in urls.iter().enumerate() {
        run.request(ctx, request, first.saturating_add(offset))
            .await?;
    }
    run.release_owned();
    check!(
        matches!(run.owned, super::super::super::ActiveMeet::Empty),
        "decoded captures released at meet boundary"
    );
    Ok(())
}

async fn acquire_unprojectable(
    ctx: &AdapterContext<'_>,
    run: &mut Run,
    fetcher: &Fetcher,
    processed: usize,
) -> TestResult {
    let meet = 726_000u64.saturating_add(u64::try_from(processed)?);
    let reference = ResultSetRef::parse(&format!(
        "https://al.milesplit.com/meets/{meet}/results/1266814/raw"
    ))
    .ok_or("controlled reference")?;
    let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
    *document
        .pointer_mut("/_embedded/meet/id")
        .ok_or("owned meet envelope")? = json!(meet.to_string());
    let rows = document
        .pointer_mut("/data")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("owned rows")?;
    for row in rows {
        *row.pointer_mut("/meetId").ok_or("owned row meet id")? = json!(meet);
    }
    seed_owned(fetcher, &reference, &serde_json::to_vec(&document)?)?;
    commit_requests(ctx, run, &[request(&reference)]).await
}

fn durable_counts(store: &Store) -> TestResult<[usize; 3]> {
    Ok([
        store
            .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?
            .len(),
        store
            .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?
            .len(),
        store.journal_payloads(RESULT_SET_PHASE)?.len(),
    ])
}

fn growth(current: [usize; 3], before: [usize; 3]) -> TestResult<[usize; 3]> {
    let [captures, manifests, receipts] = current;
    let [before_captures, before_manifests, before_receipts] = before;
    Ok([
        captures
            .checked_sub(before_captures)
            .ok_or("capture loss")?,
        manifests
            .checked_sub(before_manifests)
            .ok_or("manifest loss")?,
        receipts
            .checked_sub(before_receipts)
            .ok_or("receipt loss")?,
    ])
}

fn assert_growth(
    store: &Store,
    before: [usize; 3],
    step: [usize; 3],
    processed: usize,
) -> TestResult {
    for ((current, before), step) in durable_counts(store)?.into_iter().zip(before).zip(step) {
        check!(eq; current, before.saturating_add(step.saturating_mul(processed)), "durable source conservation");
    }
    Ok(())
}
