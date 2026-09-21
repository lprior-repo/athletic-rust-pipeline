//! Polite, resumable HTTP layer.
//!
//! Properties the rest of the crate relies on:
//!
//! * **robots.txt is enforced**, not advisory: a disallowed path returns [`FetchError::Robots`] and
//!   is counted in [`FetchStats::robots_blocked`] — unless the operator named that host on
//!   `--authorized-host`, in which case the rule is counted in [`FetchStats::robots_authorized`]
//!   and the request proceeds under the 2 rps ceiling below. Default: no host is authorized.
//! * **Hard pacing: 2 requests/second per host** — an authorized host never gets closer spacing than
//!   `MIN_AUTHORIZED_DELAY`, whatever `--delay-ms` says.
//! * Robots rules never leave the process. The rule set is fetched once per host, cached on disk and
//!   honoured for the remainder of the run.
//! * **Requests are cached on disk** by content hash, so a re-run is free and interrupted collections
//!   resume without re-fetching. Conditional GETs (`If-None-Match` / `If-Modified-Since`) are used when
//!   the origin supports them.
//! * **Politeness is per host**: at most one in-flight request per host, minimum spacing between
//!   requests (default 1 s, overridable per host — e.g. Bound's `Crawl-delay: 10`).
//! * No authentication, no cookie jar, no CAPTCHA handling, no challenge evasion.
//!
//! # Concurrency and timing guarantees
//!
//! * All shared state (hosts, robots, stats) uses `tokio::sync::Mutex` — no `std::sync::Mutex` is
//!   ever held across an `.await`.
//! * Every network request is guarded by a per-request timeout (default 45 s) and retries with
//!   bounded exponential backoff + jitter (max 3 attempts, 500 ms base delay).
//! * Response bodies are capped at 32 MiB; oversized responses return [`FetchError::TooLarge`].

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub const DEFAULT_USER_AGENT: &str =
    "midwest-census/0.1 (independent HS track & field research collector; polite; contact: repo owner)";

const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 45;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 500;

/// Hard pacing ceiling for an explicitly authorized host: 2 requests/second.
///
/// `robots.txt` stays authoritative by default. `--authorized-host` records an operator decision
/// that one host's rules are logged rather than enforced, because the operator has authorized that
/// host explicitly. Speed is *not* part of that authorization: the ceiling the collection policy
/// states (2 rps per host) is applied as a floor on spacing for every authorized host, so raising
/// `--delay-ms` never lets an authorized host be hit faster than the policy allows.
const MIN_AUTHORIZED_DELAY: Duration = Duration::from_millis(500);

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during fetch operations.
#[derive(Debug, Error)]
pub enum FetchError {
    #[error("robots.txt disallows {0}")]
    Robots(String),
    #[error("http status {status} for {url}")]
    Http { status: u16, url: String },
    #[error("response body for {url} exceeds {MAX_BODY_BYTES} bytes")]
    TooLarge { url: String },
    #[error("transport error for {url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("cache i/o for {path}: {source}")]
    Cache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("request timed out for {url} after {timeout_secs}s")]
    Timeout { url: String, timeout_secs: u64 },
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Request payload for POSTs: form pairs or a pre-serialized JSON body.
#[derive(Debug, Clone)]
enum RequestBody {
    Form(Vec<(String, String)>),
    Json(String),
}

/// Options controlling fetch behaviour.
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Ignore any cached body and hit the network (still robots-checked).
    pub refresh: bool,
    /// Treat a 404 as a normal (cached) outcome instead of an error.
    pub allow_not_found: bool,
    /// Extra request headers (e.g. `Accept: application/json`).
    pub headers: Vec<(String, String)>,
}

/// Outcome of a single fetch. The `body` field carries the raw bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOutcome {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub sha256: String,
    pub bytes: usize,
    pub fetched_at: String,
    pub from_cache: bool,
    pub content_type: Option<String>,
    #[serde(skip)]
    pub body: Vec<u8>,
}

impl FetchOutcome {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .with_context(|| format!("decoding JSON from {}", self.url))
    }
}

