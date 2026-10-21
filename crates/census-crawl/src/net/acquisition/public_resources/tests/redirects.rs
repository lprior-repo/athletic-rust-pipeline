use super::*;
use assertions::{exact_hop, retained, Witness};
use server::{serve, Reply};

#[tokio::test]
async fn allowed_redirect_retains_every_actual_hop_and_replays_without_an_extra_robots_fetch(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await?) };
    let final_reply = Reply {
        path: "/teams",
        ..Reply::directory(DIRECTORY)
    };
    let (requests, captured) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::robots(ALLOW),
                Reply::redirect("Location: /teams\r\n"),
                final_reply,
            ]
        ),
        fetch
    )?;
    check!(eq; requests.len(), 3);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    exact_hop(
        captured.directory(),
        0,
        &format!("{ORIGIN}/teams"),
        302,
        0,
        "text/html",
    )?;
    exact_hop(
        captured.directory(),
        1,
        &format!("{ORIGIN}/teams"),
        200,
        DIRECTORY.len(),
        "text/html",
    )?;
    let observed = captured
        .directory()
        .observed_http()?
        .ok_or("redirect observations")?;
    check!(eq; observed.hops().len(), 2);
    let first = observed.hops().first().ok_or("redirect first hop")?;
    let second = observed.hops().get(1).ok_or("redirect second hop")?;
    check!(eq; first.dispatch().attempt(), second.dispatch().attempt());
    check!(eq; first.dispatch().hop().get(), 0);
    check!(eq; second.dispatch().hop().get(), 1);
    check!(first
        .response()
        .headers()
        .iter()
        .any(|header| header.name() == "location" && header.value() == b"/teams"));
    let original = retained(&fixture.store, captured.directory())?;
    let before = Witness::capture(&fixture.store)?;
    let replayed = fixture.acquire().await?;
    check!(eq; retained(&fixture.store, replayed.directory())?, original);
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    Ok(())
}

#[tokio::test]
async fn disallowed_redirect_is_refused_before_the_target_dispatch() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
    let (requests, result) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::robots(ALLOW),
                Reply::redirect("Location: /private\r\n"),
            ]
        ),
        fetch
    )?;
    let digest = crate::net::cache::content_digest(ALLOW);
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail ==
        format!("robots disallows {ORIGIN}/private; policy={ORIGIN}/robots.txt sha256={digest}"))
    );
    check!(eq; requests.len(), 2);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    let work = fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let prefix = format!("work/{}/attempt/1/dispatch/", work.id()?);
    let keys = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    check!(keys
        .iter()
        .any(|key| key.starts_with(&format!("{prefix}0/"))));
    check!(keys
        .iter()
        .all(|key| !key.starts_with(&format!("{prefix}1/"))));
    Ok(())
}

#[tokio::test]
async fn robots_allowed_path_cannot_substitute_the_native_directory_resource() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let fetch =
        async { Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture.acquire().await) };
    let (requests, result) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::robots(ALLOW),
                Reply::redirect("Location: /teams/next\r\n")
            ],
        ),
        fetch,
    )?;
    check!(
        matches!(result, Err(FetchError::CaptureStore(error)) if matches!(error.as_ref(),
        census_store::StoreError::Capture(census_store::CaptureError::Domain(
            CaptureDomainError::RequestTargetMismatch))))
    );
    check!(eq; requests.len(), 2);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    let work = fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let prefix = format!("work/{}/attempt/1/dispatch/", work.id()?);
    let keys = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    check!(keys
        .iter()
        .any(|key| key.starts_with(&format!("{prefix}0/"))));
    check!(keys
        .iter()
        .all(|key| !key.starts_with(&format!("{prefix}1/"))));
    Ok(())
}
