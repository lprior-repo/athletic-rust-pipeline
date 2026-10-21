use crate::net::{CaptureTransportError, FetchError, FetchOutcome};
use census_domain::model::NormalizedObservedHttpV1;
use serde::Deserialize;

#[derive(Deserialize)]
struct Metadata {
    observed_http: Option<NormalizedObservedHttpV1>,
}

pub(super) fn observed(
    bytes: &[u8],
    capture: &FetchOutcome,
) -> Result<Option<NormalizedObservedHttpV1>, FetchError> {
    if bytes.len() > 64 * 1024 {
        return Err(CaptureTransportError::BackingUnavailable.into());
    }
    let metadata: Metadata =
        serde_json::from_slice(bytes).map_err(|source| FetchError::Decode {
            target: "retained observed HTTP metadata".into(),
            source,
        })?;
    let Some(observed) = metadata.observed_http else {
        return Ok(None);
    };
    let first = observed
        .hops()
        .first()
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    let last = observed
        .hops()
        .last()
        .ok_or(CaptureTransportError::BackingUnavailable)?;
    if capture.method != "GET"
        || capture.body.len() != capture.bytes
        || super::super::cache::content_digest(&capture.body) != capture.content_digest
        || observed.body_digest().as_str() != capture.content_digest
        || observed.body_bytes()
            != u64::try_from(capture.bytes)
                .map_err(|_| CaptureTransportError::BackingUnavailable)?
        || first.request().target_url() != capture.url
        || Some(last.response().response_url()) != capture.response_url.as_deref()
        || last.response().status() != capture.status
        || last.response().received_at() != capture.fetched_at
    {
        return Err(CaptureTransportError::BackingUnavailable.into());
    }
    Ok(Some(observed))
}

#[cfg(test)]
mod tests;
