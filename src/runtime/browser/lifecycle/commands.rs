//! The manager's command surface.
//!
//! Every method here forwards one command to the actor and, where the actor answers, maps the
//! reply channel closing into the matching `BrowserError`. The read-only methods (`is_alive`,
//! `status`, `gated_error`, `mark_human_required`) work on the shared snapshot and the profile
//! gate without touching the actor.

use super::super::{actor::Command, BrowserError, BrowserResponse, BrowserState, BrowserStatus};
use super::status::{read_status, remaining_ms, usable_manager, write_state};
use super::BrowserManager;
use tokio::sync::oneshot;

impl BrowserManager {
    /// True while the actor task can still serve commands.
    ///
    /// A closed command channel means the actor exited (browser gone or a fatal
    /// error); a terminal `Stopped` status means the same. Either way the
    /// runtime rebuilds the manager instead of reusing a dead handle.
    pub(crate) fn is_alive(&self) -> bool {
        if self.tx.is_closed() {
            return false;
        }
        self.status
            .read()
            .map_or(true, |status| usable_manager(status.state))
    }

    pub(crate) async fn fetch(
        &self,
        request: crate::runtime::source::request::RequestSpec,
    ) -> Result<BrowserResponse, BrowserError> {
        if !self.gate.is_ready() {
            return Err(self.gated_error());
        }
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Fetch { request, reply })
            .await
            .map_err(|_| BrowserError::Transport)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) fn status(&self) -> BrowserStatus {
        let status = read_status(&self.status);
        let cooldown_ms = remaining_ms(self.clock.as_ref(), &self.cooldown_until);
        // Never report Ready when the gate is closed — a stale Ready
        // observation from a previous bootstrap would incorrectly signal
        // that the browser is available. Convert to Challenged instead.
        // Preserve CoolingDown state without reclassification.
        let state = match status.state {
            BrowserState::Ready if !self.gate.is_ready() => BrowserState::Challenged,
            BrowserState::CoolingDown => BrowserState::CoolingDown,
            other => other,
        };
        BrowserStatus {
            state,
            cooldown_ms,
            ..status
        }
    }

    pub(crate) async fn inspect(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Inspect { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) async fn recover(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Recover { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    /// Operator-requested relaunch: re-arm the one-shot recovery latch and
    /// re-run the bootstrap navigation. Each explicit call is one bounded
    /// attempt; a session that latches again escalates back to human action.
    pub(crate) async fn restart(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Restart { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub(crate) fn mark_human_required(&self) {
        self.gate.revoke();
        write_state(&self.status, BrowserState::HumanRequired);
    }

    fn gated_error(&self) -> BrowserError {
        match read_status(&self.status).state {
            BrowserState::Challenged | BrowserState::HumanRequired => BrowserError::HumanRequired,
            BrowserState::Ready => BrowserError::Unavailable,
            BrowserState::CoolingDown | BrowserState::Restarting | BrowserState::Stopped => {
                BrowserError::Unavailable
            }
        }
    }
}
