use super::*;
use crate::net::bridge::BrowserResponse;
use crate::net::cache::read_cache;
use crate::net::FetchOptions;
use base64::Engine as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../../../fixtures/wire/athleticnet-browser-capture.json");
const FAILURE_FIXTURE: &str =
    include_str!("../../../../../../../fixtures/wire/athleticnet-browser-failure.json");

const HOST: &str = "www.athletic.net";

const URL: &str =
    "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf";

fn fetcher_in(dir: &Path) -> Fetcher {
    Fetcher::new(
        dir.join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
}

struct Coordinates {
    body_path: PathBuf,
    meta_path: PathBuf,
    options: FetchOptions,
}

impl Coordinates {
    fn for_get(fetcher: &Fetcher) -> Self {
        let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
        Self {
            body_path,
            meta_path,
            options: FetchOptions::default(),
        }
    }

    fn plan<'a>(&'a self, method: &'a str) -> FetchPlan<'a> {
        FetchPlan {
            method,
            url: URL,
            payload: None,
            host: HOST,
            body_path: &self.body_path,
            meta_path: &self.meta_path,
            cached: None,
            options: &self.options,
            timeout_secs: 45,
        }
    }
}

fn capture_of(status: u16, content_type: &str, body: &[u8]) -> BrowserCapture {
    BrowserCapture {
        response: BrowserResponse {
            status,
            headers: vec![("content-type".to_string(), content_type.to_string())],
            body: BASE64.encode(body),
            rankings: None,
        },
        challenge: false,
        retry_after_ms: None,
        fetched_at_ms: Some(1_758_542_400_000),
    }
}

#[tokio::test]
async fn a_challenge_capture_leaves_a_human_required_row_and_no_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    let capture: BrowserCapture =
        match serde_json::from_str(CAPTURE_FIXTURE).expect("fixture decodes") {
            BrowserOutcome::Captured(capture) => capture,
            other => panic!("the capture fixture is a capture, got {other:?}"),
        };
    assert!(
        capture.challenge,
        "the shared fixture is a challenge capture, which is the case this test is about"
    );

    let error = fetcher
        .refuse_challenge(&plan, &capture)
        .await
        .expect_err("a challenge capture is not evidence");

    match error {
        FetchError::BrowserLane {
            retryable, detail, ..
        } => {
            assert!(
                !retryable,
                "a person clears a challenge; another invocation cannot"
            );
            assert!(
                detail.contains("human verification"),
                "the refusal names what it saw: {detail}"
            );
            assert!(
                detail.contains("403"),
                "the capture's status travels in the detail, because the row's status cannot carry it: {detail}"
            );
        }
        other => panic!("expected a lane refusal, got {other:?}"),
    }
    assert!(
        !coordinates.body_path.exists() && !coordinates.meta_path.exists(),
        "a challenge page must not be cached as the source's answer"
    );

    let rows = fetcher.access_conditions().await;
    assert_eq!(rows.len(), 1, "one refusal, one row");
    assert_eq!(rows[0].kind, AccessBlockKind::HumanRequired);
    assert_eq!(rows[0].host, HOST);
    assert_eq!(rows[0].status, 0, "a lane refusal is not an HTTP status");
    assert_eq!(
        rows[0].retry_after_seconds,
        Some(30),
        "what the capture asked for is evidence"
    );
    assert_eq!(
        rows[0].cooldown_until, None,
        "a profile that wants a person is cleared by a person, so the row does not expire"
    );
}

#[tokio::test]
async fn a_capture_mints_the_evidence_an_http_body_would() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    let body = b"<html>meet results</html>".to_vec();
    let capture = capture_of(200, "text/html; charset=utf-8", &body);

    let outcome = fetcher
        .mint_capture(&plan, capture)
        .await
        .expect("a 200 capture is the source's answer");

    assert_eq!(outcome.status, 200);
    assert_eq!(outcome.url, URL);
    assert_eq!(outcome.method, "GET");
    assert!(!outcome.from_cache);
    assert_eq!(outcome.bytes, body.len());
    assert_eq!(outcome.body, body);
    assert_eq!(
        outcome.content_type.as_deref(),
        Some("text/html; charset=utf-8"),
        "the capture's header is the receipt's"
    );
    assert_eq!(
        outcome.fetched_at, "2025-09-22T12:00:00Z",
        "the transport's own stamp, in the format every other receipt uses"
    );
    let expected = "3198b794c2de260c8e302cd23fc86a0c84fb2c9a30885c616e527944ac5320e9";
    assert_eq!(
        outcome.content_digest, expected,
        "the digest is the body's, so content ids match the HTTP path's"
    );

    let (meta, _) = read_cache(&coordinates.body_path, &coordinates.meta_path)
        .expect("read cache")
        .expect("the capture's body is cached");
    assert_eq!(meta.status, 200);
    assert_eq!(meta.content_digest, outcome.content_digest);
    assert_eq!(meta.bytes, body.len());
    assert_eq!(meta.fetched_at, outcome.fetched_at);
    assert!(meta.etag.is_none() && meta.last_modified.is_none());
    assert_eq!(
        fetcher.stats.lock().await.bytes_downloaded,
        u64::try_from(body.len()).expect("small body"),
    );
    assert!(
        fetcher.access_conditions().await.is_empty(),
        "a capture is the source's answer, not an observation about the host"
    );
}

#[tokio::test]
async fn a_body_over_the_ceiling_is_refused_before_it_is_allocated() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    let over = "A".repeat(MAX_BODY_BYTES / 3 * 4 + 4);
    let capture = BrowserCapture {
        response: BrowserResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "text/html".to_string())],
            body: over,
            rankings: None,
        },
        challenge: false,
        retry_after_ms: None,
        fetched_at_ms: Some(1_758_542_400_000),
    };

    let error = fetcher
        .mint_capture(&plan, capture)
        .await
        .expect_err("a body over the ceiling is not evidence");

    assert!(
        matches!(error, FetchError::TooLarge { .. }),
        "expected the ceiling's own refusal, got {error:?}"
    );
    assert!(
        !coordinates.body_path.exists() && !coordinates.meta_path.exists(),
        "nothing over the ceiling is written"
    );
}

#[tokio::test]
async fn a_non_get_is_an_invariant_not_a_source_observation() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("POST");

    let error = fetcher
        .fetch_browser(Arc::new(Mutex::new(())), &plan)
        .await
        .expect_err("the lane cannot carry a POST");

    assert!(
        matches!(error, FetchError::Invariant { .. }),
        "expected the routing fault, got {error:?}"
    );
    assert!(
        fetcher.access_conditions().await.is_empty(),
        "a routing fault is not an observation about the host"
    );
}

#[tokio::test]
async fn a_host_with_no_lane_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");

    let error = fetcher
        .fetch_browser(Arc::new(Mutex::new(())), &plan)
        .await
        .expect_err("nothing is installed to fetch it");

    match error {
        FetchError::BrowserLane {
            url,
            detail: _,
            retryable,
        } => {
            assert_eq!(url, URL);
            assert!(!retryable, "a missing lane is terminal, not retryable");
        }
        other => panic!("expected a lane refusal, got {other:?}"),
    }
    let rows = fetcher.access_conditions().await;
    assert_eq!(rows.len(), 1, "one refusal, one row");
    assert_eq!(rows[0].kind, AccessBlockKind::BrowserUnavailable);
    assert_eq!(rows[0].host, HOST);
    assert_eq!(rows[0].status, 0, "a lane error is not an HTTP status");
    assert_eq!(
        rows[0].retry_after_seconds, None,
        "the table has no retry-after for a missing lane"
    );
}
mod cache;