/// Metadata stored alongside a cached body on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheMeta {
    url: String,
    method: String,
    status: u16,
    sha256: String,
    bytes: usize,
    fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
}

/// Aggregated fetch statistics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FetchStats {
    pub requests: u64,
    pub cache_hits: u64,
    pub conditional_304: u64,
    pub robots_blocked: u64,
    /// Requests that a robots rule disallowed but an explicit host authorization permitted. Kept
    /// separate from `robots_blocked` so the run record shows exactly what was overridden.
    pub robots_authorized: u64,
    pub bytes_downloaded: u64,
    pub errors: u64,
    pub per_host: HashMap<String, u64>,
}

/// robots.txt rule set for one host origin.
#[derive(Debug, Default, Clone)]
struct RobotsRules {
    /// (allow?, path prefix)
    rules: Vec<(bool, String)>,
    crawl_delay: Option<Duration>,
    fetched: bool,
}

impl RobotsRules {
    fn allows(&self, path: &str) -> bool {
        if !self.fetched {
            return true; // absent robots.txt == allow
        }
        let mut best: Option<(usize, bool)> = None;
        for (allow, prefix) in &self.rules {
            if prefix.is_empty() {
                continue;
            }
            if path.starts_with(prefix.as_str()) {
                let len = prefix.len();
                match best {
                    Some((best_len, _)) if best_len > len => {}
                    Some((best_len, best_allow)) if best_len == len => {
                        // Allow wins ties.
                        if *allow && !best_allow {
                            best = Some((len, true));
                        }
                    }
                    _ => best = Some((len, *allow)),
                }
            }
        }
        best.map(|(_, allow)| allow).unwrap_or(true)
    }
}

/// Per-host politeness state.
struct HostState {
    gate: Arc<Mutex<()>>,
    /// When the next request to this host may start (reserved before sleeping so that the spacing
    /// holds even when several tasks queue behind the gate).
    next_allowed: Option<std::time::Instant>,
    delay: Duration,
}

// ---------------------------------------------------------------------------
// Fetcher
// ---------------------------------------------------------------------------

/// Polite, cache-first, per-host-rate-limited HTTP fetcher.
pub struct Fetcher {
    client: reqwest::Client,
    cache_dir: PathBuf,
    user_agent: String,
    default_delay: Duration,
    host_delays: HashMap<String, Duration>,
    /// Hosts whose robots rules are recorded rather than enforced (operator authorization).
    /// A bare domain authorizes its subdomains.
    authorized_hosts: Vec<String>,
    hosts: Mutex<HashMap<String, HostState>>,
    robots: Mutex<HashMap<String, RobotsRules>>,
    stats: Mutex<FetchStats>,
}

