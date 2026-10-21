use crate::net::FetchError;
use census_domain::model::{
    CaptureDomainError, ObservedHttpHeaderV1, ObservedHttpProtocolV1, ObservedHttpRequestV1,
    ObservedHttpResponseV1,
};
use reqwest::header::HeaderMap;

pub(in crate::net) fn request(
    request: &reqwest::Request,
    started_at: &str,
) -> Result<ObservedHttpRequestV1, FetchError> {
    if request.method() != reqwest::Method::GET
        || request
            .body()
            .is_some_and(|body| body.as_bytes().is_none_or(|bytes| !bytes.is_empty()))
    {
        return Err(CaptureDomainError::Invalid {
            field: "observed request method or body",
        }
        .into());
    }
    Ok(ObservedHttpRequestV1::new(
        request.url().as_str(),
        started_at,
        headers(request.headers())?,
    )?)
}

pub(in crate::net) fn response(
    response: &reqwest::Response,
    received_at: &str,
) -> Result<ObservedHttpResponseV1, FetchError> {
    Ok(ObservedHttpResponseV1::new(
        protocol(response.version())?,
        response.status().as_u16(),
        response.url().as_str(),
        received_at,
        headers(response.headers())?,
    )?)
}

fn protocol(version: reqwest::Version) -> Result<ObservedHttpProtocolV1, CaptureDomainError> {
    match version {
        reqwest::Version::HTTP_10 => Ok(ObservedHttpProtocolV1::Http10),
        reqwest::Version::HTTP_11 => Ok(ObservedHttpProtocolV1::Http11),
        reqwest::Version::HTTP_2 => Ok(ObservedHttpProtocolV1::Http2),
        reqwest::Version::HTTP_3 => Ok(ObservedHttpProtocolV1::Http3),
        _ => Err(CaptureDomainError::Invalid {
            field: "observed response protocol",
        }),
    }
}

fn headers(headers: &HeaderMap) -> Result<Vec<ObservedHttpHeaderV1>, FetchError> {
    validate_headers(headers)?;
    let mut observed = Vec::new();
    observed
        .try_reserve_exact(headers.len())
        .map_err(|_| allocation())?;
    for (name, value) in headers {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(value.as_bytes().len())
            .map_err(|_| allocation())?;
        bytes.extend_from_slice(value.as_bytes());
        observed.push(ObservedHttpHeaderV1::new(name.as_str(), bytes)?);
    }
    Ok(observed)
}

fn validate_headers(headers: &HeaderMap) -> Result<(), CaptureDomainError> {
    if headers.len() > 32 {
        return Err(CaptureDomainError::Bound {
            field: "observed header occurrences",
            max: 32,
        });
    }
    for (name, value) in headers {
        if name.as_str().len() > 128 {
            return Err(CaptureDomainError::Bound {
                field: "observed header name",
                max: 128,
            });
        }
        if value.as_bytes().len() > 2048 {
            return Err(CaptureDomainError::Bound {
                field: "observed header value",
                max: 2048,
            });
        }
    }
    Ok(())
}

fn allocation() -> CaptureDomainError {
    CaptureDomainError::Invalid {
        field: "observed header allocation",
    }
}

#[cfg(test)]
mod tests;
