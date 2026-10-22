use super::*;

#[tokio::test]
async fn granted_foreign_port_redirect_is_refused_before_dispatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let foreign = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/start", listener.local_addr()?);
    let redirect = format!("HTTP/1.1 302 Found\r\nLocation: http://{}/public\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", foreign.local_addr()?);
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, vec![("/start", redirect)]),
            fetcher.get_public_http(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("authorized origin"))
    );
    check!(
        tokio::time::timeout(Duration::from_millis(10), foreign.accept())
            .await
            .is_err()
    );
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn actual_access_denials_have_one_attempt_and_no_robots_request() -> TestResult {
    for status in [401, 403, 429] {
        let dir = tempfile::tempdir()?;
        let fetcher = fetcher(dir.path())?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/school", listener.local_addr()?);
        let options = FetchOptions::default();
        let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(
                serve(listener, vec![("/school", response(status, "denied"))]),
                fetcher.get_public_http(&url, &options)
            )
        })
        .await?;
        served?;
        check!(matches!(result, Err(FetchError::Http { status: actual, .. }) if actual == status));
        check!(eq; fetcher.stats().await.physical_requests(), 1);
        if status != 401 {
            check!(
                fetcher
                    .host_blocked("127.0.0.1", &crate::net::now_iso8601())
                    .await
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn login_redirect_cannot_dispatch_the_authentication_target() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/start", listener.local_addr()?);
    let redirect =
        "HTTP/1.1 302 Found\r\nLocation: /login\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_owned();
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, vec![("/start", redirect)]),
            fetcher.get_public_http(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("authentication"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn browser_provider_credentials_and_private_destinations_refuse_without_dispatch(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    for url in [
        "https://tfrrs.org/results",
        "https://www.athletic.net/",
        "https://user:secret@school.example/",
        "http://192.168.1.1/",
    ] {
        check!(matches!(
            fetcher.get_public_http(url, &FetchOptions::default()).await,
            Err(FetchError::Policy { .. })
        ));
    }
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn cached_foreign_final_destination_is_refused_without_rewriting_original_bytes() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?.with_offline(true);
    let url = "http://127.0.0.1/start";
    let (body, meta) = cache(&fetcher, url, "http://127.0.0.1:8443/final", b"historical")?;
    let original = (std::fs::read(&body)?, std::fs::read(&meta)?);
    check!(
        matches!(fetcher.get_public_http(url, &FetchOptions::default()).await,
        Err(FetchError::Policy { detail }) if detail.contains("authorized origin"))
    );
    check!(eq; (std::fs::read(&body)?, std::fs::read(&meta)?), original);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn configured_pacing_remains_independent_of_public_http_scope() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path(),
        None,
        Duration::from_secs(3),
        HashMap::new(),
        vec!["127.0.0.1".to_owned()],
    )?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/school", listener.local_addr()?);
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, vec![("/school", response(200, "public"))]),
            fetcher.get_public_http(&url, &options)
        )
    })
    .await?;
    served?;
    result?;
    let buckets = fetcher.pacing.hosts.lock().await;
    let state = buckets.get("127.0.0.1").ok_or("configured host pacing")?;
    check!(eq; state.delay, Duration::from_secs(3));
    check!(state.next_allowed.is_some());
    Ok(())
}

#[tokio::test]
async fn refreshed_304_cannot_replay_a_foreign_final_destination() -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher(dir.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/start", listener.local_addr()?);
    let bytes = b"historical public page";
    let (body_path, _) = cache(&fetcher, &url, "https://foreign.example/final", bytes)?;
    let not_modified =
        "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned();
    let options = FetchOptions {
        refresh: true,
        headers: vec![("If-None-Match".to_owned(), "\"previous-page\"".to_owned())],
        ..FetchOptions::default()
    };
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            serve(listener, vec![("/start", not_modified)]),
            fetcher.get_public_http(&url, &options)
        )
    })
    .await?;
    served?;
    check!(
        matches!(result, Err(FetchError::Policy { detail }) if detail.contains("authorized origin"))
    );
    check!(eq; fetcher.stats().await.physical_requests(), 1);
    check!(eq; std::fs::read(&body_path)?, bytes);
    Ok(())
}
