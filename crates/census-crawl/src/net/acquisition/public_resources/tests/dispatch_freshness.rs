use super::*;
use assertions::{retained, Witness};
use server::{serve, Reply};
use std::sync::Arc;

#[tokio::test]
async fn dispatch_timestamp_is_actual_evidence_not_a_robots_expiry_gate() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let observed = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(2))
        .ok_or("controlled dispatch time")?;
    fixture.fetcher.capture_clock =
        crate::net::time::CaptureClock::Injected(Arc::new(fixture::FixedClock(observed)));
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (requests, captured) =
        tokio::try_join!(serve(listener, [Reply::directory(DIRECTORY)]), fetch,)?;
    check!(eq; requests.len(), 1);
    let view = captured
        .directory()
        .observed_http()?
        .ok_or("dispatch evidence")?;
    let hop = view.hops().first().ok_or("directory hop")?;
    check!(eq; hop.request().started_at(), observed.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let original = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    fixture.fetcher.capture_clock = crate::net::time::CaptureClock::System;
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.directory())?, original);
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    Ok(())
}
