use super::{request_headers, TestResult};
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex, Notify};

mod origin_tests;

const SYNTHETIC_QA_BODY: &str = "synthetic transport policy QA evidence";

type RequestLedger = Vec<(String, String)>;

struct RedirectObservation {
    outcome: Result<FetchOutcome, FetchError>,
    requests: RequestLedger,
    requested_url: String,
    destination_url: String,
}

fn pinned_fetcher(cache: &std::path::Path, host: &str, address: SocketAddr) -> TestResult<Fetcher> {
    pinned_fetcher_with(
        cache,
        &[(host, address)],
        vec!["127.0.0.1".to_string(), host.to_string()],
    )
}

fn pinned_fetcher_with(
    cache: &std::path::Path,
    pins: &[(&str, SocketAddr)],
    grants: Vec<String>,
) -> TestResult<Fetcher> {
    let mut fetcher = Fetcher::new(cache, None, Duration::ZERO, HashMap::new(), grants)?;
    let builder = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
        .timeout(Duration::from_secs(5));
    fetcher.client = pins
        .iter()
        .fold(builder, |builder, (host, address)| {
            builder.resolve(host, *address)
        })
        .build()?;
    Ok(fetcher)
}

async fn serve_synthetic_redirect(
    listener: TcpListener,
    destination_url: &str,
    body: &str,
    mut finished: oneshot::Receiver<()>,
) -> TestResult<RequestLedger> {
    let mut requests = Vec::new();
    for _ in 0..8 {
        let (mut socket, _) = tokio::select! {
            biased;
            accepted = listener.accept() => accepted?,
            completed = &mut finished => {
                completed?;
                return Ok(requests);
            }
        };
        let headers = request_headers(&mut socket).await?;
        let path = headers
            .split_whitespace()
            .nth(1)
            .ok_or("missing request path")?;
        let host = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("host"))
            .map(|(_, value)| value.trim())
            .ok_or("missing request host")?;
        requests.push((path.to_string(), host.to_string()));
        let response = match path {
            "/start" => format!(
                "HTTP/1.1 302 Found\r\nLocation: {destination_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            "/destination" if headers.to_ascii_lowercase().contains("if-none-match: \"capture-v1\"") =>
                "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            "/destination" => format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"capture-v1\"\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
            other => return Err(format!("unexpected synthetic request path {other}").into()),
        };
        socket.write_all(response.as_bytes()).await?;
    }
    Err("synthetic redirect exceeded request bound".into())
}

#[derive(Default)]
struct SyntheticObservation {
    requests: RequestLedger,
    inflight: usize,
    max_inflight: usize,
    unexpected: Option<String>,
}

type SharedObservation = Arc<Mutex<SyntheticObservation>>;
type ArrivalSender = Arc<Mutex<Option<oneshot::Sender<()>>>>;

type ArrivalChannel = (ArrivalSender, oneshot::Receiver<()>);

fn arrival_channel() -> ArrivalChannel {
    let (sender, receiver) = oneshot::channel();
    (Arc::new(Mutex::new(Some(sender))), receiver)
}

