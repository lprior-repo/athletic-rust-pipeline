use super::*;
use assertions::{retained, Witness};
use server::{serve, Reply};
use std::sync::Arc;

#[tokio::test]
async fn cold_replay_without_eligible_cache_preserves_both_occurrences_metadata_and_receipts(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (_, captured) = tokio::try_join!(
        serve(
            listener,
            [Reply::robots(ALLOW), Reply::directory(DIRECTORY)]
        ),
        fetch
    )?;
    let robots = retained(&fixture.store, captured.robots())?;
    let directory = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    let fixture = fixture.reopen()?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.robots())?, robots);
    check!(eq; retained(&fixture.store, replayed.directory())?, directory);
    check!(replayed.robots().capture().from_cache);
    check!(replayed.directory().capture().from_cache);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn completed_replay_after_24_hours_uses_original_dispatch_policy_without_new_writes(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let past = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(2))
        .ok_or("controlled old dispatch time")?;
    fixture.fetcher.capture_clock =
        crate::net::time::CaptureClock::Injected(Arc::new(fixture::FixedClock(past)));
    let fetch = seed_pair(&fixture);
    let (_, captured) = tokio::try_join!(
        serve(
            listener,
            [Reply::robots(ALLOW), Reply::directory(DIRECTORY)]
        ),
        fetch
    )?;
    check!(!crate::net::robots::policy::capture_is_current(
        captured.robots().capture(),
        chrono::Utc::now()
    )?);
    let robots = retained(&fixture.store, captured.robots())?;
    let directory = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    let mut fixture = fixture.reopen()?;
    fixture.fetcher.capture_clock = crate::net::time::CaptureClock::System;
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.robots())?, robots);
    check!(eq; retained(&fixture.store, replayed.directory())?, directory);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn stale_owned_robots_cannot_dispatch_an_unresolved_directory_or_refresh_implicitly(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let past = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(2))
        .ok_or("controlled stale robots time")?;
    fixture.fetcher.capture_clock =
        crate::net::time::CaptureClock::Injected(Arc::new(fixture::FixedClock(past)));
    let fetch = async {
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            fixture
                .fetcher
                .get_public_resource_root(
                    &fixture.context(),
                    PublicResourceRootKindV1::Robots,
                    &FetchOptions::default(),
                    crate::net::robots::Admission::Unrestricted,
                )
                .await?,
        )
    };
    let (_, robots) = tokio::try_join!(serve(listener, [Reply::robots(ALLOW)]), fetch)?;
    let original = retained(&fixture.store, &robots)?;
    fixture.fetcher.capture_clock = crate::net::time::CaptureClock::System;
    stale_refusal(&fixture).await?;
    let before = Witness::capture(&fixture.store)?;
    stale_refusal(&fixture).await?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; retained(&fixture.store, &robots)?, original);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    assertions::no_directory_attempt(
        &fixture.store,
        &fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?,
    )?;
    Ok(())
}

async fn stale_refusal(fixture: &Fixture) -> TestResult {
    check!(
        matches!(fixture.acquire().await, Err(FetchError::Policy { detail }) if
        detail == "owned robots policy is no longer current")
    );
    Ok(())
}

async fn seed_pair(fixture: &Fixture) -> TestResult<CapturedPublicDirectory> {
    let robots = fixture
        .fetcher
        .get_public_resource_root(
            &fixture.context(),
            PublicResourceRootKindV1::Robots,
            &FetchOptions::default(),
            crate::net::robots::Admission::Unrestricted,
        )
        .await?;
    let work = fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let target = crate::net::robots::policy::parse_url(work.request().original_url())?;
    let rules = crate::net::robots::policy::Policy::from_capture(
        &target,
        robots.capture(),
        work.request().effective_user_agent(),
    )?;
    rules.check(target.as_str())?;
    let directory = fixture
        .fetcher
        .get_public_resource_root(
            &fixture.context(),
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            &FetchOptions::default(),
            crate::net::robots::Admission::Checked(&rules),
        )
        .await?;
    Ok(CapturedPublicDirectory { robots, directory })
}
