use crate::net::bridge::BrowserLane;
use crate::net::{FetchError, FetchOptions, FetchOutcome, FetchStats, Fetcher};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const URL: &str =
    "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf";

fn capture(challenge: bool, body: &str, final_url: Option<&str>) -> Value {
    json!({
        "outcome": "captured", "challenge": challenge,
        "retry_after_ms": null, "fetched_at_ms": 1758542400000_u64,
        "response": {
            "status": 200, "headers": [], "body": body,
            "response_url": final_url, "rankings": null,
        },
    })
}

async fn observe(answer: Value) -> TestResult<(Result<FetchOutcome, FetchError>, FetchStats)> {
    let root = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let transport = reqwest::Client::builder().no_proxy().build()?;
    let lane = BrowserLane::over(crate::ingress::client(origin.parse()?, transport)?);
    let fetcher = Fetcher::new(
        root.path(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_browser_lane(lane);
    let body = serde_json::to_string(&answer)?;
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let mut request = [0_u8; 4096];
        let count = socket.read(&mut request).await?;
        let received = std::str::from_utf8(request.get(..count).ok_or("read boundary")?)?;
        check!(received.starts_with("POST /BrowserSession/profile-0/fetch "));
        let response = format!(
            "HTTP/1.1 200 OK\r\nx-restate-id: inv_accounting_fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let options = FetchOptions::default();
    let (served, outcome) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server, fetcher.get(URL, &options))
    })
    .await?;
    served?;
    Ok((outcome, fetcher.stats().await))
}

fn assert_charged(stats: &FetchStats) -> TestResult {
    check!(eq; stats.physical_requests(), 1);
    check!(eq; stats.cache_hits, 0);
    check!(eq;
        stats.per_host.get("www.athletic.net").ok_or("browser host charge missing")?.physical_requests(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn a_browser_challenge_charges_the_attempt_that_observed_it() -> TestResult {
    let (outcome, stats) = observe(capture(true, "eA==", None)).await?;
    check!(
        matches!(
            &outcome,
            Err(FetchError::BrowserLane {
                retryable: false,
                ..
            })
        ),
        "{outcome:?}"
    );
    check!(eq; stats.challenges, 1, "{outcome:?}");
    assert_charged(&stats)
}

#[tokio::test]
async fn a_browser_transport_failure_keeps_its_single_attempt_charge() -> TestResult {
    let (outcome, stats) = observe(json!({
        "outcome": "failed", "error": "transport", "verdict": "retryable",
    }))
    .await?;
    check!(
        matches!(
            &outcome,
            Err(FetchError::BrowserLane {
                retryable: true,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert_charged(&stats)
}

#[tokio::test]
async fn a_malformed_browser_capture_does_not_erase_the_attempt() -> TestResult {
    let (outcome, stats) = observe(capture(false, "not base64!", None)).await?;
    check!(
        matches!(&outcome, Err(FetchError::Invariant { detail }) if detail.contains("not base64")),
        "{outcome:?}"
    );
    assert_charged(&stats)
}

#[tokio::test]
async fn a_browser_capture_from_a_refused_destination_keeps_the_attempt_charge() -> TestResult {
    let (outcome, stats) =
        observe(capture(false, "eA==", Some("https://foreign.test/source"))).await?;
    check!(
        matches!(&outcome, Err(FetchError::Policy { detail }) if detail.contains("not admitted")),
        "{outcome:?}"
    );
    assert_charged(&stats)
}