impl Fetcher {
    pub fn new(
        cache_dir: impl AsRef<Path>,
        user_agent: Option<String>,
        default_delay: Duration,
        host_delays: HashMap<String, Duration>,
        authorized_hosts: Vec<String>,
    ) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("creating cache dir {}", cache_dir.display()))?;
        let user_agent = user_agent.unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .user_agent(user_agent.clone())
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            client,
            cache_dir,
            user_agent,
            default_delay,
            host_delays,
            authorized_hosts: authorized_hosts
                .into_iter()
                .map(|host| host.trim().to_ascii_lowercase())
                .filter(|host| !host.is_empty())
                .collect(),
            hosts: Mutex::new(HashMap::new()),
            robots: Mutex::new(HashMap::new()),
            stats: Mutex::new(FetchStats::default()),
        })
    }

    /// Whether the operator explicitly authorized this host.
    ///
    /// An entry authorizes exactly the host it names plus anything below it: `athletic.net`
    /// authorizes `www.athletic.net` and `api.athletic.net`, while `www.example.com` authorizes only
    /// itself and its own subdomains — never the parent domain, so naming a narrow host can never
    /// widen into a whole site.
    pub fn is_authorized_host(&self, host: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        self.authorized_hosts
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    }

    /// Borrow the user-agent string without taking ownership.
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// Borrow the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Snapshot current fetch statistics.
    #[tracing::instrument(skip(self))]
    pub async fn stats(&self) -> FetchStats {
        self.stats.lock().await.clone()
    }

    fn cache_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.cache_dir.join(format!("{key}.body")),
            self.cache_dir.join(format!("{key}.meta.json")),
        )
    }

    fn key_for(method: &str, url: &str, extra: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        hasher.update(extra.as_bytes());
        sha256_prefix16(hasher)
    }

    /// Serialize per host and enforce the configured (or robots-requested) spacing.
    async fn host_gate(&self, host: &str, robots_delay: Option<Duration>) -> Arc<Mutex<()>> {
        let configured = self
            .host_delays
            .get(host)
            .copied()
            .unwrap_or(self.default_delay);
        let mut effective = match robots_delay {
            Some(robots) if robots > configured => robots,
            _ => configured,
        };
        // Authorization permits a host whose robots rules would otherwise block the request; it
        // never permits a faster rate than the collection policy's 2 rps ceiling.
        if self.is_authorized_host(host) && effective < MIN_AUTHORIZED_DELAY {
            effective = MIN_AUTHORIZED_DELAY;
        }
        let mut hosts = self.hosts.lock().await;
        let state = hosts.entry(host.to_string()).or_insert_with(|| HostState {
            gate: Arc::new(Mutex::new(())),
            next_allowed: None,
            delay: effective,
        });
        state.delay = state.delay.max(effective);
        state.gate.clone()
    }

    async fn wait_turn(&self, host: &str) {
        let wait = {
            let mut hosts = self.hosts.lock().await;
            let state = hosts
                .get_mut(host)
                .ok_or_else(|| anyhow::anyhow!("host {host} not registered in host_gate"));
            match state {
                Ok(s) => {
                    let now = std::time::Instant::now();
                    // The reserved slot is a floor: resume from the later of "now" and the slot,
                    // then push the slot one delay further. `checked_add` keeps the instant
                    // arithmetic panic-free, and a clock far enough out to overflow `Instant`
                    // cannot occur in a run, so "no reservation" is the honest answer there.
                    let from = match s.next_allowed {
                        Some(at) if at > now => at,
                        _ => now,
                    };
                    let wait = from.saturating_duration_since(now);
                    s.next_allowed = from.checked_add(s.delay);
                    wait
                }
                Err(e) => {
                    warn!("host not registered: {e}");
                    Duration::ZERO
                }
            }
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    async fn robots_for(&self, scheme_host: &str) -> RobotsRules {
        {
            let robots = self.robots.lock().await;
            if let Some(rules) = robots.get(scheme_host) {
                return rules.clone();
            }
        }
        let url = format!("{scheme_host}/robots.txt");
        let rules = match self.fetch_text_uncached(&url).await {
            Ok((200, body)) => parse_robots(&body),
            _ => RobotsRules {
                fetched: false,
                ..Default::default()
            },
        };
        debug!(
            host = scheme_host,
            rules = rules.rules.len(),
            "robots loaded"
        );
        self.robots
            .lock()
            .await
            .insert(scheme_host.to_string(), rules.clone());
        rules
    }

    async fn fetch_text_uncached(&self, url: &str) -> Result<(u16, String)> {
        let response =
            self.client
                .get(url)
                .send()
                .await
                .map_err(|source| FetchError::Transport {
                    url: url.to_string(),
                    source,
                })?;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        Ok((status, body))
    }

    /// GET a URL with caching, robots enforcement and per-host politeness.
    #[tracing::instrument(skip(self, options), fields(url, method = "GET"))]
    pub async fn get(&self, url: &str, options: &FetchOptions) -> Result<FetchOutcome> {
        tracing::Span::current().record("url", url);
        self.fetch("GET", url, None, options, REQUEST_TIMEOUT_SECS)
            .await
    }

    /// POST a form body; cached by body content so repeated runs are free.
    #[tracing::instrument(skip(self, options, form), fields(url, method = "POST"))]
    pub async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        options: &FetchOptions,
    ) -> Result<FetchOutcome> {
        tracing::Span::current().record("url", url);
        let extra = form
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        self.fetch(
            "POST",
            url,
            Some((extra, RequestBody::Form(form.to_vec()))),
            options,
            REQUEST_TIMEOUT_SECS,
        )
        .await
    }

    /// POST a JSON body; cached by body content so repeated runs are free.
    ///
    /// Elasticsearch-backed result platforms take the query in the body, so the cache key must
    /// include that body: two different queries against one endpoint are two different documents.
    #[tracing::instrument(skip(self, options, body), fields(url, method = "POST"))]
    pub async fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        options: &FetchOptions,
    ) -> Result<FetchOutcome> {
        tracing::Span::current().record("url", url);
        let encoded = serde_json::to_string(body).context("serializing JSON request body")?;
        self.fetch(
            "POST",
            url,
            Some((encoded.clone(), RequestBody::Json(encoded))),
            options,
            REQUEST_TIMEOUT_SECS,
        )
        .await
    }

    /// Core fetch logic with caching, rate limiting, retry, and timeout.
    async fn fetch(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
    ) -> Result<FetchOutcome> {
        let extra = body
            .as_ref()
            .map(|(key, _)| key.clone())
            .unwrap_or_default();
        let key = Self::key_for(method, url, &extra);
        let (body_path, meta_path) = self.cache_paths(&key);
        let cached = read_cache(&body_path, &meta_path)?;

        if let Some(meta) = cached.as_ref() {
            if !options.refresh {
                if let Ok(bytes) = std::fs::read(&body_path) {
                    if meta.status == 200 || (options.allow_not_found && meta.status == 404) {
                        {
                            let mut stats = self.stats.lock().await;
                            stats.cache_hits = stats.cache_hits.saturating_add(1);
                        }
                        return Ok(FetchOutcome {
                            url: url.to_string(),
                            method: method.to_string(),
                            status: meta.status,
                            sha256: meta.sha256.clone(),
                            bytes: meta.bytes,
                            fetched_at: meta.fetched_at.clone(),
                            from_cache: true,
                            content_type: meta.content_type.clone(),
                            body: bytes,
                        });
                    }
                }
            }
        }

        let parsed = url::Url::parse(url).with_context(|| format!("invalid url {url}"))?;
        let host = parsed.host_str().unwrap_or_default().to_string();
        let origin = format!(
            "{}://{}",
            parsed.scheme(),
            parsed
                .port()
                .map(|p| format!("{host}:{p}"))
                .unwrap_or_else(|| host.clone())
        );
        let path_and_query = match parsed.query() {
            Some(query) => format!("{}?{}", parsed.path(), query),
            None => parsed.path().to_string(),
        };

        let rules = self.robots_for(&origin).await;
        if !rules.allows(&path_and_query) {
            if self.is_authorized_host(&host) {
                // Operator-authorized host: the rule is recorded on the run, not enforced.
                {
                    let mut stats = self.stats.lock().await;
                    stats.robots_authorized = stats.robots_authorized.saturating_add(1);
                }
                debug!(host = %host, path = %path_and_query, "robots rule overridden by host authorization");
            } else {
                let mut stats = self.stats.lock().await;
                stats.robots_blocked = stats.robots_blocked.saturating_add(1);
                return Err(FetchError::Robots(url.to_string()).into());
            }
        }
        let gate = self.host_gate(&host, rules.crawl_delay).await;

        // Retry with bounded exponential backoff + jitter.
        let mut last_err: Option<FetchError> = None;
        for attempt in 1..=MAX_RETRIES {
            let _permit = gate.lock().await;
            self.wait_turn(&host).await;

            // Build the HTTP request.
            let request = build_request(
                &self.client,
                method,
                url,
                body.as_ref().map(|(_, p)| p),
                &options.headers,
                cached.as_ref(),
                options.refresh,
            )?;

            // Execute with per-request timeout.
            let response = tokio::time::timeout(Duration::from_secs(timeout_secs), request.send())
                .await
                .map_err(|_| FetchError::Timeout {
                    url: url.to_string(),
                    timeout_secs,
                })?
                .map_err(|source| FetchError::Transport {
                    url: url.to_string(),
                    source,
                })?;

            let status = response.status().as_u16();

            // Count the request once. Bodies are counted inside `process_response`; every status
            // that never reaches it (304, 5xx, 429) is counted here instead.
            if status != 200 && status != 404 {
                let mut stats = self.stats.lock().await;
                stats.requests = stats.requests.saturating_add(1);
                let per_host = stats.per_host.entry(host.clone()).or_insert(0);
                *per_host = per_host.saturating_add(1);
            }

            match status {
                200 | 404 => {
                    // Process body.
                    let outcome = process_response(
                        response,
                        url,
                        method,
                        &host,
                        &body_path,
                        &meta_path,
                        options,
                        &mut *self.stats.lock().await,
                    )
                    .await?;
                    return Ok(outcome);
                }
                304 => {
                    // Conditional GET: use cached body, update timestamps.
                    if let Some(meta) = cached.as_ref() {
                        if let Ok(bytes) = std::fs::read(&body_path) {
                            let mut refreshed = meta.clone();
                            refreshed.fetched_at = now_iso8601();
                            write_cache(&body_path, &meta_path, &bytes, &refreshed)?;
                            {
                                let mut stats = self.stats.lock().await;
                                stats.conditional_304 = stats.conditional_304.saturating_add(1);
                            }
                            return Ok(FetchOutcome {
                                url: url.to_string(),
                                method: method.to_string(),
                                status: meta.status,
                                sha256: meta.sha256.clone(),
                                bytes: meta.bytes,
                                fetched_at: refreshed.fetched_at,
                                from_cache: false,
                                content_type: meta.content_type.clone(),
                                body: bytes,
                            });
                        }
                    }
                    // Cache body disappeared — fall through to re-fetch.
                    if attempt < MAX_RETRIES {
                        last_err = Some(FetchError::Http {
                            status: 304,
                            url: url.to_string(),
                        });
                        let delay = jittered_delay(attempt);
                        debug!(
                            attempt,
                            delay_ms = delay.as_millis(),
                            "retrying after backoff"
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    let mut stats = self.stats.lock().await;
                    stats.errors = stats.errors.saturating_add(1);
                    return Err(FetchError::Http {
                        status: 304,
                        url: url.to_string(),
                    }
                    .into());
                }
                _ => {
                    // Non-200/404/304: record error, possibly retry on transport-like codes.
                    {
                        let mut stats = self.stats.lock().await;
                        stats.errors = stats.errors.saturating_add(1);
                    }
                    warn!(status, url, "non-success response");
                    let http_error = || FetchError::Http {
                        status,
                        url: url.to_string(),
                    };
                    // Retry on server errors (5xx) and client errors (429 rate-limit).
                    if status >= 500 || status == 429 {
                        if attempt < MAX_RETRIES {
                            last_err = Some(http_error());
                            let delay = jittered_delay(attempt);
                            debug!(
                                attempt,
                                status,
                                delay_ms = delay.as_millis(),
                                "retrying on server error"
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    } else if status == 404 && !options.allow_not_found {
                        // 404 is not retried — it's a terminal result.
                        return Err(http_error().into());
                    }
                    // Other errors (4xx except 429/404) are not retried.
                    last_err = Some(http_error());
                    break;
                }
            }
        }

        // All retries exhausted.
        let err = last_err.unwrap_or_else(|| FetchError::Timeout {
            url: url.to_string(),
            timeout_secs,
        });
        Err(err.into())
    }
}

/// Build an HTTP request with headers, body, and conditional GET support.
fn build_request<'a>(
    client: &'a reqwest::Client,
    method: &str,
    url: &str,
    body: Option<&'a RequestBody>,
    headers: &[(String, String)],
    cached: Option<&'a CacheMeta>,
    refresh: bool,
) -> Result<reqwest::RequestBuilder> {
    let mut request = match method {
        "POST" => client.post(url),
        _ => client.get(url),
    };
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    if method == "POST" {
        if let Some(payload) = body {
            match payload {
                RequestBody::Form(form) => {
                    let encoded = url::form_urlencoded::Serializer::new(String::new())
                        .extend_pairs(form.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                        .finish();
                    request = request
                        .header("content-type", "application/x-www-form-urlencoded")
                        .body(encoded);
                }
                RequestBody::Json(encoded) => {
                    request = request
                        .header("content-type", "application/json")
                        .body(encoded.clone());
                }
            }
        }
    }
    if let Some(meta) = cached {
        if !refresh {
            if let Some(etag) = &meta.etag {
                request = request.header("If-None-Match", etag.as_str());
            }
            if let Some(last_modified) = &meta.last_modified {
                request = request.header("If-Modified-Since", last_modified.as_str());
            }
        }
    }
    Ok(request)
}

/// Process a successful response: check size, read body, hash, cache, update stats.
///
/// The eight parameters are the request's own coordinates plus the two cache paths it writes; a
/// struct would only move the same list one level up.
#[allow(clippy::too_many_arguments)]
async fn process_response(
    response: reqwest::Response,
    url: &str,
    method: &str,
    host: &str,
    body_path: &Path,
    meta_path: &Path,
    options: &FetchOptions,
    stats: &mut FetchStats,
) -> Result<FetchOutcome> {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let etag = headers
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let last_modified = headers
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    stats.requests = stats.requests.saturating_add(1);
    let per_host_entry = stats.per_host.entry(host.to_string()).or_insert(0);
    *per_host_entry = per_host_entry.saturating_add(1);

    if status == 304 {
        // This branch should not be reached here (304 is handled above), but guard defensively.
        bail!("unexpected 304 in process_response");
    }

    let declared = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    if declared.map(|len| len > MAX_BODY_BYTES).unwrap_or(false) {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        }
        .into());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|source| FetchError::Transport {
            url: url.to_string(),
            source,
        })?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(FetchError::TooLarge {
            url: url.to_string(),
        }
        .into());
    }
    let body_vec = bytes.to_vec();
    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(&body_vec);
        sha256_prefix16(hasher)
    };
    let meta = CacheMeta {
        url: url.to_string(),
        method: method.to_string(),
        status,
        sha256: sha256.clone(),
        bytes: body_vec.len(),
        fetched_at: now_iso8601(),
        etag,
        last_modified,
        content_type: content_type.clone(),
    };
    write_cache(body_path, meta_path, &body_vec, &meta)?;
    // `usize` -> `u64` for the byte counter, saturating where the value cannot fit.
    let downloaded = u64::try_from(body_vec.len()).unwrap_or(u64::MAX);
    stats.bytes_downloaded = stats.bytes_downloaded.saturating_add(downloaded);
    if status >= 400 && !(status == 404 && options.allow_not_found) {
        stats.errors = stats.errors.saturating_add(1);
        warn!(status, url, "non-success response");
    }
    Ok(FetchOutcome {
        url: url.to_string(),
        method: method.to_string(),
        status,
        sha256,
        bytes: body_vec.len(),
        fetched_at: meta.fetched_at,
        from_cache: false,
        content_type,
        body: body_vec,
    })
}

