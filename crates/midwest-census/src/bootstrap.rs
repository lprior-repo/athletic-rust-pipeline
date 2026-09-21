//! Service supervisor: an owned task region with a cancel/drain/finalize shutdown protocol.
//!
//! The batch CLI opens the store, does one pass, and exits. The service path has to survive
//! operators, restarts, and Restate journal replays, so it is built as an explicit region: [`serve`]
//! (or [`serve_until`]) owns exactly one task — the HTTP endpoint — waits for a stop request, drains
//! inside a bounded deadline, finalizes the store, and reports what happened. A bare `tokio::spawn`
//! whose handle is dropped would be an orphan factory; there is none here.
//!
//! Shutdown is a protocol, not a flag:
//!
//! 1. **request** — a signal, or the caller's `shutdown` future, resolves and the reason is recorded;
//! 2. **stop intake** — `HttpServer::serve_with_cancel` stops accepting and runs a hyper graceful
//!    shutdown, so in-flight invocations get to finish;
//! 3. **drain** — the region is reaped inside `drain_timeout`; whatever outlives the deadline is
//!    aborted and counted, never leaked;
//! 4. **finalize** — the store is flushed (`SyncAll`) and the Fjall database dropped, so the next
//!    start replays only what reached the journal;
//! 5. **report** — [`DrainReport`] counts accepted/completed/cancelled/timed out/aborted/panicked work
//!    plus the [`StopReason`], which is what tests and operators assert on.
//!
//! Time enters the application through the [`Clock`] capability rather than `SystemTime::now` inside
//! domain logic, so a replayed workflow can run against a fixed date.
//!
//! # Errors
//!
//! Every stage below — flag parsing, the store, the bind, drain accounting — reports
//! [`BootstrapError`]. `anyhow` survives in exactly one place: the [`serve`] and [`serve_until`]
//! signatures, which `midwest-serve` prints with its chain and the integration test drives as
//! `JoinSet<anyhow::Result<DrainReport>>`. Converting those two would push a type change into
//! `bin/**` and `tests/**`, so the conversion happens once, at the boundary.

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use restate_sdk::http_server::HttpServer;
use tokio::task::JoinSet;

use crate::restate_services;
use crate::store::Store;

mod error;

pub use error::BootstrapError;

/// Default fan-out cap for concurrent work started by the services.
pub const DEFAULT_MAX_CONCURRENT: usize = 8;
/// Default deadline for in-flight work after a stop request.
pub const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// The clock the application is allowed to read, kept behind a capability so deterministic replay is
/// possible: tests substitute a fixed date, production uses the system clock.
pub trait Clock: Send + Sync {
    /// ISO-8601 date, `YYYY-MM-DD`.
    fn today(&self) -> String;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> String {
        chrono::Utc::now().date_naive().to_string()
    }
}

/// Why the endpoint stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum StopReason {
    /// SIGINT/SIGTERM arrived.
    Signal = 0,
    /// The `shutdown` future passed to [`serve_until`] resolved.
    Requested = 1,
    /// The endpoint task ended on its own — a fault, not an operator action.
    #[default]
    ServerExit = 2,
}

impl StopReason {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0 => StopReason::Signal,
            1 => StopReason::Requested,
            _ => StopReason::ServerExit,
        }
    }
}

/// What the region did before it stopped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrainReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub stop_reason: StopReason,
}

