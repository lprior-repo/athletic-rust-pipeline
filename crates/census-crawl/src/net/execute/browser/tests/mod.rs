use super::*;
use crate::net::bridge::BrowserResponse;
use crate::net::cache::{content_digest, read_cache};
use crate::net::FetchOptions;
use base64::Engine as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../../../fixtures/wire/athleticnet-browser-capture.json");
const FAILURE_FIXTURE: &str =
    include_str!("../../../../../../../fixtures/wire/athleticnet-browser-failure.json");

const HOST: &str = "www.athletic.net";

const URL: &str =
    "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf";

fn fetcher_in(dir: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        dir.join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?)
}

struct Coordinates {
    body_path: PathBuf,
    meta_path: PathBuf,
    options: FetchOptions,
    representation: crate::net::RepresentationHeaders,
}

impl Coordinates {
    fn for_get(fetcher: &Fetcher) -> Self {
        let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
        Self {
            body_path,
            meta_path,
            options: FetchOptions::default(),
            representation: crate::net::RepresentationHeaders::default(),
        }
    }

    fn plan<'a>(&'a self, method: &'a str) -> FetchPlan<'a> {
        FetchPlan {
            method,
            url: URL,
            payload: None,
            host: HOST,
            family: None,
            body_path: &self.body_path,
            meta_path: &self.meta_path,
            cached: None,
            options: &self.options,
            representation: &self.representation,
            timeout_secs: 45,
        }
    }
}

fn capture_of(status: u16, content_type: &str, body: &[u8]) -> BrowserCapture {
    BrowserCapture {
        response: BrowserResponse {
            status,
            response_url: None,
            headers: vec![("content-type".to_string(), content_type.to_string())],
            body: BASE64.encode(body),
            rankings: None,
        },
        challenge: false,
        retry_after_ms: None,
        fetched_at_ms: Some(1_758_542_400_000),
    }
}

#[test]
fn a_challenge_capture_is_archived_and_never_cached_as_the_source_answer() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let fetcher = fetcher_in(dir.path())?;
    let coordinates = Coordinates::for_get(&fetcher);
    let plan = coordinates.plan("GET");
    let capture: BrowserCapture =
        match serde_json::from_str(CAPTURE_FIXTURE)? {
            BrowserOutcome::Captured(capture) => capture,
            other => return Err(format!("the capture fixture is a capture, got {other:?}").into()),
        };
    check!(
        capture.challenge,
        "the shared fixture is a challenge capture, which is the case this test is about"
    );

    let error = match fetcher.refuse_challenge(&plan, &capture).await {
        Err(error) => error,
        Ok(_) => return Err("a challenge capture is not evidence".into()),
    };

    match error {
        FetchError::BrowserLane {
            retryable, detail, ..
        } => {
            check!(
                !retryable,
                "a person clears a challenge; another invocation cannot"
            );
            check!(
                detail.contains("human verification"),
                "the refusal names what it saw: {detail}"
            );
            check!(
                detail.contains("403"),
                "the capture's status travels in the detail, because the row's status cannot carry it: {detail}"
            );
        }
        other => return Err(format!("expected a lane refusal, got {other:?}").into()),
    }
    check!(
        !coordinates.body_path.exists() && !coordinates.meta_path.exists(),
        "a challenge page must not be cached as the source's answer"
    );
    let challenged = BASE64.decode(capture.response.body.as_str())?;
    let archived = dir
        .path()
        .join("http/archive/bodies")
        .join(format!("{}.body", content_digest(&challenged)));
    check!(eq;
        std::fs::read(&archived)?,
        challenged,
        "the challenge page is retained as a refusal capture, byte for byte"
    );

    let rows = fetcher.access_conditions().await;
    check!(eq; rows.len(), 1, "one refusal, one row");
    check!(eq; rows[0].kind, AccessBlockKind::HumanRequired);
    check!(eq; rows[0].host, HOST);
    check!(eq; rows[0].status, 0, "a lane refusal is not an HTTP status");
    check!(eq;
        rows[0].retry_after_seconds,
        Some(30),
        "what the capture asked for is evidence"
    );
    check!(eq;
        rows[0].cooldown_until, None,
        "a profile that wants a person is cleared by a person, so the row does not expire"
    );
    Ok(())
    })
}