/// Backoff for a retry: 500 ms doubling per attempt, capped at ten seconds, plus deterministic
/// ±25% jitter so parallel fetchers do not retry in lockstep.
///
/// The jitter is a splitmix64 mix of the attempt number: no random source, no allocation, and the
/// same attempt always yields the same delay, which keeps tests and replays repeatable.
fn jittered_delay(attempt: u32) -> Duration {
    let step = attempt.saturating_sub(1).min(4);
    let cap_ms: u64 = 10_000;
    // Doubling per attempt, capped: `saturating_pow` cannot overflow and `step` is at most 4.
    let base_ms = RETRY_BASE_DELAY_MS
        .saturating_mul(2_u64.saturating_pow(step))
        .min(cap_ms);
    let spread = base_ms / 2;
    if spread == 0 {
        return Duration::from_millis(base_ms);
    }
    // The jitter is a value in `0..spread` pulled back by half a spread, so the delay lands
    // within ±25% of `base` and never leaves `0..=cap`. Subtracting the shortfall and adding the
    // excess of that shift keeps the whole calculation unsigned and saturating: nothing can
    // overflow, and the shortfall saturates at zero exactly where the old clamp did.
    let half = spread / 2;
    // `spread` is non-zero here, so the remainder is always defined; the fallback keeps the
    // calculation total without a panic path.
    let jitter = mix_attempt(attempt).checked_rem(spread).unwrap_or(0);
    let delay_ms = base_ms
        .saturating_sub(half.saturating_sub(jitter))
        .saturating_add(jitter.saturating_sub(half))
        .min(cap_ms);
    Duration::from_millis(delay_ms)
}

