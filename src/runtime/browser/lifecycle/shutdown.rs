//! Draining the actor.
//!
//! Shutdown is a bounded protocol, not a flag: revoke admission and mark the status `Stopped`
//! first, then send `Command::Shutdown` under one deadline, then await the actor task under the
//! same deadline and abort it only if the reply never arrives. The command outcome and the join
//! outcome are combined with `and`, so a transport failure still reports the actor's own result
//! when the task finished cleanly.

use super::super::{actor::Command, BrowserState, SHUTDOWN_TIMEOUT};
use super::status::write_state;
use super::BrowserManager;
use tokio::sync::oneshot;

impl BrowserManager {
    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        self.gate.revoke();
        write_state(&self.status, BrowserState::Stopped);
        let deadline = match tokio::time::Instant::now().checked_add(SHUTDOWN_TIMEOUT) {
            Some(value) => value,
            None => tokio::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(300))
                .unwrap_or_else(tokio::time::Instant::now), // overflow guard
        };
        let (reply, result) = oneshot::channel();
        let send_result =
            tokio::time::timeout_at(deadline, self.tx.send(Command::Shutdown { reply })).await;
        let command_result = match send_result {
            Ok(Ok(())) => match tokio::time::timeout_at(deadline, result).await {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => Err(anyhow::anyhow!("browser actor stopped")),
                Err(_) => Err(anyhow::anyhow!("browser shutdown reply timed out")),
            },
            Ok(Err(_)) => Err(anyhow::anyhow!("browser actor stopped")),
            Err(_) => Err(anyhow::anyhow!("browser shutdown send timed out")),
        };
        let mut guard = self.join.lock().await;
        let join = guard.take();
        drop(guard);
        let join_result = match join {
            Some(mut handle) => match tokio::time::timeout_at(deadline, &mut handle).await {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => Err(anyhow::anyhow!("browser actor panicked")),
                Err(_) => {
                    handle.abort();
                    if handle.await.is_err() {
                        tracing::warn!("browser actor aborted after shutdown timeout");
                    }
                    Err(anyhow::anyhow!("browser shutdown timed out"))
                }
            },
            None => Ok(()),
        };
        command_result.and(join_result)
    }
}
