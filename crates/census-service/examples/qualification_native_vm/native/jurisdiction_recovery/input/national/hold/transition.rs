use anyhow::{ensure, Result};
use futures::TryStreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub(super) enum Transition {
    Pause,
    Resume,
}

impl Transition {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
        }
    }

    pub(super) fn accepts(self, status: &str) -> bool {
        match self {
            Self::Pause => matches!(status, "running" | "suspended" | "ready"),
            Self::Resume => status == "paused",
        }
    }

    pub(super) fn reached(self, status: &str) -> bool {
        match self {
            Self::Pause => status == "paused",
            Self::Resume => matches!(status, "running" | "suspended" | "ready" | "completed"),
        }
    }

    pub(super) async fn send(self, client: &Client, id: &str) -> Result<Value> {
        let action = self.name();
        let response = client
            .patch(format!("http://127.0.0.1:19095/invocations/{id}/{action}"))
            .send()
            .await?;
        let status = response.status().as_u16();
        let body = response
            .bytes_stream()
            .map_err(anyhow::Error::from)
            .try_fold(Vec::new(), |mut bytes, part| async move {
                ensure!(
                    bytes
                        .len()
                        .checked_add(part.len())
                        .is_some_and(|size| size <= 4096),
                    "native control acknowledgement exceeds four KiB"
                );
                bytes.try_reserve(part.len())?;
                bytes.extend_from_slice(&part);
                Ok(bytes)
            })
            .await?;
        ensure!(
            matches!(status, 200 | 202),
            "native {action} refused with HTTP {status}"
        );
        Ok(json!({"status":status,"response_bytes":body.len(),
            "response_sha256":format!("{:x}",Sha256::digest(&body))}))
    }
}

#[cfg(test)]
mod tests {
    use super::Transition;
    use anyhow::{ensure, Result};

    #[test]
    fn native_control_refuses_resume_of_a_running_or_completed_original() -> Result<()> {
        ensure!(
            Transition::Resume.accepts("paused"),
            "paused original refused"
        );
        for status in [
            "running",
            "suspended",
            "ready",
            "completed",
            "pending",
            "backing-off",
        ] {
            ensure!(
                !Transition::Resume.accepts(status),
                "resume accepted {status}"
            );
        }
        ensure!(
            !Transition::Pause.accepts("completed"),
            "completed original paused"
        );
        ensure!(
            !Transition::Pause.reached("running"),
            "unsettled pause accepted"
        );
        ensure!(
            Transition::Resume.reached("completed"),
            "completed resume race refused"
        );
        Ok(())
    }
}
