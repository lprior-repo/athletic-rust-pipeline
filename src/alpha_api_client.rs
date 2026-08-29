use std::time::Duration;
use reqwest::{Client, Method, redirect::Policy};
use tokio::sync::Semaphore;
use url::Url;
use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_model::{AlphaRequest, PaginationConfig, RankingRecord};
use crate::alpha_model_raw::{RawNavInfoResponse, RawRankingsResponse};
#[derive(Debug)]
pub enum BodyReadError { Timeout, Other(String) }
impl std::fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { BodyReadError::Timeout => f.write_str("body read timeout"), BodyReadError::Other(m) => f.write_str(m) }
    }
}
impl std::error::Error for BodyReadError {}
impl From<reqwest::Error> for BodyReadError {
    fn from(e: reqwest::Error) -> Self { if e.is_timeout() { BodyReadError::Timeout } else { BodyReadError::Other(e.to_string()) } }
}
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
    pub(crate) fn check_completeness(&self, raw: &RawRankingsResponse) -> Result<bool, AlphaApiError> {
        if self.is_truncated(raw) { return Err(AlphaApiError::TruncatedWithoutContinuation); }
        let value = &raw.value;
        // Validate nextPage/next pointer value type.
        let validate_next = |val: Option<&serde_json::Value>, ctx: &str| -> Result<(), AlphaApiError> {
            match val {
                None => Err(AlphaApiError::Incomplete(format!("{ctx} missing"))),
                Some(serde_json::Value::Null) => Err(AlphaApiError::Incomplete(format!("{ctx} is null"))),
                Some(serde_json::Value::String(s)) if s.is_empty() => Err(AlphaApiError::Incomplete(format!("{ctx} is empty"))),
                Some(serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Object(_)) => Ok(()),
                Some(v) => Err(AlphaApiError::Incomplete(format!("{ctx} unexpected type: {v}"))),
            }
        };
        match &self.config.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                if let Some(cont) = &raw.continuation {
                    if !cont.complete { 
                        validate_next(value.get("nextPage"), "continuation.complete=false but nextPage")?; 
                        return Err(AlphaApiError::Incomplete("SingleResponse: continuation.complete=false with nextPage but SingleResponse cannot produce a continuation token".into()));
                    }
                }
                match value.get("hasMore") {
                    Some(serde_json::Value::Bool(true)) => { validate_next(value.get("nextPage"), "hasMore=true, nextPage")?; return Err(AlphaApiError::Incomplete("SingleResponse: hasMore=true with nextPage but SingleResponse cannot produce a continuation token".into())); }
                    Some(serde_json::Value::Bool(false)) | None => {}
                    Some(v) => return Err(AlphaApiError::Incomplete(format!("hasMore is not bool: {v}"))),
                }
                let ptr = Self::resolve_ptr(complete_pointer);
                match raw.value.pointer(&ptr).ok_or_else(|| AlphaApiError::MissingPointer(ptr))? {
                    serde_json::Value::Bool(b) => Ok(*b),
                    v => Err(AlphaApiError::Incomplete(format!("complete pointer {complete_pointer} not bool: {v}"))),
                }
            }
            PaginationConfig::NextPage { has_more_pointer, next_page_pointer, .. } => {
                if let Some(cont) = &raw.continuation {
                    if !cont.complete {
                        let nptr = Self::resolve_ptr(next_page_pointer);
                        validate_next(value.pointer(&nptr), "continuation.complete=false but next pointer")?;
                        return Ok(false);
                    }
                }
                let hptr = Self::resolve_ptr(has_more_pointer);
                match value.pointer(&hptr).ok_or_else(|| AlphaApiError::MissingPointer(hptr.clone()))? {
                    serde_json::Value::Bool(v) => { if !v { return Ok(true); } }
                    v => return Err(AlphaApiError::Incomplete(format!("has_more pointer {has_more_pointer} not bool: {v}"))),
                };
                let nptr = Self::resolve_ptr(next_page_pointer);
                validate_next(value.pointer(&nptr), &format!("hasMore=true, next pointer {next_page_pointer}"))?;
                Ok(false)
            }
        }
    }
    fn resolve_ptr(ptr: &str) -> String { ptr.to_string() }

    fn is_truncated(&self, raw: &RawRankingsResponse) -> bool {
        let v = &raw.value;
        for cm in &self.config.cap_markers {
            let found = if cm.starts_with('/') {
                v.pointer(cm)
            } else {
                v.get(cm)
            };
            match found {
                Some(serde_json::Value::Bool(true)) => return true,
                Some(serde_json::Value::Bool(false)) => {}
                Some(_v) => return true,
                None => {}
            }
        }
        false
    }

    /// Return status + headers without consuming the body.
    async fn check_status(builder: reqwest::RequestBuilder)
        -> Result<(u16, reqwest::header::HeaderMap, reqwest::Response), reqwest::Error>
    {
        let resp = builder.send().await?;
        Ok((resp.status().as_u16(), resp.headers().clone(), resp))
    }

    /// Read response body with a timeout.
    /// Returns BodyReadError::Timeout on deadline; BodyReadError::Other for failures.
    async fn read_body_with_timeout(resp: reqwest::Response, timeout: Duration)
        -> Result<String, BodyReadError>
    {
        tokio::time::timeout(timeout, resp.text())
            .await
            .map_err(|_| BodyReadError::Timeout)
            .and_then(|r| r.map_err(BodyReadError::from))
    }

    /// Unified retry loop — body timeouts retry whole request,
    /// 5xx retries; 2xx reads body; other non-2xx reads body).
    async fn execute_request(&self, method: Method, url: Url, body: Option<&serde_json::Value>)
        -> Result<String, AlphaApiError> {
        let mut total_wait_ms: u64 = 0;
        let max_retry = self.config.max_retries;
        let timeout_ms = self.config.timeout_seconds.saturating_mul(1000);
        let timeout_dur = Duration::from_secs(self.config.timeout_seconds);
        let mut attempt = 0usize;
        loop {
            let builder = self.client.request(method.clone(), url.as_str())
                .timeout(Duration::from_secs(self.config.timeout_seconds));
            let builder = match body { Some(b) => builder.header("Content-Type", "application/json").json(b), None => builder };
            let (status, headers, resp) = match Self::check_status(builder).await {
                Ok(r) => r,
                Err(e) if e.is_timeout() => { if attempt >= max_retry { return Err(AlphaApiError::Timeout { milliseconds: timeout_ms }); } attempt += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                Err(e) => return Err(AlphaApiError::Request(e)),
            };
            if status == 401 { return Err(AlphaApiError::Unauthorized(format!("HTTP {status}"))); }
            if status == 403 { return Err(AlphaApiError::Forbidden(format!("HTTP {status}"))); }
            if status == 429 {
                let retry_after_ms = match headers.get("Retry-After").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()) {
                    Some(d) => d.saturating_mul(1000),
                    None => return Err(AlphaApiError::RateLimitedNoRetryAfter),
                };
                let wait_ms = retry_after_ms.max(self.config.min_delay_ms);
                if attempt >= max_retry { return Err(AlphaApiError::RateLimitedExhausted { max_retries: max_retry, total_delay_ms: total_wait_ms }); }
                total_wait_ms = total_wait_ms.saturating_add(wait_ms);
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                continue;
            }
            if status >= 500 {
                if attempt < max_retry { attempt += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                match Self::read_body_with_timeout(resp, timeout_dur).await {
                    Err(BodyReadError::Timeout) => { if attempt >= max_retry { return Err(AlphaApiError::Timeout { milliseconds: timeout_ms }); } attempt += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                    Err(e) => return Err(AlphaApiError::Incomplete(e.to_string())),
                    Ok(_) => {}
                };
                return Err(AlphaApiError::ServerErrorExhausted { status, retries: attempt });
            }
            if status < 200 || status >= 300 {
                let resp_body = match Self::read_body_with_timeout(resp, timeout_dur).await {
                    Ok(b) => b,
                    Err(BodyReadError::Timeout) => { if attempt >= max_retry { return Err(AlphaApiError::Timeout { milliseconds: timeout_ms }); } attempt += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                    Err(e) => return Err(AlphaApiError::Incomplete(e.to_string())),
                };
                return Err(AlphaApiError::UnexpectedStatus { status, body: resp_body });
            }
            match Self::read_body_with_timeout(resp, timeout_dur).await {
                Ok(resp_body) => return Ok(resp_body),
                Err(BodyReadError::Timeout) => { if attempt >= max_retry { return Err(AlphaApiError::Timeout { milliseconds: timeout_ms }); } attempt += 1; tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await; continue; }
                Err(e) => return Err(AlphaApiError::Incomplete(e.to_string())),
            }
        }
    }


    pub async fn rankings(&self, req: &AlphaRequest) -> Result<RankingPage, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await.map_err(|_| AlphaApiError::Incomplete("semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
        let mut body = Self::serialize_rankings_body(req);
        body["qParams"] = Self::build_qparams(&self.config.pagination, &req.continuation);
        let base = Url::parse(&self.config.base_url).map_err(|e| AlphaApiError::Incomplete(format!("bad base_url: {e}")))?;
        crate::alpha_route_validation::validate_route(&self.config.rankings_path, &base, &self.config.allowed_routes).map_err(|e| AlphaApiError::Incomplete(format!("route: {e}")))?;
        let url = base.join(&self.config.rankings_path).map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        self.validate_pre_send_allowed_fields()?;
        let text = self.execute_request(Method::POST, url, Some(&body)).await?;
        let raw = RawRankingsResponse::from_json(&text).map_err(|e| AlphaApiError::Incomplete(e))?;
        let complete = self.check_completeness(&raw)?;
        let continuation = if !complete {
            match &self.config.pagination {
                PaginationConfig::SingleResponse { .. } => None,
                PaginationConfig::NextPage { next_page_pointer, .. } => {
                    raw.value.pointer(&Self::resolve_ptr(next_page_pointer)).cloned()
                }
            }
        } else {
            None
        };
        let validated_json = self.enforce_response_allowed_fields(raw.value)?;
        let validated_raw = RawRankingsResponse::from_json(&validated_json.to_string()).map_err(|e| AlphaApiError::Incomplete(format!("reparse: {e}")))?;
        let records = self.parse_rankings_strict(&validated_raw).map_err(|e| AlphaApiError::Incomplete(e))?;
        Ok(RankingPage { records, complete, continuation })
    }

    pub async fn nav_info(&self, season_id: i32, indoor: bool) -> Result<RawNavInfoResponse, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await.map_err(|_| AlphaApiError::Incomplete("semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
        let base = Url::parse(&self.config.base_url).map_err(|e| AlphaApiError::Incomplete(format!("bad base_url: {e}")))?;
        crate::alpha_route_validation::validate_route(&self.config.nav_info_path, &base, &self.config.allowed_routes).map_err(|e| AlphaApiError::Incomplete(format!("route: {e}")))?;
        let mut url = base.join(&self.config.nav_info_path).map_err(|e| AlphaApiError::Incomplete(format!("url join: {e}")))?;
        url.query_pairs_mut().append_pair("season_id", &season_id.to_string()).append_pair("indoor", &indoor.to_string());
        let text = self.execute_request(Method::GET, url, None).await?;
        let nav: RawNavInfoResponse = serde_json::from_str(&text).map_err(|e| AlphaApiError::Incomplete(format!("parse error: {e}")))?;
        nav.validate().map_err(|e| AlphaApiError::Incomplete(e.to_string()))?;
        Ok(nav)
    }

    pub fn walk_pointer_value<'a>(v: &'a serde_json::Value, ptr: &str) -> Option<&'a serde_json::Value> {
        v.pointer(ptr)
    }
}
