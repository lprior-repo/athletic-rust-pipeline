use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use anyhow::Result;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::Endpoint;

use crate::ingress;
use crate::restate_services;
use crate::spawn::Spawner;
use census_crawl::net::bridge::BrowserLane;
use census_store::Store;

use super::drain::{
    await_endpoint_shutdown, count_error, DrainReport, EndpointShutdown, ENDPOINT_SHUTDOWN_GRACE,
};
use super::error::BootstrapError;
use super::options::ServeOptions;
use super::stop::{await_stop, StopOutcome, StopReason};
use crate::restate_services::CaptureExecution;

mod finalize;
use finalize::{finalize_region, finish_startup};

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
    let memory = super::guard::MemoryReader::prepare(&options)?;
    let region = Arc::new(Spawner::new());
    supervise_region(options, shutdown, memory, region).await
}

async fn supervise_region(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
    memory: super::guard::MemoryReader,
    region: Arc<Spawner>,
) -> Result<DrainReport, BootstrapError> {
    let (store, capture) =
        super::store::open(&region, options.data_dir.clone(), options.capture_protocol).await?;
    let (listener, bound) = bind_listener(&options).await?;
    let lane = lane_client(&options)?;

    if http_exit_fault_requested(std::env::var_os("ATHLETIC_FAULT_HTTP_EXIT").as_deref()) {
        tracing::warn!("ATHLETIC_FAULT_HTTP_EXIT active: endpoint will fail");
    }
    let reason = Arc::new(AtomicU8::new(StopReason::ServerExit.to_raw()));
    let over_budget = Arc::new(tokio::sync::Notify::new());
    let (cancel, endpoint_done) =
        spawn_endpoint(&region, &store, capture, &options, listener, lane)?;
    let watch_result = region
        .spawn(super::guard::watch_memory(
            memory,
            Arc::clone(&over_budget),
            region.stopping(),
        ))
        .map_err(count_error);
    let stop_outcome = match &watch_result {
        Ok(()) => {
            tracing::info!(
                %bound,
                max_concurrent = options.max_concurrent,
                memory_budget_gib = options.memory_budget_bytes / (1024 * 1024 * 1024),
                "census service listening"
            );
            await_stop(
                Arc::clone(&reason),
                Arc::clone(&over_budget),
                shutdown,
                endpoint_done,
            )
            .await
        }
        Err(_) => StopOutcome::ReceiverPending(endpoint_done),
    };
    cancel.send(()).ok();
    let endpoint_shutdown = match stop_outcome {
        StopOutcome::EndpointAlreadyEnded => EndpointShutdown::Completed,
        StopOutcome::ReceiverPending(receiver) => {
            await_endpoint_shutdown(receiver, ENDPOINT_SHUTDOWN_GRACE).await
        }
    };
    let finalized = finalize_region(
        store,
        region,
        reason,
        endpoint_shutdown,
        options.drain_timeout,
    )
    .await;
    finish_startup(watch_result, finalized)
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
    capture: CaptureExecution,
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
        capture,
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
        .spawn(run_endpoint(endpoint, listener, stop, ended))
        .map_err(count_error)?;
    Ok((cancel, endpoint_done))
}

async fn run_endpoint(
    endpoint: Endpoint,
    listener: tokio::net::TcpListener,
    stop: impl Future<Output = ()> + Send,
    ended: tokio::sync::oneshot::Sender<()>,
) {
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

#[cfg(test)]
#[path = "../../tests/installation/bootstrap.rs"]
mod installation_tests;

#[cfg(test)]
#[path = "capture_tests.rs"]
mod capture_tests;

#[cfg(test)]
#[path = "endpoint_region_tests.rs"]
mod endpoint_region_tests;

#[cfg(test)]
#[path = "watch_cleanup_tests.rs"]
mod watch_cleanup_tests;
