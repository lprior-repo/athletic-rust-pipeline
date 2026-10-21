use super::*;
use crate::net::acquisition::CaptureInterruption;

#[tokio::test]
async fn prepared_archive_recovers_through_real_owned_parser_with_zero_new_requests() -> TestResult
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::AfterArchive;
    let fetch = async {
        let result = fixture.acquire(2026, false).await?;
        check!(matches!(result, Err(ref error) if injected(error)));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::None;
    let fixture = fixture.reopen()?;
    let recovered = fixture.acquire(2026, false).await??;
    check!(eq; Fixture::occurrence(&recovered)?.attempt().ordinal().get(), 1);
    check!(eq; recovered.capture().fetched_at, CAPTURED);
    check!(matches!(
        recovered.verdict(),
        crate::milesplit::OwnedMeetVerdict::Parsed(_)
    ));
    check!(eq; fixture.count(), 1);
    let replay = fixture.acquire(2026, false).await??;
    check!(eq; Fixture::occurrence(&recovered)?, Fixture::occurrence(&replay)?);
    Ok(())
}

#[tokio::test]
async fn prepared_without_archive_refuses_recovery_without_hidden_refetch() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::AfterPrepared;
    let fetch = async {
        let result = fixture.acquire(2026, false).await?;
        check!(matches!(result, Err(ref error) if injected(error)));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::None;
    let fixture = fixture.reopen()?;
    let recovered = fixture.acquire(2026, false).await?;
    check!(matches!(
        recovered,
        Err(crate::CrawlError::Fetch(
            crate::net::FetchError::CaptureStore(_)
        ))
    ));
    check!(eq; fixture.count(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn observed_interruption_requires_separate_original_owner_continuation() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::AfterObserved;
    let fetch = async {
        let result = fixture.acquire(2026, false).await?;
        check!(matches!(result, Err(ref error) if injected(error)));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::None;
    let fixture = fixture.reopen()?;
    let recovered = fixture.acquire(2026, false).await?;
    check!(matches!(
        recovered,
        Err(crate::CrawlError::Fetch(crate::net::FetchError::CaptureStore(error)))
                if matches!(error.as_ref(), census_store::StoreError::Capture(
                    census_store::CaptureError::RetainedOutcomeUnknown
                ))
    ));
    check!(eq; fixture.count(), 1);
    let rows = fixture
        .store
        .journal_payloads(census_domain::model::CaptureProtocolVersion::V2.phase())?;
    check!(rows.iter().any(|row| row
        .get("terminal")
        .and_then(|terminal| terminal.get("kind"))
        .and_then(serde_json::Value::as_str)
        == Some("outcome_unknown")));
    Ok(())
}

#[tokio::test]
async fn armed_interruption_records_no_local_execute_and_never_reuses_the_arm() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::AfterArm;
    let first = fixture.acquire(2026, false).await?;
    check!(matches!(first, Err(ref error) if injected(error)));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    fixture.fetcher.capture_interrupt = CaptureInterruption::None;
    let fixture = fixture.reopen()?;
    let recovery = fixture.acquire(2026, false).await?;
    check!(matches!(
        recovery,
        Err(crate::CrawlError::Fetch(crate::net::FetchError::CaptureStore(error)))
                if matches!(error.as_ref(), census_store::StoreError::Capture(
                    census_store::CaptureError::RetainedOutcomeUnknown
                ))
    ));
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
    check!(eq; Fixture::occurrence(&captured)?.attempt().ordinal().get(), 2);
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn injected_prearm_timeout_reopens_reserved_without_consuming_attempt() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.capture_interrupt = CaptureInterruption::BeforeArmTimeout;
    let first = fixture.acquire(2026, false).await?;
    check!(matches!(
        first,
        Err(crate::CrawlError::Fetch(
            crate::net::FetchError::Timeout { .. }
        ))
    ));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    let keys = fixture
        .store
        .journal_keys(census_domain::model::CaptureProtocolVersion::V2.phase())?;
    check!(!keys
        .iter()
        .any(|key| key.ends_with("/armed") || key.ends_with("/terminal")));
    fixture.resume_capture();
    let fixture = fixture.reopen()?;
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
    check!(eq; Fixture::occurrence(&captured)?.attempt().ordinal().get(), 1);
    check!(eq; fixture.count(), 1);
    Ok(())
}
