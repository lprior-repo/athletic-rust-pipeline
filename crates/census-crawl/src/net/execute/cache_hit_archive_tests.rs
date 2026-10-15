use super::{offline_fetcher, TestResult};
use crate::net::cache::content_digest;
use crate::net::{FetchError, FetchOptions, Fetcher};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const URL: &str = "https://example.test/legacy";
const CAPTURED: &str = "2026-09-27T12:34:56Z";
const BODY: &[u8] = b"original retained bytes";

fn metadata(body: &[u8]) -> Value {
    json!({
        "url": URL, "method": "GET", "status": 200,
        "content_digest": content_digest(body), "bytes": body.len(),
        "fetched_at": CAPTURED,
        "provider_capture": { "request_id": "historical-request" }
    })
}

fn seed(fetcher: &Fetcher, value: &Value, body: &[u8], extra: &str) -> TestResult {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, extra));
    std::fs::write(body_path, body)?;
    std::fs::write(meta_path, serde_json::to_vec(value)?)?;
    Ok(())
}

fn archived_metadata(
    root: &Path,
    value: &Value,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let mut canonical = value.clone();
    canonical.sort_all_objects();
    let digest = content_digest(&serde_json::to_vec_pretty(&canonical)?);
    Ok(root
        .join("archive/captures")
        .join(content_digest(BODY))
        .join(format!("{digest}.meta.json")))
}

async fn no_hits(fetcher: &Fetcher) -> TestResult {
    let stats = fetcher.stats().await;
    check!(eq; (stats.requests, stats.cache_hits, stats.physical_requests()), (0, 0, 0));
    Ok(())
}

#[tokio::test]
async fn unarchived_legacy_cache_hit_preserves_unknown_and_absent_metadata_without_redating(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let value = metadata(BODY);
    seed(&fetcher, &value, BODY, "")?;
    let (_, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    let original = std::fs::read(&meta_path)?;
    check!(!root.path().join("archive").exists());
    let outcome = fetcher.get(URL, &FetchOptions::default()).await?;
    check!(eq; outcome.body, BODY);
    check!(eq; outcome.fetched_at, CAPTURED);
    check!(outcome.from_cache);
    check!(outcome.response_url.is_none());
    let archived = archived_metadata(root.path(), &value)?;
    let encoded = std::fs::read(&archived)?;
    let retained: Value = serde_json::from_slice(&encoded)?;
    check!(eq; retained, value);
    for absent in [
        "response_url",
        "redirects",
        "representation",
        "content_type",
    ] {
        check!(retained.get(absent).is_none());
    }
    let body_path = root
        .path()
        .join("archive/bodies")
        .join(format!("{}.body", content_digest(BODY)));
    check!(eq; std::fs::read(body_path)?, BODY);
    check!(eq; std::fs::read(&meta_path)?, original);
    let modified = std::fs::metadata(&archived)?.modified()?;
    let replay = fetcher.get(URL, &FetchOptions::default()).await?;
    check!(eq; replay.fetched_at, CAPTURED);
    check!(eq; std::fs::read(&archived)?, encoded);
    check!(eq; std::fs::metadata(&archived)?.modified()?, modified);
    check!(eq; std::fs::read_dir(archived.parent().ok_or("metadata parent")?)?.count(), 1);
    let stats = fetcher.stats().await;
    check!(eq; (stats.requests, stats.cache_hits, stats.physical_requests()), (2, 2, 0));
    Ok(())
}

#[tokio::test]
async fn cache_hit_preserves_explicit_redirects_and_representation() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let options = FetchOptions {
        headers: vec![("Accept".into(), "text/html".into())],
        ..FetchOptions::default()
    };
    let representation = super::super::representation::canonical_request(URL, &options.headers)?;
    let mut value = metadata(BODY);
    value["representation"] = serde_json::to_value(&representation)?;
    value["response_url"] = json!("https://example.test/final");
    value["redirects"] = json!([{ "status": 302, "url": "https://example.test/final" }]);
    seed(&fetcher, &value, BODY, &representation.identity())?;
    let outcome = fetcher.get(URL, &options).await?;
    check!(eq; outcome.response_url.as_deref(), Some("https://example.test/final"));
    check!(eq; outcome.fetched_at, CAPTURED);
    let retained: Value =
        serde_json::from_slice(&std::fs::read(archived_metadata(root.path(), &value)?)?)?;
    check!(eq; retained, value);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn cache_hit_archive_failure_is_typed_and_precedes_hit_accounting() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let value = metadata(BODY);
    seed(&fetcher, &value, BODY, "")?;
    std::fs::write(root.path().join("archive"), b"occupied archive path")?;
    match fetcher.get(URL, &FetchOptions::default()).await {
        Err(FetchError::Cache { source, .. }) => {
            check!(eq; source.kind(), std::io::ErrorKind::InvalidData)
        }
        other => return Err(format!("archive failure was not returned: {other:?}").into()),
    }
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    check!(eq; std::fs::read(body_path)?, BODY);
    check!(eq; std::fs::read(meta_path)?, serde_json::to_vec(&value)?);
    no_hits(&fetcher).await
}

