use super::{capture_of, fetcher_in, Coordinates, TestResult, FAILURE_FIXTURE};
use crate::net::bridge::{BrowserError, BrowserFailure, BrowserOutcome, Verdict};
use crate::net::cache::read_cache;
use crate::net::{AccessBlockKind, FetchError};

#[test]
fn a_failure_is_graded_as_the_transport_graded_it() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let coordinates = Coordinates::for_get(&fetcher);
            let plan = coordinates.plan("GET");

            let retryable = match fetcher
                .refuse_failure(&plan, BrowserError::Timeout, Verdict::Retryable)
                .await
            {
                Err(error) => error,
                Ok(_) => return Err("a failure is not evidence".into()),
            };
            check!(
                retryable.retryable(),
                "the transport's own retryable verdict survives the seat: {retryable:?}"
            );
            check!(
                fetcher.access_conditions().await.is_empty(),
                "a transport fault is not an observation about the host"
            );

            let unavailable = match fetcher
                .refuse_failure(&plan, BrowserError::Unavailable, Verdict::Terminal)
                .await
            {
                Err(error) => error,
                Ok(_) => return Err("a failure is not evidence".into()),
            };
            check!(!unavailable.retryable());
            let rows = fetcher.access_conditions().await;
            check!(eq; rows.len(), 1);
            check!(eq; rows[0].kind, AccessBlockKind::BrowserUnavailable);

            let failure: BrowserFailure = match serde_json::from_str(FAILURE_FIXTURE)? {
                BrowserOutcome::Failed(failure) => failure,
                other => {
                    return Err(format!("the failure fixture is a failure, got {other:?}").into())
                }
            };
            check!(eq;
                failure.verdict,
                Verdict::HumanRequired,
                "the shared failure fixture is the human-required case"
            );
            let human = match fetcher
                .refuse_failure(&plan, failure.error, failure.verdict)
                .await
            {
                Err(error) => error,
                Ok(_) => return Err("a failure is not evidence".into()),
            };
            check!(!human.retryable());
            let rows = fetcher.access_conditions().await;
            check!(eq; rows.len(), 2);
            let human_row = rows
                .iter()
                .find(|row| row.kind == AccessBlockKind::HumanRequired)
                .ok_or("the human requirement left a row")?;
            check!(eq; human_row.cooldown_until, None);
            check!(eq; human_row.retry_after_seconds, None);
            Ok(())
        })
}

#[test]
fn an_allowed_404_capture_is_observed_but_not_cached() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let mut coordinates = Coordinates::for_get(&fetcher);
            coordinates.options.allow_not_found = true;
            let plan = coordinates.plan("GET");
            let body = b"<html>not found</html>".to_vec();
            let capture = capture_of(404, "text/html; charset=utf-8", &body);

            let outcome = fetcher.handle_404_capture(&plan, capture).await?;

            check!(eq; outcome.status, 404);
            check!(eq; outcome.body, body);
            check!(eq;
                outcome.content_digest,
                "23c62f7de04040a4949182f3ca7f84a99b7106c3040e11734aae9e1329c0691b"
            );
            check!(read_cache(&coordinates.body_path, &coordinates.meta_path)?.is_none());
            check!(!coordinates.body_path.exists());
            check!(!coordinates.meta_path.exists());
            Ok(())
        })
}

#[test]
fn a_disallowed_404_capture_returns_err() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = fetcher_in(dir.path())?;
            let mut coordinates = Coordinates::for_get(&fetcher);
            coordinates.options.allow_not_found = false;
            let plan = coordinates.plan("GET");
            let body = b"<html>not found</html>".to_vec();
            let capture = capture_of(404, "text/html; charset=utf-8", &body);

            let error = match fetcher.handle_404_capture(&plan, capture).await {
                Err(error) => error,
                Ok(_) => return Err("a 404 capture without allow_not_found is Err".into()),
            };

            check!(
                matches!(error, FetchError::Http { status: 404, .. }),
                "expected Http(404), got {error:?}"
            );

            check!(read_cache(&coordinates.body_path, &coordinates.meta_path)?.is_none());
            check!(!coordinates.body_path.exists());
            check!(!coordinates.meta_path.exists());
            Ok(())
        })
}
