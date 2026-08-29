use std::time::Duration;

use reqwest::{Client, Method, redirect::Policy};
use tokio::sync::Semaphore;
use url::Url;

use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_model::{AlphaRequest, PaginationConfig, RankingRecord};
use crate::alpha_model_raw::{RawNavInfoResponse, RawRankingsResponse};

/// A typed rankings page with completeness information.
#[derive(Debug)]
pub struct RankingPage {
    pub records: Vec<RankingRecord>,
    pub complete: bool,
    #[allow(dead_code)]
    pub continuation: Option<serde_json::Value>,
}

/// Alpha API client using reqwest + rustls.
pub struct AlphaApiClient {
    client: Client,
    config: AlphaApiClientConfig,
    concurrency_semaphore: Semaphore,
}

impl AlphaApiClient {
    pub fn new(config: AlphaApiClientConfig) -> Result<Self, AlphaApiError> {
        let max_concurrent = config.max_concurrent_requests.max(1);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .redirect(Policy::none())
            .build()
            .map_err(|e| AlphaApiError::Incomplete(format!("reqwest builder failed: {e}")))?;
        Ok(AlphaApiClient {
            client,
            config,
            concurrency_semaphore: Semaphore::new(max_concurrent),
        })
    }


    pub fn serialize_rankings_body(req: &AlphaRequest) -> serde_json::Value {
        serde_json::json!({"reportType":"div","mode":"list","divListId":req.state_id,"indoor":req.indoor,"eventShort":req.event_short.clone(),"gender":req.gender.clone(),"qualifyingListKey":"","version":2,"debug":""})
    }

    pub fn build_qparams(
        pagination: &PaginationConfig,
        continuation: &Option<serde_json::Value>,
    ) -> serde_json::Value {
        match (pagination, continuation) {
            (PaginationConfig::NextPage { request_page_key, .. }, Some(c)) => serde_json::json!({ request_page_key: c }),
            _ => serde_json::json!({}),
        }
    }

    fn parse_rankings_strict(&self, raw: &RawRankingsResponse) -> Result<Vec<RankingRecord>, String> {
        let mut records = Vec::new();
        for group in &raw.grouped_rankings {
            for r in group { records.append(&mut r.to_flattened_records()?); }
        }
        Ok(records)
    }

