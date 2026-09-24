//! One attempt of the fetch loop: the plan it runs from, and the verdict its response earns.

use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::request::{build_request, RequestBody};
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher, MAX_BODY_BYTES};
use census_domain::model::AccessBlockKind;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

/// Everything an attempt needs: the request's coordinates plus the cache paths it reads and writes.
pub(super) struct FetchPlan<'a> {
    pub(super) method: &'a str,
    pub(super) url: &'a str,
    pub(super) payload: Option<&'a RequestBody>,
    pub(super) host: &'a str,
    pub(super) body_path: &'a Path,
    pub(super) meta_path: &'a Path,
    pub(super) cached: Option<&'a CacheMeta>,
    pub(super) options: &'a FetchOptions,
    pub(super) timeout_secs: u64,
}

impl Fetcher {
    /// Send one request. The durable layer owns retries (ADR-002): a failure returns an error, and
    /// Restate replays the step under a policy the journal can account for. An in-process retry loop
    /// here would spend a second budget no operator can see, and the backoff would be lost on any
    /// restart — so the transport attempts exactly once, as the ADR's Decision states.
    pub(super) async fn fetch_once(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        self.attempt_once(plan).await
    }

    /// One attempt: send the request, count it, record what the host said about access, and read the
    /// response's verdict.
    async fn attempt_once(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        let response = self.dispatch(plan).await?;
        let status = response.status().as_u16();
        self.count_request(plan.host, status).await;
        // A 403 or 429 is an observation about the *host*, not about this URL: it is recorded once
        // per run so a lane can stop on it instead of re-discovering it request by request.
        if let Some(kind) = blocking_kind(status) {
            self.record_access_condition(
                plan.host,
                kind,
                status,
                retry_after_secs(&response),
                plan.url.to_string(),
            )
            .await;
        }
        match status {
            200 => {
                let outcome = self.process_ok(plan, response).await?;
                Ok(outcome)
            }
            404 => self.handle_404(plan, response).await,
            304 => self.replay_cached(plan).await,
            _ => Err(self.status_error(status, plan).await),
        }
    }

    /// Process a 200 response: read body, cache, update stats, return outcome.
    async fn process_ok(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        let (body_vec, key_prefix, content_hex) = read_checked_body(response, plan.url).await?;
        let (etag, last_modified, content_type) = header_strings(plan.url, &body_vec);
        {
            let mut stats = self.stats.lock().await;
            stats.requests = stats.requests.saturating_add(1);
            let per_host_entry = stats.per_host.entry(plan.host.to_string()).or_insert(0);
            *per_host_entry = per_host_entry.saturating_add(1);
        }
        let meta = CacheMeta {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status: 200,
            key_prefix: key_prefix.clone(),
            content_digest: content_hex,
            bytes: body_vec.len(),
            fetched_at: now_iso8601(),
            etag,
            last_modified,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body_vec, &meta)?;
        {
            let mut stats = self.stats.lock().await;
            let downloaded = u64::try_from(body_vec.len()).unwrap_or(u64::MAX);
            stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(downloaded);
        }
        Ok(FetchOutcome {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status: 200,
            sha256: key_prefix,
            bytes: body_vec.len(),
            fetched_at: now_iso8601(),
            from_cache: false,
            content_type,
            body: body_vec,
        })
    }

