//! The stop protocol: why the endpoint stopped, and the watchers that observe it.

use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use super::error::BootstrapError;

/// Why the endpoint stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum StopReason {
    /// SIGINT/SIGTERM arrived.
    Signal = 0,
    /// The `shutdown` future passed to [`serve_until`](super::serve_until) resolved.
    Requested = 1,
    /// The endpoint task ended on its own — a fault, not an operator action.
    #[default]
    ServerExit = 2,
}

impl StopReason {
    pub(super) fn from_raw(raw: u8) -> Self {
        match raw {
            0 => StopReason::Signal,
            1 => StopReason::Requested,
            _ => StopReason::ServerExit,
        }
    }
}

/// Resolve when a shutdown signal arrives or the caller's `shutdown` future resolves, recording
/// which of the two stopped the endpoint.
pub(super) async fn stop_watch(
    reason: Arc<AtomicU8>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) {
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
