use std::time::Duration;
use base64::Engine;
use chromiumoxide::Page;
use super::{BrowserError, BrowserResponse};
use crate::runtime::protocol::{RankingPageObservation, RankingsCapture};
use crate::runtime::source::request::RankingsAction;
use reqwest::header::HeaderMap;
/// Verified browser-realm interceptor prototype.
/// Replaces __RANKINGS_CAPTURE_CONFIG__ with serde_json-serialized config.
const INTERCEPTOR_TEMPLATE: &str = r#"(() => {
  const config = __RANKINGS_CAPTURE_CONFIG__;
  if (window !== window.top) return;
  window.__RANKINGS_ORIGINAL_FETCH = window.fetch;
  const originalFetch = window.fetch;
  const responseLimit = 8 * 1024 * 1024;
  const requestLimit = 64 * 1024;
  const failure = code => Object.assign(new Error(code), { captureCode: code });
  async function readBounded(stream, limit) {
    if (!stream) return new Uint8Array(0);
    const reader = stream.getReader();
    let buffer = new Uint8Array(Math.min(limit, 65536));
    let length = 0;
    try {
      for (;;) {
        const part = await reader.read();
        if (part.done) return buffer.subarray(0, length);
        const needed = length + part.value.byteLength;
        if (needed > limit) {
          void reader.cancel().catch(() => {});
          throw failure('payload_limit');
        }
        if (needed > buffer.byteLength) {
          const expanded = new Uint8Array(Math.max(needed, Math.min(limit, buffer.byteLength * 2)));
          expanded.set(buffer.subarray(0, length));
          buffer = expanded;
        }
        buffer.set(part.value, length);
        length = needed;
      }
    } finally {
      reader.releaseLock();
    }
  }
  function base64(bytes) {
    const chunks = [];
    const stride = 24576;
    for (let offset = 0; offset < bytes.byteLength; offset += stride) {
      chunks.push(btoa(String.fromCharCode(...bytes.subarray(offset, offset + stride))));
    }
    return chunks.join('');
  }
  function describe(input, init) {
    const rawUrl = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
    const url = new URL(rawUrl, location.href);
    if (url.origin !== config.origin) return null;
    const kind = url.pathname === '/api/v1/tfRankings/GetNavInfo' ? 'navigation'
      : url.pathname === '/api/v1/tfRankings/GetRankings' ? 'results' : null;
    if (!kind) return null;
    const method = String(init?.method ?? input?.method ?? 'GET').toUpperCase();
    let bodyInput;
    let requestClone;
    if (method !== 'GET' && method !== 'HEAD') {
      if (init && Object.prototype.hasOwnProperty.call(init, 'body')) bodyInput = init.body;
      else if (input instanceof Request) requestClone = input.clone();
    }
    return { kind, url: url.toString(), method, bodyInput, requestClone };
  }
  async function requestBody(description) {
    if (description.requestClone) {
      return new TextDecoder('utf-8', { fatal: true }).decode(await readBounded(description.requestClone.body, requestLimit));
    }
    if (description.bodyInput === undefined || description.bodyInput === null) return null;
    if (typeof description.bodyInput !== 'string') throw failure('unsupported_request_body');
    if (description.bodyInput.length > requestLimit || new TextEncoder().encode(description.bodyInput).byteLength > requestLimit) throw failure('request_payload_limit');
    return description.bodyInput;
  }
  function publish(description, fields) {
    const binding = window[config.binding];
    if (typeof binding !== 'function') return;
    binding(JSON.stringify({ nonce: config.nonce, kind: description.kind, requestUrl: description.url, method: description.method, ...fields }));
  }
  window.fetch = function(input, init) {
    let description;
    try { description = describe(input, init); } catch { description = null; }
    const promise = Reflect.apply(originalFetch, this, arguments);
    if (!description) return promise;
    void promise.then(async response => {
      const headers = {};
      for (const name of ['content-type', 'cf-mitigated', 'retry-after', 'location']) {
        const value = response.headers.get(name);
        if (value !== null) headers[name] = value;
      }
      const metadata = { status: response.status, headers };
      try {
        const retained = response.clone();
        const body = await requestBody(description);
        const bytes = await readBounded(retained.body, responseLimit);
        publish(description, { ...metadata, requestBody: body, body: base64(bytes), bodyBytes: bytes.byteLength });
      } catch (error) {
        publish(description, { ...metadata, error: error?.captureCode ?? 'capture_failed' });
      }
    }, () => publish(description, { error: 'fetch_failed' })).catch(() => {});
    return promise;
  };
})();"#;