fn bump(counter: &mut u64, by: u64) -> u64 {
    *counter = counter.saturating_add(by);
    *counter
}

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
async fn supervise(
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<DrainReport, BootstrapError> {
    init_tracing();
    // Opening the store is synchronous, fsync-heavy work: it belongs on the blocking pool, not on
    // the runtime thread that will own the endpoint.
    let store = open_store(options.data_dir.clone()).await?;
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

    let endpoint = restate_services::build_endpoint(store.clone(), options.max_concurrent);
    let mut tasks: JoinSet<()> = JoinSet::new();
    tasks.spawn(async move {
        HttpServer::new(endpoint)
            .serve_with_cancel(listener, stop)
            .await;
    });
    tracing::info!(%bound, max_concurrent = options.max_concurrent, "census service listening");

    let mut report = drain(tasks, options.drain_timeout).await?;
    report.stop_reason = StopReason::from_raw(reason.load(Ordering::SeqCst));

    // Finalize after the region is empty: nothing can still be writing when the journal is synced.
    let finalized = store.flush();
    drop(store);
    finalized.map_err(|source| BootstrapError::StoreFlush { source })?;
    tracing::info!(?report, "census service stopped");
    Ok(report)
}

/// Open (creating if needed) the store on the blocking pool.
async fn open_store(data_dir: PathBuf) -> Result<Arc<Store>, BootstrapError> {
    tokio::task::spawn_blocking(move || {
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
    .await
    .map_err(|source| BootstrapError::StoreTask { source })?
}

/// Resolve when a shutdown signal arrives or the caller's `shutdown` future resolves, recording
/// which of the two stopped the endpoint.
async fn stop_watch(reason: Arc<AtomicU8>, shutdown: impl Future<Output = ()> + Send + 'static) {
    tokio::select! {
        outcome = wait_for_shutdown_signal() => {
            if let Err(error) = outcome {
                tracing::warn!(%error, "shutdown signal watcher failed");
            }
            reason.store(StopReason::Signal as u8, Ordering::SeqCst);
        }
        () = shutdown => {
            reason.store(StopReason::Requested as u8, Ordering::SeqCst);
        }
    }
}

/// Wait for SIGINT/SIGTERM (or Ctrl-C where the platform has no signals).
async fn wait_for_shutdown_signal() -> Result<(), BootstrapError> {
    #[cfg(unix)]
    {
        wait_for_unix_signal().await
    }
    #[cfg(not(unix))]
    {
        wait_for_ctrl_c().await
    }
}

/// Install the SIGTERM/SIGINT subscriptions and return when either one arrives.
#[cfg(unix)]
async fn wait_for_unix_signal() -> Result<(), BootstrapError> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate =
        signal(SignalKind::terminate()).map_err(|source| BootstrapError::Signal {
            signal: "SIGTERM",
            source,
        })?;
    let mut interrupt =
        signal(SignalKind::interrupt()).map_err(|source| BootstrapError::Signal {
            signal: "SIGINT",
            source,
        })?;
    tokio::select! {
        _ = terminate.recv() => Ok(()),
        _ = interrupt.recv() => Ok(()),
    }
}

/// Wait for Ctrl-C on the platforms that have no signal subscriptions.
#[cfg(not(unix))]
async fn wait_for_ctrl_c() -> Result<(), BootstrapError> {
    tokio::signal::ctrl_c()
        .await
        .map_err(|source| BootstrapError::Signal {
            signal: "Ctrl-C",
            source,
        })
}

/// Bounded drain of an owned task region: reap natural exits, then abort and count the rest.
async fn drain(mut tasks: JoinSet<()>, timeout: Duration) -> Result<DrainReport, BootstrapError> {
    let mut report = DrainReport::default();
    let accepted = u64::try_from(tasks.len()).map_err(|_| BootstrapError::TaskCountOverflow)?;
    bump(&mut report.accepted, accepted);
    let deadline = tokio::time::Instant::now() + timeout;

    while !tasks.is_empty() {
        match tokio::time::timeout_at(deadline, tasks.join_next()).await {
            Ok(Some(Ok(()))) => {
                bump(&mut report.completed, 1);
            }
            Ok(Some(Err(error))) => {
                if error.is_panic() {
                    tracing::error!(%error, "task panicked during shutdown");
                    bump(&mut report.panicked, 1);
                } else {
                    bump(&mut report.cancelled, 1);
                }
            }
            Ok(None) => break,
            Err(_) => {
                tracing::warn!(?timeout, "drain deadline reached; aborting remaining tasks");
                let remaining =
                    u64::try_from(tasks.len()).map_err(|_| BootstrapError::TaskCountOverflow)?;
                bump(&mut report.timed_out, remaining);
                tasks.abort_all();
                // Reap the aborted tasks so their resources (sockets, file handles) are released
                // before this returns.
                while tasks.join_next().await.is_some() {
                    bump(&mut report.aborted, 1);
                }
                break;
            }
        }
    }
    Ok(report)
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

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub max_concurrent: usize,
    pub drain_timeout: Duration,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([127, 0, 0, 1], 9080)),
            data_dir: PathBuf::from("var/midwest-census"),
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
        }
    }
}

const USAGE: &str = "midwest-serve [--listen ADDR] [--data-dir DIR] \
                     [--max-concurrent N] [--drain-timeout SECONDS] [--help]";

impl ServeOptions {
    /// Parse `--flag value` pairs. Unknown flags are rejected instead of ignored: a typo in a
    /// deployment script must not silently serve the wrong directory.
    pub fn from_env(args: impl Iterator<Item = String>) -> Result<Self, BootstrapError> {
        let mut options = ServeOptions::default();
        let mut args = args.peekable();
        while let Some(flag) = args.next() {
            let mut value = |flag: &str| -> Result<String, BootstrapError> {
                args.next().ok_or_else(|| BootstrapError::MissingValue {
                    flag: flag.to_string(),
                })
            };
            match flag.as_str() {
                "--listen" => {
                    let raw = value("--listen")?;
                    options.listen = raw
                        .parse()
                        .map_err(|source| BootstrapError::ListenNotAnAddress { raw, source })?;
                }
                "--data-dir" => options.data_dir = PathBuf::from(value("--data-dir")?),
                "--max-concurrent" => {
                    let raw = value("--max-concurrent")?;
                    let parsed: usize = raw
                        .parse()
                        .map_err(|source| BootstrapError::ConcurrencyNotANumber { raw, source })?;
                    if parsed == 0 {
                        return Err(BootstrapError::ConcurrencyIsZero);
                    }
                    options.max_concurrent = parsed;
                }
                "--drain-timeout" => {
                    let raw = value("--drain-timeout")?;
                    let seconds: u64 =
                        raw.parse()
                            .map_err(|source| BootstrapError::DrainTimeoutNotASeconds {
                                raw,
                                source,
                            })?;
                    options.drain_timeout = Duration::from_secs(seconds);
                }
                "--help" | "-h" => return Err(BootstrapError::HelpRequested),
                other => {
                    return Err(BootstrapError::UnknownFlag {
                        flag: other.to_string(),
                    })
                }
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests;
