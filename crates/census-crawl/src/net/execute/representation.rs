use crate::net::representation::{MAX_REPRESENTATION_HEADERS, MAX_REPRESENTATION_VALUE_BYTES};
use crate::net::request::RequestBody;
use crate::net::{FetchError, RepresentationHeaders};

pub(in crate::net) fn observed_headers(
    headers: &[(String, String)],
    user_agent: &str,
) -> Result<Vec<(String, String)>, FetchError> {
    canonical_request("https://example.com/", headers)?;
    let mut headers = headers.to_vec();
    for (name, value) in [
        ("accept", "*/*"),
        ("user-agent", user_agent),
        ("accept-encoding", "identity"),
    ] {
        if !headers
            .iter()
            .any(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        {
            headers.push((name.to_owned(), value.to_owned()));
        }
    }
    canonical_request("https://example.com/", &headers)?;
    Ok(headers)
}

pub(in crate::net) fn materialize_observed(
    request: &mut reqwest::Request,
    user_agent: &str,
) -> Result<(), FetchError> {
    for (name, value) in [
        ("accept", "*/*"),
        ("user-agent", user_agent),
        ("accept-encoding", "identity"),
    ] {
        if !request.headers().contains_key(name) {
            let value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|_| crate::net::CaptureTransportError::RequestMismatch)?;
            request
                .headers_mut()
                .insert(reqwest::header::HeaderName::from_static(name), value);
        }
    }
    Ok(())
}

pub(super) fn is_validator(name: &str) -> bool {
    name.eq_ignore_ascii_case("if-none-match") || name.eq_ignore_ascii_case("if-modified-since")
}

pub(in crate::net) fn canonical_request(
    url: &str,
    headers: &[(String, String)],
) -> Result<RepresentationHeaders, FetchError> {
    if headers.len() > MAX_REPRESENTATION_HEADERS {
        return Err(FetchError::Policy {
            detail: format!("request exceeds {MAX_REPRESENTATION_HEADERS} caller headers"),
        });
    }
    if headers.iter().all(|(name, _)| !is_validator(name)) {
        return RepresentationHeaders::for_request(url, headers);
    }
    let mut selected = Vec::with_capacity(headers.len());
    for (name, value) in headers {
        if is_validator(name) {
            if value.len() > MAX_REPRESENTATION_VALUE_BYTES
                || reqwest::header::HeaderValue::from_str(value).is_err()
            {
                return Err(FetchError::Policy {
                    detail: format!(
                        "conditional validator {name} exceeds its bounded HTTP value contract"
                    ),
                });
            }
        } else {
            selected.push((name.clone(), value.clone()));
        }
    }
    RepresentationHeaders::for_request(url, &selected)
}

pub(in crate::net) fn cache_extra(
    body: Option<&(String, RequestBody)>,
    headers: &RepresentationHeaders,
) -> String {
    let body_key = body.map_or("", |(key, _)| key.as_str());
    if headers.is_empty() {
        return body_key.to_string();
    }
    let identity = headers.identity();
    if body.is_none() {
        return identity;
    }
    format!("{body_key}\u{1f}{identity}")
}
