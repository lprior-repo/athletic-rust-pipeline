use super::{request_headers, TestResult};
use crate::net::destination_guard::GuardedResolver;
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

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
    let redirects = Arc::clone(&fetcher.destination);
    let resolver: Arc<dyn reqwest::dns::Resolve> =
        Arc::new(GuardedResolver(Arc::clone(&fetcher.destination)));
    let builder = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .dns_resolver(resolver);
    fetcher.client = pins
        .iter()
        .fold(builder, |builder, (host, address)| {
            builder.resolve(host, *address)
        })
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            redirects.redirect(attempt)
        }))
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
            "/robots.txt" =>
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
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
                ["/robots.txt", "/start"]
            );
            match observation.outcome {
                Err(FetchError::Transport { url, source }) => {
                    check!(eq; url, observation.requested_url);
                    check!(
                        source.is_redirect(),
                        "expected redirect-policy refusal: {source}"
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
                ["/robots.txt", "/start", "/destination"]
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
