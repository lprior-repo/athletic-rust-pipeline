mod cache;
mod failures;
pub(crate) mod fixture;
mod grants;
mod recovery;
use fixture::*;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn actual_https_success_and_replay_without_eligible_cache_have_one_dispatch() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        fixture
            .acquire(2026, false)
            .await?
            .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })
    };
    let ((), captured) = tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 1);
    let original = Fixture::occurrence(&captured)?.clone();
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::remove_file(body)?;
    std::fs::remove_file(metadata)?;
    let fixture = fixture.reopen()?;
    let replay = fixture.acquire(2026, false).await??;
    check!(eq; Fixture::occurrence(&replay)?, &original);
    check!(eq; replay.capture.fetched_at, CAPTURED);
    check!(eq; replay.capture.body, BODY);
    check!(eq; fixture.count(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn actual_equal_byte_same_second_dispatches_keep_distinct_occurrences() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        let first = fixture.acquire(2025, true).await??;
        let second = fixture.acquire(2026, true).await??;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((first, second))
    };
    let replies = [Reply::ok(BODY), Reply::ok(BODY)];
    let ((), (first, second)) = tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 2);
    check!(Fixture::occurrence(&first)? != Fixture::occurrence(&second)?);
    check!(eq; first.capture.fetched_at, CAPTURED);
    check!(eq; second.capture.fetched_at, CAPTURED);
    check!(eq; first.capture.content_digest, second.capture.content_digest);
    let mut budget = census_store::AcquisitionReadBudget::for_work();
    let retained = fixture
        .store
        .read_retained_acquisition_v1(Fixture::occurrence(&second)?, &mut budget)?;
    check!(eq; retained.response().observed_at(), CAPTURED);
    let metadata: crate::net::cache::CacheMeta =
        serde_json::from_slice(retained.prepared().metadata_bytes())?;
    check!(eq; metadata.fetched_at, CAPTURED);
    check!(eq; metadata.response_url.as_deref(), Some(retained.response().response_url()));
    check!(eq; metadata.status, retained.response().status());
    let files = std::fs::read_dir(
        fixture
            .store
            .http_cache_dir()
            .join("archive/captures")
            .join(&metadata.content_digest),
    )?
    .count();
    check!(eq; files, 1);
    Ok(())
}

#[tokio::test]
async fn actual_retryable_responses_keep_one_original_slot_and_three_attempt_limit() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        for _ in 0..3 {
            let outcome = fixture.acquire(2026, true).await?;
            check!(matches!(
                outcome,
                Err(crate::CrawlError::Fetch(crate::net::FetchError::Http {
                    status: 500,
                    ..
                }))
            ));
        }
        let fourth = fixture.acquire(2026, true).await?;
        check!(matches!(
            fourth,
            Err(crate::CrawlError::Fetch(crate::net::FetchError::CaptureStore(error)))
                if matches!(error.as_ref(), census_store::StoreError::Capture(
                    census_store::CaptureError::AcquisitionBudgetExhausted
                ))
        ));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let replies = [
        Reply {
            status: 500,
            body: b"failure",
            headers: "",
            before: Mutation::None,
        },
        Reply {
            status: 500,
            body: b"failure",
            headers: "",
            before: Mutation::None,
        },
        Reply {
            status: 500,
            body: b"failure",
            headers: "",
            before: Mutation::None,
        },
    ];
    tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 3);
    let keys = fixture
        .store
        .journal_keys(census_domain::model::SOURCE_ACQUISITION_PHASE)?;
    check!(eq; keys.iter().filter(|key| key.starts_with("slot/")).count(), 1);
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 3);
    Ok(())
}

mod lock_order;
