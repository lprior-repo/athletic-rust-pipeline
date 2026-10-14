pub(super) use super::attempt_helper::blocking_kind;
use super::attempt_helper::retry_after_secs;
use super::cache_writer::{cache_and_record, record_response};
use crate::net::acquisition::CaptureSession;
use crate::net::cache::{replay_cache, CacheMeta};
use crate::net::request::RequestBody;
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::path::Path;
use tokio::sync::OwnedMutexGuard;
use tracing::warn;

pub(super) struct FetchPlan<'a> {
    pub(super) method: &'a str,
    pub(super) url: &'a str,
    pub(super) payload: Option<&'a RequestBody>,
    pub(super) host: &'a str,
    pub(super) family: Option<String>,
    pub(super) body_path: &'a Path,
    pub(super) meta_path: &'a Path,
    pub(super) cached: Option<&'a CacheMeta>,
    pub(super) options: &'a FetchOptions,
    pub(super) representation: &'a crate::net::RepresentationHeaders,
    pub(super) timeout_secs: u64,
}

impl Fetcher {
    pub(super) async fn fetch_once(
        &self,
        plan: &FetchPlan<'_>,
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let mut current_url = plan.url.to_string();
        let original_url = plan.url.to_string();
        let mut redirects: Vec<crate::net::RedirectHop> = Vec::new();
        let mut permit: Option<OwnedMutexGuard<()>> = None;
        for hop in 0..5 {
            let (host, _hop_permit) = self.admit_hop(plan, &current_url, hop, &mut permit).await?;
            let response = self.dispatch_at(&current_url, &host, plan, session).await?;
            let status = response.status().as_u16();
            let target = match self.resolve_redirect_target(&current_url, &original_url, &response)
            {
                Ok(target) => target,
                Err(error) => {
                    session.refuse(census_domain::model::AcquisitionFailureV1::RedirectRefused)?;
                    return Err(error);
                }
            };
            match target {
                Some(next_url) => {
                    redirects.push(crate::net::RedirectHop {
                        status,
                        url: next_url.to_string(),
                    });
                    current_url = next_url.to_string();
                }
                None => {
                    return self
                        .handle_final_response(
                            plan,
                            &current_url,
                            &host,
                            response,
                            &redirects,
                            session,
                        )
                        .await
                }
            }
        }
        session.refuse(census_domain::model::AcquisitionFailureV1::RedirectLimit)?;
        Err(FetchError::Policy {
            detail: format!("redirect limit exceeded after 5 hops for {original_url}"),
        })
    }

    async fn admit_hop(
        &self,
        plan: &FetchPlan<'_>,
        current_url: &str,
        hop: usize,
        permit: &mut Option<OwnedMutexGuard<()>>,
    ) -> Result<(String, Option<tokio::sync::OwnedSemaphorePermit>), FetchError> {
        let (host, origin) = super::request_target(current_url)?;
        self.destination.validate_url(current_url)?;
        self.origin_locks.ensure(&origin)?;
        if self.host_blocked(&host, &crate::net::now_iso8601()).await {
            return Err(FetchError::Cooldown { host });
        }
        let hop_permit = if hop == 0 || self.family_of(&host) == plan.family {
            None
        } else {
            self.family_permit_now(&host).await?
        };
        let gate = self.host_gate(&host).await;
        drop(permit.take());
        *permit = Some(gate.lock_owned().await);
        self.wait_turn(&host).await;
        if self.host_blocked(&host, &crate::net::now_iso8601()).await {
            return Err(FetchError::Cooldown { host });
        }
        Ok((host, hop_permit))
    }

    async fn handle_final_response(
        &self,
        plan: &FetchPlan<'_>,
        current_url: &str,
        current_host: &str,
        response: reqwest::Response,
        redirects: &[crate::net::RedirectHop],
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let status = response.status().as_u16();
        if let Some(kind) = blocking_kind(status) {
            self.record_access_condition(
                current_host,
                kind,
                status,
                retry_after_secs(&response),
                current_url.to_string(),
            )
            .await;
        }
        match status {
            200 => self.process_ok(plan, response, redirects, session).await,
            404 => self.handle_404(plan, response, redirects, session).await,
            304 => self.replay_cached(plan, session).await,
            _ => {
                self.handle_refusal(plan, response, redirects, status, session)
                    .await
            }
        }
    }

    async fn handle_refusal(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
        redirects: &[crate::net::RedirectHop],
        status: u16,
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        if let Err(error) = record_response(response, plan, status, redirects, session).await {
            if session.is_adopted() {
                return Err(error);
            }
            self.record_refusal_evidence_loss(
                plan.url,
                status,
                crate::net::EvidenceLane::Http,
                error.to_string(),
            )
            .await;
            warn!(
                status,
                url = plan.url,
                error = %error,
                "refusal capture was not retained"
            );
        }
        Err(self.status_error(status, plan).await)
    }

    async fn process_ok(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
        redirects: &[crate::net::RedirectHop],
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        cache_and_record(response, plan, 200, &self.stats, redirects, session).await
    }

    async fn handle_404(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
        redirects: &[crate::net::RedirectHop],
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let outcome =
            cache_and_record(response, plan, status, &self.stats, redirects, session).await?;
        if plan.options.allow_not_found {
            Ok(outcome)
        } else {
            {
                let mut stats = self.stats.lock().await;
                stats.errors = stats.errors.saturating_add(1);
                warn!(status, url = plan.url, "non-success response");
            }
            Err(FetchError::Http {
                status,
                url: plan.url.to_string(),
            })
        }
    }

    async fn replay_cached(
        &self,
        plan: &FetchPlan<'_>,
        session: &mut CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        if let Some(outcome) = session.replay_not_modified(plan.body_path, plan.meta_path)? {
            let mut stats = self.stats.lock().await;
            stats.conditional_304 = stats.conditional_304.saturating_add(1);
            return Ok(outcome);
        }
        if let Some(meta) = plan.cached {
            if let Some(bytes) = replay_cache(plan.body_path, plan.meta_path, meta)? {
                {
                    let mut stats = self.stats.lock().await;
                    stats.conditional_304 = stats.conditional_304.saturating_add(1);
                }
                return Ok(FetchOutcome {
                    url: plan.url.to_string(),
                    response_url: meta.response_url.clone(),
                    method: plan.method.to_string(),
                    status: meta.status,
                    content_digest: meta.content_digest.clone(),
                    bytes: meta.bytes,
                    fetched_at: meta.fetched_at.clone(),
                    from_cache: false,
                    content_type: meta.content_type.clone(),
                    body: bytes,
                });
            }
        }
        let mut stats = self.stats.lock().await;
        stats.errors = stats.errors.saturating_add(1);
        Err(FetchError::Http {
            status: 304,
            url: plan.url.to_string(),
        })
    }
}

#[cfg(test)]
#[path = "attempt_accounting_tests.rs"]
mod attempt_accounting_tests;
