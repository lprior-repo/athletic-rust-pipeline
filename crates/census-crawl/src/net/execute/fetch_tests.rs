use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn offline_fetcher(root: &std::path::Path) -> Result<Fetcher, FetchError> {
    Ok(Fetcher::new(
        root,
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["example.test".into()],
    )?
    .with_offline(true))
}

#[tokio::test]
async fn legacy_cache_replay_precedes_offline_refusal_and_preserves_all_public_fields() -> TestResult
{
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let url = "https://example.test/legacy";
    let body = b"retained original bytes";
    let meta = CacheMeta {
        url: url.into(),
        response_url: Some("https://example.test/retained-destination".into()),
        method: "GET".into(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-27T00:00:00Z".into(),
        content_type: Some("text/plain".into()),
        ..CacheMeta::default()
    };
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    write_cache(&body_path, &meta_path, body, &meta)?;
    let retained_meta = std::fs::read(&meta_path)?;
    let outcome = fetcher.get(url, &FetchOptions::default()).await?;
    check!(eq; outcome.url, meta.url);
    check!(eq; outcome.response_url, meta.response_url);
    check!(eq; outcome.method, meta.method);
    check!(eq; outcome.status, meta.status);
    check!(eq; outcome.content_digest, meta.content_digest);
    check!(eq; outcome.bytes, meta.bytes);
    check!(eq; outcome.fetched_at, meta.fetched_at);
    check!(eq; outcome.content_type, meta.content_type);
    check!(eq; outcome.body.as_slice(), body);
    check!(eq; outcome.from_cache, true);
    check!(eq; std::fs::read(&body_path)?.as_slice(), body);
    check!(eq; std::fs::read(&meta_path)?, retained_meta);
    let stats = fetcher.stats().await;
    check!(eq; stats.requests, 1);
    check!(eq; stats.cache_hits, 1);
    check!(eq; stats.physical_requests(), 0);
    check!(eq; stats.per_host.get("example.test").ok_or("host accounting")?.cache_hits, 1);
    Ok(())
}

#[tokio::test]
async fn invalid_conditional_headers_refuse_before_cache_reuse_offline_and_attempt_charging(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let url = "https://example.test/legacy";
    let body = b"retained original bytes";
    let meta = CacheMeta {
        url: url.into(),
        method: "GET".into(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-27T00:00:00Z".into(),
        ..CacheMeta::default()
    };
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    write_cache(&body_path, &meta_path, body, &meta)?;
    let retained_meta = std::fs::read(&meta_path)?;
    let options = FetchOptions {
        headers: vec![("If-None-Match".into(), "invalid\nvalidator".into())],
        ..FetchOptions::default()
    };
    match fetcher.get(url, &options).await {
        Err(FetchError::Policy { detail }) => {
            check!(eq; detail, "conditional validator If-None-Match exceeds its bounded HTTP value contract")
        }
        other => return Err(format!("invalid validator={other:?}").into()),
    }
    check!(eq; std::fs::read(body_path)?.as_slice(), body);
    check!(eq; std::fs::read(meta_path)?, retained_meta);
    let stats = fetcher.stats().await;
    check!(eq; stats.requests, 0);
    check!(eq; stats.cache_hits, 0);
    check!(eq; stats.physical_requests(), 0);
    check!(eq; stats.per_host.len(), 0);
    Ok(())
}
