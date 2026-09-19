use super::is_challenge;
use crate::runtime::browser::actor::Actor;
use crate::runtime::browser::{
    actor::{ChallengeTarget, JobResult},
    pool::Pending,
    transport, BrowserError, BrowserResponse,
};
use tokio::sync::oneshot;

impl Actor {
    pub(in crate::runtime::browser) fn accept_fetch(
        &mut self,
        request: crate::runtime::source::request::RequestSpec,
        reply: oneshot::Sender<Result<BrowserResponse, BrowserError>>,
    ) {
        if !self.gate.is_ready() {
            let _ = reply.send(Err(BrowserError::HumanRequired));
            return;
        }
        if self.pending.len() >= self.queue_capacity {
            let _ = reply.send(Err(BrowserError::Unavailable));
            return;
        }
        self.pending.push_back(Pending { request, reply });
    }

    pub(in crate::runtime::browser) fn schedule_pending(&mut self) {
        if !self.gate.is_ready() || self.shutdown.is_cancelled() {
            return;
        }
        while let Some(slot) = self.pages.iter().position(|value| !value.busy) {
            let Some(item) = self.pending.pop_front() else {
                break;
            };
            let page = self.pages[slot].page.clone();
            self.pages[slot].busy = true;
            self.challenge_target = Some(ChallengeTarget {
                url: item.request.url.clone(),
                post: item.request.body.is_some(),
            });
            let gate = self.gate.clone();
            let shutdown = self.shutdown.clone();
            let timeout = self.settings.request_timeout;
            let request = item.request.clone();
            let is_rankings = matches!(
                &request.action,
                crate::runtime::source::request::RequestAction::Rankings(_)
            );
            let nonce = self.next_capture_nonce();
            let source_origin = self.settings.source_origin.clone();
            self.jobs.spawn(async move {
                let result = if is_rankings {
                    if let crate::runtime::source::request::RequestAction::Rankings(ref action) = request.action {
                        tokio::select! {
                            _ = shutdown.cancelled() => Err(BrowserError::Shutdown),
                            result = transport::fetch_rankings(&page, action, timeout, gate, &source_origin, nonce) => result,
                        }
                    } else {
                        Err(BrowserError::Protocol)
                    }
                } else {
                    tokio::select! {
                        _ = shutdown.cancelled() => Err(BrowserError::Shutdown),
                        result = transport::fetch(&page, &request, timeout, gate) => result,
                    }
                };
                JobResult {
                    slot,
                    request,
                    reply: item.reply,
                    result,
                }
            });
        }
    }

    pub(in crate::runtime::browser) fn complete_job(
        &mut self,
        job: Option<Result<JobResult, tokio::task::JoinError>>,
    ) {
        match job {
            Some(Ok(job)) => {
                if let Some(slot) = self.pages.get_mut(job.slot) {
                    slot.busy = false;
                }
                if let Ok(response) = &job.result {
                    if is_challenge(response) {
                        self.latch_challenge(Some(&job.request));
                    }
                    if response.status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.apply_cooldown(response);
                    }
                }
                let _ = job.reply.send(job.result);
            }
            Some(Err(_)) => {
                tracing::warn!("browser job panicked or was aborted; requesting shutdown");
                self.jobs.abort_all();
                self.pages.iter_mut().for_each(|p| p.busy = false);
                self.gate.revoke();
                self.panic_shutdown = true;
            }
            None => {}
        }
        self.update_active();
    }
}
