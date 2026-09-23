use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived, GetRequestPostDataParams, GetResponseBodyParams,
    RequestId,
};
use chromiumoxide::Page;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::BrowserError;
use crate::protocol::MAX_SOURCE_RESPONSE_BYTES;

pub(crate) fn response_headers(event: &EventResponseReceived) -> Result<HeaderMap, BrowserError> {
    let mut headers = HeaderMap::new();
    let Some(values) = event.response.headers.inner().as_object() else {
        return Err(BrowserError::Protocol);
    };
    values.iter().try_for_each(|(name, value)| {
        let value = value.as_str().ok_or(BrowserError::Protocol)?;
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| BrowserError::Protocol)?;
        value.split('\n').try_for_each(|value| {
            let value = HeaderValue::from_str(value.trim_end_matches('\r'))
                .map_err(|_| BrowserError::Protocol)?;
            headers.append(name.clone(), value);
            Ok::<(), BrowserError>(())
        })?;
        Ok::<(), BrowserError>(())
    })?;
    Ok(headers)
}

/// Charset-decoded CDP text is used only for navigation classification.
/// Retained source evidence uses the Fetch byte stream instead.
pub(crate) async fn capture_body(
    page: &Page,
    request_id: RequestId,
) -> Result<Vec<u8>, BrowserError> {
    let result = page
        .execute(GetResponseBodyParams::new(request_id))
        .await
        .map_err(|_| BrowserError::Transport)?
        .result;
    if result.base64_encoded {
        decode_body(&result.body)
    } else {
        if result.body.len() > MAX_SOURCE_RESPONSE_BYTES {
            return Err(BrowserError::PayloadLimit);
        }
        Ok(result.body.into_bytes())
    }
}

pub(super) fn decode_body(encoded: &str) -> Result<Vec<u8>, BrowserError> {
    let encoded_len = match u64::try_from(encoded.len()) {
        Ok(value) => value,
        Err(_) => return Err(BrowserError::Protocol),
    };
    // Base64 expands by 4/3, so the largest admissible body is the ceiling.
    let max_encoded = match u64::try_from(MAX_SOURCE_RESPONSE_BYTES)
        .ok()
        .and_then(|value| value.checked_add(2))
        .and_then(|value| value.checked_div(3))
        .and_then(|value| value.checked_mul(4))
    {
        Some(value) => value,
        None => return Err(BrowserError::Protocol),
    };
    if encoded_len > max_encoded {
        return Err(BrowserError::PayloadLimit);
    }
    if !encoded_len.is_multiple_of(4) {
        return Err(BrowserError::Protocol);
    }
    let padding = match u64::try_from(
        encoded
            .bytes()
            .rev()
            .take(2)
            .filter(|byte| *byte == b'=')
            .count(),
    ) {
        Ok(value) => value,
        Err(_) => return Err(BrowserError::Protocol),
    };
    let decoded_len = match encoded_len
        .checked_div(4)
        .and_then(|value| value.checked_mul(3))
        .and_then(|value| value.checked_sub(padding))
    {
        Some(value) => value,
        None => return Err(BrowserError::Protocol),
    };
    let max_decoded = match u64::try_from(MAX_SOURCE_RESPONSE_BYTES) {
        Ok(value) => value,
        Err(_) => return Err(BrowserError::Protocol),
    };
    if decoded_len > max_decoded {
        return Err(BrowserError::PayloadLimit);
    }
    BASE64_STANDARD
        .decode(encoded.as_bytes())
        .map_err(|_| BrowserError::Protocol)
}

pub(super) async fn matches_request_body(
    page: &Page,
    event: &EventRequestWillBeSent,
    expected: Option<&str>,
) -> Result<bool, BrowserError> {
    let Some(expected) = expected else {
        return Ok(event.request.has_post_data != Some(true));
    };
    let actual = page
        .execute(GetRequestPostDataParams::new(event.request_id.clone()))
        .await
        .map_err(|_| BrowserError::Transport)?
        .result
        .post_data;
    Ok(actual == expected)
}
