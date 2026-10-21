use super::{
    validation, CaptureDomainError, ObservedHttpHeaderV1, ObservedHttpHopV1, ObservedHttpRequestV1,
    ObservedHttpResponseV1,
};
use serde::Deserializer;

pub(super) fn validate(
    hops: &[ObservedHttpHopV1],
    completed_at: &str,
) -> Result<(), CaptureDomainError> {
    if hops.is_empty() || hops.len() > 5 {
        return Err(CaptureDomainError::Bound {
            field: "observed hops",
            max: 5,
        });
    }
    let first = hops.first().ok_or(CaptureDomainError::Invalid {
        field: "observed first hop",
    })?;
    let mut header_bytes = 0usize;
    let mut previous_head = validation::timestamp(first.request().started_at())?;
    for (ordinal, hop) in hops.iter().enumerate() {
        if hop.dispatch().attempt() != first.dispatch().attempt()
            || usize::from(hop.dispatch().hop().get()) != ordinal
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed dispatch chain",
            });
        }
        if validation::timestamp(hop.request().started_at())? < previous_head {
            return Err(CaptureDomainError::Invalid {
                field: "observed chain chronology",
            });
        }
        previous_head = validation::timestamp(hop.response().received_at())?;
        header_bytes = count_headers(hop, header_bytes)?;
    }
    if validation::timestamp(completed_at)? < previous_head {
        return Err(CaptureDomainError::Invalid {
            field: "observed completion chronology",
        });
    }
    if hops
        .last()
        .is_some_and(|hop| hop.response().status() == 304)
    {
        return Err(CaptureDomainError::Invalid {
            field: "observed revalidated body",
        });
    }
    validate_redirects(hops)
}

fn validate_redirects(hops: &[ObservedHttpHopV1]) -> Result<(), CaptureDomainError> {
    for (previous, next) in hops.iter().zip(hops.iter().skip(1)) {
        successor(previous.response(), next.request())?;
    }
    Ok(())
}

pub(super) fn successor(
    response: &ObservedHttpResponseV1,
    request: &ObservedHttpRequestV1,
) -> Result<(), CaptureDomainError> {
    if !matches!(response.status(), 301 | 302 | 303 | 307 | 308) {
        return Err(CaptureDomainError::Invalid {
            field: "observed redirect status",
        });
    }
    if validation::timestamp(request.started_at())? < validation::timestamp(response.received_at())?
    {
        return Err(CaptureDomainError::Invalid {
            field: "observed chain chronology",
        });
    }
    if redirect_target(response)? != validation::url(request.target_url())? {
        return Err(CaptureDomainError::RequestTargetMismatch);
    }
    Ok(())
}

fn redirect_target(response: &ObservedHttpResponseV1) -> Result<url::Url, CaptureDomainError> {
    let invalid = CaptureDomainError::Invalid {
        field: "observed redirect location",
    };
    let mut headers = response
        .headers()
        .iter()
        .filter(|header| header.name() == "location");
    let header = headers.next().ok_or_else(|| invalid.clone())?;
    if headers.next().is_some() {
        return Err(invalid);
    }
    let location = std::str::from_utf8(header.value()).map_err(|_| invalid.clone())?;
    if location.is_empty() || location.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(invalid);
    }
    validation::url(response.response_url())?
        .join(location)
        .map_err(|_| invalid)
}

fn count_headers(hop: &ObservedHttpHopV1, initial: usize) -> Result<usize, CaptureDomainError> {
    let count = ObservedHttpHeaderV1::charge_chain_bytes(hop.request().headers(), initial)?;
    ObservedHttpHeaderV1::charge_chain_bytes(hop.response().headers(), count)
}

pub(super) fn deserialize_hops<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ObservedHttpHopV1>, D::Error> {
    crate::model::serde_bounds::sequence::<D, ObservedHttpHopV1, 5>(deserializer)
}