#[tokio::test]
async fn cache_hit_expanded_metadata_limit_refuses_without_quarantining_the_verified_pair(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let mut value = metadata(BODY);
    value["provider_capture"] = json!({ "values": vec![0_u8; 10_000] });
    check!(serde_json::to_vec(&value)?.len() < 64 * 1024);
    check!(serde_json::to_vec_pretty(&value)?.len() > 64 * 1024);
    seed(&fetcher, &value, BODY, "")?;
    match fetcher.get(URL, &FetchOptions::default()).await {
        Err(FetchError::Encode { .. }) => {}
        other => {
            return Err(format!("metadata retention refusal was not returned: {other:?}").into())
        }
    }
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    check!(eq; std::fs::read(body_path)?, BODY);
    check!(eq; std::fs::read(meta_path)?, serde_json::to_vec(&value)?);
    check!(!root.path().join("quarantine").exists());
    check!(!root
        .path()
        .join("archive/bodies")
        .join(format!("{}.body", content_digest(BODY)))
        .exists());
    no_hits(&fetcher).await
}

#[tokio::test]
async fn changed_cache_between_lookup_and_preservation_is_a_miss() -> TestResult {
    use crate::net::cache::archive_lock_test::{self, LockEvent};

    let replacement = b"later retained bytes";
    let mut expanded = metadata(replacement);
    expanded["provider_capture"] = json!({ "values": vec![0_u8; 10_000] });
    for value in [metadata(replacement), expanded] {
        let root = tempfile::tempdir()?;
        let fetcher = offline_fetcher(root.path())?;
        seed(&fetcher, &metadata(BODY), BODY, "")?;
        let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
        let encoded = serde_json::to_vec(&value)?;
        let changed_body = body_path.clone();
        let _observation = archive_lock_test::observe(move |_, event| {
            if event == LockEvent::BeforeAcquire {
                std::fs::write(&changed_body, replacement).map_err(|source| FetchError::Cache {
                    path: changed_body.clone(),
                    source,
                })?;
                std::fs::write(&meta_path, &encoded).map_err(|source| FetchError::Cache {
                    path: meta_path.clone(),
                    source,
                })?;
            }
            Ok(())
        })?;
        check!(matches!(
            fetcher.get(URL, &FetchOptions::default()).await,
            Err(FetchError::Offline { .. })
        ));
        check!(eq; std::fs::read(body_path)?, replacement);
        check!(!root.path().join("archive/bodies").exists());
        no_hits(&fetcher).await?;
    }
    Ok(())
}

#[test]
fn cache_replay_retains_damage_misses_and_typed_envelope_refusal() -> TestResult {
    use crate::net::cache::{replay_cache, CacheMeta};
    use crate::net::CaptureTransportError;

    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let value = metadata(BODY);
    let expected: CacheMeta = serde_json::from_value(value.clone())?;
    check!(replay_cache(&body_path, &meta_path, &expected)?.is_none());
    for (encoded, body) in [
        (b"invalid JSON".to_vec(), BODY.to_vec()),
        (serde_json::to_vec(&value)?, b"wrong body".to_vec()),
    ] {
        std::fs::write(&body_path, body)?;
        std::fs::write(&meta_path, encoded)?;
        check!(replay_cache(&body_path, &meta_path, &expected)?.is_none());
    }
    std::fs::write(&body_path, BODY)?;
    std::fs::write(&meta_path, br#"{"version":3}"#)?;
    check!(matches!(
        replay_cache(&body_path, &meta_path, &expected),
        Err(FetchError::Capture(
            CaptureTransportError::CacheEnvelopeUnsupported
        ))
    ));
    check!(!root.path().join("archive/bodies").exists());
    check!(!root.path().join("quarantine").exists());
    Ok(())
}

#[tokio::test]
async fn missing_legacy_retrieval_time_remains_an_offline_gap() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    let mut value = metadata(BODY);
    check!(value
        .as_object_mut()
        .ok_or("metadata object")?
        .remove("fetched_at")
        .is_some());
    seed(&fetcher, &value, BODY, "")?;
    check!(matches!(
        fetcher.get(URL, &FetchOptions::default()).await,
        Err(FetchError::Offline { .. })
    ));
    check!(!root.path().join("archive").exists());
    no_hits(&fetcher).await
}

#[tokio::test]
async fn refresh_does_not_archive_or_serve_a_legacy_cache_hit() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = offline_fetcher(root.path())?;
    seed(&fetcher, &metadata(BODY), BODY, "")?;
    let options = FetchOptions {
        refresh: true,
        ..FetchOptions::default()
    };
    check!(matches!(
        fetcher.get(URL, &options).await,
        Err(FetchError::Offline { .. })
    ));
    check!(!root.path().join("archive").exists());
    no_hits(&fetcher).await
}
