use super::{BrowserError, BrowserResponse};
use crate::runtime::protocol::{RankingPageObservation, RankingsCapture};
use crate::runtime::source::request::RankingsAction;
use base64::Engine;
use chromiumoxide::{
    cdp::js_protocol::runtime::{RemoteObjectSubtype, RemoteObjectType},
    js::EvaluationResult,
    Page,
};
use futures::{StreamExt, TryStreamExt};
use reqwest::header::HeaderMap;
use std::time::Duration;
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

/// Map a CDP failure to the coarse transport error the operator contract
/// exposes, keeping the underlying cause in the log.
pub(super) fn transport<T, E: std::fmt::Display>(
    result: Result<T, E>,
    stage: &'static str,
) -> Result<T, BrowserError> {
    result.map_err(|error| {
        tracing::warn!(stage, "browser transport failure: {error}");
        BrowserError::Transport
    })
}

/// Validate captured data and build BrowserResponse.
pub(super) fn build_response(captured: CapturedRanking) -> Result<BrowserResponse, BrowserError> {
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

/// Build the navigation URL the source serves unchanged:
/// `/TrackAndField/rankings/list/{list_id}/{gender}/{event}?grades=N`.
///
/// The page depth travels in the API payload (`qParams.page`), never in this
/// landing URL. The source canonicalises a trailing slash and a `page` query
/// away, and a canonicalising navigation makes Chromium abort the original
/// document, which the navigation layer must not read as a transport failure.
pub(super) fn build_ui_url(
    source_origin: &url::Url,
    action: &RankingsAction,
) -> Result<String, BrowserError> {
    let mut builder = source_origin
        .join(&format!(
            "/TrackAndField/rankings/list/{}/{}/{}",
            action.list_id, action.gender, action.event_short
        ))
        .map_err(|_| BrowserError::Protocol)?;
    if let Some(grade) = action.grade {
        builder
            .query_pairs_mut()
            .append_pair("grades", &grade.to_string());
    }
    Ok(builder.to_string())
}
#[derive(serde::Deserialize)]
struct RankingRequest<'a> {
    #[serde(rename = "qParams")]
    q_params: RankingQueryParams,
    #[serde(rename = "divListId")]
    div_list_id: u64,
    #[serde(rename = "eventShort")]
    event_short: &'a str,
    gender: &'a str,
}

#[derive(serde::Deserialize)]
struct RankingQueryParams {
    page: Option<u32>,
    grades: Vec<u64>,
}

#[derive(serde::Deserialize)]
struct RankingsEnvelope {
    #[serde(rename = "groupedRankings")]
    grouped_rankings: Vec<Vec<serde::de::IgnoredAny>>,
    #[serde(default)]
    settings: Option<EnvelopeSettings>,
    #[serde(rename = "defaultSettings", default)]
    default_settings: Option<EnvelopeSettings>,
}

/// The page-depth fields the source declares for a list response. Rows stay
/// `IgnoredAny`; only the declared depth is materialized.
#[derive(serde::Deserialize)]
struct EnvelopeSettings {
    depth: Option<u64>,
}

impl RankingsEnvelope {
    fn row_count(&self) -> u64 {
        self.grouped_rankings
            .iter()
            .map(|group| group.len() as u64)
            .sum()
    }

    /// The declared page depth: `settings.depth` when the request echoed its
    /// settings, otherwise the default settings echoed alongside.
    fn declared_page_depth(&self) -> Option<u64> {
        self.settings
            .as_ref()
            .and_then(|settings| settings.depth)
            .or_else(|| {
                self.default_settings
                    .as_ref()
                    .and_then(|settings| settings.depth)
            })
    }
}

/// Parse and validate the captured request body once, returning its validated page.
pub(super) fn validate_request(
    captured: &CapturedRanking,
    action: &RankingsAction,
    allow_page_one: bool,
) -> Result<Option<u32>, BrowserError> {
    if action.capture == RankingsCapture::Navigation {
        return Ok(None);
    }
    let body = captured
        .request_body
        .as_deref()
        .ok_or(BrowserError::Protocol)?;
    let request: RankingRequest<'_> =
        serde_json::from_str(body).map_err(|_| BrowserError::Protocol)?;
    let Some(request_page) = request.q_params.page else {
        return Ok(None);
    };
    let page_matches =
        request_page == action.page || (allow_page_one && action.page > 1 && request_page == 1);
    if !page_matches {
        return Ok(None);
    }
    let grades_match = match action.grade {
        Some(grade) => {
            request.q_params.grades.len() == 1
                && request.q_params.grades.first() == Some(&u64::from(grade))
        }
        None => request.q_params.grades.is_empty(),
    };
    if !grades_match
        || request.div_list_id != action.list_id
        || request.event_short != action.event_short
        || request.gender != action.gender
    {
        return Ok(None);
    }
    Ok(Some(request_page))
}

