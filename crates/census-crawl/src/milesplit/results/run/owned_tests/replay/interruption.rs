use super::*;

#[test]
fn collect_interruption_between_meets_reopens_with_only_completed_projection_windows() -> TestResult
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
            check!(eq; store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?, raw);
            check!(eq; store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?, owned);
            let before = entities(&store)?;
            let physical_before = physical(&store)?;
            let receipts = store.journal_payloads(RESULT_SET_PHASE)?;
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
                "completed projection effects do not become physical writes"
            );
            check!(eq; entities(&store)?, before);
            check!(eq; physical(&store)?, physical_before);
            check!(eq; store.journal_payloads(RESULT_SET_PHASE)?, receipts);
            Ok(())
        })
}

#[test]
fn a_later_meets_offline_failure_keeps_the_completed_meets_projection_resumable() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        seed_metadata(&fetcher, &reference)?;
        let unread = ResultSetRef::parse("https://al.milesplit.com/meets/725219/results/1266816/raw")
            .ok_or("uncached result set of a later meet")?;
        let mut both = options(&reference);
        both.urls.push(crate::milesplit::ResultSetRequest {
            url: unread.url.clone(), jurisdiction: census_domain::UsJurisdiction::Alabama,
        });
        interrupt_on_unread_meet(&store, &fetcher, &both, &unread).await?;
        assert_completed_meet_only(&store, &reference)?;
        let before_entities = entities(&store)?;
        let before_physical = physical(&store)?;
        let before_captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
        let before_receipts = store.journal_payloads(RESULT_SET_PHASE)?;
        drop(store);
        let store = Store::open(dir.path().join("store"))?;
        assert_completed_meet_only(&store, &reference)?;
        interrupt_on_unread_meet(&store, &fetcher, &both, &unread).await?;
        check!(eq; entities(&store)?, before_entities);
        check!(eq; physical(&store)?, before_physical);
        check!(eq; store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?, before_captures);
        check!(eq; store.journal_payloads(RESULT_SET_PHASE)?, before_receipts);
        Ok(())
    })
}

fn assert_completed_meet_only(store: &Store, reference: &ResultSetRef) -> TestResult {
    assert_projected(store)?;
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let identified: Vec<&str> = meets
        .iter()
        .flat_map(|meet| meet.source_identities.iter())
        .map(|identity| identity.id.as_str())
        .collect();
    check!(eq; identified, vec![reference.meet_id.as_str()],
        "only the completed meet is projected; the interrupted meet leaves no half-visible rows");
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