    pub(crate) fn check_completeness(&self, raw: &RawRankingsResponse) -> Result<bool, AlphaApiError> {
        if self.is_truncated(raw) { return Err(AlphaApiError::TruncatedWithoutContinuation); }
        match &self.config.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                let ptr = Self::resolve_ptr(complete_pointer);
                match raw.value.pointer(&ptr).ok_or_else(|| AlphaApiError::MissingPointer(ptr))? {
                    serde_json::Value::Bool(true) => Ok(true),
                    serde_json::Value::Bool(false) => Ok(false),
                    v => Err(AlphaApiError::Incomplete(format!("complete pointer {complete_pointer} not bool: {v}"))),
                }
            }
            PaginationConfig::NextPage { has_more_pointer, next_page_pointer, .. } => {
                let value = &raw.value;
                // Fix 3: continuation.complete=false overrides hasMore=false => incomplete.
                if let Some(cont) = &raw.continuation {
                    if !cont.complete {
                        return Ok(false);
                    }
                }
                let hptr = Self::resolve_ptr(has_more_pointer);
                match value.pointer(&hptr).ok_or_else(|| AlphaApiError::MissingPointer(hptr.clone()))? {
                    serde_json::Value::Bool(v) => {
                        if !v { return Ok(true); }
                    }
                    v => return Err(AlphaApiError::Incomplete(format!("has_more pointer {has_more_pointer} not bool: {v}"))),
                };
                let nptr = Self::resolve_ptr(next_page_pointer);
                let np = value.pointer(&nptr);
                match np {
                    None => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} missing"))),
                    Some(serde_json::Value::Null) => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} is null"))),
                    Some(serde_json::Value::String(s)) if s.is_empty() => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} is empty"))),
                    Some(serde_json::Value::String(_)) | Some(serde_json::Value::Number(_)) | Some(serde_json::Value::Object(_)) => {},
                    Some(v) => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} unexpected type: {v}"))),
                }
                // Check all configured cap markers for truncation.
                for cm in &self.config.cap_markers {
                    if value.get(cm).and_then(|x| x.as_bool()) == Some(true) {
                        return Err(AlphaApiError::TruncatedWithoutContinuation);
                    }
                }
                Ok(false)
            }
        }
    }
    fn resolve_ptr(ptr: &str) -> String {
        if ptr.starts_with('/') { ptr.to_string() } else { format!("/{ptr}") }
    }
    fn is_truncated(&self, raw: &RawRankingsResponse) -> bool {
        let v = &raw.value;
        for cm in &self.config.cap_markers {
            if v.get(cm).and_then(|x| x.as_bool()) == Some(true) { return true; }
        }
        v.get("__truncated").and_then(|x| x.as_bool()) == Some(true)
            || v.get("__cap").and_then(|x| x.as_bool()) == Some(true)
            || (v.get("hasMore").and_then(|x| x.as_bool()) == Some(true) && v.get("nextPage").is_none())
    }

    async fn execute_request(
        &self,
        method: Method,
        url: Url,
        body: Option<&serde_json::Value>,
    ) -> Result<String, AlphaApiError> {
        let mut retry = 0usize;
        let max_retry = self.config.max_retries;
        let timeout_ms = self.config.timeout_seconds * 1000;
        loop {
            let builder = self.client.request(method.clone(), url.as_str())
                .timeout(Duration::from_secs(self.config.timeout_seconds));
            let builder = match body {
                Some(b) => builder.header("Content-Type", "application/json").json(b),
                None => builder,
            };
            let resp = builder.send().await;
            let resp = match resp {
                Ok(r) => r,
                Err(e) if e.is_timeout() => {
                    if retry >= max_retry {
                        return Err(AlphaApiError::Timeout { milliseconds: timeout_ms });
                    }
                    retry += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue;
                }
                Err(e) => return Err(AlphaApiError::Request(e)),
            };
            let status = resp.status().as_u16();
            if status == 401 { return Err(AlphaApiError::Unauthorized(format!("HTTP {status}"))); }
            if status == 403 { return Err(AlphaApiError::Forbidden(format!("HTTP {status}"))); }
            if status == 429 {
                let delay = resp.headers().get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|s| s.saturating_mul(1000).max(self.config.min_delay_ms));
                match delay {
                    Some(d) => {
                        if retry >= max_retry {
                            return Err(AlphaApiError::RateLimitedExhausted { max_retries: max_retry, total_delay_ms: d * retry as u64 });
                        }
                        tokio::time::sleep(Duration::from_millis(d)).await; retry += 1; continue;
                    }
                    None => return Err(AlphaApiError::RateLimitedNoRetryAfter),
                }
            }
            if status >= 500 {
                if retry < max_retry { retry += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                return Err(AlphaApiError::ServerErrorExhausted { status, retries: retry });
            }
            if status < 200 || status >= 300 {
                let body = resp.text().await.map_err(AlphaApiError::Request)?;
                return Err(AlphaApiError::UnexpectedStatus { status, body });
            }
            // Buffer body inside the retry loop so timeouts on text() also retry.
            let body = resp.text().await.map_err(AlphaApiError::Request)?;
            return Ok(body);
        }
    }

    pub async fn rankings(&self, req: &AlphaRequest) -> Result<RankingPage, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await
            .map_err(|_| AlphaApiError::Incomplete("concurrency semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
        let mut body = Self::serialize_rankings_body(req);
        body["qParams"] = Self::build_qparams(&self.config.pagination, &req.continuation);
        let route = &self.config.rankings_path;
        let base = Url::parse(&self.config.base_url)
            .map_err(|e| AlphaApiError::Incomplete(format!("bad base_url: {e}")))?;
        crate::alpha_route_validation::validate_route(route, &base, &self.config.allowed_routes)
            .map_err(|e| AlphaApiError::Incomplete(format!("route: {e}")))?;
        let url = base.join(route)
            .map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        let text = self.execute_request(Method::POST, url, Some(&body)).await?;
        let raw = RawRankingsResponse::from_json(&text).map_err(|e| AlphaApiError::Incomplete(e))?;
        let complete = self.check_completeness(&raw)?;
        // SingleResponse continuation is a boolean flag, not a token => None.
        let continuation = match &self.config.pagination {
            PaginationConfig::SingleResponse { .. } => None,
            PaginationConfig::NextPage { next_page_pointer, .. } => {
                let ptr = Self::resolve_ptr(next_page_pointer);
                raw.value.pointer(&ptr).cloned()
            }
        };
        // Filter allowed_fields BEFORE parsing groupedRankings,
        // so disallowed fields (e.g. Wind) never reach RankingRecord.
        let validated_json = self.enforce_response_allowed_fields(raw.value)?;
        let validated_raw = RawRankingsResponse::from_json(&validated_json.to_string())
            .map_err(|e| AlphaApiError::Incomplete(format!("reparse filtered response: {e}")))?;
        let records = self.parse_rankings_strict(&validated_raw)
            .map_err(|e| AlphaApiError::Incomplete(e))?;
        Ok(RankingPage { records, complete, continuation })
    }
    pub async fn nav_info(&self, season_id: i32, indoor: bool) -> Result<RawNavInfoResponse, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await
            .map_err(|_| AlphaApiError::Incomplete("concurrency semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
        let route = &self.config.nav_info_path;
        let base = Url::parse(&self.config.base_url)
            .map_err(|e| AlphaApiError::Incomplete(format!("bad base_url: {e}")))?;
        crate::alpha_route_validation::validate_route(route, &base, &self.config.allowed_routes)
            .map_err(|e| AlphaApiError::Incomplete(format!("route: {e}")))?;
        let mut url = base.join(route)
            .map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        url.query_pairs_mut()
            .append_pair("season_id", &season_id.to_string())
            .append_pair("indoor", &indoor.to_string());
        let text = self.execute_request(Method::GET, url, None).await?;
        let nav: RawNavInfoResponse = serde_json::from_str(&text)
            .map_err(|e| AlphaApiError::Incomplete(format!("JSON parse error: {e}")))?;
        nav.validate().map_err(|e| AlphaApiError::Incomplete(e.to_string()))?;
        Ok(nav)
    }

    pub(crate) fn enforce_response_allowed_fields(
        &self, mut value: serde_json::Value,
    ) -> Result<serde_json::Value, AlphaApiError> {
        let allowed = &self.config.allowed_fields;
        if allowed.is_empty() { return Ok(value); }
        // Every retained source field must be explicitly authorized in allowed_fields.
        let required_fields = ["AthleteID", "AthleteName", "GradeID", "TeamName", "State", "MeetID", "MeetName", "IDResult", "EventShort", "Measure", "ResultDate", "SeasonID"];
        for &f in &required_fields {
            if !allowed.iter().any(|a| a == f) {
                return Err(AlphaApiError::Incomplete(format!("required source field '{f}' not in allowed_fields")));
            }
        }
        fn filter_fields(obj: &mut serde_json::Map<String, serde_json::Value>, allowed: &[String]) {
            let allowed_set: std::collections::HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
            obj.retain(|k, _| allowed_set.contains(k.as_str()));
        }
        if let Some(obj) = value.as_object_mut() {
            if let Some(groups) = obj.get_mut("groupedRankings").and_then(|v| v.as_array_mut()) {
                for g in groups {
                    if let Some(records) = g.as_array_mut() {
                        for rec in records {
                            if let Some(rec_obj) = rec.as_object_mut() {
                                // Preserve "Results" (structural key) before filtering, filter nested results, then re-attach.
                                let results: Option<Vec<serde_json::Value>> = rec_obj.remove("Results").and_then(|v| {
                                    match v {
                                        serde_json::Value::Array(arr) => Some(arr),
                                        _ => None,
                                    }
                                });
                                filter_fields(rec_obj, allowed);
                                if let Some(mut results) = results {
                                    for r in results.iter_mut() {
                                        if let Some(res_obj) = r.as_object_mut() {
                                            filter_fields(res_obj, allowed);
                                        }
                                    }
                                    rec_obj.insert("Results".into(), serde_json::Value::Array(results));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(value)
    }


    pub fn walk_pointer_value<'a>(v: &'a serde_json::Value, ptr: &str) -> Option<&'a serde_json::Value> {
        v.pointer(ptr)
    }
}
