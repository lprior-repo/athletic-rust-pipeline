use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn fetcher(root: &std::path::Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        root,
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["127.0.0.1".into()],
    )?)
}

async fn request_headers(socket: &mut TcpStream) -> TestResult<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    for _ in 0..16 {
        let count = socket.read(&mut buffer).await?;
        if count == 0 {
            return Err("request ended before headers".into());
        }
        bytes.extend_from_slice(buffer.get(..count).ok_or("request read boundary")?);
        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            return Ok(String::from_utf8(bytes)?);
        }
    }
    Err("request exceeded fixture bound".into())
}

async fn assert_attempts(fetcher: &Fetcher, expected: u64) -> TestResult {
    let stats = fetcher.stats().await;
    check!(eq; stats.physical_requests(), expected);
    check!(eq;
        stats.per_host.values().map(|host| host.physical_requests()).sum::<u64>(),
        expected
    );
    Ok(())
}

#[tokio::test]
async fn a_received_success_with_a_truncated_body_still_charges_one_attempt() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/truncated", listener.local_addr()?);
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        request_headers(&mut socket).await?;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nx")
            .await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server, fetcher.get(&url, &options))
    })
    .await?;
    served?;
    check!(matches!(result, Err(FetchError::Transport { .. })));
    assert_attempts(&fetcher, 1).await
}

#[tokio::test]
async fn two_failed_transports_against_one_origin_charge_each_attempt_once() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let server = async {
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await?;
            request_headers(&mut socket).await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let client = async {
        for path in ["first", "retry"] {
            check!(matches!(
                fetcher
                    .get(&format!("{origin}/{path}"), &FetchOptions::default())
                    .await,
                Err(FetchError::Transport { .. })
            ));
        }
        assert_attempts(&fetcher, 2).await
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(server, client)
    })
    .await??;
    Ok(())
}

#[tokio::test]
async fn cancellation_after_request_arrival_keeps_the_attempt_charge() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path())?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/cancelled", listener.local_addr()?);
    let options = FetchOptions::default();
    let request = fetcher.get(&url, &options);
    let arrival = async {
        let (mut socket, _) = listener.accept().await?;
        request_headers(&mut socket).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(socket)
    };
    let socket = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::select! {
            arrived = arrival => arrived,
            result = request => Err(format!("request completed before cancellation: {result:?}").into()),
        }
    })
    .await??;
    assert_attempts(&fetcher, 1).await?;
    drop(socket);
    Ok(())
}

#[tokio::test]
async fn an_admitted_redirect_charges_the_actual_destination_host() -> TestResult {
    let root = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let mut fetcher = Fetcher::new(
        root.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec![
            "127.0.0.1".into(),
            "source.test".into(),
            "destination.test".into(),
        ],
    )?;
    fetcher.client = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
        .resolve("source.test", address)
        .resolve("destination.test", address)
        .build()?;
    let url = format!("http://source.test:{}/start", address.port());
    let destination = format!("http://destination.test:{}/finish", address.port());
    let server = async {
        for response in [
            format!("HTTP/1.1 302 Found\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".into(),
        ] {
            let (mut socket, _) = listener.accept().await?;
            request_headers(&mut socket).await?;
            socket.write_all(response.as_bytes()).await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let options = FetchOptions::default();
    let (served, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server, fetcher.get(&url, &options))
    })
    .await?;
    served?;
    let outcome = outcome?;
    check!(eq; outcome.body, b"ok");
    assert_attempts(&fetcher, 2).await?;
    let stats = fetcher.stats().await;
    check!(eq; stats.per_host.get("source.test").ok_or("source charge missing")?.physical_requests(), 1);
    check!(eq; stats.per_host.get("destination.test").ok_or("destination charge missing")?.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn two_real_fetches_share_one_recorded_origin_without_replay_double_charge() -> TestResult {
    let root = tempfile::tempdir()?;
    let first = fetcher(&root.path().join("first"))?.with_origin_locks(root.path().join("locks"));
    let second = fetcher(&root.path().join("second"))?
        .with_shared_pacing(first.pacing_state())
        .with_origin_locks(root.path().join("locks"));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let first_url = format!("{origin}/first");
    let second_url = format!("{origin}/second");
    let server = async {
        let mut arrivals = Vec::new();
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await?;
            let request = request_headers(&mut socket).await?;
            arrivals.push((tokio::time::Instant::now(), request));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(arrivals)
    };
    let options = FetchOptions::default();
    let client = async {
        let (left, right) = tokio::join!(
            first.get(&first_url, &options),
            second.get(&second_url, &options),
        );
        check!(eq; left?.body, b"ok");
        check!(eq; right?.body, b"ok");
        let replay = first.get(&first_url, &options).await?;
        check!(replay.from_cache);
        check!(eq; replay.body, b"ok");
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let (arrivals, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(server, client)
    })
    .await??;
    let [(first_at, _), (second_at, _)] = arrivals.as_slice() else {
        return Err("expected exactly two physical requests".into());
    };
    let spacing = second_at.saturating_duration_since(*first_at);
    assert_attempts(&first, 1).await?;
    assert_attempts(&second, 1).await?;
    let first_stats = first.stats().await;
    let second_stats = second.stats().await;
    check!(eq; first_stats.requests, 2);
    check!(eq; first_stats.cache_hits, 1);
    check!(eq; second_stats.requests, 1);
    check!(eq; second_stats.cache_hits, 0);
    println!(
        "origin={origin} observed_requests=2 physical_charges=2 logical_attempts=3 cache_hits=1 spacing_ms={} lock_root={}",
        spacing.as_millis(), root.path().join("locks").display()
    );
    Ok(())
}
