use crate::{
    config::DiscoveryConfig,
    model::{Prospect, SearchHit},
};
use anyhow::Result;
use futures::TryStreamExt;
use regex::Regex;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    cmp::Reverse,
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        LazyLock,
    },
    time::Duration,
};
use tokio::{
    sync::Mutex,
    time::{sleep, Instant},
};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};
use url::Url;

const MAX_RESPONSE_BODY_BYTES: u64 = 16 * 1024 * 1024;

pub struct AthleticNetClient {
    client: Client,
    endpoint: String,
    max_candidates: usize,
    search_delay: Duration,
    max_attempts: u32,
    circuit_breaker_threshold: u32,
    total_denials: AtomicU32,
    circuit_open: AtomicBool,
    next_request_at: Mutex<Instant>,
}

#[derive(Debug, Deserialize)]
struct SearchEnvelope {
    d: SearchPayload,
}

#[derive(Debug, Deserialize, Default)]
struct SearchPayload {
    results: String,
    #[serde(default)]
    count: Option<i64>,
    #[serde(default)]
    pager: String,
}

#[derive(Debug)]
pub struct SearchExecution {
    pub hits: Vec<SearchHit>,
    pub attempts: u32,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct SearchFailure {
    pub message: String,
    pub retryable: bool,
    pub attempts: u32,
    pub status: Option<u16>,
    retry_after: Duration,
}

impl SearchFailure {
    fn new(message: String, retryable: bool, status: Option<u16>) -> Self {
        Self {
            message,
            retryable,
            attempts: 0,
            status,
            retry_after: Duration::ZERO,
        }
    }

    fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = retry_after;
        self
    }

    fn with_attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts;
        self
    }
}

