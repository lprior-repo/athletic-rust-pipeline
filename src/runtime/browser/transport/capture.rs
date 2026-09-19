use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived, GetRequestPostDataParams, GetResponseBodyParams,
    RequestId,
};
use chromiumoxide::Page;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::BrowserError;
use crate::runtime::protocol::MAX_SOURCE_RESPONSE_BYTES;

pub(in crate::runtime::browser) fn response_headers(
    event: &EventResponseReceived,
) -> Result<HeaderMap, BrowserError> {
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
pub(in crate::runtime::browser) async fn capture_body(
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
    let max_encoded = MAX_SOURCE_RESPONSE_BYTES.div_ceil(3) * 4;
    if encoded.len() > max_encoded {
        return Err(BrowserError::PayloadLimit);
    }
    if !encoded.len().is_multiple_of(4) {
        return Err(BrowserError::Protocol);
    }
    let padding = encoded
        .bytes()
        .rev()
        .take(2)
        .filter(|byte| *byte == b'=')
        .count();
    let decoded_len = (encoded.len() / 4) * 3 - padding;
    if decoded_len > MAX_SOURCE_RESPONSE_BYTES {
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