async fn serve_gated(
    listener: TcpListener,
    observation: SharedObservation,
    destination_url: String,
    arrival: ArrivalSender,
    release: Arc<Notify>,
) {
    while let Ok((mut socket, _)) = listener.accept().await {
        let observation = Arc::clone(&observation);
        let destination_url = destination_url.clone();
        let arrival = Arc::clone(&arrival);
        let release = Arc::clone(&release);
        tokio::spawn(async move {
            let Ok(headers) = request_headers(&mut socket).await else {
                return;
            };
            let Some(path) = headers.split_whitespace().nth(1).map(str::to_string) else {
                return;
            };
            let host = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("host"))
                .map_or_else(String::new, |(_, value)| value.trim().to_string());
            {
                let mut state = observation.lock().await;
                state.requests.push((path.clone(), host));
            }
            let response = match path.as_str() {
                "/start" => format!(
                    "HTTP/1.1 302 Found\r\nLocation: {destination_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ),
                "/destination" | "/first" => {
                    {
                        let mut state = observation.lock().await;
                        state.inflight += 1;
                        state.max_inflight = state.max_inflight.max(state.inflight);
                    }
                    let first = arrival.lock().await.take();
                    if let Some(first) = first {
                        let _ = first.send(());
                        release.notified().await;
                    }
                    {
                        let mut state = observation.lock().await;
                        state.inflight -= 1;
                    }
                    if path == "/first" {
                        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{SYNTHETIC_QA_BODY}",
                            SYNTHETIC_QA_BODY.len()
                        )
                    }
                }
                "/limited" =>
                    "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
                "/other" => format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{SYNTHETIC_QA_BODY}",
                    SYNTHETIC_QA_BODY.len()
                ),
                other => {
                    observation.lock().await.unexpected = Some(other.to_string());
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                }
            };
            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
}

async fn observe_redirect(destination_host: &str) -> TestResult<RedirectObservation> {
    let root = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let requested_url = format!("http://{address}/start");
    let destination_url = format!("http://{destination_host}:{}/destination", address.port());
    let fetcher = pinned_fetcher(root.path(), destination_host, address)?;
    let (finished, completion) = oneshot::channel();
    let acquire = async {
        let outcome = fetcher.get(&requested_url, &FetchOptions::default()).await;
        finished
            .send(())
            .map_err(|()| "synthetic server ended before acquisition")?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
    };
    let (requests, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::try_join!(
            serve_synthetic_redirect(listener, &destination_url, SYNTHETIC_QA_BODY, completion),
            acquire
        )
    })
    .await??;
    Ok(RedirectObservation {
        outcome,
        requests,
        requested_url,
        destination_url,
    })
}

#[test]
fn authorized_browser_redirect_is_refused_before_direct_http_destination_dispatch() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let observation = observe_redirect("www.athletic.net").await?;
            let destination_requests = observation
                .requests
                .iter()
                .filter(|(path, _)| path == "/destination")
                .count();
            check!(eq;
                destination_requests, 0,
                "browser-only destination received direct HTTP"
            );
            check!(eq;
                observation
                    .requests
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>(),
                ["/start"]
            );
            match observation.outcome {
                Err(FetchError::Policy { detail }) => {
                    check!(
                        detail.contains("browser-transport"),
                        "expected browser-transport refusal: {detail}"
                    );
                }
                other => {
                    return Err(
                        format!("expected direct-HTTP redirect refusal, got {other:?}").into(),
                    )
                }
            }
            Ok(())
        })
}

#[test]
fn authorized_http_redirect_still_dispatches_and_returns_destination_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let observation = observe_redirect("www.piaa.org").await?;
            let capture = observation.outcome?;
            check!(eq; capture.status, 200);
            check!(eq; capture.body, SYNTHETIC_QA_BODY.as_bytes());
            check!(eq; capture.url, observation.requested_url);
            check!(eq;
                capture.response_url.as_deref(),
                Some(observation.destination_url.as_str())
            );
            check!(eq;
                observation
                    .requests
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>(),
                ["/start", "/destination"]
            );
            let destination_host = url::Url::parse(&observation.destination_url)?
                .authority()
                .to_string();
            check!(eq;
                observation
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/destination")
                    .map(|(_, host)| host.as_str())
                    .collect::<Vec<_>>(),
                [destination_host.as_str()]
            );
            Ok(())
        })
}

#[test]
fn a_redirect_hop_to_a_host_under_cooldown_is_refused_before_dispatch() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let source_host = "www.piaa.org";
            let destination_host = "example.test";
            let requested = format!("http://{source_host}:{}/start", address.port());
            let destination = format!("http://{destination_host}:{}/destination", address.port());
            let fetcher = pinned_fetcher(root.path(), source_host, address)?;
            fetcher
                .record_access_condition(
                    destination_host,
                    census_domain::model::AccessBlockKind::RateLimited,
                    429,
                    Some(60),
                    "rate limited before redirect dispatch",
                )
                .await;

            let (finished, completion) = oneshot::channel();
            let acquisition = async {
                let result = fetcher.get(&requested, &FetchOptions::default()).await;
                finished
                    .send(())
                    .map_err(|()| "server ended before acquisition")?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(result)
            };
            let (requests, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(
                    serve_synthetic_redirect(listener, &destination, SYNTHETIC_QA_BODY, completion),
                    acquisition
                )
            })
            .await??;

            check!(eq;
                requests
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>(),
                ["/start"]
            );
            check!(
                matches!(outcome, Err(FetchError::Policy { .. })),
                "a cooled-down redirect destination must be refused: {outcome:?}"
            );
            Ok(())
        })
}

