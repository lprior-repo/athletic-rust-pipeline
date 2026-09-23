use crate::protocol::{RankingPageObservation, RankingsCapture};
use crate::{BrowserError, BrowserResponse};
use base64::Engine;
use reqwest::header::HeaderMap;
/// Parse the binding event payload into a CapturedRanking.
pub(crate) fn parse_binding(
    data: &serde_json::Value,
    capture_kind: RankingsCapture,
) -> Result<CapturedRanking, BrowserError> {
    // Parse error BEFORE required success fields.
    if let Some(err) = data.get("error").and_then(|value| value.as_str()) {
        return Err(binding_error(err));
    }

    // Required success fields.
    let status = binding_status(data)?;
    let request_url = required_str(data, "requestUrl")?.to_string();
    let method = required_str(data, "method")?.to_string();
    let request_body = data.get("requestBody").and_then(|value| value.as_str());
    if request_body.is_some_and(|body| body.len() > 64 * 1024) {
        return Err(BrowserError::PayloadLimit);
    }

    let (body, body_bytes) = binding_body(data)?;

    // Collect allowed response headers.
    let headers = binding_headers(data)?;

    // Challenge flag from cf-mitigated header.
    let challenge = headers.get("cf-mitigated").is_some();
    Ok(CapturedRanking {
        status,
        body,
        body_bytes,
        method,
        request_url,
        request_body: request_body.map(str::to_owned),
        challenge,
        headers,
        capture_kind,
    })
}

/// Map the interceptor's `error` field onto the error the operator contract exposes.
fn binding_error(err: &str) -> BrowserError {
    match err {
        "payload_limit" => BrowserError::PayloadLimit,
        "fetch_failed"
        | "capture_failed"
        | "unsupported_request_body"
        | "request_payload_limit" => BrowserError::Transport,
        _other => BrowserError::Protocol,
    }
}

/// Read one required string field of the binding payload.
fn required_str<'a>(data: &'a serde_json::Value, key: &str) -> Result<&'a str, BrowserError> {
    data.get(key)
        .and_then(|value| value.as_str())
        .ok_or(BrowserError::Protocol)
}

/// Read the required response status.
fn binding_status(data: &serde_json::Value) -> Result<u16, BrowserError> {
    data.get("status")
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(BrowserError::Protocol)
}

/// Decode the required body plus its declared byte count, enforcing both limits.
fn binding_body(data: &serde_json::Value) -> Result<(Vec<u8>, usize), BrowserError> {
    let body_str = required_str(data, "body")?;
    let body_bytes = data
        .get("bodyBytes")
        .and_then(|value| value.as_u64())
        .ok_or(BrowserError::Protocol)
        .and_then(|value| usize::try_from(value).map_err(|_| BrowserError::PayloadLimit))?;
    if body_bytes > 8 * 1024 * 1024 {
        return Err(BrowserError::PayloadLimit);
    }
    if body_str.len() > 11_184_812 {
        return Err(BrowserError::PayloadLimit);
    }
    let body = base64::engine::general_purpose::STANDARD
        .decode(body_str)
        .map_err(|_| BrowserError::Protocol)?;
    Ok((body, body_bytes))
}

/// Collect the allowed response headers, rejecting malformed names and values.
fn binding_headers(data: &serde_json::Value) -> Result<HeaderMap, BrowserError> {
    let Some(values) = data.get("headers").and_then(|value| value.as_object()) else {
        return Err(BrowserError::Protocol);
    };
    values
        .iter()
        .try_fold(HeaderMap::new(), |mut headers, (name, value)| {
            if let Some(value) = value.as_str() {
                let key = name
                    .parse::<reqwest::header::HeaderName>()
                    .map_err(|_| BrowserError::Protocol)?;
                let header = value
                    .parse::<reqwest::header::HeaderValue>()
                    .map_err(|_| BrowserError::Protocol)?;
                headers.insert(key, header);
            }
            Ok::<_, BrowserError>(headers)
        })
}
#[derive(Debug)]
pub(crate) struct CapturedRanking {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
    pub(crate) body_bytes: usize,
    pub(crate) method: String,
    pub(crate) request_url: String,
    pub(crate) request_body: Option<String>,
    pub(crate) challenge: bool,
    pub(crate) headers: HeaderMap,
    pub(crate) capture_kind: RankingsCapture,
}

/// Map a CDP failure to the coarse transport error the operator contract
/// exposes, keeping the underlying cause in the log.
pub(crate) fn transport<T, E: std::fmt::Display>(
    result: Result<T, E>,
    stage: &'static str,
) -> Result<T, BrowserError> {
    result.map_err(|error| {
        tracing::warn!(stage, "browser transport failure: {error}");
        BrowserError::Transport
    })
}

/// Validate captured data and build BrowserResponse.
pub(crate) fn build_response(captured: CapturedRanking) -> Result<BrowserResponse, BrowserError> {
    // Require captured headers.
    if captured.headers.is_empty() {
        return Err(BrowserError::Protocol);
    }
    // Verify decoded length == bodyBytes and <= 8MiB.
    if captured.body.len() != captured.body_bytes {
        return Err(BrowserError::Protocol);
    }
    if captured.body_bytes > 8 * 1024 * 1024 {
        return Err(BrowserError::PayloadLimit);
    }
    let body = captured.body;
    Ok(BrowserResponse {
        status: reqwest::StatusCode::from_u16(captured.status)
            .map_err(|_| BrowserError::Protocol)?,
        headers: captured.headers,
        body,
        rankings: Some(RankingPageObservation {
            capture: captured.capture_kind,
            request_method: captured.method,
            request_url: captured.request_url,
            request_body: captured.request_body,
            next_page: None,
        }),
    })
}