/// Interceptor config serialized as JSON for the JS prototype.
#[derive(serde::Serialize)]
struct InterceptorConfig<'a> {
    origin: &'a str,
    binding: &'static str,
    nonce: u64,
}
pub(super) const BINDING_NAME: &str = "retainRankingResponse";

/// Build the interceptor script with serialized config.
pub(super) fn build_interceptor_script(origin: &str, nonce: u64) -> Result<String, BrowserError> {
    let config = InterceptorConfig {
        origin,
        binding: BINDING_NAME,
        nonce,
    };
    let json = serde_json::to_string(&config).map_err(|_| BrowserError::Protocol)?;
    let script = INTERCEPTOR_TEMPLATE.replace("__RANKINGS_CAPTURE_CONFIG__", &json);
    Ok(script)
}

/// Parse the binding event payload into a CapturedRanking.
pub(super) fn parse_binding(
    data: &serde_json::Value,
    capture_kind: RankingsCapture,
) -> Result<CapturedRanking, BrowserError> {

    // Parse error BEFORE required success fields.
    if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
        return match err {
            "payload_limit" => Err(BrowserError::PayloadLimit),
            "fetch_failed" | "capture_failed" | "unsupported_request_body" | "request_payload_limit" => {
                Err(BrowserError::Transport)
            }
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

    let request_body = data
        .get("requestBody")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

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
    let headers = match data
        .get("headers")
        .and_then(|v| v.as_object())
    {
        Some(h) => h.iter().try_fold(HeaderMap::new(), |mut hm, (k, val)| {
            if let Some(s) = val.as_str() {
                let key = k.parse::<reqwest::header::HeaderName>()
                    .map_err(|_| BrowserError::Protocol)?;
                let value = s.parse::<reqwest::header::HeaderValue>()
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
        request_body,
        challenge,
        headers,
        capture_kind,
    })
}
#[derive(Debug)]
pub(super) struct CapturedRanking {
    pub(super) status: u16,
    pub(super) body: Vec<u8>,
    pub(super) body_bytes: usize,
    pub(super) method: String,
    pub(super) request_url: String,
    pub(super) request_body: Option<String>,
    pub(super) challenge: bool,
    pub(super) headers: HeaderMap,
    pub(super) capture_kind: RankingsCapture,
}

/// Validate captured data and build BrowserResponse.
pub(super) fn build_response(
    captured: CapturedRanking,
) -> Result<BrowserResponse, BrowserError> {
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

/// Build the UI URL with trailing slash: /.../list/{list_id}/{gender}/{event}/?page=N
pub(super) fn build_ui_url(source_origin: &url::Url, action: &RankingsAction) -> Result<String, BrowserError> {
    let mut builder = source_origin
        .join(&format!(
            "/TrackAndField/rankings/list/{}/{}/{}/",
            action.list_id, action.gender, action.event_short
        ))
        .map_err(|_| BrowserError::Protocol)?;
    builder.query_pairs_mut().append_pair("page", &action.page.to_string());
    if let Some(grade) = action.grade {
        builder.query_pairs_mut().append_pair("grades", &grade.to_string());
    }
    Ok(builder.to_string())
}
/// Validate the request body against the requested rankings scope.
pub(super) fn validate_request_scope(
    captured: &CapturedRanking,
    action: &RankingsAction,
    allow_page_one: bool,
) -> Result<bool, BrowserError> {
    if action.capture == RankingsCapture::Navigation {
        return Ok(captured.request_body.is_none());
    }
    let body = captured.request_body.as_deref().ok_or(BrowserError::Protocol)?;
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| BrowserError::Protocol)?;
    let qparams = value
        .get("qParams")
        .and_then(serde_json::Value::as_object)
        .ok_or(BrowserError::Protocol)?;
    let page = qparams
        .get("page")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let Some(request_page) = page else {
        return Ok(false);
    };
    let page_matches = request_page == action.page
        || (allow_page_one && action.page > 1 && request_page == 1);
    let grades = qparams
        .get("grades")
        .and_then(serde_json::Value::as_array)
        .ok_or(BrowserError::Protocol)?;
    let grades_match = match action.grade {
        Some(grade) => {
            grades.len() == 1
                && grades
                    .first()
                    .and_then(serde_json::Value::as_u64)
                    == Some(u64::from(grade))
        }
        None => grades.is_empty(),
    };
    Ok(page_matches
        && grades_match
        && value.get("divListId").and_then(serde_json::Value::as_u64) == Some(action.list_id)
        && value.get("eventShort").and_then(serde_json::Value::as_str)
            == Some(action.event_short.as_str())
        && value.get("gender").and_then(serde_json::Value::as_str)
            == Some(action.gender.as_str()))
}

pub(super) fn response_has_rows(body: &[u8]) -> Result<bool, BrowserError> {
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| BrowserError::Protocol)?;
    let groups = value
        .get("groupedRankings")
        .and_then(serde_json::Value::as_array)
        .ok_or(BrowserError::Protocol)?;
    Ok(groups.iter().any(|group| {
        group
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    }))
}

pub(super) fn request_page(captured: &CapturedRanking) -> Result<Option<u32>, BrowserError> {
    let Some(body) = captured.request_body.as_deref() else {
        return Ok(None);
    };
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| BrowserError::Protocol)?;
    let page = value
        .get("qParams")
        .and_then(serde_json::Value::as_object)
        .and_then(|params| params.get("page"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    Ok(page)
}

async fn active_page(page: &Page) -> Result<Option<u32>, BrowserError> {
    let result = page
        .evaluate(
            r#"(() => {
                const pagination = document.querySelector('.pagination');
                if (!pagination) return null;
                const link = pagination.querySelector(
                    '.page-item.active .page-link, [aria-current="page"], [aria-current="page"] .page-link'
                );
                if (!link) return null;
                const text = link.textContent.trim();
                const value = Number(text);
                return Number.isInteger(value) && value > 0 ? value : null;
            })()"#,
        )
        .await
        .map_err(|_| BrowserError::Transport)?;
    result
        .into_value::<Option<u32>>()
        .map_err(|_| BrowserError::Protocol)
}

pub(super) async fn wait_for_active_page(
    page: &Page,
    deadline: tokio::time::Instant,
) -> Result<u32, BrowserError> {
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(BrowserError::Timeout);
        }
        if let Some(page_number) = active_page(page).await? {
            return Ok(page_number);
        }
        let pause = remaining.min(Duration::from_millis(50));
        tokio::time::timeout_at(deadline, tokio::time::sleep(pause))
            .await
            .map_err(|_| BrowserError::Timeout)?;
    }
}

