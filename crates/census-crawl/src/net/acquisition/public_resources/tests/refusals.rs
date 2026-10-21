use super::*;
use assertions::Witness;
use std::time::Duration;

#[tokio::test]
async fn foreign_cache_refuses_before_registration_or_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let foreign = tempfile::tempdir()?;
    fixture.fetcher.cache_dir = foreign.path().to_path_buf();
    let before = Witness::capture(&fixture.store)?;
    check!(matches!(fixture.acquire().await,
        Err(FetchError::CaptureStore(error)) if matches!(error.as_ref(),
            census_store::StoreError::Capture(census_store::CaptureError::CacheRootMismatch))));
    unchanged(&fixture, &listener, &before).await
}

#[tokio::test]
async fn changed_directory_headers_or_intent_refuse_before_registration_or_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let before = Witness::capture(&fixture.store)?;
    for options in [
        FetchOptions {
            refresh: true,
            ..FetchOptions::default()
        },
        FetchOptions {
            headers: vec![("accept".into(), "application/json".into())],
            ..FetchOptions::default()
        },
    ] {
        let result = fixture
            .fetcher
            .get_public_resource_directory(&fixture.context(), &options)
            .await;
        check!(matches!(
            result,
            Err(FetchError::Capture(CaptureTransportError::RequestMismatch))
        ));
        unchanged(&fixture, &listener, &before).await?;
    }
    Ok(())
}

#[tokio::test]
async fn separately_bound_robots_headers_refuse_before_either_root_is_registered() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let robots = FetchOptions {
        headers: vec![("accept".into(), "text/plain".into())],
        ..FetchOptions::default()
    };
    let fixture = Fixture::with_robots_options(&listener, &robots)?;
    let before = Witness::capture(&fixture.store)?;
    check!(matches!(
        fixture.acquire().await,
        Err(FetchError::Capture(CaptureTransportError::RequestMismatch))
    ));
    unchanged(&fixture, &listener, &before).await
}

async fn unchanged(fixture: &Fixture, listener: &TcpListener, before: &Witness) -> TestResult {
    check!(eq; &Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    check!(
        tokio::time::timeout(Duration::from_millis(10), listener.accept())
            .await
            .is_err()
    );
    Ok(())
}