const SPORT_FILTERS: [&str; 2] = ["a:tf", "a:xc"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SearchRequest {
    pub query: String,
    pub filter: String,
    pub stage: u8,
}

impl SearchRequest {
    pub fn new(query: &str, filter: &str, stage: u8) -> Self {
        Self {
            query: collapse_spaces(query),
            filter: filter.to_owned(),
            stage,
        }
    }

    pub fn cache_key(&self) -> String {
        format!(
            "{}\n{}",
            self.filter,
            collapse_spaces(&self.query).to_lowercase()
        )
    }
}

#[derive(Debug, Clone)]
pub struct QueryPlan {
    stages: Vec<Vec<SearchRequest>>,
}

impl QueryPlan {
    pub fn for_prospect(prospect: &Prospect) -> Self {
        let name = prospect.full_name();
        let mut seen = HashSet::new();
        let first_stage = paired_requests(0, std::iter::once(name.clone()), &mut seen);
        let context_queries = [
            (!prospect.school.trim().is_empty())
                .then(|| format!("{name} {}", prospect.school.trim())),
            (!(prospect.city.trim().is_empty() && prospect.state.trim().is_empty()))
                .then(|| format!("{name} {} {}", prospect.city.trim(), prospect.state.trim())),
            prospect
                .expected_graduation_year
                .map(|year| format!("{name} {year}")),
        ]
        .into_iter()
        .flatten();
        let second_stage = paired_requests(1, context_queries, &mut seen);
        let normalized_name = normalize_name_for_search(&name);
        let reversed = format!(
            "{} {}",
            prospect.last_name.trim(),
            prospect.first_name.trim()
        );
        let initial = prospect
            .first_name
            .trim()
            .chars()
            .next()
            .map(|value| format!("{value} {}", prospect.last_name.trim()));
        let third_stage = paired_requests(
            2,
            [Some(normalized_name), Some(reversed), initial]
                .into_iter()
                .flatten(),
            &mut seen,
        );
        Self {
            stages: vec![first_stage, second_stage, third_stage],
        }
    }

    pub fn stage(&self, index: usize) -> Option<&[SearchRequest]> {
        self.stages.get(index).map(Vec::as_slice)
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = &SearchRequest> {
        self.stages.iter().flatten()
    }
}

fn paired_requests(
    stage: u8,
    queries: impl IntoIterator<Item = String>,
    seen: &mut HashSet<String>,
) -> Vec<SearchRequest> {
    queries
        .into_iter()
        .flat_map(|query| {
            SPORT_FILTERS
                .into_iter()
                .map(move |filter| SearchRequest::new(&query, filter, stage))
        })
        .filter(|request| !request.query.is_empty())
        .filter(|request| seen.insert(request.cache_key()))
        .collect()
}

fn collapse_spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl AthleticNetClient {
    pub fn new(config: &DiscoveryConfig) -> Result<Self> {
        if config.max_candidates == 0 {
            anyhow::bail!("discovery max_candidates must be greater than zero");
        }
        if config.request_timeout_seconds == 0 {
            anyhow::bail!("discovery request_timeout_seconds must be greater than zero");
        }
        if config.max_attempts == 0 {
            anyhow::bail!("discovery max_attempts must be greater than zero");
        }
        if config.circuit_breaker_threshold == 0 {
            anyhow::bail!("discovery circuit_breaker_threshold must be greater than zero");
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .user_agent("athletic-rust-pipeline/0.1 (+authorized research)")
            .build()?;
        Ok(Self {
            client,
            endpoint: config.athletic_search_url.clone(),
            max_candidates: config.max_candidates,
            search_delay: Duration::from_millis(config.search_delay_ms),
            max_attempts: config.max_attempts,
            circuit_breaker_threshold: config.circuit_breaker_threshold,
            total_denials: AtomicU32::new(0),
            circuit_open: AtomicBool::new(false),
            next_request_at: Mutex::new(Instant::now()),
        })
    }

    pub async fn search(&self, prospect: &Prospect) -> Result<Vec<SearchHit>> {
        let filter = sport_filter(&prospect.sport);
        let mut hits = Vec::new();
        for query in build_queries(prospect) {
            let request = SearchRequest::new(&query, filter, 0);
            let outcome = self.execute(&request).await.map_err(anyhow::Error::new)?;
            hits.extend(outcome.hits);
        }
        hits.sort_by_key(|hit| Reverse(hit_relevance(prospect, hit)));
        hits.truncate(self.max_candidates);
        Ok(hits)
    }

    pub async fn execute(
        &self,
        request: &SearchRequest,
    ) -> std::result::Result<SearchExecution, SearchFailure> {
        let (payload, attempts) = self.execute_page(request, 0).await?;
        let mut hits = Vec::new();
        append_results(
            &mut hits,
            &payload.results,
            &request.query,
            &request.filter,
            self.max_candidates,
        )
        .map_err(|failure| failure.with_attempts(attempts))?;
        Ok(SearchExecution { hits, attempts })
    }

    async fn execute_page(
        &self,
        request: &SearchRequest,
        start: usize,
    ) -> std::result::Result<(SearchPayload, u32), SearchFailure> {
        if self.circuit_open.load(Ordering::Acquire) {
            return Err(SearchFailure::new(
                "Athletic.net request circuit is open".to_owned(),
                true,
                None,
            ));
        }
        for attempt in 1..=self.max_attempts {
            self.wait_for_request_slot().await;
            if self.circuit_open.load(Ordering::Acquire) {
                return Err(SearchFailure::new(
                    "Athletic.net request circuit is open".to_owned(),
                    true,
                    None,
                )
                .with_attempts(attempt.saturating_sub(1)));
            }
            match self.send_once(request, start).await {
                Ok(envelope) => return Ok((envelope.d, attempt)),
                Err(failure) if self.circuit_open.load(Ordering::Acquire) => {
                    return Err(failure.with_attempts(attempt));
                }
                Err(failure) if failure.retryable && attempt < self.max_attempts => {
                    let delay = self.retry_delay(attempt).max(failure.retry_after);
                    self.defer_requests(delay).await;
                }
                Err(failure) => return Err(failure.with_attempts(attempt)),
            }
        }
        Err(SearchFailure::new(
            "Athletic.net search exhausted without an outcome".to_owned(),
            true,
            None,
        )
        .with_attempts(self.max_attempts))
    }

    async fn wait_for_request_slot(&self) {
        // This is the sole scheduler lock; no network I/O or other lock is acquired
        // while held. Sleeping under it prevents bunched starts after a stalled task.
        let mut next = self.next_request_at.lock().await;
        sleep(next.saturating_duration_since(Instant::now())).await;
        let now = Instant::now();
        *next = now
            .checked_add(self.search_delay)
            .map_or(now, |value| value);
    }

    async fn defer_requests(&self, delay: Duration) {
        let mut next = self.next_request_at.lock().await;
        let deadline = Instant::now().checked_add(delay);
        if let Some(deadline) = deadline {
            *next = (*next).max(deadline);
        }
    }

    async fn send_once(
        &self,
        request: &SearchRequest,
        start: usize,
    ) -> std::result::Result<SearchEnvelope, SearchFailure> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(&json!({
                "q": request.query,
                "fq": format!("t:a {}", request.filter),
                "start": start,
            }))
            .send()
            .await
            .map_err(|error| {
                SearchFailure::new(
                    format!("querying Athletic.net at {}: {error}", self.endpoint),
                    error.is_timeout() || error.is_connect(),
                    error.status().map(|status| status.as_u16()),
                )
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(self.failure_for_status(status, response.headers()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BODY_BYTES)
        {
            return Err(SearchFailure::new(
                format!(
                    "Athletic.net search response exceeds {} byte limit",
                    MAX_RESPONSE_BODY_BYTES
                ),
                false,
                Some(status.as_u16()),
            ));
        }
        let body = response
            .bytes_stream()
            .map_err(|error| {
                SearchFailure::new(
                    format!("reading Athletic.net search response body: {error}"),
                    true,
                    Some(status.as_u16()),
                )
            })
            .try_fold(Vec::new(), |mut body, chunk| async move {
                let next_length = body.len().checked_add(chunk.len()).ok_or_else(|| {
                    SearchFailure::new(
                        "Athletic.net search response size overflow".to_owned(),
                        false,
                        Some(status.as_u16()),
                    )
                })?;
                if u64::try_from(next_length)
                    .map_or(true, |length| length > MAX_RESPONSE_BODY_BYTES)
                {
                    return Err(SearchFailure::new(
                        format!(
                            "Athletic.net search response exceeds {} byte limit",
                            MAX_RESPONSE_BODY_BYTES
                        ),
                        false,
                        Some(status.as_u16()),
                    ));
                }
                body.try_reserve(chunk.len()).map_err(|error| {
                    SearchFailure::new(
                        format!("allocating Athletic.net search response body: {error}"),
                        false,
                        Some(status.as_u16()),
                    )
                })?;
                body.extend_from_slice(&chunk);
                Ok::<Vec<u8>, SearchFailure>(body)
            })
            .await?;
        serde_json::from_slice::<SearchEnvelope>(&body).map_err(|error| {
            SearchFailure::new(
                format!("decoding Athletic.net search response: {error}"),
                false,
                Some(status.as_u16()),
            )
        })
    }

    fn failure_for_status(
        &self,
        status: StatusCode,
        headers: &reqwest::header::HeaderMap,
    ) -> SearchFailure {
        let denied = status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS;
        if denied {
            let count = self
                .total_denials
                .fetch_add(1, Ordering::AcqRel)
                .saturating_add(1);
            if status == StatusCode::FORBIDDEN || count >= self.circuit_breaker_threshold {
                self.circuit_open.store(true, Ordering::Release);
            }
        }
        let retryable = crate::retry_policy::transient_status(status);
        let retry_after =
            match crate::retry_policy::retry_after(headers, std::time::SystemTime::now()) {
                Ok(delay) => delay,
                Err(error) => {
                    return SearchFailure::new(
                        format!("Athletic.net returned HTTP {}: {error:#}", status.as_u16()),
                        false,
                        Some(status.as_u16()),
                    )
                }
            };
        SearchFailure::new(
            format!("Athletic.net returned HTTP {}", status.as_u16()),
            retryable,
            Some(status.as_u16()),
        )
        .with_retry_after(retry_after)
    }

    fn retry_delay(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(16);
        let factor = 1_u32.checked_shl(shift).map_or(u32::MAX, |value| value);
        self.search_delay
            .saturating_mul(factor)
            .min(Duration::from_secs(60))
    }
}

fn build_queries(prospect: &Prospect) -> Vec<String> {
    let name = prospect.full_name();
    let first = prospect.first_name.trim();
    let last = prospect.last_name.trim();
    let normalized_name = normalize_name_for_search(&name);
    let context = if !prospect.school.trim().is_empty() {
        prospect.school.trim().to_owned()
    } else {
        format!("{} {}", prospect.city.trim(), prospect.state.trim())
    };
    let mut queries = Vec::with_capacity(5);
    push_query(&mut queries, name.clone());
    if !context.trim().is_empty() {
        push_query(&mut queries, format!("{name} {context}"));
    }
    if normalized_name != name {
        push_query(&mut queries, format!("{normalized_name} {context}"));
    }
    if !first.is_empty() && !last.is_empty() {
        push_query(&mut queries, format!("{last} {first} {context}"));
        if let Some(initial) = first.chars().next() {
            push_query(&mut queries, format!("{initial} {last} {context}"));
        }
    }
    queries
}

fn push_query(queries: &mut Vec<String>, query: String) {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if !query.is_empty() && !queries.iter().any(|existing| existing == &query) {
        queries.push(query);
    }
}

fn normalize_name_for_search(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn hit_relevance(prospect: &Prospect, hit: &SearchHit) -> usize {
    let evidence = normalize_name_for_search(&format!("{} {}", hit.title, hit.snippet));
    let name_tokens = normalize_name_for_search(&prospect.full_name());
    let mut score = name_tokens
        .split_whitespace()
        .filter(|token| evidence.split_whitespace().any(|value| value == *token))
        .count()
        .saturating_mul(10);
    let school = normalize_name_for_search(&prospect.school);
    if !school.is_empty() && evidence.contains(&school) {
        score = score.saturating_add(20);
    }
    let location = normalize_name_for_search(&format!("{} {}", prospect.city, prospect.state));
    if !location.trim().is_empty() && evidence.contains(&location) {
        score = score.saturating_add(10);
    }
    score
}

fn sport_filter(sport: &str) -> &'static str {
    let normalized = sport.to_ascii_lowercase();
    if normalized.contains("cross country") && !normalized.contains("track") {
        "a:xc"
    } else {
        "a:tf"
    }
}

fn append_results(
    hits: &mut Vec<SearchHit>,
    html: &str,
    query: &str,
    filter: &str,
    max_candidates: usize,
) -> std::result::Result<(), SearchFailure> {
    let Some(pattern) = ATHLETE_ANCHOR.as_ref() else {
        return Err(SearchFailure::new(
            "athlete result parser is unavailable".to_owned(),
            false,
            None,
        ));
    };
    let Some(row_pattern) = RESULT_ROW.as_ref() else {
        return Err(SearchFailure::new(
            "athlete result row parser is unavailable".to_owned(),
            false,
            None,
        ));
    };
    let query_start = hits.len();
    let mut seen: HashSet<String> = hits.iter().map(|hit| hit.url.clone()).collect();
    let rows = row_pattern
        .find_iter(html)
        .map(|row| row.as_str())
        .collect::<Vec<_>>();
    let rows = if rows.is_empty() { vec![html] } else { rows };
    for row in rows {
        let lower_row = row.to_ascii_lowercase();
        let has_athlete_reference = lower_row.contains("/athlete/");
        let has_requested_reference = lower_row.contains(if filter == "a:xc" {
            "cross-country"
        } else {
            "track-and-field"
        });
        if has_athlete_reference && has_requested_reference && !pattern.is_match(row) {
            return Err(SearchFailure::new(
                "search result row contains a malformed athlete link".to_owned(),
                true,
                None,
            ));
        }
        for capture in pattern.captures_iter(row) {
            if hits.len().saturating_sub(query_start) >= max_candidates {
                return Ok(());
            }
            let Some(raw_url) = capture.get(1).map(|value| value.as_str()) else {
                return Err(SearchFailure::new(
                    "search result row contains an incomplete athlete link".to_owned(),
                    true,
                    None,
                ));
            };
            let required_path = if filter == "a:xc" {
                "cross-country"
            } else {
                "track-and-field"
            };
            if !raw_url.to_ascii_lowercase().contains(required_path) {
                continue;
            }
            let absolute = if raw_url.starts_with('/') {
                format!("https://www.athletic.net{raw_url}")
            } else {
                raw_url.to_owned()
            };
            let Some(url) = allowed_profile_url(&absolute) else {
                return Err(SearchFailure::new(
                    "search result row contains an athlete link with an invalid ID".to_owned(),
                    true,
                    None,
                ));
            };
            let Some(name_html) = capture.get(2) else {
                return Err(SearchFailure::new(
                    "search result row contains an incomplete athlete name".to_owned(),
                    true,
                    None,
                ));
            };
            let name = compact_html(name_html.as_str());
            if name.is_empty() {
                return Err(SearchFailure::new(
                    "search result row contains an athlete link with an empty name".to_owned(),
                    true,
                    None,
                ));
            }
            if !seen.insert(url.clone()) {
                return Err(SearchFailure::new(
                    format!("search returned duplicate athlete result {url}"),
                    true,
                    None,
                ));
            }
            let snippet = compact_html(row);
            hits.push(SearchHit {
                url,
                title: name,
                snippet,
                query: query.to_owned(),
                filter: filter.to_owned(),
            });
        }
    }
    Ok(())
}

pub fn allowed_profile_url(value: &str) -> Option<String> {
    let mut url = Url::parse(value).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    if host != "athletic.net" && host != "www.athletic.net" {
        return None;
    }
    if !url.path().to_ascii_lowercase().starts_with("/athlete/") {
        return None;
    }
    if url.scheme() != "https" && url.scheme() != "http" {
        return None;
    }
    let mut path_parts = url.path().split('/');
    if path_parts.next() != Some("")
        || path_parts.next() != Some("athlete")
        || path_parts.next().is_none_or(|id| {
            id.is_empty() || !id.chars().all(|character| character.is_ascii_digit())
        })
    {
        return None;
    }
    url.set_fragment(None);
    url.set_query(None);
    Some(url.to_string())
}

static RESULT_ROW: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b[^>]*>.*?</tr\s*>").ok());
static ATHLETE_ANCHOR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<a\b[^>]*href\s*=\s*["']((?:https?://(?:www\.)?athletic\.net)?/athlete/[^"'<>\s]*)["'][^>]*>(.*?)</a\s*>"#,
    )
    .ok()
});
static HTML_TAGS: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").ok());
static HTML_SPACE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\s+").ok());

fn compact_html(html: &str) -> String {
    let Some(tags) = HTML_TAGS.as_ref() else {
        return html.to_owned();
    };
    let Some(space) = HTML_SPACE.as_ref() else {
        return html.to_owned();
    };
    let without_tags = tags.replace_all(html, " ");
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    space.replace_all(&decoded, " ").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn filters_candidate_urls() {
        assert!(
            allowed_profile_url("https://www.athletic.net/athlete/123/track-and-field").is_some()
        );
        assert!(allowed_profile_url("https://evil.example/athlete/123").is_none());
        assert!(allowed_profile_url("https://athletic.net/team/123").is_none());
    }

    #[test]
    fn extracts_only_requested_athlete_results() -> Result<()> {
        let html = r#"<tr><td><a href="/athlete/12345/track-and-field?x=1">Jane Doe</a></td></tr>
            <tr><td><a href="/athlete/99999/cross-country">Other sport</a>
            <a href="https://evil.example/athlete/99">Evil</a></td></tr>"#;
        let mut hits = Vec::new();
        append_results(&mut hits, html, "Jane Doe", "a:tf", 3)?;
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits.first().map(|hit| hit.url.as_str()),
            Some("https://www.athletic.net/athlete/12345/track-and-field")
        );
        Ok(())
    }

    #[test]
    fn selects_cross_country_filter_only_for_xc() {
        assert_eq!(sport_filter("Cross Country: Womens"), "a:xc");
        assert_eq!(sport_filter("Track and Field: Womens"), "a:tf");
    }

    #[test]
    fn builds_reordered_initial_and_diacritic_name_queries() {
        let prospect = Prospect {
            first_name: "José".to_owned(),
            last_name: "Smith-Jones".to_owned(),
            school: "Example High School".to_owned(),
            ..Default::default()
        };
        let queries = build_queries(&prospect);
        assert!(queries.iter().any(|query| query == "José Smith-Jones"));
        assert!(queries
            .iter()
            .any(|query| query == "José Smith-Jones Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "Jose Smith Jones Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "Smith-Jones José Example High School"));
        assert!(queries
            .iter()
            .any(|query| query == "J Smith-Jones Example High School"));
    }

    #[test]
    fn first_stage_queries_full_name_for_both_sports() -> Result<()> {
        let prospect = Prospect {
            first_name: "Ada".to_owned(),
            last_name: "Runner".to_owned(),
            ..Default::default()
        };
        let plan = QueryPlan::for_prospect(&prospect);
        let stage = plan.stage(0).context("missing stage zero")?;
        assert_eq!(
            stage,
            [
                SearchRequest::new("Ada Runner", "a:tf", 0),
                SearchRequest::new("Ada Runner", "a:xc", 0),
            ]
        );
        Ok(())
    }

    #[test]
    fn contextual_stage_keeps_school_and_location_queries() -> Result<()> {
        let prospect = Prospect {
            first_name: "Ada".to_owned(),
            last_name: "Runner".to_owned(),
            school: "Central".to_owned(),
            city: "Austin".to_owned(),
            state: "TX".to_owned(),
            ..Default::default()
        };
        let plan = QueryPlan::for_prospect(&prospect);
        let stage = plan.stage(1).context("missing stage one")?;
        for filter in ["a:tf", "a:xc"] {
            assert!(stage.contains(&SearchRequest::new("Ada Runner Central", filter, 1)));
            assert!(stage.contains(&SearchRequest::new("Ada Runner Austin TX", filter, 1)));
        }
        Ok(())
    }

    #[test]
    fn query_plan_deduplicates_equivalent_variants() {
        let prospect = Prospect {
            first_name: "Ada".to_owned(),
            last_name: "Runner".to_owned(),
            ..Default::default()
        };
        let plan = QueryPlan::for_prospect(&prospect);
        let keys = plan
            .iter()
            .map(SearchRequest::cache_key)
            .collect::<HashSet<_>>();
        assert_eq!(keys.len(), plan.iter().count());
    }

    #[tokio::test]
    async fn global_gate_spaces_requests_from_different_prospects() -> Result<()> {
        let (mut server, url) = tokio::task::spawn_blocking(|| {
            let server = mockito::Server::new();
            let url = server.url();
            (server, url)
        })
        .await?;
        let track_mock = server
            .mock("POST", "/search")
            .match_body(mockito::Matcher::Json(json!({
                "q": "Ada One",
                "fq": "t:a a:tf",
                "start": 0
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"d":{"results":""}}"#)
            .create();
        let xc_mock = server
            .mock("POST", "/search")
            .match_body(mockito::Matcher::Json(json!({
                "q": "Grace Two",
                "fq": "t:a a:xc",
                "start": 0
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"d":{"results":""}}"#)
            .create();
        let client = AthleticNetClient::new(&DiscoveryConfig {
            athletic_search_url: format!("{url}/search"),
            max_candidates: 3,
            request_timeout_seconds: 2,
            search_delay_ms: 500,
            max_attempts: 2,
            circuit_breaker_threshold: 3,
            ambiguity_margin: 0.03,
        })?;
        let started = std::time::Instant::now();
        client
            .execute(&SearchRequest::new("Ada One", "a:tf", 0))
            .await?;
        client
            .execute(&SearchRequest::new("Grace Two", "a:xc", 0))
            .await?;
        assert!(started.elapsed() >= Duration::from_millis(450));
        track_mock.assert();
        xc_mock.assert();
        Ok(())
    }

    #[tokio::test]
    async fn circuit_breaker_stops_retries_at_threshold() -> Result<()> {
        let (mut server, url) = tokio::task::spawn_blocking(|| {
            let server = mockito::Server::new();
            let url = server.url();
            (server, url)
        })
        .await?;
        let denied = server
            .mock("POST", "/search")
            .with_status(429)
            .with_header("retry-after", "0")
            .create();
        let client = AthleticNetClient::new(&DiscoveryConfig {
            athletic_search_url: format!("{url}/search"),
            max_candidates: 3,
            request_timeout_seconds: 2,
            search_delay_ms: 1,
            max_attempts: 4,
            circuit_breaker_threshold: 1,
            ambiguity_margin: 0.03,
        })?;
        let error = match client
            .execute(&SearchRequest::new("Ada Runner", "a:tf", 0))
            .await
        {
            Ok(_) => anyhow::bail!("rate-limited search unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error.attempts, 1);
        assert_eq!(error.status, Some(429));
        assert!(error.retryable);
        denied.assert();
        Ok(())
    }
}

#[path = "discovery_exhaustive.rs"]
mod exhaustive;

#[cfg(test)]
#[path = "discovery_retry_tests.rs"]
mod retry_tests;
