//! The browser lane's seat: what one capture becomes here, and what a refusal leaves behind.
//!
//! The wire fixtures under `fixtures/wire/` are the contract the two crates share, so they are read
//! back through the seat's own decisions rather than through a model of them: a challenge the
//! transport flagged must not arrive here as evidence, and the row it leaves must be the one the
//! source policy names. No test opens a socket — the decisions are driven directly, and the one
//! process-level path covered (a host with no lane installed) answers before any request exists.

use super::*;
use crate::net::bridge::{BrowserFailure, BrowserResponse};
use crate::net::cache::read_cache;
use crate::net::FetchOptions;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The transport's own challenge capture. The relative path climbs from this file's directory
/// (`crates/census-crawl/src/net/execute/browser`) to the repository root, where the fixture the
/// crate's own tests encode lives: one file, read by both sides.
const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../../fixtures/wire/athleticnet-browser-capture.json");
/// The transport's own failure answer, for the same reason.
const FAILURE_FIXTURE: &str =
    include_str!("../../../../../../fixtures/wire/athleticnet-browser-failure.json");

/// The host every plan here names: the origin the registry's Athletic.net row declares.
const HOST: &str = "www.athletic.net";

/// One athlete-bio address, in the shape the adapter builds.
const URL: &str =
    "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf";

/// A fetcher whose cache is a throwaway directory. It is built the way the client builds one, so the
/// lane field starts `None` — which is itself one of the cases tested below.
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

/// The cache coordinates and options one plan for [`URL`] runs with, owned here because [`FetchPlan`]
/// borrows them.
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

/// A capture of a `200` answer with `body`, as the transport would hand it over.
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

/// A challenge capture is the verification page, not the source's answer: it must never be minted as
/// evidence, and the row it leaves must be `human_required` with no cooldown to expire.
#[tokio::test]
async fn a_challenge_capture_leaves_a_human_required_row_and_no_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    // The fixture is the transport's whole answer, so it is read as one: the capture is the arm it
    // carries, and reading it through the envelope is what keeps the tag part of the contract.
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

/// A capture that is the source's answer mints the same evidence an HTTP body does: the bytes, their
/// digest, the content type, the receipt's own timestamp format, and the cached pair on disk.
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
    let mut hasher = Sha256::new();
    hasher.update(&body);
    assert_eq!(
        outcome.sha256,
        sha256_prefix16(hasher),
        "the digest is the body's, so content ids match the HTTP path's"
    );

    let meta = read_cache(&coordinates.body_path, &coordinates.meta_path)
        .expect("read cache")
        .expect("the capture's body is cached");
    assert_eq!(meta.status, 200);
    assert_eq!(meta.sha256, outcome.sha256);
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

/// The ceiling is enforced on the encoded length, before the decoder can allocate a body over it, and
/// nothing over the ceiling is written.
#[tokio::test]
async fn a_body_over_the_ceiling_is_refused_before_it_is_allocated() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    // Base64 is four characters per three bytes, so this encodes to just over the ceiling. The
    // allocation is the point: the guard exists so the decoder never sees a body this size.
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

/// The lane carries GETs and nothing else, so a non-GET reaching the seat is a routing fault that
/// fails loudly rather than being refused as though the source had said something.
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

/// A host the table routes to the browser, in a process with no lane installed, is refused by name and
/// recorded as the applicability row — the conditional half of the lane's policy.
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
            detail,
            retryable,
        } => {
            assert_eq!(url, URL);
            assert_eq!(url, URL);
            assert!(
                detail.contains("no browser lane is installed"),
                "the refusal says what is missing: {detail}"
            );
            assert!(!retryable);
        }
        other => panic!("expected a lane refusal, got {other:?}"),
    }

    let rows = fetcher.access_conditions().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, AccessBlockKind::BrowserUnavailable);
    assert_eq!(rows[0].status, 0);
    assert!(
        rows[0].cooldown_until.is_some(),
        "a lane that is not there yet may be there later, so this row expires"
    );
    assert!(
        !coordinates.body_path.exists(),
        "a refusal leaves no cached body behind"
    );
}

/// A failure verdict is read, not re-derived: retryable stays retryable, a missing browser is the
/// deployment row, and a human requirement becomes the same row a challenge leaves.
#[tokio::test]
async fn a_failure_is_graded_as_the_transport_graded_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");

    let retryable = fetcher
        .refuse_failure(&plan, BrowserError::Timeout, Verdict::Retryable)
        .await
        .expect_err("a failure is not evidence");
    assert!(
        retryable.retryable(),
        "the transport's own retryable verdict survives the seat: {retryable:?}"
    );
    assert!(
        fetcher.access_conditions().await.is_empty(),
        "a transport fault is not an observation about the host"
    );

    let unavailable = fetcher
        .refuse_failure(&plan, BrowserError::Unavailable, Verdict::Terminal)
        .await
        .expect_err("a failure is not evidence");
    assert!(!unavailable.retryable());
    let rows = fetcher.access_conditions().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, AccessBlockKind::BrowserUnavailable);

    let failure: BrowserFailure =
        match serde_json::from_str(FAILURE_FIXTURE).expect("fixture decodes") {
            BrowserOutcome::Failed(failure) => failure,
            other => panic!("the failure fixture is a failure, got {other:?}"),
        };
    assert_eq!(
        failure.verdict,
        Verdict::HumanRequired,
        "the shared failure fixture is the human-required case"
    );
    let human = fetcher
        .refuse_failure(&plan, failure.error, failure.verdict)
        .await
        .expect_err("a failure is not evidence");
    assert!(!human.retryable());
    let rows = fetcher.access_conditions().await;
    assert_eq!(rows.len(), 2);
    let human_row = rows
        .iter()
        .find(|row| row.kind == AccessBlockKind::HumanRequired)
        .expect("the human requirement left a row");
    assert_eq!(human_row.cooldown_until, None);
    assert_eq!(human_row.retry_after_seconds, None);
}
