use crate::runtime::browser::{BrowserError, BrowserResponse};
use crate::runtime::protocol::{RankingPageObservation, RankingsCapture};
use base64::Engine;
use reqwest::header::HeaderMap;
/// Parse the binding event payload into a CapturedRanking.
pub(crate) fn parse_binding(
    data: &serde_json::Value,
    capture_kind: RankingsCapture,
) -> Result<CapturedRanking, BrowserError> {
    // Parse error BEFORE required success fields.
    if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
        return match err {
            "payload_limit" => Err(BrowserError::PayloadLimit),
            "fetch_failed"
            | "capture_failed"
            | "unsupported_request_body"
            | "request_payload_limit" => Err(BrowserError::Transport),
            _other => Err(BrowserError::Protocol),
        };
    }

    // Required success fields.
    let status = data
        .get("status")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .ok_or(BrowserError::Protocol)?;

    let request_url = data
        .get("requestUrl")
        .and_then(|v| v.as_str())
        .ok_or(BrowserError::Protocol)?
        .to_string();

    let method = data
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or(BrowserError::Protocol)?
        .to_string();

    let request_body = data.get("requestBody").and_then(|value| value.as_str());
    if request_body.is_some_and(|body| body.len() > 64 * 1024) {
        return Err(BrowserError::PayloadLimit);
    }

    let body_str = data
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or(BrowserError::Protocol)?;

    let body_bytes = data
        .get("bodyBytes")
        .and_then(|v| v.as_u64())
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

    // Collect allowed response headers.
    let headers = match data.get("headers").and_then(|v| v.as_object()) {
        Some(h) => h.iter().try_fold(HeaderMap::new(), |mut hm, (k, val)| {
            if let Some(s) = val.as_str() {
                let key = k
                    .parse::<reqwest::header::HeaderName>()
                    .map_err(|_| BrowserError::Protocol)?;
                let value = s
                    .parse::<reqwest::header::HeaderValue>()
                    .map_err(|_| BrowserError::Protocol)?;
                hm.insert(key, value);
            }
            Ok::<_, BrowserError>(hm)
        })?,
        None => return Err(BrowserError::Protocol),
    };

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
