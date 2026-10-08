use super::super::{acquire, assert_capture, BODY};
use super::{pinned_fetcher_with, serve_synthetic_redirect, TestResult};
use crate::net::cache::read_cache;
use crate::net::{FetchError, FetchOptions, Fetcher};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

mod refusal;

#[test]
fn empty_grants_same_origin_redirect_preserves_archive_cache_offline_and_304_provenance(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let cache = root.path().join("http");
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let origin = format!("http://source.example:{}", address.port());
            let requested = format!("{origin}/start");
            let destination = format!("{origin}/destination");
            let fetcher = pinned_fetcher_with(&cache, &[("source.example", address)], Vec::new())?;
            check!(eq; fetcher.authorized_hosts, Vec::<String>::new());
            let (finished, completion) = oneshot::channel();
            let acquisition = async {
                let acquired_at = acquire(&fetcher, &cache, &requested, &destination).await?;
                finished
                    .send(())
                    .map_err(|()| "fixture server stopped early")?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(acquired_at)
            };
            let (requests, acquired_at) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::try_join!(
                    serve_synthetic_redirect(
                        listener,
                        &destination,
                        std::str::from_utf8(BODY)?,
                        completion
                    ),
                    acquisition
                )
            })
            .await??;
            check!(eq;
                requests,
                [
                    "/robots.txt",
                    "/start",
                    "/destination",
                    "/start",
                    "/destination"
                ]
                .map(|path| (
                    path.to_string(),
                    format!("source.example:{}", address.port())
                ))
            );
            check!(eq; fetcher.stats().await.conditional_304, 1);
            let (body_path, meta_path) =
                fetcher.cache_paths(&Fetcher::key_for("GET", &requested, ""));
            let (meta, body) = read_cache(
                &body_path,
                &meta_path,
                "GET",
                &requested,
                &crate::net::RepresentationHeaders::default(),
            )?
            .ok_or("requested cache missing")?;
            check!(eq; meta.url, requested);
            check!(eq; meta.response_url.as_deref(), Some(destination.as_str()));
            check!(eq; meta.fetched_at, acquired_at);
            check!(eq; meta.etag.as_deref(), Some("\"capture-v1\""));
            check!(eq; meta.bytes, BODY.len());
            check!(eq; body, BODY);
            let offline = fetcher.with_offline(true);
            let replay = offline.get(&requested, &FetchOptions::default()).await?;
            assert_capture(&replay, &requested, &destination)?;
            check!(eq; replay.method, "GET");
            check!(eq; replay.bytes, BODY.len());
            check!(eq; replay.from_cache, true);
            check!(eq; replay.fetched_at, acquired_at);
            match offline.get(&destination, &FetchOptions::default()).await {
                Err(FetchError::Offline { url }) => check!(eq; url, destination),
                other => {
                    return Err(
                        format!("final URL incorrectly became a cache key: {other:?}").into(),
                    )
                }
            }
            Ok(())
        })
}