/// splitmix64 over one counter: cheap, deterministic, and dependency-free.
fn mix_attempt(attempt: u32) -> u64 {
    let mut z = u64::from(attempt).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

/// Hex of the leading 16 bytes of a SHA-256 hash.
///
/// The crate keys cached bodies, cache metadata and content ids by that prefix. SHA-256 always
/// digests to 32 bytes, so the prefix is present; `get` keeps the extraction total without a
/// panic path.
fn sha256_prefix16(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let head = match digest.get(..16) {
        Some(head) => head,
        None => digest.as_slice(),
    };
    head.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_cache(body_path: &Path, meta_path: &Path) -> Result<Option<CacheMeta>> {
    if !meta_path.exists() || !body_path.exists() {
        return Ok(None);
    }
    let meta = std::fs::read_to_string(meta_path)
        .with_context(|| format!("reading cache metadata {}", meta_path.display()))?;
    let meta: CacheMeta = serde_json::from_str(&meta)
        .with_context(|| format!("decoding cache metadata {}", meta_path.display()))?;
    Ok(Some(meta))
}

fn write_cache(body_path: &Path, meta_path: &Path, body: &[u8], meta: &CacheMeta) -> Result<()> {
    let tmp_body = body_path.with_extension("body.tmp");
    std::fs::write(&tmp_body, body).map_err(|source| FetchError::Cache {
        path: tmp_body.clone(),
        source,
    })?;
    std::fs::rename(&tmp_body, body_path).map_err(|source| FetchError::Cache {
        path: body_path.to_path_buf(),
        source,
    })?;
    let tmp_meta = meta_path.with_extension("meta.json.tmp");
    std::fs::write(&tmp_meta, serde_json::to_vec_pretty(meta)?).map_err(|source| {
        FetchError::Cache {
            path: tmp_meta.clone(),
            source,
        }
    })?;
    std::fs::rename(&tmp_meta, meta_path).map_err(|source| FetchError::Cache {
        path: meta_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// robots.txt parsing
// ---------------------------------------------------------------------------

/// Minimal, correct robots.txt parsing for the `*` and named user-agent groups.
fn parse_robots(body: &str) -> RobotsRules {
    let mut rules = Vec::new();
    let mut crawl_delay = None;
    // We only honour the `*` group: our own product token never appears in published rule sets, and
    // RFC 9309 says a crawler with no matching group follows the wildcard group.
    let mut applies = false;
    let mut saw_any_group = false;

    for line in body.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        let field = field.trim().to_ascii_lowercase();
        let value = value.trim();
        match field.as_str() {
            "user-agent" => {
                saw_any_group = true;
                applies = value == "*";
            }
            "disallow" | "allow" if applies && !value.is_empty() => {
                rules.push((field == "allow", value.to_string()));
            }
            "crawl-delay" if applies => {
                if let Ok(seconds) = value.parse::<f64>() {
                    crawl_delay = Some(Duration::from_secs_f64(seconds.max(0.0)));
                }
            }
            _ => {}
        }
    }
    if !saw_any_group {
        rules.clear();
    }
    RobotsRules {
        rules,
        crawl_delay,
        fetched: true,
    }
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a fetcher whose cache lives in a throwaway directory. Authorization is the only
    /// property under test here, so no request is ever issued.
    fn fetcher_with(authorized: Vec<String>) -> (Fetcher, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let fetcher = Fetcher::new(
            dir.path().join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            authorized,
        )
        .expect("fetcher");
        (fetcher, dir)
    }

    #[test]
    fn no_host_is_authorized_by_default() {
        let (fetcher, _dir) = fetcher_with(Vec::new());
        assert!(!fetcher.is_authorized_host("www.athletic.net"));
        assert!(!fetcher.is_authorized_host("milesplit.com"));
    }

    #[test]
    fn a_bare_domain_authorizes_its_subdomains_but_not_lookalikes() {
        let (fetcher, _dir) = fetcher_with(vec!["athletic.net".to_string()]);
        assert!(fetcher.is_authorized_host("athletic.net"));
        assert!(fetcher.is_authorized_host("www.athletic.net"));
        assert!(fetcher.is_authorized_host("WWW.Athletic.NET"));
        // Suffix matching must not leak to a domain that merely ends with the same text.
        assert!(!fetcher.is_authorized_host("notathletic.net"));
        assert!(!fetcher.is_authorized_host("athletic.net.evil.com"));
    }

    #[test]
    fn an_exact_host_never_widens_into_its_parent_domain() {
        let (fetcher, _dir) = fetcher_with(vec!["www.example.com".to_string()]);
        assert!(fetcher.is_authorized_host("www.example.com"));
        assert!(fetcher.is_authorized_host("cdn.www.example.com"));
        assert!(!fetcher.is_authorized_host("example.com"));
        assert!(!fetcher.is_authorized_host("other.example.com"));
    }

    #[test]
    fn authorization_never_relaxes_the_two_rps_ceiling() {
        let (fetcher, _dir) = fetcher_with(vec!["athletic.net".to_string()]);
        // The policy ceiling is a floor on spacing: even a 1 ms configured delay is raised.
        assert!(MIN_AUTHORIZED_DELAY >= Duration::from_millis(500));
        let (unauthorized, _dir2) = fetcher_with(Vec::new());
        assert!(!unauthorized.is_authorized_host("athletic.net"));
        assert!(fetcher.is_authorized_host("athletic.net"));
    }

    #[test]
    fn robots_rules_honour_longest_match_and_allow_ties() {
        let rules = parse_robots(
            "User-agent: *\nDisallow: /rankings\nDisallow: /api/\nAllow: /api/public/\nCrawl-delay: 2\n",
        );
        assert!(!rules.allows("/rankings/leaders"));
        assert!(!rules.allows("/api/v1/meets"));
        assert!(rules.allows("/api/public/x"));
        assert!(rules.allows("/teams/1234/roster"));
        assert_eq!(rules.crawl_delay, Some(Duration::from_secs(2)));
    }

    #[test]
    fn robots_absent_or_empty_allows_everything() {
        let empty = parse_robots("");
        assert!(empty.allows("/anything"));
        let no_group = parse_robots("Disallow: /x\n");
        assert!(no_group.allows("/x"), "rules outside a group do not apply");
    }

    #[test]
    fn robots_named_agent_groups_are_ignored() {
        let rules =
            parse_robots("User-agent: GPTBot\nDisallow: /\n\nUser-agent: *\nDisallow: /private\n");
        assert!(rules.allows("/teams"));
        assert!(!rules.allows("/private/x"));
    }

    #[test]
    fn jittered_delay_increases_with_attempt() {
        let d1 = jittered_delay(1);
        let d2 = jittered_delay(2);
        let d3 = jittered_delay(3);
        // Jitter adds noise, so strict ordering isn't guaranteed.
        // But the expected value increases.
        assert!(d1 <= d2);
        assert!(d2 <= d3);
        assert!(d3 <= Duration::from_secs(10));
    }

    #[test]
    fn jittered_delay_respects_cap() {
        for attempt in 1..=10 {
            let d = jittered_delay(attempt);
            assert!(
                d <= Duration::from_secs(10),
                "delay for attempt {attempt} exceeds 10s cap"
            );
        }
    }

    #[test]
    fn cache_key_pins_the_on_disk_cache_layout() {
        // Cached bodies live at `{key}.body` / `{key}.meta.json`, so the key derivation is part
        // of the on-disk layout: `GET`-with-no-body and `POST`-with-a-body must keep the same
        // keys across refactors, or a re-run stops being free.
        assert_eq!(
            Fetcher::key_for("GET", "https://example.com/teams", ""),
            "2ee9e0985d9a4ffc8864d9dfaae08524"
        );
        assert_eq!(
            Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
            "fd3996c5f6d99f4badb15fb729c483c8"
        );
        // Two queries against one endpoint are two documents.
        assert_ne!(
            Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
            Fetcher::key_for("POST", "https://example.com/api", "q=1&page=3")
        );
    }
}