/// The row count and declared page depth of a captured list response, used to
/// decide whether another page can exist. A body that is not a rankings
/// envelope is a protocol error for the caller to treat as terminal.
pub(super) fn page_extent(body: &[u8]) -> Result<(u64, Option<u64>), BrowserError> {
    let envelope: RankingsEnvelope =
        serde_json::from_slice(body).map_err(|_| BrowserError::Protocol)?;
    Ok((envelope.row_count(), envelope.declared_page_depth()))
}

async fn active_page(page: &Page) -> Result<Option<u32>, BrowserError> {
    let result = transport(
        page.evaluate(
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
        .await,
        "active_page",
    )?;
    decode_page_value(result)
}

fn decode_page_value(result: EvaluationResult) -> Result<Option<u32>, BrowserError> {
    let object = result.object();
    match (&object.r#type, &object.subtype) {
        (RemoteObjectType::Object, Some(RemoteObjectSubtype::Null)) if object.value.is_none() => {
            Ok(None)
        }
        (RemoteObjectType::Number, None) => {
            let page = result
                .into_value::<u32>()
                .map_err(|_| BrowserError::Protocol)?;
            if page == 0 {
                return Err(BrowserError::Protocol);
            }
            Ok(Some(page))
        }
        _ => Err(BrowserError::Protocol),
    }
}

const MAX_ACTIVE_PAGE_POLLS: usize = 256;

pub(super) async fn wait_for_active_page(
    page: &Page,
    requested_page: u32,
    deadline: tokio::time::Instant,
) -> Result<u32, BrowserError> {
    let polls = futures::stream::iter(0..MAX_ACTIVE_PAGE_POLLS)
        .then(|_| async {
            match active_page(page).await? {
                Some(page_number) if page_number == requested_page => Ok(Some(page_number)),
                _ => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(None)
                }
            }
        })
        .try_filter_map(|candidate| async move { Ok(candidate) });
    futures::pin_mut!(polls);
    tokio::time::timeout_at(deadline, polls.next())
        .await
        .map_err(|_| BrowserError::Timeout)?
        .transpose()?
        .ok_or(BrowserError::Timeout)
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
    transport(page.evaluate(&*js).await, "click_numeric_page")?
        .into_value::<bool>()
        .map_err(|_| BrowserError::Protocol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromiumoxide::cdp::js_protocol::runtime::RemoteObject;
    use serde_json::json;

    #[test]
    fn terminal_page_null_is_absent_not_protocol_failure() -> anyhow::Result<()> {
        let raw = json!({"type": "object", "subtype": "null", "value": null});
        let object: RemoteObject = serde_json::from_value(raw)?;
        assert_eq!(decode_page_value(EvaluationResult::new(object))?, None);
        let object = serde_json::from_value(json!({"type": "number", "value": 2}))?;
        assert_eq!(decode_page_value(EvaluationResult::new(object))?, Some(2));
        Ok(())
    }

    #[test]
    fn invalid_page_values_do_not_become_terminal_pages() -> anyhow::Result<()> {
        for raw in [
            json!({"type": "undefined"}),
            json!({"type": "number", "value": 0}),
            json!({"type": "number", "value": -1}),
            json!({"type": "number", "value": 1.5}),
            json!({"type": "number", "value": 4294967296_u64}),
            json!({"type": "string", "value": "2"}),
        ] {
            let object = serde_json::from_value(raw)?;
            assert!(matches!(
                decode_page_value(EvaluationResult::new(object)),
                Err(BrowserError::Protocol)
            ));
        }
        Ok(())
    }

    #[test]
    fn navigation_url_is_already_canonical() -> anyhow::Result<()> {
        let origin = url::Url::parse("https://www.athletic.net")?;
        let action = RankingsAction {
            list_id: 173_005,
            gender: "m".to_owned(),
            grade: Some(11),
            event_short: "55m".to_owned(),
            page: 3,
            capture: RankingsCapture::Navigation,
        };
        // The source strips a trailing slash and the `page` query, and a
        // canonicalising navigation aborts its own first document.
        assert_eq!(
            build_ui_url(&origin, &action)?,
            "https://www.athletic.net/TrackAndField/rankings/list/173005/m/55m?grades=11"
        );
        let ungraded = RankingsAction {
            grade: None,
            ..action
        };
        assert_eq!(
            build_ui_url(&origin, &ungraded)?,
            "https://www.athletic.net/TrackAndField/rankings/list/173005/m/55m"
        );
        Ok(())
    }
}
