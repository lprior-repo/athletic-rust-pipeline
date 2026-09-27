use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use anyhow::Result;
use restate_sdk::http_server::HttpServer;

use crate::ingress;
use crate::outcome::{DrainState, Outcome};
use crate::restate_services;
use crate::spawn::Spawner;
use census_crawl::net::bridge::BrowserLane;
use census_store::Store;

use super::drain::{count_error, DrainReport};
use super::error::BootstrapError;
use super::options::ServeOptions;
use super::stop::{stop_watch, StopReason};

pub async fn serve(options: ServeOptions) -> Result<DrainReport> {
    serve_until(options, std::future::pending::<()>()).await
}

pub async fn serve_until(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<DrainReport> {
    supervise(options, shutdown)
        .await
        .map_err(anyhow::Error::from)
}

#[tracing::instrument(skip_all)]
pub(super) async fn supervise(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<DrainReport, BootstrapError> {
    init_tracing();
    let region = Arc::new(Spawner::new());
    let store = open_store(&region, options.data_dir.clone()).await?;
    let (listener, bound) = bind_listener(&options).await?;
    let lane = lane_client(&options)?;

    let reason = Arc::new(AtomicU8::new(StopReason::ServerExit.to_raw()));
    let over_budget = Arc::new(tokio::sync::Notify::new());
    let (cancel, endpoint_done) = spawn_endpoint(&region, &store, &options, listener, lane);
    let _watcher = tokio::spawn(super::guard::watch_memory(
        super::DEFAULT_MEMORY_BUDGET_BYTES,
        Arc::clone(&over_budget),
    ));
    tracing::info!(
        %bound,
        max_concurrent = options.max_concurrent,
        memory_budget_gib = super::DEFAULT_MEMORY_BUDGET_BYTES / (1024 * 1024 * 1024),
        "census service listening"
    );

    await_stop(
        Arc::clone(&reason),
        Arc::clone(&over_budget),
        shutdown,
        endpoint_done,
    )
    .await;
    cancel.send(()).ok();
    let counted = region
        .drain(options.drain_timeout)
        .await
        .map_err(count_error)?;
    let mut report = DrainReport::from_counted(counted);
    report.stop_reason = StopReason::from_raw(reason.load(Ordering::SeqCst));

    let finalized = store.flush();
    drop(store);
    finalized.map_err(|source| BootstrapError::StoreFlush { source })?;
    tracing::info!(?report, "census service stopped");
    Ok(report)
}

fn lane_client(options: &ServeOptions) -> Result<Option<BrowserLane>, BootstrapError> {
    if options.lane.is_none() {
        return Ok(None);
    }
    let origin = ingress::DEFAULT_ORIGIN;
    let client = ingress::client(origin).map_err(|error| BootstrapError::LaneIngressUnusable {
        origin: origin.to_string(),
        detail: error.to_string(),
    })?;
    Ok(Some(BrowserLane::over(client)))
}

async fn await_stop(
    reason: Arc<AtomicU8>,
    over_budget: Arc<tokio::sync::Notify>,
    shutdown: impl Future<Output = ()> + Send + 'static,
    endpoint_done: impl Future + Send,
) {
    tokio::select! {
        () = stop_watch(Arc::clone(&reason), shutdown) => {}
        () = over_budget.notified() => {
            reason.store(StopReason::MemoryBudget.to_raw(), Ordering::SeqCst);
        }
        _ = endpoint_done => {
            reason.store(StopReason::ServerExit.to_raw(), Ordering::SeqCst);
        }
    }
}

async fn bind_listener(
    options: &ServeOptions,
) -> Result<(tokio::net::TcpListener, SocketAddr), BootstrapError> {
    if !options.listen.ip().is_loopback() {
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
    Ok((listener, bound))
}

fn spawn_endpoint(
    region: &Arc<Spawner>,
    store: &Arc<Store>,
    options: &ServeOptions,
    listener: tokio::net::TcpListener,
    lane: Option<BrowserLane>,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let (cancel, cancelled) = tokio::sync::oneshot::channel::<()>();
    let (ended, endpoint_done) = tokio::sync::oneshot::channel::<()>();
    let stop = async move {
        cancelled.await.ok();
    };
    let endpoint = restate_services::build_endpoint(
        store.clone(),
        options.max_concurrent,
        Arc::clone(region),
        options.lane.clone(),
        lane,
    );
    region.spawn(async move {
        HttpServer::new(endpoint)
            .serve_with_cancel(listener, stop)
            .await;
        ended.send(()).ok();
    });
    (cancel, endpoint_done)
}

#[tracing::instrument(skip_all)]
async fn open_store(region: &Spawner, data_dir: PathBuf) -> Result<Arc<Store>, BootstrapError> {
    let outcome = region
        .blocking(move || {
            std::fs::create_dir_all(&data_dir).map_err(|source| BootstrapError::CreateDir {
                path: data_dir.clone(),
                source,
            })?;
            let store = Store::open(&data_dir).map_err(|source| BootstrapError::StoreOpen {
                path: data_dir,
                source,
            })?;
            Ok(Arc::new(store))
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
    }
}

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

#[cfg(test)]
#[path = "serve_tests.rs"]
mod tests;
