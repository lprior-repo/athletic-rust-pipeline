use super::*;
use crate::outcome::Outcome;
use crate::restate_services::Jobs;
use crate::spawn::Spawner;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const WAIT: Duration = Duration::from_secs(5);

#[test]
fn dedicated_results_selector_and_tokio_dns_finish_with_one_blocking_thread() -> TestResult {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        let root = tempfile::tempdir()?;
        let store = Arc::new(Store::open(root.path())?);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://localhost:{}/capture", listener.local_addr()?.port());
        let mut server = tokio::task::JoinSet::new();
        server.spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut request = [0u8; 4096];
            let read = socket.read(&mut request).await?;
            if read == 0 {
                return Err(std::io::Error::other("request missing"));
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await
        });
        let fetcher = Arc::new(Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            std::collections::HashMap::new(),
            vec!["localhost".into()],
        )?);
        let region = Arc::new(Spawner::with_capacity(1));
        let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
        let admission = jobs
            .admit_work("jurisdiction:WI:2026-27:1")
            .await
            .map_err(crate::restate_services::tests::sdk_error)?;
        let handle = tokio::runtime::Handle::current();
        let selected =
            SourceSelection::parse(&[]).map_err(crate::restate_services::tests::sdk_error)?;
        let historical = scope(2026)?;
        let work = region.dedicated(admission, move || {
            handle.block_on(async move {
                let Json(outcome) = results_stage(
                    store,
                    Arc::clone(&fetcher),
                    historical,
                    selected,
                    ResultsCaptureExecution::UnadoptedSources,
                )
                .await?;
                let captured = fetcher
                    .get(&url, &census_crawl::net::FetchOptions::default())
                    .await
                    .map_err(|error| {
                        crate::restate_services::jobs::invariant(&error.to_string())
                    })?;
                if captured.body != b"ok" {
                    return Err(crate::restate_services::jobs::invariant(
                        "resolver request body mismatch",
                    ));
                }
                Ok::<_, HandlerError>(outcome)
            })
        });
        let outcome = tokio::time::timeout(WAIT, work).await??;
        match outcome {
            Outcome::Ok(outcome) => check!(outcome.per_source.is_empty()),
            Outcome::Err(error) => return Err(crate::restate_services::tests::sdk_error(error)),
            other => return Err(format!("dedicated results outcome: {other:?}").into()),
        }
        tokio::time::timeout(WAIT, server.join_next())
            .await?
            .ok_or("HTTP fixture missing")???;
        let drained = region.drain(WAIT).await?;
        check!(eq; drained.accepted, 1);
        check!(eq; drained.completed, 1);
        check!(eq; drained.remaining, 0);
        Ok(())
    });
    runtime.shutdown_timeout(WAIT);
    result
}
