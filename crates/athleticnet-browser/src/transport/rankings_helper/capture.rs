use crate::protocol::{RankingPageObservation, RankingsCapture};
use crate::{BrowserError, BrowserResponse};
use base64::Engine;
use reqwest::header::HeaderMap;
pub(crate) fn parse_binding(
    data: &serde_json::Value,
    capture_kind: RankingsCapture,
) -> Result<CapturedRanking, BrowserError> {
    if let Some(err) = data.get("error").and_then(|value| value.as_str()) {
        return Err(binding_error(err));
    }

    let status = binding_status(data)?;
    let request_url = required_str(data, "requestUrl")?.to_string();
    let response_url = required_str(data, "responseUrl")?.to_string();
    let method = required_str(data, "method")?.to_string();
    let request_body = data.get("requestBody").and_then(|value| value.as_str());
    if request_body.is_some_and(|body| body.len() > 64 * 1024) {
        return Err(BrowserError::PayloadLimit);
    }

    let (body, body_bytes) = binding_body(data)?;

    let headers = binding_headers(data)?;

    let challenge = headers.get("cf-mitigated").is_some();
    Ok(CapturedRanking {
        status,
        body,
        body_bytes,
        method,
        request_url,
        response_url,
        request_body: request_body.map(str::to_owned),
        challenge,
        headers,
        capture_kind,
    })
}

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

fn required_str<'a>(data: &'a serde_json::Value, key: &str) -> Result<&'a str, BrowserError> {
    data.get(key)
        .and_then(|value| value.as_str())
        .ok_or(BrowserError::Protocol)
}

fn binding_status(data: &serde_json::Value) -> Result<u16, BrowserError> {
    data.get("status")
        .and_then(|value| value.as_u64())
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(BrowserError::Protocol)
}

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
    pub(crate) response_url: String,
    pub(crate) request_body: Option<String>,
    pub(crate) challenge: bool,
    pub(crate) headers: HeaderMap,
    pub(crate) capture_kind: RankingsCapture,
}

pub(crate) fn transport<T, E: std::fmt::Display>(
    result: Result<T, E>,
    stage: &'static str,
) -> Result<T, BrowserError> {
    result.map_err(|error| {
        tracing::warn!(stage, "browser transport failure: {error}");
        BrowserError::Transport
    })
}

pub(crate) fn build_response(captured: CapturedRanking) -> Result<BrowserResponse, BrowserError> {
    if captured.headers.is_empty() {
        return Err(BrowserError::Protocol);
    }
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
        response_url: Some(captured.response_url),
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
