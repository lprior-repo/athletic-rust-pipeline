use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use restate_sdk::http_server::HttpServer;

use crate::ingress;
use crate::outcome::{DrainState, Outcome};
use crate::restate_services;
use crate::spawn::Spawner;
use census_crawl::net::bridge::BrowserLane;
use census_store::Store;

use super::drain::{count_error, DrainReport, EndpointShutdown};
use super::error::BootstrapError;
use super::options::ServeOptions;
use super::stop::{stop_watch, StopReason};

pub(super) const ENDPOINT_SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

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

    if http_exit_fault_requested(std::env::var_os("ATHLETIC_FAULT_HTTP_EXIT").as_deref()) {
        tracing::warn!("ATHLETIC_FAULT_HTTP_EXIT active: endpoint will fail");
    }
    let reason = Arc::new(AtomicU8::new(StopReason::ServerExit.to_raw()));
    let over_budget = Arc::new(tokio::sync::Notify::new());
    let (cancel, mut endpoint_done) = spawn_endpoint(&region, &store, &options, listener, lane)?;
    region
        .spawn(super::guard::watch_memory(
            super::DEFAULT_MEMORY_BUDGET_BYTES,
            Arc::clone(&over_budget),
            region.stopping(),
        ))
        .map_err(count_error)?;
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
        &mut endpoint_done,
    )
    .await;
    cancel.send(()).ok();
    let endpoint_shutdown = await_endpoint_shutdown(endpoint_done, ENDPOINT_SHUTDOWN_GRACE).await;
    let counted = region
        .drain(options.drain_timeout)
        .await
        .map_err(count_error)?;
    let mut report = DrainReport::from_counted(counted);
    report.stop_reason = StopReason::from_raw(reason.load(Ordering::SeqCst));
    report.endpoint_shutdown = endpoint_shutdown;

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
    endpoint_done: &mut tokio::sync::oneshot::Receiver<()>,
) {
    tokio::select! {
        () = stop_watch(Arc::clone(&reason), shutdown) => {}
        () = over_budget.notified() => {
            reason.store(StopReason::MemoryBudget.to_raw(), Ordering::SeqCst);
        }
        _ = &mut *endpoint_done => {
            reason.store(StopReason::ServerExit.to_raw(), Ordering::SeqCst);
        }
    }
}

async fn await_endpoint_shutdown(
    endpoint_done: tokio::sync::oneshot::Receiver<()>,
    grace: Duration,
) -> EndpointShutdown {
    match tokio::time::timeout(grace, endpoint_done).await {
        Ok(Ok(())) | Ok(Err(_)) => EndpointShutdown::Completed,
        Err(_) => {
            tracing::warn!(
                grace_seconds = grace.as_secs_f64(),
                "the endpoint did not finish within the shutdown grace; closing effect admission"
            );
            EndpointShutdown::TimedOut
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
) -> Result<
    (
        tokio::sync::oneshot::Sender<()>,
        tokio::sync::oneshot::Receiver<()>,
    ),
    BootstrapError,
> {
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
    )
    .map_err(|error| BootstrapError::ConcurrencyTooLarge {
        value: error.value,
        ceiling: error.ceiling,
    })?;
    region
        .spawn(async move {
            let fault = std::env::var_os("ATHLETIC_FAULT_HTTP_EXIT");
            if http_exit_fault_requested(fault.as_deref()) {
                match http_exit_trigger_path(fault.as_deref()) {
                    Some(trigger) => {
                        let server = HttpServer::new(endpoint).serve_with_cancel(listener, stop);
                        tokio::select! {
                            () = server => {}
                            () = wait_for_trigger(trigger) => {
                                tracing::error!(
                                    "HTTP endpoint exiting due to the ATHLETIC_FAULT_HTTP_EXIT trigger"
                                );
                            }
                        }
                    }
                    None => {
                        tracing::error!("HTTP endpoint exiting due to ATHLETIC_FAULT_HTTP_EXIT");
                    }
                }
            } else {
                HttpServer::new(endpoint)
                    .serve_with_cancel(listener, stop)
                    .await;
            }
            ended.send(()).ok();
        })
        .map_err(count_error)?;
    Ok((cancel, endpoint_done))
}

#[tracing::instrument(skip_all)]
async fn open_store(region: &Spawner, data_dir: PathBuf) -> Result<Arc<Store>, BootstrapError> {
    let outcome = region
        .blocking(move || {
            census_store::fs::create_dir_all_synced(&data_dir).map_err(|source| {
                BootstrapError::CreateDir {
                    path: data_dir.clone(),
                    source,
                }
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
    let filter = match EnvFilter::try_from_default_env() {
        Ok(value) => value,
        Err(_) => EnvFilter::new("info"),
    };
    let installed = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
    if installed.is_err() {
        tracing::debug!("tracing already installed");
    }
}

#[cfg(feature = "native-fault-injection")]
fn http_exit_fault_requested(state: Option<&std::ffi::OsStr>) -> bool {
    state.is_some()
}

#[cfg(not(feature = "native-fault-injection"))]
fn http_exit_fault_requested(_state: Option<&std::ffi::OsStr>) -> bool {
    false
}

#[cfg(feature = "native-fault-injection")]
fn http_exit_trigger_path(state: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf> {
    let value = state?;
    if value.is_empty() || value == std::ffi::OsStr::new("1") {
        return None;
    }
    Some(std::path::PathBuf::from(value))
}

#[cfg(not(feature = "native-fault-injection"))]
fn http_exit_trigger_path(_state: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf> {
    None
}

#[cfg(feature = "native-fault-injection")]
async fn wait_for_trigger(path: std::path::PathBuf) {
    while !path.exists() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[cfg(not(feature = "native-fault-injection"))]
async fn wait_for_trigger(_path: std::path::PathBuf) {}

#[cfg(test)]
#[path = "serve_tests.rs"]
mod tests;
