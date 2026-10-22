use super::*;
use assertions::{retained, Witness};
use server::{serve, Reply};
use std::sync::Arc;

#[tokio::test]
async fn cold_replay_without_eligible_cache_preserves_original_occurrence_metadata_and_receipts(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (_, captured) = tokio::try_join!(serve(listener, [Reply::directory(DIRECTORY)]), fetch)?;
    let directory = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    let fixture = fixture.reopen()?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.directory())?, directory);
    check!(replayed.directory().capture().from_cache);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn completed_replay_after_24_hours_keeps_original_dispatch_evidence_without_new_writes(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let past = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(2))
        .ok_or("controlled old dispatch time")?;
    fixture.fetcher.capture_clock =
        crate::net::time::CaptureClock::Injected(Arc::new(fixture::FixedClock(past)));
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (_, captured) = tokio::try_join!(serve(listener, [Reply::directory(DIRECTORY)]), fetch)?;
    let directory = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    let mut fixture = fixture.reopen()?;
    fixture.fetcher.capture_clock = crate::net::time::CaptureClock::System;
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.directory())?, directory);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}
