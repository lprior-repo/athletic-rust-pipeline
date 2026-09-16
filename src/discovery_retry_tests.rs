use super::*;

fn config() -> Result<DiscoveryConfig> {
    let config: crate::config::Config = toml::from_str(include_str!("../config.exhaustive.toml"))?;
    Ok(config.discovery)
}

#[tokio::test(start_paused = true)]
async fn concurrent_admission_preserves_global_spacing() -> Result<()> {
    let mut config = config()?;
    config.search_delay_ms = 500;
    let client = AthleticNetClient::new(&config)?;
    let start = Instant::now();
    let slot = || async {
        client.wait_for_request_slot().await;
        Instant::now().duration_since(start)
    };
    let (a, b, c) = tokio::join!(slot(), slot(), slot());
    let mut slots = [a, b, c];
    slots.sort();
    assert_eq!(
        slots,
        [
            Duration::ZERO,
            Duration::from_millis(500),
            Duration::from_secs(1)
        ]
    );
    client.defer_requests(Duration::from_secs(7)).await;
    let deferred = Instant::now();
    client.wait_for_request_slot().await;
    assert_eq!(deferred.elapsed(), Duration::from_secs(7));
    Ok(())
}

#[tokio::test]
async fn forbidden_opens_circuit_without_retry_or_sibling_reset() -> Result<()> {
    let mut server = mockito::Server::new_async().await;
    let denied = server
        .mock("POST", "/")
        .with_status(403)
        .expect(1)
        .create_async()
        .await;
    let mut config = config()?;
    config.athletic_search_url = server.url();
    config.search_delay_ms = 0;
    let client = AthleticNetClient::new(&config)?;
    let request = SearchRequest::new("Ada Runner", "a:tf", 0);
    let first = client.execute(&request).await;
    assert!(matches!(
        first,
        Err(SearchFailure {
            status: Some(403),
            attempts: 1,
            ..
        })
    ));
    let second = client.execute(&request).await;
    assert!(matches!(second, Err(SearchFailure { attempts: 0, .. })));
    denied.assert_async().await;
    Ok(())
}
