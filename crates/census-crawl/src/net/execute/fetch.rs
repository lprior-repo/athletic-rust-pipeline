use super::attempt::FetchPlan;
use super::{representation, request_target};
use crate::net::cache::{read_cache, replay_cache, CacheMeta};
use crate::net::request::RequestBody;
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::time::Instant;

impl Fetcher {
    pub(in crate::net) async fn fetch(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
    ) -> Result<FetchOutcome, FetchError> {
        let mut session = crate::net::acquisition::CaptureSession::Legacy;
        self.fetch_with_session(method, url, body, options, timeout_secs, &mut session)
            .await
    }

    pub(in crate::net) async fn fetch_with_session(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
        session: &mut crate::net::acquisition::CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        self.fetch_with_scope(
            method,
            url,
            body,
            options,
            timeout_secs,
            session,
            crate::net::public_http::TransportScope::Provider,
        )
        .await
    }

    pub(in crate::net) async fn fetch_with_scope(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
        session: &mut crate::net::acquisition::CaptureSession<'_>,
        scope: crate::net::public_http::TransportScope<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        scope.check(url)?;
        self.destination.validate_url(url)?;
        let representation = representation::canonical_request(url, &options.headers)?;
        let extra = representation::cache_extra(body.as_ref(), &representation);
        let key = Self::key_for(method, url, &extra);
        let (body_path, meta_path) = self.cache_paths(&key);
        let mut cached = if session.is_adopted() {
            session.cached().cloned().map(|meta| (meta, Vec::new()))
        } else {
            read_cache(&body_path, &meta_path, method, url, &representation)?
        };
        if !session.is_adopted() {
            if let Some(outcome) = self.reuse_cached(&mut cached, options, &key).await? {
                return scope.accept(outcome);
            }
        }
        if self.offline {
            return Err(FetchError::Offline { url: url.into() });
        }

        let host = self.admit_transport_host(url).await?;
        let gate = self.host_gate(&host).await;
        let plan = FetchPlan {
            scope,
            method,
            url,
            payload: body.as_ref().map(|(_, payload)| payload),
            host: &host,
            family: self.family_of(&host),
            body_path: &body_path,
            meta_path: &meta_path,
            cached: cached.as_ref().map(|(meta, _)| meta),
            options,
            representation: &representation,
            timeout_secs,
        };
        let _family_permit = self.family_permit(&host).await?;
        let started = Instant::now();
        let outcome = self
            .fetch_admitted_transport(gate, &plan, session)
            .await
            .and_then(|outcome| scope.accept(outcome));
        self.record_transport(started, &outcome).await;
        outcome
    }

    async fn admit_transport_host(&self, url: &str) -> Result<String, FetchError> {
        let (host, origin) = request_target(url)?;
        if let Some(reason) = crate::registry::acquisition_refusal_for_host(&host) {
            return Err(FetchError::Policy {
                detail: reason.to_string(),
            });
        }
        self.origin_locks.ensure(&origin)?;
        if let Some(refusal) = self.refused_admission(&host, url).await {
            return Err(refusal);
        }
        if self.host_blocked(&host, &crate::net::now_iso8601()).await {
            return Err(FetchError::Cooldown { host });
        }
        Ok(host)
    }

    async fn fetch_admitted_transport(
        &self,
        gate: std::sync::Arc<tokio::sync::Mutex<()>>,
        plan: &FetchPlan<'_>,
        session: &mut crate::net::acquisition::CaptureSession<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        if matches!(
            plan.scope,
            crate::net::public_http::TransportScope::PublicHttp(_)
        ) && crate::registry::transport_for_host(plan.host)
            == Some(crate::registry::TransportKind::Browser)
        {
            return Err(FetchError::Policy {
                detail: "public-source research requires the public HTTP lane".to_owned(),
            });
        }
        match crate::registry::transport_for_host(plan.host) {
            Some(crate::registry::TransportKind::Browser) if session.is_adopted() => {
                Err(crate::net::CaptureTransportError::UnsupportedLane.into())
            }
            Some(crate::registry::TransportKind::Browser) => self.fetch_browser(gate, plan).await,
            _ => self.fetch_once(plan, session).await,
        }
    }

    async fn reuse_cached(
        &self,
        cached: &mut Option<(CacheMeta, Vec<u8>)>,
        options: &FetchOptions,
        key: &str,
    ) -> Result<Option<FetchOutcome>, FetchError> {
        let (body_path, meta_path) = self.cache_paths(key);
        match cached {
            Some((meta, body)) if !options.refresh => {
                drop(std::mem::take(body));
                let Some(body) = replay_cache(&body_path, &meta_path, meta)? else {
                    return Ok(None);
                };
                self.cached_outcome(&meta.method, &meta.url, meta, options, body)
                    .await
            }
            _ => Ok(None),
        }
    }

    async fn cached_outcome(
        &self,
        method: &str,
        url: &str,
        meta: &CacheMeta,
        options: &FetchOptions,
        body: Vec<u8>,
    ) -> Result<Option<FetchOutcome>, FetchError> {
        if options.refresh {
            return Ok(None);
        }
        if meta.status != 200 && !(options.allow_not_found && meta.status == 404) {
            return Ok(None);
        }
        {
            let mut stats = self.stats.lock().await;
            stats.cache_hits = stats.cache_hits.saturating_add(1);
            stats.requests = stats.requests.saturating_add(1);
            let entry = stats.per_host.entry(crate::net::host_of(url)).or_default();
            entry.requests = entry.requests.saturating_add(1);
            entry.cache_hits = entry.cache_hits.saturating_add(1);
        }
        Ok(Some(FetchOutcome {
            url: url.to_string(),
            response_url: meta.response_url.clone(),
            method: method.to_string(),
            status: meta.status,
            content_digest: meta.content_digest.clone(),
            bytes: meta.bytes,
            fetched_at: meta.fetched_at.clone(),
            from_cache: true,
            content_type: meta.content_type.clone(),
            body,
        }))
    }
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "acquisition_policy_tests.rs"]
mod acquisition_policy_tests;

#[cfg(test)]
#[path = "tfrrs_policy_tests.rs"]
mod tfrrs_policy_tests;
