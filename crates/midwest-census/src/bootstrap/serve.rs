//! The region itself: open the store, bind, own the endpoint task, drain, finalize.

use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use anyhow::Result;
use restate_sdk::http_server::HttpServer;

use crate::outcome::{DrainState, Outcome};
use crate::restate_services;
use crate::spawn::Spawner;
use crate::store::Store;

use super::drain::{count_error, DrainReport};
use super::error::BootstrapError;
use super::options::ServeOptions;
use super::stop::{stop_watch, StopReason};

/// Run until SIGINT/SIGTERM, then drain and finalize.
pub async fn serve(options: ServeOptions) -> Result<DrainReport> {
    serve_until(options, std::future::pending::<()>()).await
}

/// Run until a signal or `shutdown` resolves, then drain and finalize.
///
/// The `anyhow::Result` here is the boundary documented at the top of this module: `midwest-serve`
/// prints the chain, and the integration test drives this through
/// `JoinSet<anyhow::Result<DrainReport>>`. One conversion, at the edge; every stage below is typed.
pub async fn serve_until(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<DrainReport> {
    supervise(options, shutdown)
        .await
        .map_err(anyhow::Error::from)
}

/// The supervisor itself: open, bind, own the region, drain, finalize.
#[tracing::instrument(skip_all)]
pub(super) async fn supervise(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<DrainReport, BootstrapError> {
    init_tracing();
    // One region for the whole service: the store's first writer, the services' blocking jobs and
    // the endpoint task are all started through it, so the drain below owns everything this process
    // started and finalize cannot race a writer the region had forgotten.
    let region = Arc::new(Spawner::new());
    let store = open_store(&region, options.data_dir.clone()).await?;
    if !options.listen.ip().is_loopback() {
        // The endpoint carries no request-identity key, so the SDK's verifier accepts every caller.
        return Err(BootstrapError::NonLoopbackListen {
            listen: options.listen,
        });
    }
    let listener = tokio::net::TcpListener::bind(options.listen)
        .await
        .map_err(|source| BootstrapError::Bind {
            listen: options.listen,
            source,
        })?;
    let bound = listener
        .local_addr()
        .map_err(|source| BootstrapError::BoundAddress { source })?;

    let reason = Arc::new(AtomicU8::new(StopReason::ServerExit as u8));
    let stop = stop_watch(Arc::clone(&reason), shutdown);

    let endpoint = restate_services::build_endpoint(
        store.clone(),
        options.max_concurrent,
        Arc::clone(&region),
    );
    region.spawn(async move {
        HttpServer::new(endpoint)
            .serve_with_cancel(listener, stop)
            .await;
    });
    tracing::info!(%bound, max_concurrent = options.max_concurrent, "census service listening");

    let counted = region
        .drain(options.drain_timeout)
        .await
        .map_err(count_error)?;
    let mut report = DrainReport::from_counted(counted);
    report.stop_reason = StopReason::from_raw(reason.load(Ordering::SeqCst));

    // Finalize after the region is empty: the drain waited for every task it started, so nothing can
    // still be writing when the journal is synced.
    let finalized = store.flush();
    drop(store);
    finalized.map_err(|source| BootstrapError::StoreFlush { source })?;
    tracing::info!(?report, "census service stopped");
    Ok(report)
}

/// Open (creating if needed) the store as a region task on the blocking pool.
#[tracing::instrument(skip_all)]
async fn open_store(region: &Spawner, data_dir: PathBuf) -> Result<Arc<Store>, BootstrapError> {
    let outcome = region
        .blocking(move || {
            std::fs::create_dir_all(&data_dir).map_err(|source| BootstrapError::CreateDir {
                path: data_dir.clone(),
                source,
            })?;
            Store::open(&data_dir)
                .map(Arc::new)
                .map_err(|source| BootstrapError::StoreOpen {
                    path: data_dir,
                    source,
                })
        })
        .await;
    match outcome {
        Outcome::Ok(store) => Ok(store),
        Outcome::Err(e) => Err(e),
        Outcome::Panicked => Err(BootstrapError::StoreTask {
            state: DrainState::Panicked,
            reason: "task panicked".to_string(),
        }),
        Outcome::Cancelled => Err(BootstrapError::StoreTask {
            state: DrainState::Cancelled,
            reason: "task cancelled".to_string(),
        }),
        Outcome::Timeout => Err(BootstrapError::StoreTask {
            state: DrainState::Cancelled,
            reason: "task timed out".to_string(),
        }),
    }
}

/// Install a tracing subscriber once. A second call in the same process (tests) is a no-op instead
/// of a panic.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let installed = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
    if installed.is_err() {
        tracing::debug!("tracing already installed");
    }
}
