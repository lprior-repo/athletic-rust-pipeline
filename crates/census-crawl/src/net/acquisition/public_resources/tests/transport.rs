use super::*;
use assertions::{exact_hop, no_directory_attempt, retained, Witness};
use server::{serve, Reply};

#[tokio::test]
async fn directory_is_owned_without_robots_and_keeps_exact_actual_http_and_receipts() -> TestResult
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (requests, captured) =
        tokio::try_join!(serve(listener, [Reply::directory(DIRECTORY)]), fetch,)?;
    check!(eq; requests.len(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(eq; captured.directory().capture().body, DIRECTORY);
    check!(!captured.directory().capture().from_cache);
    exact_hop(
        captured.directory(),
        0,
        &format!("{ORIGIN}/teams"),
        200,
        DIRECTORY.len(),
        "text/html",
    )?;
    retained(&fixture.store, captured.directory())?;
    let historical = fixture.work(PublicResourceRootKindV1::Robots)?;
    no_directory_attempt(&fixture.store, &historical)?;
    let prefix = format!("work/{}/", historical.id()?);
    check!(fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?
        .iter()
        .all(|key| !key.starts_with(&prefix)));
    check!(eq; captured.directory().observed_http()?.ok_or("directory hops")?.hops().len(), 1);
    Ok(())
}

#[tokio::test]
async fn actual_directory_denials_survive_reopen_without_another_http_or_robots_fetch() -> TestResult
{
    for status in [401, 403, 429] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let fixture = Fixture::new(&listener)?;
        let reply = Reply {
            status,
            ..Reply::directory(b"denied")
        };
        let fetch =
            async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
        let (requests, result) = tokio::try_join!(serve(listener, [reply]), fetch)?;
        check!(
            matches!(result, Err(FetchError::Http { status: actual, url })
            if actual == status && url == format!("{ORIGIN}/teams"))
        );
        check!(eq; requests.len(), 1);
        let captured = held_directory(&fixture)?;
        check!(eq; captured.capture().status, status);
        check!(eq; captured.capture().body, b"denied");
        exact_hop(
            &captured,
            0,
            &format!("{ORIGIN}/teams"),
            status,
            6,
            "text/html",
        )?;
        let original = retained(&fixture.store, &captured)?;
        let before = Witness::capture(&fixture.store)?;
        let fixture = fixture.reopen()?;
        let reopened = held_directory(&fixture)?;
        check!(eq; retained(&fixture.store, &reopened)?, original);
        check!(eq; Witness::capture(&fixture.store)?, before);
        check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
        no_directory_attempt(
            &fixture.store,
            &fixture.work(PublicResourceRootKindV1::Robots)?,
        )?;
    }
    Ok(())
}

fn held_directory(fixture: &Fixture) -> TestResult<crate::net::CapturedFetch> {
    let owned = fixture
        .context()
        .registered(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let decision = fixture
        .store
        .inspect_acquisition_work(&owned, &mut census_store::AcquisitionReadBudget::for_work())?;
    let reference = match decision {
        census_store::AcquireDecision::Replay(reference) => reference,
        census_store::AcquireDecision::RetryOwed(terminal)
        | census_store::AcquireDecision::Settled(terminal) => terminal.occurrence()?,
        _ => return Err("directory response did not retain its owned result".into()),
    };
    Ok(crate::net::acquisition::read::replay(
        &fixture.store,
        &reference,
    )?)
}
