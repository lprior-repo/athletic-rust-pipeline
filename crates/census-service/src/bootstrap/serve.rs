//! The region itself: open the store, bind, own the endpoint task, drain, finalize.

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

/// Run until SIGINT/SIGTERM, then drain and finalize.
pub async fn serve(options: ServeOptions) -> Result<DrainReport> {
    serve_until(options, std::future::pending::<()>()).await
}

/// Run until a signal or `shutdown` resolves, then drain and finalize.
///
/// The `anyhow::Result` here is the boundary documented at the top of this module: `census-serve`
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
    let (listener, bound) = bind_listener(&options).await?;
    let lane = lane_client(&options)?;

    let reason = Arc::new(AtomicU8::new(StopReason::ServerExit.to_raw()));
    let over_budget = Arc::new(tokio::sync::Notify::new());
    let (cancel, endpoint_done) = spawn_endpoint(&region, &store, &options, listener, lane);
    // The budget watcher reads /proc and has to keep watching while the region drains — a drain can
    // take the whole timeout, and a swap during it is still the operator's problem. So it runs
    // outside the region: the drain owns writers, and a watchdog that only returns when the budget
    // trips would otherwise leave every stop report with one task `timed_out`.
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
    // A failed send means the endpoint's own watch is already gone; the drain below is what matters.
    cancel.send(()).ok();
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

/// The browser lane this process acquires browser-transported hosts through, when it serves one.
///
/// One process owns the headed profile and it is this one, so the census reaches its own
/// `BrowserSession` object through the deployment's ingress — the same node every CLI command
/// addresses. A deployment that serves no lane gets no client, and that is deliberate: a fetcher
/// holding a client for a lane nobody serves would fail every browser-transported request instead of
/// refusing that source by name before spending anything on it.
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

/// Wait for whatever stops this process first, and record the reason.
///
/// The deadline bounds the reap *after* a stop request, never the wait for one: draining before the
/// watch resolves would abort a healthy endpoint at the deadline and report `ServerExit`, which is
/// exactly the fault this ordering exists to keep visible.
///
/// The endpoint's task ending is a stop request too. Its HTTP server returned — a panic, or an
/// accept loop that gave up — so nothing is serving the port while the process still holds the
/// store's exclusive lock. Swallowing that is what let a dead endpoint look like a healthy one.
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

/// Bind the service listener, refusing anything but a loopback address and reporting the address the
/// kernel actually bound (a configured port 0 asks it to pick one).
async fn bind_listener(
    options: &ServeOptions,
) -> Result<(tokio::net::TcpListener, SocketAddr), BootstrapError> {
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
    Ok((listener, bound))
}

/// Spawn the region-owned endpoint task.
///
/// Returns the sender that cancels it, and a receiver that fires if the task ends on its own. That
/// second half is not optional: the cancel signal is the supervisor's, but a returned HTTP server is
/// the endpoint's own event, and a supervisor that cannot see it cannot report or recover from it.
///
/// The cancel signal is the supervisor's, not the stop watch: the supervisor has to observe the stop
/// request itself to know when the region may be drained, so the watch and the endpoint's cancel are
/// deliberately two observers of one event.
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
        // The stop future only has to observe the cancel; a dropped sender ends it too.
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
        // Fires on both exits: the supervisor may already be draining, in which case the send is a
        // no-op, but an exit *without* a cancel is what the supervisor is waiting to hear about.
        ended.send(()).ok();
    });
    (cancel, endpoint_done)
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
            let store = Store::open(&data_dir).map_err(|source| BootstrapError::StoreOpen {
                path: data_dir.clone(),
                source,
            })?;
            // The service owns the store for the live route, so it is one of the paths that migrate: a
            // pre-Fjall corpus reaches the census only because the open that owns it imported it.
            // Opening a store is otherwise a read, so a verb that only measures one no longer writes.
            let imported = store
                .import_legacy()
                .map_err(|source| BootstrapError::StoreOpen {
                    path: data_dir,
                    source,
                })?;
            if imported.observations > 0 || imported.skipped > 0 {
                tracing::info!(
                    observations = imported.observations,
                    skipped = imported.skipped,
                    "imported the pre-Fjall journals"
                );
            }
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

#[cfg(test)]
#[path = "serve_tests.rs"]
mod tests;