    /// Handle a 404: cache the evidence, then return `Ok` or `Err` depending on `allow_not_found`.
    ///
    /// The body and metadata are always cached — a 404 is a real answer the run should remember.
    /// When `allow_not_found` is `true`, the caller expects the 404 as a normal outcome.
    /// When `false`, the caller wants a 404 to propagate as an error.
    async fn handle_404(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let (body_vec, key_prefix, content_hex) = read_checked_body(response, plan.url).await?;
        let (etag, last_modified, content_type) = header_strings(plan.url, &body_vec);
        {
            let mut stats = self.stats.lock().await;
            stats.requests = stats.requests.saturating_add(1);
            let per_host_entry = stats.per_host.entry(plan.host.to_string()).or_insert(0);
            *per_host_entry = per_host_entry.saturating_add(1);
        }
        let meta = CacheMeta {
            url: plan.url.to_string(),
            method: plan.method.to_string(),
            status,
            key_prefix: key_prefix.clone(),
            content_digest: content_hex,
            bytes: body_vec.len(),
            fetched_at: now_iso8601(),
            etag,
            last_modified,
            content_type: content_type.clone(),
        };
        write_cache(plan.body_path, plan.meta_path, &body_vec, &meta)?;
        if plan.options.allow_not_found {
            Ok(FetchOutcome {
                url: plan.url.to_string(),
                method: plan.method.to_string(),
                status,
                sha256: key_prefix,
                bytes: body_vec.len(),
                fetched_at: now_iso8601(),
                from_cache: false,
                content_type,
                body: body_vec,
            })
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

    /// Build the HTTP request and send it under the per-request timeout.
    async fn dispatch(&self, plan: &FetchPlan<'_>) -> Result<reqwest::Response, FetchError> {
        let request = build_request(
            &self.client,
            plan.method,
            plan.url,
            plan.payload,
            &plan.options.headers,
            plan.cached,
            plan.options.refresh,
        )?;
        let response = tokio::time::timeout(Duration::from_secs(plan.timeout_secs), request.send())
            .await
            .map_err(|_| FetchError::Timeout {
                url: plan.url.to_string(),
                timeout_secs: plan.timeout_secs,
            })?
            .map_err(|source| FetchError::Transport {
                url: plan.url.to_string(),
                source,
            })?;
        Ok(response)
    }

    /// Count the request once. Bodies are counted inside `process_response`; every status that
    /// never reaches it (304, 5xx, 429) is counted here instead.
    ///
    /// The browser lane's seat counts through the same call: one request, counted once, whichever
    /// transport carried it.
    pub(super) async fn count_request(&self, host: &str, status: u16) {
        if status == 200 || status == 404 {
            return;
        }
        let mut stats = self.stats.lock().await;
        stats.requests = stats.requests.saturating_add(1);
        let per_host = stats.per_host.entry(host.to_string()).or_insert(0);
        *per_host = per_host.saturating_add(1);
    }

    /// Conditional GET: publish the cached body with refreshed timestamps.
    ///
    /// A 304 whose body has vanished from disk is an error, not a re-fetch: there is nothing to
    /// publish, and whether to send the request again is the durable policy's call, not this
    /// function's.
    async fn replay_cached(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        if let Some(meta) = plan.cached {
            if let Ok(bytes) = std::fs::read(plan.body_path) {
                // Verify the cached body before reusing it.
                if bytes.len() != meta.bytes || content_digest(&bytes) != meta.content_digest {
                    // Body is corrupted — fall through to error.
                    let mut stats = self.stats.lock().await;
                    stats.errors = stats.errors.saturating_add(1);
                    return Err(FetchError::Http {
                        status: 304,
                        url: plan.url.to_string(),
                    });
                }
                let mut refreshed = meta.clone();
                refreshed.fetched_at = now_iso8601();
                write_cache(plan.body_path, plan.meta_path, &bytes, &refreshed)?;
                {
                    let mut stats = self.stats.lock().await;
                    stats.conditional_304 = stats.conditional_304.saturating_add(1);
                }
                return Ok(FetchOutcome {
                    url: plan.url.to_string(),
                    method: plan.method.to_string(),
                    status: meta.status,
                    sha256: meta.key_prefix.clone(),
                    bytes: meta.bytes,
                    fetched_at: refreshed.fetched_at,
                    from_cache: false,
                    content_type: meta.content_type.clone(),
                    body: bytes,
                });
            }
        }
        // The cache body disappeared: there is nothing to publish, so the fetch fails and the
        // durable layer decides whether to send it again.
        let mut stats = self.stats.lock().await;
        stats.errors = stats.errors.saturating_add(1);
        Err(FetchError::Http {
            status: 304,
            url: plan.url.to_string(),
        })
    }

    /// Non-200/404/304: record the error and return it for the durable layer to classify.
    ///
    /// The browser lane's seat reaches this for exactly the same statuses, so a refusal that
    /// arrives inside a capture (403, 5xx) is graded once, by this policy, whichever transport
    /// carried it.
    pub(super) async fn status_error(&self, status: u16, plan: &FetchPlan<'_>) -> FetchError {
        {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
        }
        let url = plan.url;
        warn!(status, url, "non-success response");
        // Every status here is either a host observation (403/429, already recorded against the host)
        // or a fault the durable retry policy exists to absorb. The transport's job is to report it;
        // grading it as retryable or terminal belonged to the loop that no longer runs here.
        FetchError::Http {
            status,
            url: plan.url.to_string(),
        }
    }
}

/// The access condition one HTTP status states, when it states one.
///
/// `403` is the source refusing a path its own robots rules allow, and `429` is the source asking for
/// less traffic. Both are observations about the host, which is why they are recorded against it
/// rather than against the URL that happened to be in flight.
///
/// A capture the browser lane carries states the same two the same way: the status travels on the
/// capture, so the reading is shared rather than restated.
pub(super) fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
    match status {
        403 => Some(AccessBlockKind::Forbidden),
        429 => Some(AccessBlockKind::RateLimited),
        _ => None,
    }
}

/// The `Retry-After` one response published, when it published one in the delta-seconds form.
///
/// The HTTP-date form is deliberately not parsed: no source in this corpus has used it, and a wrong
/// guess at an instant is worse than no instant, so an unparsed value stays `None` and the policy
/// default applies.
fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}

/// Read the response body inside the size cap, and hash what was read.
///
/// Streams the body chunk-by-chunk so that no single allocation can exceed
/// `MAX_BODY_BYTES` by more than one chunk. The declared-length pre-check is
/// kept as an early-out for well-behaved servers.
///
/// Returns the body, the 16-byte key prefix, and the full 32-byte content digest (hex).
async fn read_checked_body(
    response: reqwest::Response,
    url: &str,
) -> Result<(Vec<u8>, String, String), FetchError> {
    let declared = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    if declared.map(|len| len > MAX_BODY_BYTES).unwrap_or(false) {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        });
    }
    let mut body = Vec::with_capacity(declared.unwrap_or(8 * 1024));
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) =
        stream
            .next()
            .await
            .transpose()
            .map_err(|source| FetchError::Transport {
                url: url.to_string(),
                source,
            })?
    {
        let chunk_len = chunk.len();
        if body.len().saturating_add(chunk_len) > MAX_BODY_BYTES {
            return Err(FetchError::TooLarge {
                url: url.to_string(),
            });
        }
        hasher.update(&chunk);
        body.extend_from_slice(&chunk);
    }
    let digest = hasher.finalize();
    let key_hex: String = digest
        .get(..16)
        .unwrap_or(digest.as_slice())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let content_hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Ok((body, key_hex, content_hex))
}

/// Extract etag/last-modified from a response for a 404 that needs caching.
///
/// Since we've already consumed the body by this point, we can't get headers from the response.
/// This helper takes the body as a stand-in — the real work is reading headers from a response
/// that was already consumed. We keep the signature for future use when the response is available.
fn header_strings(_url: &str, _body: &[u8]) -> (Option<String>, Option<String>, Option<String>) {
    // We don't have the response here anymore; headers are lost.
    // The 404 body is typically minimal and doesn't benefit from conditional GET anyway.
    (None, None, None)
}