#[test]
fn a_capture_mints_the_evidence_an_http_body_would() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");
            let body = b"<html>meet results</html>".to_vec();
            let capture = capture_of(200, "text/html; charset=utf-8", &body);

            let outcome = fetcher.mint_capture(&plan, capture).await?;

            check!(eq; outcome.status, 200);
            check!(eq; outcome.url, URL);
            check!(eq; outcome.method, "GET");
            check!(!outcome.from_cache);
            check!(eq; outcome.bytes, body.len());
            check!(eq; outcome.body, body);
            check!(eq;
                outcome.content_type.as_deref(),
                Some("text/html; charset=utf-8"),
                "the capture's header is the receipt's"
            );
            check!(eq;
                outcome.fetched_at, "2025-09-22T12:00:00Z",
                "the transport's own stamp, in the format every other receipt uses"
            );
            let expected = "3198b794c2de260c8e302cd23fc86a0c84fb2c9a30885c616e527944ac5320e9";
            check!(eq;
                outcome.content_digest, expected,
                "the digest is the body's, so content ids match the HTTP path's"
            );

            let (meta, _) = read_cache(
                &coordinates.body_path,
                &coordinates.meta_path,
                "GET",
                URL,
                &coordinates.representation,
            )?
            .ok_or("the capture's body is cached")?;
            check!(eq; meta.status, 200);
            check!(eq; meta.content_digest, outcome.content_digest);
            check!(eq; meta.bytes, body.len());
            check!(eq; meta.fetched_at, outcome.fetched_at);
            check!(meta.etag.is_none() && meta.last_modified.is_none());
            check!(eq;
                fetcher.stats.lock().await.bytes_downloaded,
                u64::try_from(body.len())?,
            );
            check!(
                fetcher.access_conditions().await.is_empty(),
                "a capture is the source's answer, not an observation about the host"
            );
            Ok(())
        })
}

#[test]
fn a_browser_refusal_status_is_archived_without_replacing_a_success() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");
            let success = b"<html>roster</html>".to_vec();
            fetcher
                .mint_capture(&plan, capture_of(200, "text/html; charset=utf-8", &success))
                .await?;

            let refusal = b"<html>maintenance</html>".to_vec();
            let error = match fetcher
                .accept_capture(&plan, capture_of(429, "text/html; charset=utf-8", &refusal))
                .await
            {
                Err(error) => error,
                Ok(outcome) => {
                    return Err(format!("a 429 capture is a refusal, got {outcome:?}").into())
                }
            };
            check!(
                matches!(error, FetchError::Http { status: 429, .. }),
                "the refusal keeps its status: {error}"
            );
            check!(error.retryable(), "a throttled source stays retryable: {error}");

            let archived = dir
                .path()
                .join("http/archive/bodies")
                .join(format!("{}.body", content_digest(&refusal)));
            check!(eq;
                std::fs::read(&archived)?,
                refusal,
                "the refusal body is retained byte for byte"
            );
            let (meta, served) = read_cache(
                &coordinates.body_path,
                &coordinates.meta_path,
                "GET",
                URL,
                &coordinates.representation,
            )?
            .ok_or("the earlier success is still cached")?;
            check!(eq; meta.status, 200);
            check!(eq; served, success);
            check!(eq;
                fetcher.access_conditions().await.len(),
                1,
                "a throttled capture leaves one access condition"
            );
            Ok(())
        })
}

