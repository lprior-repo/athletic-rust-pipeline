use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use super::error::BootstrapError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum StopReason {
    Signal = 0,
    Requested = 1,
    #[default]
    ServerExit = 2,
    MemoryBudget = 3,
}

impl StopReason {
    pub(super) fn from_raw(raw: u8) -> Self {
        match raw {
            0 => StopReason::Signal,
            1 => StopReason::Requested,
            3 => StopReason::MemoryBudget,
            _ => StopReason::ServerExit,
        }
    }

    pub(super) const fn to_raw(self) -> u8 {
        match self {
            StopReason::Signal => 0,
            StopReason::Requested => 1,
            StopReason::ServerExit => 2,
            StopReason::MemoryBudget => 3,
        }
    }
}

pub(super) async fn stop_watch(
    reason: Arc<AtomicU8>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) {
    tokio::select! {
        outcome = wait_for_shutdown_signal() => {
            if let Err(error) = outcome {
                tracing::warn!(%error, "shutdown signal watcher failed");
            }
            reason.store(StopReason::Signal.to_raw(), Ordering::SeqCst);
        }
        () = shutdown => {
            reason.store(StopReason::Requested.to_raw(), Ordering::SeqCst);
        }
    }
}

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

#[cfg(not(unix))]
async fn wait_for_ctrl_c() -> Result<(), BootstrapError> {
    tokio::signal::ctrl_c()
        .await
        .map_err(|source| BootstrapError::Signal {
            signal: "Ctrl-C",
            source,
        })
}
