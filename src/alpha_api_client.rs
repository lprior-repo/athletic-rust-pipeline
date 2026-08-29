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
    pub fn new(config: AlphaApiClientConfig) -> Self {
        let max_concurrent = config.max_concurrent_requests.max(1);
        AlphaApiClient {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout_seconds))
                .redirect(Policy::none())
                .build()
                .expect("reqwest builder must not fail with valid config"),
            config,
            concurrency_semaphore: Semaphore::new(max_concurrent),
        }
    }

    fn validate_route(&self, route: &str) -> Result<(), AlphaApiError> {
        let base = Url::parse(&self.config.base_url)
            .map_err(|e| AlphaApiError::Incomplete(format!("invalid base_url: {}", e)))?;
        crate::alpha_route_validation::validate_route(route, &base, &self.config.allowed_routes)
            .map_err(|e| AlphaApiError::Incomplete(format!("route validation: {}", e)))
    }

    /// Serialize the POST body for a rankings request.
    /// Uses numeric divListId. qParams built separately from pagination config.
    pub fn serialize_rankings_body(req: &AlphaRequest) -> serde_json::Value {
        serde_json::json!({
            "reportType": "div",
            "mode": "list",
            "divListId": req.state_id,
            "indoor": req.indoor,
            "eventShort": req.event_short.clone(),
            "gender": req.gender.clone(),
            "qualifyingListKey": "",
            "version": 2,
            "debug": ""
        })
    }

    /// Build qParams object from pagination config and optional continuation.
    /// NextPage mode uses configured request_page_key; SingleResponse always returns {}.
    pub fn build_qparams(
        pagination: &PaginationConfig,
        continuation: &Option<serde_json::Value>,
    ) -> serde_json::Value {
        match pagination {
            PaginationConfig::NextPage { request_page_key, .. } => match continuation {
                Some(cont) => serde_json::json!({ request_page_key: cont }),
                None => serde_json::json!({}),
            },
            PaginationConfig::SingleResponse { .. } => serde_json::json!({}),
        }
    }

    fn parse_rankings(&self, raw: &RawRankingsResponse) -> Vec<RankingRecord> {
        raw.grouped_rankings.iter()
            .flat_map(|group| group.iter().filter_map(|r| r.to_flattened_records().ok()).flatten())
            .collect()
    }

    /// Evaluate completeness using RFC 6901 JSON pointers.
    ///
    /// Fail-closed semantics:
    /// - Missing or wrong-type has_more / next pointer => Incomplete
    /// - has_more == false => complete
    /// - has_more == true => incomplete (return continuation if present)
    /// - No cap/truncation marker detected
    pub(crate) fn check_completeness(&self, raw: &RawRankingsResponse) -> Result<bool, AlphaApiError> {
        // Cap/truncation detection: if hasMore is true but no continuation, fail closed.
        if self.is_truncated(raw) {
            return Err(AlphaApiError::TruncatedWithoutContinuation);
        }

        match &self.config.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                let ptr = if complete_pointer.starts_with('/') {
                    complete_pointer.as_str()
                } else { &format!("/{}", complete_pointer) };
                let val = raw.value.pointer(ptr)
                    .ok_or_else(|| AlphaApiError::MissingPointer(ptr.to_string()))?;
                match val {
                    serde_json::Value::Bool(true) => Ok(true),
                    serde_json::Value::Bool(false) => Ok(false),
                    _ => Err(AlphaApiError::Incomplete(format!(
                        "complete pointer {} is not a bool", complete_pointer
                    ))),
                }
            }
            PaginationConfig::NextPage { has_more_pointer, next_page_pointer, .. } => {
                let value = &raw.value;

                // has_more pointer
                let hptr = if has_more_pointer.starts_with('/') {
                    has_more_pointer.as_str()
                } else { &format!("/{}", has_more_pointer) };
                let hm = value.pointer(hptr)
                    .ok_or_else(|| AlphaApiError::MissingPointer(hptr.to_string()))?;
                let has_more = match hm {
                    serde_json::Value::Bool(v) => *v,
                    _ => return Err(AlphaApiError::Incomplete(format!(
                        "has_more pointer {} is not a bool", has_more_pointer
                    ))),
                };

                if !has_more {
                    return Ok(true);
                }

                // has_more == true: next pointer must exist
                let nptr = if next_page_pointer.starts_with('/') {
                    next_page_pointer.as_str()
                } else { &format!("/{}", next_page_pointer) };
                let np = value.pointer(nptr);
                if np.is_none() || np == Some(&serde_json::Value::Null) {
                    return Err(AlphaApiError::Incomplete(format!(
                        "has_more is true but next pointer {} is missing/null", next_page_pointer
                    )));
                }

                Ok(false)
            }
        }
    }

    /// Detect cap/truncation markers in the response.
    ///
    /// Returns true if the response appears truncated/capped without proper
    /// continuation metadata, enabling fail-closed behavior.
    fn is_truncated(&self, raw: &RawRankingsResponse) -> bool {
        let value = &raw.value;
        if let Some(marker) = value.get("__truncated").and_then(|v| v.as_bool()) {
            if marker { return true; }
        }
        if let Some(marker) = value.get("__cap").and_then(|v| v.as_bool()) {
            if marker { return true; }
        }
        false
    }

    /// Execute a single HTTP request with shared retry logic for timeout, 5xx, and 429.
    /// Enforces semaphore=1, min delay, timeout, 5xx bounded retry,
    async fn execute_request(
        &self,
        method: Method,
        url: Url,
        body: Option<&serde_json::Value>,
    ) -> Result<reqwest::Response, AlphaApiError> {
        let mut retry_count = 0usize;
        let max_retries = self.config.max_retries;
        let timeout_ms = self.config.timeout_seconds * 1000;

        loop {
            let request_builder = self.client.request(method.clone(), url.as_str())
                .timeout(Duration::from_secs(self.config.timeout_seconds));
            let request_builder = match body {
                Some(b) => request_builder.header("Content-Type", "application/json").json(b),
                None => request_builder,
            };
            let resp = request_builder.send().await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) if e.is_timeout() => {
                    if retry_count >= max_retries {
                        return Err(AlphaApiError::Timeout { milliseconds: timeout_ms });
                    }
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(AlphaApiError::Request(e)),
            };

            let status = resp.status().as_u16();

            if status == 401 {
                return Err(AlphaApiError::Unauthorized(format!("HTTP {}", status)));
            }
            if status == 403 {
                return Err(AlphaApiError::Forbidden(format!("HTTP {}", status)));
            }

            if status == 429 {
                match resp.headers().get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    Some(delay_secs) => {
                        let delay_ms = delay_secs.saturating_mul(1000);
                        let wait = delay_ms.max(self.config.min_delay_ms);
                        if retry_count >= max_retries {
                            return Err(AlphaApiError::RateLimitedExhausted {
                                max_retries,
                                total_delay_ms: wait * retry_count as u64,
                            });
                        }
                        tokio::time::sleep(Duration::from_millis(wait)).await;
                        retry_count += 1;
                        continue;
                    }
                    None => return Err(AlphaApiError::RateLimitedNoRetryAfter),
                }
            }

            if status >= 500 {
                if retry_count < max_retries {
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;
                    continue;
                }
                return Err(AlphaApiError::ServerErrorExhausted { status, retries: retry_count });
            }

            if status < 200 || status >= 300 {
                let body = resp.text().await.unwrap_or_default();
                return Err(AlphaApiError::UnexpectedStatus { status, body });
            }

            return Ok(resp);
        }
    }

    pub async fn rankings(&self, req: &AlphaRequest) -> Result<RankingPage, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await
            .map_err(|_| AlphaApiError::Incomplete("concurrency semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;

        let body = Self::serialize_rankings_body(req);
        let qparams = Self::build_qparams(&self.config.pagination, &req.continuation);
        let body = {
            let mut body = body;
            body["qParams"] = qparams;
            body
        };

        let route = &self.config.rankings_path;
        self.validate_route(route)?;
        let base = Url::parse(&self.config.base_url)
            .map_err(|e| AlphaApiError::Incomplete(format!("invalid base_url: {}", e)))?;
        let url = base.join(route).map_err(|e| AlphaApiError::Incomplete(format!("url join error: {}", e)))?;

        // allowed_fields is for RESPONSE filtering only, never outbound.
        let resp = self.execute_request(Method::POST, url, Some(&body)).await?;
        let text = resp.text().await.map_err(AlphaApiError::Request)?;

        // Parse then validate response against allowed_fields.
        let raw = RawRankingsResponse::from_json(&text)
            .map_err(|e| AlphaApiError::Incomplete(format!("JSON parse error: {}", e)))?;
        let validated_json = self.enforce_response_allowed_fields(raw.value);
        let validated_raw = RawRankingsResponse {
            grouped_rankings: raw.grouped_rankings,
            page: raw.page,
            complete: raw.complete,
            continuation: raw.continuation,
            value: validated_json,
        };

        // RFC 6901 pointer-based completeness check with fail-closed.
        let complete = self.check_completeness(&validated_raw)?;

        Ok(RankingPage {
            records: self.parse_rankings(&validated_raw),
            complete,            continuation: validated_raw.continuation.map(|c| serde_json::json!({ "page": c.page, "complete": c.complete })),
        })
    }

    /// Build nav_info URL using base.join(route) with query params.
    pub async fn nav_info(&self, season_id: i32, indoor: bool) -> Result<RawNavInfoResponse, AlphaApiError> {
        let _permit = self.concurrency_semaphore.acquire().await
            .map_err(|_| AlphaApiError::Incomplete("concurrency semaphore closed".into()))?;
        tokio::time::sleep(Duration::from_millis(self.config.min_delay_ms)).await;

        let route = &self.config.nav_info_path;
        self.validate_route(route)?;
        let base = Url::parse(&self.config.base_url)
            .map_err(|e| AlphaApiError::Incomplete(format!("invalid base_url: {}", e)))?;
        let url = base.join(route).map_err(|e| AlphaApiError::Incomplete(format!("url join error: {}", e)))?;
        let mut url = url;
        url.query_pairs_mut()
            .append_pair("season_id", &season_id.to_string())
            .append_pair("indoor", &indoor.to_string());
        let resp = self.execute_request(Method::GET, url, None).await?;
        let text = resp.text().await.map_err(AlphaApiError::Request)?;
        serde_json::from_str(&text).map_err(|e| AlphaApiError::Incomplete(format!("JSON parse error: {}", e)))
    }

    /// Enforce allowed_fields on the **response** body only, never on
    /// the outbound protocol request body.
    /// Only filters records inside groupedRankings, preserving the
    /// response envelope (complete, page, continuation, etc.).
    fn enforce_response_allowed_fields(&self, value: serde_json::Value) -> serde_json::Value {
        let allowed = &self.config.allowed_fields;
        if allowed.is_empty() { return value; }
        let mut obj = value.as_object().cloned().unwrap_or_default();
        // Only filter groupedRankings records.
        if let Some(groups) = obj.get_mut("groupedRankings").and_then(|v| v.as_array_mut()) {
            for group in groups {
                if let Some(records) = group.as_array_mut() {
                    for rec in records {
                        if let Some(obj) = rec.as_object_mut() {
                            obj.retain(|key, _| allowed.iter().any(|a| a == key));
                        }
                    }
                }
            }
        }
        serde_json::Value::Object(obj)
    }

    /// Walk a JSON pointer using serde_json::Value::pointer (RFC 6901).
    pub fn walk_pointer_value<'a>(value: &'a serde_json::Value, pointer: &str) -> Option<&'a serde_json::Value> {
        value.pointer(pointer)
    }
}
