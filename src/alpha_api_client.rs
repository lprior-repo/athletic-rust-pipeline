use std::time::Duration;
use reqwest::{Client, Method, redirect::Policy};
use tokio::sync::Semaphore;
use url::Url;
use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_model::{AlphaRequest, PaginationConfig, RankingRecord};
use crate::alpha_model_raw::{RawNavInfoResponse, RawRankingsResponse};

#[derive(Debug)]
pub struct RankingPage {
    pub records: Vec<RankingRecord>,
    pub complete: bool,
    #[allow(dead_code)]
    pub continuation: Option<serde_json::Value>,
}

pub struct AlphaApiClient {
    client: Client,
    pub(crate) config: AlphaApiClientConfig,
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
        Ok(AlphaApiClient { client, config, concurrency_semaphore: Semaphore::new(max_concurrent) })
    }

    pub fn serialize_rankings_body(req: &AlphaRequest) -> serde_json::Value {
        serde_json::json!({"reportType":"div","mode":"list","divListId":req.state_id,"indoor":req.indoor,"eventShort":req.event_short.clone(),"gender":req.gender.clone(),"qualifyingListKey":"","version":2,"debug":""})
    }

    pub fn build_qparams(pagination: &PaginationConfig, continuation: &Option<serde_json::Value>) -> serde_json::Value {
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

    /// FIX 1: SingleResponse — hasMore controls flow explicitly.
    /// hasMore=false|absent: fall through to complete_pointer; hasMore=true:
    /// validate nextPage then return Ok(false); continuation.complete=false wins.
    pub(crate) fn check_completeness(&self, raw: &RawRankingsResponse) -> Result<bool, AlphaApiError> {
        if self.is_truncated(raw) { return Err(AlphaApiError::TruncatedWithoutContinuation); }
        match &self.config.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                let value = &raw.value;
                if let Some(cont) = &raw.continuation {
                    if !cont.complete {
                        let np = value.get("nextPage");
                        match np {
                            None => return Err(AlphaApiError::Incomplete("continuation.complete=false but nextPage missing".into())),
                            Some(serde_json::Value::Null) => return Err(AlphaApiError::Incomplete("continuation.complete=false but nextPage is null".into())),
                            Some(serde_json::Value::String(s)) if s.is_empty() => return Err(AlphaApiError::Incomplete("continuation.complete=false but nextPage is empty".into())),
                            Some(serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Object(_)) => {}
                            Some(v) => return Err(AlphaApiError::Incomplete(format!("continuation.complete=false but nextPage unexpected type: {v}"))),
                        }
                        return Ok(false);
                    }
                }
                match value.get("hasMore").and_then(|x| x.as_bool()) {
                    Some(true) => {
                        let np = value.get("nextPage");
                        match np {
                            None => return Err(AlphaApiError::Incomplete("hasMore=true, nextPage missing".into())),
                            Some(serde_json::Value::Null) => return Err(AlphaApiError::Incomplete("hasMore=true, nextPage is null".into())),
                            Some(serde_json::Value::String(s)) if s.is_empty() => return Err(AlphaApiError::Incomplete("hasMore=true, nextPage is empty".into())),
                            Some(serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Object(_)) => {}
                            Some(v) => return Err(AlphaApiError::Incomplete(format!("hasMore=true, nextPage unexpected type: {v}"))),
                        }
                        return Ok(false); // hasMore=true with valid nextPage => incomplete
                    }
                    Some(false) | None => {} // fall through to complete_pointer
                }
                let ptr = Self::resolve_ptr(complete_pointer);
                match raw.value.pointer(&ptr).ok_or_else(|| AlphaApiError::MissingPointer(ptr))? {
                    serde_json::Value::Bool(true) => Ok(true),
                    serde_json::Value::Bool(false) => Ok(false),
                    v => Err(AlphaApiError::Incomplete(format!("complete pointer {complete_pointer} not bool: {v}"))),
                }
            }
            PaginationConfig::NextPage { has_more_pointer, next_page_pointer, .. } => {
                let value = &raw.value;
                // continuation.complete=false overrides hasMore/nextPage.
                if let Some(cont) = &raw.continuation {
                    if !cont.complete {
                        let nptr = Self::resolve_ptr(next_page_pointer);
                        let np = value.pointer(&nptr);
                        match np {
                            None => return Err(AlphaApiError::Incomplete("continuation.complete=false but next pointer missing".into())),
                            Some(serde_json::Value::Null) => return Err(AlphaApiError::Incomplete("continuation.complete=false but next pointer is null".into())),
                            Some(serde_json::Value::String(s)) if s.is_empty() => return Err(AlphaApiError::Incomplete("continuation.complete=false but next pointer is empty".into())),
                            Some(serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Object(_)) => {}
                            Some(v) => return Err(AlphaApiError::Incomplete(format!("continuation.complete=false but next pointer unexpected type: {v}"))),
                        }
                        return Ok(false); // continue with the token
                    }
                }
                // Continuation complete or absent; check hasMore.
                let hptr = Self::resolve_ptr(has_more_pointer);
                match value.pointer(&hptr).ok_or_else(|| AlphaApiError::MissingPointer(hptr.clone()))? {
                    serde_json::Value::Bool(v) => { if !v { return Ok(true); } }
                    v => return Err(AlphaApiError::Incomplete(format!("has_more pointer {has_more_pointer} not bool: {v}"))),
                };
                let nptr = Self::resolve_ptr(next_page_pointer);
                let np = value.pointer(&nptr);
                match np {
                    None => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} missing"))),
                    Some(serde_json::Value::Null) => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} is null"))),
                    Some(serde_json::Value::String(s)) if s.is_empty() => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} is empty"))),
                    Some(serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Object(_)) => {}
                    Some(v) => return Err(AlphaApiError::Incomplete(format!("hasMore=true, next pointer {next_page_pointer} unexpected type: {v}"))),
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

    /// FIX 2: send+read in one step; body timeout retries the whole request.
    async fn send_and_read(builder: reqwest::RequestBuilder)
        -> Result<(u16, reqwest::header::HeaderMap, String), reqwest::Error>
    {
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let body = resp.text().await?;
        Ok((status, headers, body))
    }

    /// FIX 2: unified retry loop — body timeouts retry whole request,
    /// final UnexpectedStatus preserved.
    async fn execute_request(&self, method: Method, url: Url, body: Option<&serde_json::Value>)
        -> Result<String, AlphaApiError>
    {
        let mut total_wait_ms: u64 = 0;
        let max_retry = self.config.max_retries;
        let timeout_ms = self.config.timeout_seconds.saturating_mul(1000);
        let mut attempt = 0usize;
        loop {
            let builder = self.client.request(method.clone(), url.as_str())
                .timeout(Duration::from_secs(self.config.timeout_seconds));
            let builder = match body {
                Some(b) => builder.header("Content-Type", "application/json").json(b),
                None => builder,
            };
            let (status, headers, resp_body) = match Self::send_and_read(builder).await {
                Ok(r) => r,
                Err(e) if e.is_timeout() => {
                    if attempt >= max_retry { return Err(AlphaApiError::Timeout { milliseconds: timeout_ms }); }
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(AlphaApiError::Request(e)),
            };
            if status == 401 { return Err(AlphaApiError::Unauthorized(format!("HTTP {status}"))); }
            if status == 403 { return Err(AlphaApiError::Forbidden(format!("HTTP {status}"))); }
            if status == 429 {
                let retry_after_ms = match headers.get("Retry-After")
                    .and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()) {
                    Some(d) => d.saturating_mul(1000),
                    None => return Err(AlphaApiError::RateLimitedNoRetryAfter),
                };
                let wait_ms = retry_after_ms.max(self.config.min_delay_ms);
                if attempt >= max_retry {
                    return Err(AlphaApiError::RateLimitedExhausted { max_retries: max_retry, total_delay_ms: total_wait_ms });
                }
                total_wait_ms = total_wait_ms.saturating_add(wait_ms);
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                continue;
            }
            if status >= 500 {
                if attempt < max_retry {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
                    continue;
                }
                return Err(AlphaApiError::ServerErrorExhausted { status, retries: attempt });
            }
            if status < 200 || status >= 300 {
                return Err(AlphaApiError::UnexpectedStatus { status, body: resp_body });
            }
            return Ok(resp_body);
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
        let url = base.join(route).map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        self.validate_pre_send_allowed_fields()?;
        let text = self.execute_request(Method::POST, url, Some(&body)).await?;
        let raw = RawRankingsResponse::from_json(&text).map_err(|e| AlphaApiError::Incomplete(e))?;
        let complete = self.check_completeness(&raw)?;
        let continuation = match &self.config.pagination {
            PaginationConfig::SingleResponse { .. } => None,
            PaginationConfig::NextPage { next_page_pointer, .. } => {
                let ptr = Self::resolve_ptr(next_page_pointer);
                raw.value.pointer(&ptr).cloned()
            }
        };
        let validated_json = self.enforce_response_allowed_fields(raw.value)?;
        let validated_raw = RawRankingsResponse::from_json(&validated_json.to_string())
            .map_err(|e| AlphaApiError::Incomplete(format!("reparse filtered response: {e}")))?;
        let records = self.parse_rankings_strict(&validated_raw).map_err(|e| AlphaApiError::Incomplete(e))?;
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
        let mut url = base.join(route).map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        url.query_pairs_mut().append_pair("season_id", &season_id.to_string())
            .append_pair("indoor", &indoor.to_string());
        let text = self.execute_request(Method::GET, url, None).await?;
        let nav: RawNavInfoResponse = serde_json::from_str(&text)
            .map_err(|e| AlphaApiError::Incomplete(format!("JSON parse error: {e}")))?;
        nav.validate().map_err(|e| AlphaApiError::Incomplete(e.to_string()))?;
        Ok(nav)
    }

    pub fn walk_pointer_value<'a>(v: &'a serde_json::Value, ptr: &str) -> Option<&'a serde_json::Value> {
        v.pointer(ptr)
    }
}
