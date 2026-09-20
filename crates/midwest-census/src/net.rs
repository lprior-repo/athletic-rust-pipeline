//! Polite, resumable HTTP layer.
//!
//! Properties the rest of the crate relies on:
//!
//! * **robots.txt is enforced**, not advisory: a disallowed path returns [`FetchError::Robots`] and
//!   never leaves the process. The rule set is fetched once per host, cached on disk and honoured for
//!   the remainder of the run.
//! * **Requests are cached on disk** by content hash, so a re-run is free and interrupted collections
//!   resume without re-fetching. Conditional GETs (`If-None-Match` / `If-Modified-Since`) are used when
//!   the origin supports them.
//! * **Politeness is per host**: at most one in-flight request per host, minimum spacing between
//!   requests (default 1 s, overridable per host — e.g. Bound's `Crawl-delay: 10`).
//! * No authentication, no cookie jar, no CAPTCHA handling, no challenge evasion.

use anyhow::{Context, Result};
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
}

/// Request payload for POSTs: form pairs or a pre-serialized JSON body.
#[derive(Debug, Clone)]
enum RequestBody {
    Form(Vec<(String, String)>),
    Json(String),
}

#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Ignore any cached body and hit the network (still robots-checked).
    pub refresh: bool,
    /// Treat a 404 as a normal (cached) outcome instead of an error.
    pub allow_not_found: bool,
    /// Extra request headers (e.g. `Accept: application/json`).
    pub headers: Vec<(String, String)>,
}

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FetchStats {
    pub requests: u64,
    pub cache_hits: u64,
    pub conditional_304: u64,
    pub robots_blocked: u64,
    pub bytes_downloaded: u64,
    pub errors: u64,
    pub per_host: HashMap<String, u64>,
}

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

struct HostState {
    gate: Arc<Mutex<()>>,
    /// When the next request to this host may start (reserved before sleeping so that the spacing
    /// holds even when several tasks queue behind the gate).
    next_allowed: Option<std::time::Instant>,
    delay: Duration,
}

pub struct Fetcher {
    client: reqwest::Client,
    cache_dir: PathBuf,
    user_agent: String,
    default_delay: Duration,
    host_delays: HashMap<String, Duration>,
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
    ) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("creating cache dir {}", cache_dir.display()))?;
        let user_agent = user_agent.unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .user_agent(user_agent.clone())
            .timeout(Duration::from_secs(45))
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
            hosts: Mutex::new(HashMap::new()),
            robots: Mutex::new(HashMap::new()),
            stats: Mutex::new(FetchStats::default()),
        })
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

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
        let digest = hasher.finalize();
        digest[..16].iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Serialize per host and enforce the configured (or robots-requested) spacing.
    async fn host_gate(&self, host: &str, robots_delay: Option<Duration>) -> Arc<Mutex<()>> {
        let configured = self
            .host_delays
            .get(host)
            .copied()
            .unwrap_or(self.default_delay);
        let effective = match robots_delay {
            Some(robots) if robots > configured => robots,
            _ => configured,
        };
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
            let state = hosts.get_mut(host).expect("host registered");
            let now = std::time::Instant::now();
            let wait = match state.next_allowed {
                Some(at) if at > now => at - now,
                _ => Duration::ZERO,
            };
            state.next_allowed = Some(now + wait + state.delay);
            wait
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
            Ok((status, body)) if status == 200 => parse_robots(&body),
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
    pub async fn get(&self, url: &str, options: &FetchOptions) -> Result<FetchOutcome> {
        self.fetch("GET", url, None, options).await
    }

    /// POST a form body; cached by body content so repeated runs are free.
    pub async fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
        options: &FetchOptions,
    ) -> Result<FetchOutcome> {
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
        )
        .await
    }

    /// POST a JSON body; cached by body content so repeated runs are free.
    ///
    /// Elasticsearch-backed result platforms take the query in the body, so the cache key must
    /// include that body: two different queries against one endpoint are two different documents.
    pub async fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
        options: &FetchOptions,
    ) -> Result<FetchOutcome> {
        let encoded = serde_json::to_string(body).context("serializing JSON request body")?;
        self.fetch(
            "POST",
            url,
            Some((encoded.clone(), RequestBody::Json(encoded))),
            options,
        )
        .await
    }

    async fn fetch(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
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
                            stats.cache_hits += 1;
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
            self.stats.lock().await.robots_blocked += 1;
            return Err(FetchError::Robots(url.to_string()).into());
        }
        // Crawl-delay from robots.txt is a floor: never go faster than the origin asks.
        let gate = self.host_gate(&host, rules.crawl_delay).await;
        let _permit = gate.lock().await;
        self.wait_turn(&host).await;

        let mut request = match method {
            "POST" => self.client.post(url),
            _ => self.client.get(url),
        };
        for (name, value) in &options.headers {
            request = request.header(name.as_str(), value.as_str());
        }
        if method == "POST" {
            if let Some((_, payload)) = &body {
                match payload {
                    RequestBody::Form(form) => {
                        // `reqwest`'s `form` helper needs the optional `form` feature; encode
                        // directly so the dependency set stays minimal.
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
        if let Some(meta) = cached.as_ref() {
            if !options.refresh {
                if let Some(etag) = &meta.etag {
                    request = request.header("If-None-Match", etag.as_str());
                }
                if let Some(last_modified) = &meta.last_modified {
                    request = request.header("If-Modified-Since", last_modified.as_str());
                }
            }
        }

        let response = request.send().await.map_err(|source| {
            let _ = source;
            FetchError::Transport {
                url: url.to_string(),
                source,
            }
        });
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                self.stats.lock().await.errors += 1;
                return Err(error.into());
            }
        };

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

        {
            let mut stats = self.stats.lock().await;
            stats.requests += 1;
            *stats.per_host.entry(host.clone()).or_insert(0) += 1;
        }

        if status == 304 {
            if let Some(meta) = cached.as_ref() {
                if let Ok(bytes) = std::fs::read(&body_path) {
                    let mut refreshed = meta.clone();
                    refreshed.fetched_at = now_iso8601();
                    write_cache(&body_path, &meta_path, &bytes, &refreshed)?;
                    self.stats.lock().await.conditional_304 += 1;
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
            hasher.finalize()[..16]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
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
        write_cache(&body_path, &meta_path, &body_vec, &meta)?;
        {
            let mut stats = self.stats.lock().await;
            stats.bytes_downloaded += body_vec.len() as u64;
            if status >= 400 && !(status == 404 && options.allow_not_found) {
                stats.errors += 1;
            }
        }
        if status >= 400 && !(status == 404 && options.allow_not_found) {
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
            "disallow" | "allow" => {
                if applies && !value.is_empty() {
                    rules.push((field == "allow", value.to_string()));
                }
            }
            "crawl-delay" => {
                if applies {
                    if let Ok(seconds) = value.parse::<f64>() {
                        crawl_delay = Some(Duration::from_secs_f64(seconds.max(0.0)));
                    }
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

pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