#[test]
fn an_unlisted_redirect_destination_is_refused_before_contact() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let requested_url = format!("http://{address}/start");
            let destination = format!("http://unlisted.example:{}/hidden", address.port());
            let fetcher = pinned_fetcher(root.path(), "www.piaa.org", address)?;
            let (finished, completion) = oneshot::channel();
            let acquisition = async {
                let result = fetcher.get(&requested_url, &FetchOptions::default()).await;
                finished
                    .send(())
                    .map_err(|()| "server ended before acquisition")?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(result)
            };
            let (requests, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(
                    serve_synthetic_redirect(listener, &destination, SYNTHETIC_QA_BODY, completion),
                    acquisition
                )
            })
            .await??;

            check!(eq;
                requests
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>(),
                ["/start"]
            );
            check!(
                matches!(outcome, Err(FetchError::Policy { .. })),
                "an unlisted redirect destination must be refused: {outcome:?}"
            );
            Ok(())
        })
}

#[test]
fn a_redirect_hop_to_an_origin_held_elsewhere_is_refused_before_contact() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let cache = tempfile::tempdir()?;
            let locks = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let requested_url = format!("http://{address}/start");
            let destination_url = format!("http://www.piaa.org:{}/destination", address.port());
            let origin = format!("http://www.piaa.org:{}", address.port());
            let holder_path = locks
                .path()
                .join(crate::net::origin_lock_file_name(&origin));
            let mut holder = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&holder_path)?;
            holder.try_lock()?;
            std::io::Write::write_all(&mut holder, br#"{"pid":424242}"#)?;
            let fetcher = pinned_fetcher(cache.path(), "www.piaa.org", address)?
                .with_origin_locks(locks.path().to_path_buf());
            let (finished, completion) = oneshot::channel();
            let acquisition = async {
                let outcome = fetcher.get(&requested_url, &FetchOptions::default()).await;
                finished
                    .send(())
                    .map_err(|()| "server ended before acquisition")?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(outcome)
            };
            let (requests, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(
                    serve_synthetic_redirect(
                        listener,
                        &destination_url,
                        SYNTHETIC_QA_BODY,
                        completion
                    ),
                    acquisition
                )
            })
            .await??;

            check!(
                matches!(outcome, Err(FetchError::OriginHeld { .. })),
                "a redirect hop into a foreign origin hold must be refused: {outcome:?}"
            );
            check!(eq;
                requests
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>(),
                ["/start"]
            );
            Ok(())
        })
}

#[test]
fn two_origins_redirecting_to_one_destination_share_its_inflight_gate() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let cache = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let destination_url = format!("http://www.piaa.org:{}/destination", address.port());
            let fetcher = Arc::new(pinned_fetcher_with(
                cache.path(),
                &[
                    ("source-a.example", address),
                    ("source-b.example", address),
                    ("www.piaa.org", address),
                ],
                vec![
                    "source-a.example".to_string(),
                    "source-b.example".to_string(),
                    "www.piaa.org".to_string(),
                ],
            )?);
            let observation: SharedObservation =
                Arc::new(Mutex::new(SyntheticObservation::default()));
            let (arrival, arrived) = arrival_channel();
            let release = Arc::new(Notify::new());
            let server = tokio::spawn(serve_gated(
                listener,
                Arc::clone(&observation),
                destination_url,
                arrival,
                Arc::clone(&release),
            ));
            let first = tokio::spawn({
                let fetcher = Arc::clone(&fetcher);
                let url = format!("http://source-a.example:{}/start", address.port());
                async move { fetcher.get(&url, &FetchOptions::default()).await }
            });
            let second = tokio::spawn({
                let fetcher = Arc::clone(&fetcher);
                let url = format!("http://source-b.example:{}/start", address.port());
                async move { fetcher.get(&url, &FetchOptions::default()).await }
            });
            arrived
                .await
                .map_err(|_| "the destination never received a request")?;
            tokio::time::sleep(Duration::from_millis(1200)).await;
            {
                let state = observation.lock().await;
                let destinations = state
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/destination")
                    .count();
                check!(eq; destinations, 1,
                    "a second destination dispatch reached the server while the first was in flight"
                );
                check!(eq; state.max_inflight, 1,
                    "the destination received concurrent dispatches"
                );
            }
            release.notify_waiters();
            let first = tokio::time::timeout(Duration::from_secs(10), first).await???;
            let second = tokio::time::timeout(Duration::from_secs(10), second).await???;
            check!(eq; first.body, SYNTHETIC_QA_BODY.as_bytes());
            check!(eq; second.body, SYNTHETIC_QA_BODY.as_bytes());
            {
                let state = observation.lock().await;
                check!(eq; state.unexpected, None);
                let destinations = state
                    .requests
                    .iter()
                    .filter(|(path, _)| path == "/destination")
                    .count();
                check!(eq; destinations, 2);
            }
            server.abort();
            Ok(())
        })
}

