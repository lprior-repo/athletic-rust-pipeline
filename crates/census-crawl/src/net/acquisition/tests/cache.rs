use super::*;
use crate::net::{CaptureProvenanceView, CaptureTransportError, FetchError};
use crate::CrawlError;

#[tokio::test]
async fn historical_or_unsupported_cache_is_a_pre_http_gap_without_fallback() -> TestResult {
    for version in [None, Some(99)] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let fixture = Fixture::new(&listener)?;
        let (body_path, meta_path) = fixture.cache_paths()?;
        std::fs::write(&body_path, b"historical")?;
        let url = format!(
            "https://{HOST}/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
            crate::milesplit::OWNED_FIELDS
        );
        let historical = crate::net::cache::CacheMeta {
            url,
            method: "GET".into(),
            status: 200,
            fetched_at: CAPTURED.into(),
            content_digest: crate::net::cache::content_digest(b"historical"),
            bytes: 10,
            ..crate::net::cache::CacheMeta::default()
        };
        let value = match version {
            None => serde_json::to_value(historical)?,
            Some(version) => {
                serde_json::json!({"version": version, "metadata": {}, "original_occurrence": null})
            }
        };
        std::fs::write(&meta_path, serde_json::to_vec(&value)?)?;
        let result = fixture.acquire(2026, false).await?;
        match version {
            None => check!(matches!(
                result,
                Err(CrawlError::Fetch(FetchError::Capture(
                    CaptureTransportError::HistoricalAttemptUnknown
                )))
            )),
            Some(_) => check!(matches!(
                result,
                Err(CrawlError::Fetch(FetchError::Capture(
                    CaptureTransportError::CacheEnvelopeUnsupported
                )))
            )),
        }
        check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn actual_304_keeps_original_occurrence_date_and_eligible_pointer() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        let first = fixture.acquire(2025, true).await??;
        let pointer_before = std::fs::read(fixture.cache_paths()?.1)?;
        let owner = fixture.context_owner(2026, true)?;
        let ctx = fixture.context(&owner, true)?;
        let mut options = ctx.fetch_options();
        options
            .headers
            .push(("If-None-Match".into(), "\"capture-v1\"".into()));
        let second =
            crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &options).await?;
        check!(eq; second.capture().body, first.capture().body);
        check!(eq; second.capture().fetched_at, first.capture().fetched_at);
        check!(eq; std::fs::read(fixture.cache_paths()?.1)?, pointer_before);
        match second.provenance() {
            CaptureProvenanceView::Retained {
                occurrence,
                original,
            } => {
                check!(occurrence != original);
                check!(eq; original, Fixture::occurrence(&first)?);
            }
            _ => return Err("304 lost durable provenance".into()),
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(second)
    };
    let replies = [
        Reply::ok(BODY),
        Reply {
            status: 304,
            body: b"",
            headers: "",
            before: Mutation::None,
        },
    ];
    let ((), captured) = tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    let mut budget = census_store::AcquisitionReadBudget::for_work();
    let retained = fixture
        .store
        .read_retained_acquisition_v1(Fixture::occurrence(&captured)?, &mut budget)?;
    check!(eq; retained.response().observed_at(), CAPTURED);
    check!(eq; retained.revalidation().map(|response| response.status()), Some(304));
    check!(eq; fixture.count(), 2);
    let replay = fixture.acquire(2026, true).await??;
    check!(eq; Fixture::occurrence(&replay)?, Fixture::occurrence(&captured)?);
    let meta_path = fixture.cache_paths()?.1;
    let mut pointer: serde_json::Value = serde_json::from_slice(&std::fs::read(&meta_path)?)?;
    *pointer
        .get_mut("original_occurrence")
        .ok_or("original occurrence")? = serde_json::to_value(Fixture::occurrence(&captured)?)?;
    std::fs::write(&meta_path, serde_json::to_vec(&pointer)?)?;
    let chained = fixture.acquire(2024, false).await?;
    check!(matches!(
        chained,
        Err(CrawlError::Fetch(FetchError::Capture(
            CaptureTransportError::CacheOccurrenceMismatch
        )))
    ));
    check!(eq; fixture.count(), 2);
    Ok(())
}

#[tokio::test]
async fn actual_304_cache_compare_fence_rejects_changed_metadata_without_refetch() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let meta_path = fixture.cache_paths()?.1;
    let fetch = async {
        fixture.acquire(2025, true).await??;
        let owner = fixture.context_owner(2026, true)?;
        let ctx = fixture.context(&owner, true)?;
        let mut options = ctx.fetch_options();
        options
            .headers
            .push(("If-None-Match".into(), "\"capture-v1\"".into()));
        let result =
            crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &options).await;
        check!(matches!(
            result,
            Err(CrawlError::Fetch(FetchError::Capture(
                CaptureTransportError::CacheOccurrenceMismatch
            )))
        ));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let replies = [
        Reply::ok(BODY),
        Reply {
            status: 304,
            body: b"",
            headers: "",
            before: Mutation::Metadata(&meta_path),
        },
    ];
    tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 2);
    Ok(())
}

#[tokio::test]
async fn eligible_cache_reuse_finishes_new_work_without_new_attempt_or_date() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        fixture
            .acquire(2025, true)
            .await?
            .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })
    };
    let ((), original) = tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    let reused = fixture.acquire(2026, false).await??;
    check!(eq; Fixture::occurrence(&reused)?, Fixture::occurrence(&original)?);
    check!(eq; reused.capture().fetched_at, CAPTURED);
    check!(eq; fixture.count(), 1);
    let keys = fixture
        .store
        .journal_keys(census_domain::model::CaptureProtocolVersion::V2.phase())?;
    check!(eq; keys.iter().filter(|key| key.starts_with("slot/")).count(), 2);
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 1);
    Ok(())
}

#[tokio::test]
async fn mismatched_eligible_metadata_cannot_publish_a_work_result() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch = async {
        fixture
            .acquire(2025, true)
            .await?
            .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    let metadata = fixture.cache_paths()?.1;
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&metadata)?)?;
    let field = value
        .get_mut("metadata")
        .and_then(|value| value.get_mut("fetched_at"))
        .ok_or("eligible metadata date")?;
    *field = serde_json::json!("2026-10-02T23:44:16Z");
    std::fs::write(metadata, serde_json::to_vec(&value)?)?;
    let result = fixture.acquire(2026, false).await?;
    check!(matches!(
        result,
        Err(CrawlError::Fetch(FetchError::Capture(
            CaptureTransportError::CacheOccurrenceMismatch
        )))
    ));
    let keys = fixture
        .store
        .journal_keys(census_domain::model::CaptureProtocolVersion::V2.phase())?;
    check!(eq; keys.iter().filter(|key| key.ends_with("/result")).count(), 1);
    check!(eq; fixture.count(), 1);
    Ok(())
}
