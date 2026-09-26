//! Draining the actor from the manager's side.
//!
//! Shutdown is a bounded protocol, not a flag: revoke admission and mark the status `Stopped`
//! first, then send `Command::Shutdown` under one deadline, then await the actor task under the
//! same deadline and abort it only if the reply never arrives. The end of the protocol is the
//! actor's region certificate, so it is returned even when the teardown failed: the actor finishes
//! its counts before it touches the browser process, and a lost certificate would hide exactly the
//! units the drain could not account for.

use super::super::{actor::Command, BrowserState, SHUTDOWN_TIMEOUT};
use super::status::write_state;
use super::BrowserManager;
use crate::drain::DrainReport;
use tokio::sync::oneshot;

impl BrowserManager {
    /// Drain the actor and return its region certificate plus any teardown failure.
    pub async fn shutdown(&self) -> (DrainReport, Option<anyhow::Error>) {
        self.gate.revoke();
        write_state(&self.status, BrowserState::Stopped);
        let deadline = match self.clock.now_instant().checked_add(SHUTDOWN_TIMEOUT) {
            Some(value) => value,
            None => self.clock.now_instant(),
        };
        let (report, mut failure) = self.send_shutdown_command(deadline).await;
        if let Some(join) = self.join_actor(deadline).await {
            failure = Some(join);
        }
        (report, failure)
    }

    /// Send `Command::Shutdown` and await the actor's certificate, both under the deadline.
    ///
    /// A failure here means the actor never delivered a certificate, so the report is the empty
    /// one and the reason names the step that gave up.
    async fn send_shutdown_command(
        &self,
        deadline: tokio::time::Instant,
    ) -> (DrainReport, Option<anyhow::Error>) {
        let (reply, result) = oneshot::channel();
        match tokio::time::timeout_at(deadline, self.tx.send(Command::Shutdown { reply })).await {
            Ok(Ok(())) => match tokio::time::timeout_at(deadline, result).await {
                Ok(Ok(report)) => (report, None),
                Ok(Err(_)) => (
                    DrainReport::default(),
                    Some(anyhow::anyhow!("browser actor stopped")),
                ),
                Err(_) => (
                    DrainReport::default(),
                    Some(anyhow::anyhow!("browser shutdown reply timed out")),
                ),
            },
            Ok(Err(_)) => (
                DrainReport::default(),
                Some(anyhow::anyhow!("browser actor stopped")),
            ),
            Err(_) => (
                DrainReport::default(),
                Some(anyhow::anyhow!("browser shutdown send timed out")),
            ),
        }
    }

    /// Wait for the actor task, aborting it when the deadline expires.
    ///
    /// The actor reports its own teardown failure as its task result, so a joined task that failed
    /// is the failure here — and it is the same failure the certificate was mailed with.
    async fn join_actor(&self, deadline: tokio::time::Instant) -> Option<anyhow::Error> {
        let mut guard = self.join.lock().await;
        let join = guard.take();
        drop(guard);
        let mut handle = join?;
        match tokio::time::timeout_at(deadline, &mut handle).await {
            Ok(Ok(Ok(()))) => None,
            Ok(Ok(Err(error))) => Some(error),
            Ok(Err(_)) => Some(anyhow::anyhow!("browser actor panicked")),
            Err(_) => {
                handle.abort();
                if handle.await.is_err() {
                    tracing::warn!("browser actor aborted after shutdown timeout");
                }
                Some(anyhow::anyhow!("browser shutdown timed out"))
            }
        }
    }
}