pub(super) async fn click_numeric_page(
    page: &Page,
    requested_page: u32,
) -> Result<bool, BrowserError> {
    let js = format!(
        r#"(() => {{
            const pagination = document.querySelector('.pagination');
            if (!pagination) return false;
            const links = pagination.querySelectorAll('.page-link');
            for (const link of links) {{
                const text = link.textContent.trim();
                const value = Number(text);
                if (Number.isInteger(value) && value === {requested_page}) {{
                    const parent = link.closest('.page-item');
                    if (!parent || parent.classList.contains('disabled')
                        || link.getAttribute('aria-disabled') === 'true') return false;
                    link.click();
                    return true;
                }}
            }}
            return false;
        }})()"#,
        requested_page = requested_page,
    );
    page.evaluate(&js[..])
        .await
        .map_err(|_| BrowserError::Transport)?
        .into_value::<bool>()
        .map_err(|_| BrowserError::Protocol)
}

/// Extract next page after the requested numeric page has rendered.
pub(super) async fn extract_next_page(
    page: &Page,
    current_page: u32,
    deadline: tokio::time::Instant,
    has_rows: bool,
) -> Result<Option<u32>, BrowserError> {
    if !has_rows {
        return Ok(None);
    }
    let active = wait_for_active_page(page, deadline).await?;
    if active != current_page {
        return Err(BrowserError::Transport);
    }
    let next = current_page.checked_add(1).ok_or(BrowserError::Protocol)?;
    let js = format!(
        r#"(() => {{
            const pagination = document.querySelector('.pagination');
            if (!pagination) return null;
            for (const link of pagination.querySelectorAll('.page-link')) {{
                const text = link.textContent.trim();
                const value = Number(text);
                if (Number.isInteger(value) && value === {next}) {{
                    const parent = link.closest('.page-item');
                    if (parent && !parent.classList.contains('disabled')
                        && link.getAttribute('aria-disabled') !== 'true') return value;
                }}
            }}
            return null;
        }})()"#,
        next = next,
    );
    page.evaluate(&js[..])
        .await
        .map_err(|_| BrowserError::Transport)?
        .into_value::<Option<u32>>()
        .map_err(|_| BrowserError::Protocol)
}
