use super::super::{actor::Command, BrowserError, BrowserOutcome, BrowserState, BrowserStatus};
use super::status::{read_status, remaining_ms, usable_manager, write_state};
use super::BrowserManager;
use tokio::sync::oneshot;

impl BrowserManager {
    pub fn is_alive(&self) -> bool {
        if self.tx.is_closed() {
            return false;
        }
        self.status
            .read()
            .map_or(true, |status| usable_manager(status.state))
    }

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
