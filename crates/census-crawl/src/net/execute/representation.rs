use crate::net::representation::{MAX_REPRESENTATION_HEADERS, MAX_REPRESENTATION_VALUE_BYTES};
use crate::net::request::RequestBody;
use crate::net::{FetchError, RepresentationHeaders};

pub(super) fn is_validator(name: &str) -> bool {
    name.eq_ignore_ascii_case("if-none-match") || name.eq_ignore_ascii_case("if-modified-since")
}

pub(super) fn canonical_request(
    headers: &[(String, String)],
) -> Result<RepresentationHeaders, FetchError> {
    if headers.len() > MAX_REPRESENTATION_HEADERS {
        return Err(FetchError::Policy {
            detail: format!("request exceeds {MAX_REPRESENTATION_HEADERS} caller headers"),
        });
    }
    if headers.iter().all(|(name, _)| !is_validator(name)) {
        return RepresentationHeaders::canonical(headers);
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
    RepresentationHeaders::canonical(&selected)
}

pub(super) fn cache_extra(
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
