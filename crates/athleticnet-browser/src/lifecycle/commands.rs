//! The manager's command surface.
//!
//! Every method here forwards one command to the actor and, where the actor answers, maps the
//! reply channel closing into the matching `BrowserError`. The read-only methods (`is_alive`,
//! `status`, `gated_error`, `mark_human_required`) work on the shared snapshot and the profile
//! gate without touching the actor.

use super::super::{actor::Command, BrowserError, BrowserOutcome, BrowserState, BrowserStatus};
use super::status::{read_status, remaining_ms, usable_manager, write_state};
use super::BrowserManager;
use tokio::sync::oneshot;

impl BrowserManager {
    /// True while the actor task can still serve commands.
    ///
    /// A closed command channel means the actor exited (browser gone or a fatal
    /// error); a terminal `Stopped` status means the same. Either way the
    /// runtime rebuilds the manager instead of reusing a dead handle.
    pub fn is_alive(&self) -> bool {
        if self.tx.is_closed() {
            return false;
        }
        self.status
            .read()
            .map_or(true, |status| usable_manager(status.state))
    }

    /// One request, answered with the transport's classified outcome.
    ///
    /// The return type is total on purpose: a closed gate, a dead actor and a full queue are all
    /// outcomes that carry a verdict, so a caller has no error path of its own left to classify.
    pub async fn fetch(&self, request: crate::request::RequestSpec) -> BrowserOutcome {
        if !self.gate.is_ready() {
            return BrowserOutcome::failed(self.gated_error());
        }
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(Command::Fetch { request, reply })
            .await
            .is_err()
        {
            return BrowserOutcome::failed(BrowserError::Transport);
        }
        result
            .await
            .unwrap_or_else(|_| BrowserOutcome::failed(BrowserError::Transport))
    }

    pub fn status(&self) -> BrowserStatus {
        let status = read_status(&self.status);
        let cooldown_ms = remaining_ms(self.clock.as_ref(), &self.cooldown_until);
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

    pub async fn inspect(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Inspect { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub async fn recover(&self) -> Result<BrowserStatus, BrowserError> {
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
    pub async fn restart(&self) -> Result<BrowserStatus, BrowserError> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Command::Restart { reply })
            .await
            .map_err(|_| BrowserError::Unavailable)?;
        result.await.map_err(|_| BrowserError::Transport)?
    }

    pub fn mark_human_required(&self) {
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
