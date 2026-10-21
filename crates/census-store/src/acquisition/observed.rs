use super::{facts::AttemptFacts, CaptureError};
use crate::StoreResult;
use census_domain::model::*;

pub(super) fn request(work: &AcquisitionWorkV1, dispatch: &DispatchRequestV1) -> StoreResult<()> {
    let Some(observed) = dispatch.observed_http() else {
        return if work.request().observed_http_required() {
            Err(CaptureError::TransitionRefused.into())
        } else {
            Ok(())
        };
    };
    if !work.request().observed_http_required() {
        return Err(CaptureError::TransitionRefused.into());
    }
    work.slot()
        .public_resource()
        .ok_or(CaptureError::ObligationConflict)?
        .validate_target(work.slot().source_site(), dispatch.target_url())
        .map_err(CaptureError::Domain)?;
    let representation = work.request().representation_headers();
    let mut expected = representation.len();
    for (name, value) in representation {
        header(observed, name, Some(value.as_bytes()))?;
    }
    let (etag, modified) = match dispatch.conditional() {
        ConditionalRequestV1::Unconditional => (None, None),
        ConditionalRequestV1::Pinned {
            etag,
            last_modified,
            ..
        } => (etag.as_deref(), last_modified.as_deref()),
    };
    for (name, value) in [("if-none-match", etag), ("if-modified-since", modified)] {
        header(observed, name, value.map(str::as_bytes))?;
        expected = expected
            .checked_add(usize::from(value.is_some()))
            .ok_or(CaptureError::CorruptClosure)?;
    }
    if observed.headers().len() != expected {
        return Err(CaptureError::TransitionRefused.into());
    }
    Ok(())
}

fn header(request: &ObservedHttpRequestV1, name: &str, expected: Option<&[u8]>) -> StoreResult<()> {
    let mut entries = request
        .headers()
        .iter()
        .filter(|header| header.name() == name);
    if entries.next().map(|header| header.value()) != expected || entries.next().is_some() {
        return Err(CaptureError::TransitionRefused.into());
    }
    Ok(())
}

pub(super) fn response(
    work: &AcquisitionWorkV1,
    dispatch: &AcquisitionDispatchId,
    request: &DispatchRequestV1,
    response: &ResponseObservationV1,
) -> StoreResult<()> {
    match (
        work.request().observed_http_required(),
        request.observed_http(),
        response.observed_http(),
    ) {
        (false, None, None) => Ok(()),
        (true, Some(request), Some(response)) => {
            ObservedHttpHopV1::new(dispatch.clone(), request.clone(), response.clone())
                .map_err(CaptureError::Domain)?;
            Ok(())
        }
        _ => Err(CaptureError::TransitionRefused.into()),
    }
}

pub(super) fn metadata(
    work: &AcquisitionWorkV1,
    attempt: &AttemptFacts,
    capture: &PreparedCaptureInputV1,
    normalized: Option<&NormalizedObservedHttpV1>,
) -> StoreResult<()> {
    let Some(normalized) = normalized else {
        return if work.request().observed_http_required() {
            Err(CaptureError::BackingIntegrity.into())
        } else {
            Ok(())
        };
    };
    if !work.request().observed_http_required()
        || normalized.hops().len() != attempt.dispatches.len()
        || normalized.body_digest() != capture.content_digest()
        || normalized.body_bytes() != capture.content_bytes()
    {
        return Err(CaptureError::BackingIntegrity.into());
    }
    for (hop, (armed, observed)) in normalized.hops().iter().zip(&attempt.dispatches) {
        if hop.dispatch() != &armed.dispatch
            || Some(hop.request()) != armed.request.observed_http()
            || Some(hop.response())
                != observed
                    .as_ref()
                    .and_then(|entry| entry.response.observed_http())
        {
            return Err(CaptureError::BackingIntegrity.into());
        }
    }
    Ok(())
}

pub(super) fn redirect(
    work: &AcquisitionWorkV1,
    prior: &ResponseObservationV1,
    next: &DispatchRequestV1,
) -> StoreResult<()> {
    match (
        work.request().observed_http_required(),
        prior.observed_http(),
        next.observed_http(),
    ) {
        (false, None, None) => Ok(()),
        (true, Some(prior), Some(next)) => prior
            .validate_redirect_to(next)
            .map_err(|error| CaptureError::Domain(error).into()),
        _ => Err(CaptureError::TransitionRefused.into()),
    }
}

pub(super) fn prefix(
    work: &AcquisitionWorkV1,
    attempt: &AttemptFacts,
    pending_request: Option<&DispatchRequestV1>,
    pending_response: Option<&ResponseObservationV1>,
) -> StoreResult<()> {
    if !work.request().observed_http_required() {
        return Ok(());
    }
    let mut count = 0;
    for (armed, observed) in &attempt.dispatches {
        let request = armed
            .request
            .observed_http()
            .ok_or(CaptureError::CorruptClosure)?;
        count = ObservedHttpHeaderV1::charge_chain_bytes(request.headers(), count)
            .map_err(CaptureError::Domain)?;
        if let Some(observed) = observed {
            let response = observed
                .response
                .observed_http()
                .ok_or(CaptureError::CorruptClosure)?;
            count = ObservedHttpHeaderV1::charge_chain_bytes(response.headers(), count)
                .map_err(CaptureError::Domain)?;
        }
    }
    if let Some(request) = pending_request {
        let request = request
            .observed_http()
            .ok_or(CaptureError::CorruptClosure)?;
        count = ObservedHttpHeaderV1::charge_chain_bytes(request.headers(), count)
            .map_err(CaptureError::Domain)?;
    }
    if let Some(response) = pending_response {
        let response = response
            .observed_http()
            .ok_or(CaptureError::CorruptClosure)?;
        ObservedHttpHeaderV1::charge_chain_bytes(response.headers(), count)
            .map_err(CaptureError::Domain)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
