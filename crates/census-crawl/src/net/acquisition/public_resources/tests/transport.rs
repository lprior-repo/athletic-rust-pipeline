use super::*;
use assertions::{exact_hop, no_directory_attempt, retained};
use server::{serve, Reply};

#[tokio::test]
async fn allowed_roots_are_owned_ordered_and_keep_exact_actual_http_and_receipts() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let (requests, captured) = tokio::try_join!(
        serve(
            listener,
            [Reply::robots(ALLOW), Reply::directory(DIRECTORY)]
        ),
        fetch,
    )?;
    check!(eq; requests.len(), 2);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; captured.robots().capture().body, ALLOW);
    check!(eq; captured.directory().capture().body, DIRECTORY);
    check!(!captured.robots().capture().from_cache);
    check!(!captured.directory().capture().from_cache);
    exact_hop(
        captured.robots(),
        0,
        &format!("{ORIGIN}/robots.txt"),
        200,
        ALLOW.len(),
        "text/plain",
    )?;
    exact_hop(
        captured.directory(),
        0,
        &format!("{ORIGIN}/teams"),
        200,
        DIRECTORY.len(),
        "text/html",
    )?;
    let robots = retained(&fixture.store, captured.robots())?;
    let directory = retained(&fixture.store, captured.directory())?;
    check!(ne; robots, directory);
    let robots_work = fixture.work(PublicResourceRootKindV1::Robots)?;
    let directory_work = fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    check!(ne; robots_work.slot().id()?, directory_work.slot().id()?);
    check!(eq; captured.robots().observed_http()?.ok_or("robots hops")?.hops().len(), 1);
    check!(eq; captured.directory().observed_http()?.ok_or("directory hops")?.hops().len(), 1);
    Ok(())
}

#[tokio::test]
async fn owned_robots_disallow_blocks_directory_without_dispatch() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let body = b"User-agent: *\nDisallow: /teams\n";
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
    let (requests, result) = tokio::try_join!(serve(listener, [Reply::robots(body)]), fetch)?;
    let digest = crate::net::cache::content_digest(body);
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail ==
        format!("robots disallows {ORIGIN}/teams; policy={ORIGIN}/robots.txt sha256={digest}"))
    );
    check!(eq; requests.len(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    no_directory_attempt(
        &fixture.store,
        &fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?,
    )?;
    let robots = held_robots(&fixture)?;
    check!(eq; robots.capture().body, body);
    retained(&fixture.store, &robots)?;
    Ok(())
}

#[tokio::test]
async fn owned_robots_403_is_retained_and_blocks_directory() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let reply = Reply {
        status: 403,
        ..Reply::robots(b"denied")
    };
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
    let (requests, result) = tokio::try_join!(serve(listener, [reply]), fetch)?;
    check!(
        matches!(result, Err(FetchError::Http { status: 403, url }) if url == format!("{ORIGIN}/robots.txt"))
    );
    check!(eq; requests.len(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    no_directory_attempt(
        &fixture.store,
        &fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?,
    )?;
    let robots = held_robots(&fixture)?;
    check!(eq; robots.capture().status, 403);
    check!(eq; robots.capture().body, b"denied");
    retained(&fixture.store, &robots)?;
    Ok(())
}

#[tokio::test]
async fn robots_html_is_not_an_allow_policy_and_404_remains_an_owned_allow() -> TestResult {
    for status in [200, 404] {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let fixture = Fixture::new(&listener)?;
        let fetch =
            async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
        if status == 200 {
            let reply = Reply {
                content_type: "text/html",
                ..Reply::robots(b"<html>login</html>")
            };
            let (requests, result) = tokio::try_join!(serve(listener, [reply]), fetch)?;
            check!(matches!(result, Err(FetchError::Policy { detail }) if
                detail == "robots endpoint returned HTML or an unsupported document"));
            check!(eq; requests.len(), 1);
            no_directory_attempt(
                &fixture.store,
                &fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?,
            )?;
        } else {
            let reply = Reply {
                status: 404,
                ..Reply::robots(b"")
            };
            let (requests, result) =
                tokio::try_join!(serve(listener, [reply, Reply::directory(DIRECTORY)]), fetch)?;
            let captured = result?;
            check!(eq; requests.len(), 2);
            check!(eq; captured.robots().capture().status, 404);
            retained(&fixture.store, captured.robots())?;
            retained(&fixture.store, captured.directory())?;
        }
    }
    Ok(())
}

fn held_robots(fixture: &Fixture) -> TestResult<crate::net::CapturedFetch> {
    let work = fixture.work(PublicResourceRootKindV1::Robots)?;
    let grant =
        fixture
            .writer
            .continuation_for_public_resource(&fixture.store, &fixture.owner, &work)?;
    let owned = fixture
        .store
        .register_acquisition_work(&fixture.owner, work, grant)?;
    let decision = fixture
        .store
        .inspect_acquisition_work(&owned, &mut census_store::AcquisitionReadBudget::for_work())?;
    let census_store::AcquireDecision::Replay(reference) = decision else {
        return Err("robots response did not retain its owned result".into());
    };
    Ok(crate::net::acquisition::read::replay(
        &fixture.store,
        &reference,
    )?)
}
