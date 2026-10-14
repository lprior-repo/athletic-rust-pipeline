use crate::net::cache::PinnedEligibleCache;
use crate::net::{CaptureTransportError, FetchError};
use census_domain::model::{AcquisitionFailureV1, ConditionalRequestV1, ResponseObservationV1};

pub(super) fn conditional(
    request: &reqwest::Request,
    cached: Option<&PinnedEligibleCache>,
) -> Result<ConditionalRequestV1, FetchError> {
    let etag = request_header(request, reqwest::header::IF_NONE_MATCH)?;
    let last_modified = request_header(request, reqwest::header::IF_MODIFIED_SINCE)?;
    if etag.is_none() && last_modified.is_none() {
        return Ok(ConditionalRequestV1::Unconditional);
    }
    let pinned = cached.ok_or(CaptureTransportError::CacheOccurrenceMismatch)?;
    if etag
        .as_ref()
        .is_some_and(|value| Some(value) != pinned.envelope.metadata.etag.as_ref())
        || last_modified
            .as_ref()
            .is_some_and(|value| Some(value) != pinned.envelope.metadata.last_modified.as_ref())
    {
        return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
    }
    Ok(ConditionalRequestV1::Pinned {
        original: pinned.envelope.original_occurrence.clone(),
        etag,
        last_modified,
    })
}

fn request_header(
    request: &reqwest::Request,
    name: reqwest::header::HeaderName,
) -> Result<Option<String>, FetchError> {
    request
        .headers()
        .get(name)
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|_| CaptureTransportError::RequestMismatch.into())
        })
        .transpose()
}

pub(super) fn observation_of(
    response: &reqwest::Response,
    observed_at: &str,
) -> Result<ResponseObservationV1, FetchError> {
    let mut headers = Vec::new();
    for name in [
        reqwest::header::CONTENT_TYPE,
        reqwest::header::ETAG,
        reqwest::header::LAST_MODIFIED,
        reqwest::header::LOCATION,
        reqwest::header::RETRY_AFTER,
    ] {
        if let Some(value) = response.headers().get(&name) {
            headers.push((
                name.as_str().to_owned(),
                value
                    .to_str()
                    .map_err(|_| CaptureTransportError::RequestMismatch)?
                    .to_owned(),
            ));
        }
    }
    headers.sort();
    Ok(ResponseObservationV1::new(
        response.status().as_u16(),
        response.url().as_str(),
        observed_at,
        headers,
    )?)
}

pub(super) fn failure_of(error: &FetchError) -> AcquisitionFailureV1 {
    match error {
        FetchError::RecoveryCleanup(error) => match error.primary() {
            crate::net::RecoveryFailurePrimary::Capture(
                CaptureTransportError::CacheOccurrenceMismatch,
            ) => AcquisitionFailureV1::CacheChanged,
            _ => AcquisitionFailureV1::AdmissionRefused,
        },
        FetchError::Timeout { .. } => AcquisitionFailureV1::Timeout,
        FetchError::Transport { .. } => AcquisitionFailureV1::Transport,
        FetchError::TooLarge { .. } => AcquisitionFailureV1::BodyTooLarge,
        FetchError::ArchiveCapacity { .. } => AcquisitionFailureV1::ResourceLimit,
        FetchError::Cache { .. } => AcquisitionFailureV1::ArchiveIntegrity,
        FetchError::Cooldown { .. } | FetchError::BrowserLane { .. } => {
            AcquisitionFailureV1::SourceStopped
        }
        FetchError::Capture(CaptureTransportError::CacheOccurrenceMismatch) => {
            AcquisitionFailureV1::CacheChanged
        }
        _ => AcquisitionFailureV1::AdmissionRefused,
    }
}
