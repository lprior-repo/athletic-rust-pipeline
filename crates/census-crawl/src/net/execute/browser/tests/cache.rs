use super::{capture_of, Coordinates, Fetcher, FAILURE_FIXTURE};
use crate::net::bridge::{BrowserError, BrowserFailure, BrowserOutcome, Verdict};
use crate::net::cache::read_cache;
use crate::net::{AccessBlockKind, FetchError};

fn fetcher_in(dir: &std::path::Path) -> Fetcher {
    Fetcher::new(
        dir.join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
}

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

#[tokio::test]
async fn an_allowed_404_capture_is_observed_but_not_cached() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let mut coordinates = Coordinates::for_get(&fetcher);
    coordinates.options.allow_not_found = true;
    let plan = coordinates.plan("GET");
    let body = b"<html>not found</html>".to_vec();
    let capture = capture_of(404, "text/html; charset=utf-8", &body);

    let outcome = fetcher
        .handle_404_capture(&plan, capture)
        .await
        .expect("a 404 capture with allow_not_found is Ok");

    assert_eq!(outcome.status, 404);
    assert_eq!(outcome.body, body);
    assert_eq!(
        outcome.content_digest,
        "23c62f7de04040a4949182f3ca7f84a99b7106c3040e11734aae9e1329c0691b"
    );
    assert!(read_cache(&coordinates.body_path, &coordinates.meta_path)
        .expect("read cache")
        .is_none());
    assert!(!coordinates.body_path.exists());
    assert!(!coordinates.meta_path.exists());
}

#[tokio::test]
async fn a_disallowed_404_capture_returns_err() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path());
    let mut coordinates = Coordinates::for_get(&fetcher);
    coordinates.options.allow_not_found = false;
    let plan = coordinates.plan("GET");
    let body = b"<html>not found</html>".to_vec();
    let capture = capture_of(404, "text/html; charset=utf-8", &body);

    let error = fetcher
        .handle_404_capture(&plan, capture)
        .await
        .expect_err("a 404 capture without allow_not_found is Err");

    assert!(
        matches!(error, FetchError::Http { status: 404, .. }),
        "expected Http(404), got {error:?}"
    );

    assert!(read_cache(&coordinates.body_path, &coordinates.meta_path)
        .expect("read cache")
        .is_none());
    assert!(!coordinates.body_path.exists());
    assert!(!coordinates.meta_path.exists());
}