#[test]
fn a_body_over_the_ceiling_is_refused_before_it_is_allocated() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");
            let over = "A".repeat(MAX_BODY_BYTES / 3 * 4 + 4);
            let capture = BrowserCapture {
                response: BrowserResponse {
                    status: 200,
                    response_url: None,
                    headers: vec![("content-type".to_string(), "text/html".to_string())],
                    body: over,
                    rankings: None,
                },
                challenge: false,
                retry_after_ms: None,
                fetched_at_ms: Some(1_758_542_400_000),
            };

            let error = match fetcher.mint_capture(&plan, capture).await {
                Err(error) => error,
                Ok(_) => return Err("a body over the ceiling is not evidence".into()),
            };

            check!(
                matches!(error, FetchError::TooLarge { .. }),
                "expected the ceiling's own refusal, got {error:?}"
            );
            check!(
                !coordinates.body_path.exists() && !coordinates.meta_path.exists(),
                "nothing over the ceiling is written"
            );
            Ok(())
        })
}

#[test]
fn a_non_get_is_an_invariant_not_a_source_observation() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("POST");

            let error = match fetcher.fetch_browser(Arc::new(Mutex::new(())), &plan).await {
                Err(error) => error,
                Ok(_) => return Err("the lane cannot carry a POST".into()),
            };

            check!(
                matches!(error, FetchError::Invariant { .. }),
                "expected the routing fault, got {error:?}"
            );
            check!(
                fetcher.access_conditions().await.is_empty(),
                "a routing fault is not an observation about the host"
            );
            Ok(())
        })
}

#[test]
fn a_host_with_no_lane_is_refused_by_name() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");

            let error = match fetcher.fetch_browser(Arc::new(Mutex::new(())), &plan).await {
                Err(error) => error,
                Ok(_) => return Err("nothing is installed to fetch it".into()),
            };

            match error {
                FetchError::BrowserLane {
                    url,
                    detail: _,
                    retryable,
                } => {
                    check!(eq; url, URL);
                    check!(!retryable, "a missing lane is terminal, not retryable");
                }
                other => return Err(format!("expected a lane refusal, got {other:?}").into()),
            }
            let rows = fetcher.access_conditions().await;
            check!(eq; rows.len(), 1, "one refusal, one row");
            check!(eq; rows[0].kind, AccessBlockKind::BrowserUnavailable);
            check!(eq; rows[0].host, HOST);
            check!(eq; rows[0].status, 0, "a lane error is not an HTTP status");
            check!(eq;
                rows[0].retry_after_seconds, None,
                "the table has no retry-after for a missing lane"
            );
            Ok(())
        })
}

#[test]
fn a_capture_whose_final_document_is_foreign_is_refused_before_evidence() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");
            let body = b"<html>somewhere else</html>".to_vec();
            let mut foreign = capture_of(200, "text/html; charset=utf-8", &body);
            foreign.response.response_url = Some("https://other.example/landing".into());

            let error = match fetcher.accept_capture(&plan, foreign).await {
                Err(error) => error,
                Ok(_) => return Err("a foreign final document is not this source's answer".into()),
            };

            match error {
                FetchError::Policy { detail } => check!(
                    detail.contains("not admitted"),
                    "the refusal says the final document was not admitted: {detail}"
                ),
                other => return Err(format!("expected a policy refusal, got {other:?}").into()),
            }
            check!(
                !coordinates.body_path.exists() && !coordinates.meta_path.exists(),
                "a refused capture is never cached as the source's answer"
            );
            check!(
                fetcher.access_conditions().await.is_empty(),
                "a foreign document is not an observation about the host"
            );

            let mut served = capture_of(200, "text/html; charset=utf-8", &body);
            served.response.response_url = Some(URL.into());
            let outcome = fetcher.accept_capture(&plan, served).await?;
            check!(
                eq;
                outcome.status, 200,
                "the source's own document is admitted as evidence"
            );
            Ok(())
        })
}

mod cache;
