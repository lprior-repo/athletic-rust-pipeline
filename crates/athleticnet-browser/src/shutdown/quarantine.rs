use crate::actor::{Actor, Command};
use crate::lifecycle::error::BrowserStartupError;
use crate::{BrowserError, BrowserOutcome, BrowserState, SHUTDOWN_TIMEOUT};

impl Actor {
    pub(super) fn report_cleanup_failure(&mut self, error: &BrowserStartupError) {
        tracing::error!(%error, pages = self.pages.len(), "browser cleanup remains owned in quarantine");
        self.set_state(BrowserState::Restarting);
        if let Some(reply) = self.shutdown_reply.take() {
            if reply.send(self.region.clone()).is_err() {
                tracing::debug!("quarantined browser shutdown report receiver dropped");
            }
        }
    }

    pub(super) async fn await_reap_command(&mut self) {
        loop {
            match self.rx.recv().await {
                Some(Command::Shutdown { reply }) => {
                    self.shutdown_reply = Some(reply);
                    return;
                }
                Some(Command::Inspect { reply }) => {
                    if reply.send(Ok(self.status())).is_err() {
                        tracing::debug!("quarantined browser status receiver dropped");
                    }
                }
                Some(Command::Fetch { reply, .. }) => {
                    if reply
                        .send(BrowserOutcome::failed(BrowserError::Shutdown))
                        .is_err()
                    {
                        tracing::debug!("quarantined browser fetch refusal receiver dropped");
                    }
                }
                Some(Command::Recover { reply } | Command::Restart { reply }) => {
                    if reply.send(Err(BrowserError::Shutdown)).is_err() {
                        tracing::debug!("quarantined browser control refusal receiver dropped");
                    }
                }
                Some(Command::Bootstrap { reply }) => {
                    if reply
                        .send(Err(BrowserStartupError::ShutdownFailed(
                            "browser is awaiting physical reaping".to_string(),
                        )))
                        .is_err()
                    {
                        tracing::debug!("quarantined browser bootstrap refusal receiver dropped");
                    }
                }
                None => {
                    tracing::error!("browser supervisor lost while cleanup remains owned; reaping the same physical child");
                    tokio::time::sleep(SHUTDOWN_TIMEOUT).await;
                    return;
                }
            }
        }
    }
}