#[test]
fn a_request_queued_behind_a_429_is_refused_at_the_last_admission_point() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let cache = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let fetcher = Arc::new(pinned_fetcher(cache.path(), "www.piaa.org", address)?);
            let observation: SharedObservation =
                Arc::new(Mutex::new(SyntheticObservation::default()));
            let (arrival, arrived) = arrival_channel();
            let release = Arc::new(Notify::new());
            let server = tokio::spawn(serve_gated(
                listener,
                Arc::clone(&observation),
                String::new(),
                arrival,
                Arc::clone(&release),
            ));
            let first_url = format!("http://www.piaa.org:{}/first", address.port());
            let second_url = format!("http://www.piaa.org:{}/other", address.port());
            let first = tokio::spawn({
                let fetcher = Arc::clone(&fetcher);
                async move { fetcher.get(&first_url, &FetchOptions::default()).await }
            });
            arrived
                .await
                .map_err(|_| "the first request never arrived")?;
            let second = tokio::spawn({
                let fetcher = Arc::clone(&fetcher);
                async move { fetcher.get(&second_url, &FetchOptions::default()).await }
            });
            tokio::task::yield_now().await;
            release.notify_waiters();
            let first = tokio::time::timeout(Duration::from_secs(10), first).await??;
            let second = tokio::time::timeout(Duration::from_secs(10), second).await??;
            check!(
                matches!(first, Err(FetchError::Http { status: 429, .. })),
                "{first:?}"
            );
            check!(
                matches!(&second, Err(FetchError::Cooldown { host }) if host == "www.piaa.org"),
                "{second:?}"
            );
            {
                let state = observation.lock().await;
                check!(eq; state.unexpected, None);
                check!(eq;
                    state
                        .requests
                        .iter()
                        .map(|(path, _)| path.as_str())
                        .collect::<Vec<_>>(),
                    ["/first"]
                );
            }
            server.abort();
            Ok(())
        })
}

#[test]
fn fetchers_sharing_an_origin_budget_share_recorded_cooldowns() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let cache = tempfile::tempdir()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let shared = Arc::new(crate::net::PacingState::new());
            let first = pinned_fetcher(cache.path(), "www.piaa.org", address)?
                .with_shared_pacing(Arc::clone(&shared));
            let second = pinned_fetcher(cache.path(), "www.piaa.org", address)?
                .with_shared_pacing(Arc::clone(&shared));
            let observation: SharedObservation =
                Arc::new(Mutex::new(SyntheticObservation::default()));
            let (arrival, _arrived) = arrival_channel();
            let release = Arc::new(Notify::new());
            let server = tokio::spawn(serve_gated(
                listener,
                Arc::clone(&observation),
                String::new(),
                arrival,
                Arc::clone(&release),
            ));
            let limited = first
                .get(
                    &format!("http://www.piaa.org:{}/limited", address.port()),
                    &FetchOptions::default(),
                )
                .await;
            check!(
                matches!(limited, Err(FetchError::Http { status: 429, .. })),
                "{limited:?}"
            );
            let refused = second
                .get(
                    &format!("http://www.piaa.org:{}/other", address.port()),
                    &FetchOptions::default(),
                )
                .await;
            check!(
                matches!(refused, Err(FetchError::Cooldown { ref host }) if host == "www.piaa.org"),
                "a fetcher sharing the origin budget must observe the recorded cooldown: {refused:?}"
            );
            {
                let state = observation.lock().await;
                check!(eq; state.unexpected, None);
                check!(eq;
                    state
                        .requests
                        .iter()
                        .map(|(path, _)| path.as_str())
                        .collect::<Vec<_>>(),
                    ["/limited"]
                );
            }
            server.abort();
            Ok(())
        })
}
