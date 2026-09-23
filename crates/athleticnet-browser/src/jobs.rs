use crate::actor::Actor;
use crate::{
    actor::{ChallengeTarget, JobResult},
    pool::Pending,
    transport, BrowserError, BrowserOutcome,
};
use tokio::sync::oneshot;
use tracing::Instrument;

impl Actor {
    pub(crate) fn accept_fetch(
        &mut self,
        request: crate::request::RequestSpec,
        reply: oneshot::Sender<BrowserOutcome>,
    ) {
        if !self.gate.is_ready() {
            if reply
                .send(BrowserOutcome::failed(BrowserError::HumanRequired))
                .is_err()
            {
                tracing::debug!("human required reply dropped");
            }
            return;
        }
        if self.pending.len() >= self.queue_capacity {
            if reply
                .send(BrowserOutcome::failed(BrowserError::Unavailable))
                .is_err()
            {
                tracing::debug!("unavailable reply dropped");
            }
            return;
        }
        self.pending.push_back(Pending { request, reply });
    }

    pub(crate) fn schedule_pending(&mut self) {
        if !self.gate.is_ready() || self.shutdown.is_cancelled() {
            return;
        }
        while let Some(slot) = self.pages.iter().position(|value| !value.busy) {
            let Some(item) = self.pending.pop_front() else {
                break;
            };
            let page = match self.pages.get(slot) {
                Some(slot) => slot.page.clone(),
                None => continue,
            };
            if let Some(slot) = self.pages.get_mut(slot) {
                slot.busy = true;
            }
            self.challenge_target = Some(ChallengeTarget {
                url: item.request.url.clone(),
                post: item.request.body().is_some(),
            });
            let gate = self.gate.clone();
            let shutdown = self.shutdown.clone();
            let timeout = self.settings.request_timeout;
            let request = item.request.clone();
            let is_rankings = matches!(&request.action, crate::request::RequestAction::Rankings(_));
            let nonce = self.next_capture_nonce();
            let source_origin = self.settings.source_origin.clone();
            let clock = self.clock.clone();
            let span = tracing::info_span!("browser.job", slot, nonce);
            self.jobs.spawn(
                async move {
                let result = if is_rankings {
                    if let crate::request::RequestAction::Rankings(ref action) = request.action {
                        tokio::select! {
                            _ = shutdown.cancelled() => Err(BrowserError::Shutdown),
                            result = transport::fetch_rankings(&page, action, timeout, gate, &source_origin, nonce, clock.as_ref()) => result,
                        }
                    } else {
                        Err(BrowserError::Protocol)
                    }
                } else {
                    tokio::select! {
                        _ = shutdown.cancelled() => Err(BrowserError::Shutdown),
                        result = transport::fetch(&page, &request, timeout, gate, clock.as_ref()) => result,
                    }
                };
                JobResult {
                    slot,
                    request,
                    reply: item.reply,
                    result,
                }
                }
                .instrument(span),
            );
        }
    }

    pub(crate) fn complete_job(&mut self, job: Option<Result<JobResult, tokio::task::JoinError>>) {
        match job {
            Some(Ok(job)) => {
                if let Some(slot) = self.pages.get_mut(job.slot) {
                    slot.busy = false;
                }
                let outcome = match job.result {
                    Ok(response) => BrowserOutcome::captured(response, self.clock.as_ref()),
                    Err(error) => BrowserOutcome::failed(error),
                };
                if let BrowserOutcome::Captured(capture) = &outcome {
                    // The verdict the caller receives drives the profile's own state: the latch and
                    // the cooldown read that one classification, so a caller can never observe
                    // "challenged" on a gate that stayed ready.
                    if capture.challenge {
                        self.latch_challenge(Some(&job.request));
                    }
                    if capture.response.status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.apply_cooldown(&capture.response);
                    }
                }
                if job.reply.send(outcome).is_err() {
                    tracing::debug!("job reply dropped");
                }
            }
            Some(Err(error)) if error.is_panic() => {
                tracing::error!("browser job panicked; requesting shutdown");
                self.abort_jobs(BrowserError::TaskPanicked);
            }
            Some(Err(error)) if error.is_cancelled() => {
                tracing::debug!("browser job cancelled; releasing slots");
                self.abort_jobs(BrowserError::Unavailable);
            }
            Some(Err(_)) => {
                tracing::warn!("browser job aborted; requesting shutdown");
                self.abort_jobs(BrowserError::Unavailable);
            }
            None => {}
        }
        self.update_active();
    }

    /// Release every page slot and latch a shutdown after a job ended outside its own result path.
    fn abort_jobs(&mut self, cause: BrowserError) {
        self.jobs.abort_all();
        self.pages.iter_mut().for_each(|p| p.busy = false);
        self.gate.revoke();
        crate::pool::reject_pending(&mut self.pending, cause);
        self.panic_shutdown = true;
    }
}
