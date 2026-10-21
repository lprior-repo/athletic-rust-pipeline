use super::validation;
use super::{CaptureDomainError, ObservedHttpHeaderV1, ObservedHttpProtocolV1};
use crate::model::AcquisitionDispatchId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RequestWire")]
pub struct ObservedHttpRequestV1 {
    target_url: String,
    started_at: String,
    headers: Vec<ObservedHttpHeaderV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    target_url: String,
    started_at: String,
    #[serde(deserialize_with = "validation::deserialize_headers")]
    headers: Vec<ObservedHttpHeaderV1>,
}

impl ObservedHttpRequestV1 {
    pub fn new(
        target_url: &str,
        started_at: &str,
        headers: Vec<ObservedHttpHeaderV1>,
    ) -> Result<Self, CaptureDomainError> {
        validation::url(target_url)?;
        validation::timestamp(started_at)?;
        validation::headers(&headers)?;
        Ok(Self {
            target_url: target_url.to_owned(),
            started_at: started_at.to_owned(),
            headers,
        })
    }

    pub fn target_url(&self) -> &str {
        &self.target_url
    }

    pub fn started_at(&self) -> &str {
        &self.started_at
    }

    pub fn headers(&self) -> &[ObservedHttpHeaderV1] {
        &self.headers
    }
}

impl TryFrom<RequestWire> for ObservedHttpRequestV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: RequestWire) -> Result<Self, Self::Error> {
        Self::new(&wire.target_url, &wire.started_at, wire.headers)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ResponseWire")]
pub struct ObservedHttpResponseV1 {
    protocol: ObservedHttpProtocolV1,
    status: u16,
    response_url: String,
    received_at: String,
    headers: Vec<ObservedHttpHeaderV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseWire {
    protocol: ObservedHttpProtocolV1,
    status: u16,
    response_url: String,
    received_at: String,
    #[serde(deserialize_with = "validation::deserialize_headers")]
    headers: Vec<ObservedHttpHeaderV1>,
}

impl ObservedHttpResponseV1 {
    pub fn new(
        protocol: ObservedHttpProtocolV1,
        status: u16,
        response_url: &str,
        received_at: &str,
        headers: Vec<ObservedHttpHeaderV1>,
    ) -> Result<Self, CaptureDomainError> {
        if !(100..=599).contains(&status) {
            return Err(CaptureDomainError::Invalid {
                field: "observed response status",
            });
        }
        validation::url(response_url)?;
        validation::timestamp(received_at)?;
        validation::headers(&headers)?;
        validation::identity_encoding(&headers)?;
        Ok(Self {
            protocol,
            status,
            response_url: response_url.to_owned(),
            received_at: received_at.to_owned(),
            headers,
        })
    }

    pub fn protocol(&self) -> ObservedHttpProtocolV1 {
        self.protocol
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn response_url(&self) -> &str {
        &self.response_url
    }

    pub fn received_at(&self) -> &str {
        &self.received_at
    }

    pub fn headers(&self) -> &[ObservedHttpHeaderV1] {
        &self.headers
    }
    pub fn validate_redirect_to(
        &self,
        request: &ObservedHttpRequestV1,
    ) -> Result<(), CaptureDomainError> {
        super::chain::successor(self, request)
    }
}

impl TryFrom<ResponseWire> for ObservedHttpResponseV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: ResponseWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.protocol,
            wire.status,
            &wire.response_url,
            &wire.received_at,
            wire.headers,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HopWire")]
pub struct ObservedHttpHopV1 {
    dispatch: AcquisitionDispatchId,
    request: ObservedHttpRequestV1,
    response: ObservedHttpResponseV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HopWire {
    dispatch: AcquisitionDispatchId,
    request: ObservedHttpRequestV1,
    response: ObservedHttpResponseV1,
}

impl ObservedHttpHopV1 {
    pub fn new(
        dispatch: AcquisitionDispatchId,
        request: ObservedHttpRequestV1,
        response: ObservedHttpResponseV1,
    ) -> Result<Self, CaptureDomainError> {
        if validation::url(request.target_url())? != validation::url(response.response_url())? {
            return Err(CaptureDomainError::RequestTargetMismatch);
        }
        if validation::timestamp(request.started_at())?
            > validation::timestamp(response.received_at())?
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed hop chronology",
            });
        }
        Ok(Self {
            dispatch,
            request,
            response,
        })
    }

    pub fn dispatch(&self) -> &AcquisitionDispatchId {
        &self.dispatch
    }

    pub fn request(&self) -> &ObservedHttpRequestV1 {
        &self.request
    }

    pub fn response(&self) -> &ObservedHttpResponseV1 {
        &self.response
    }
}

impl TryFrom<HopWire> for ObservedHttpHopV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: HopWire) -> Result<Self, Self::Error> {
        Self::new(wire.dispatch, wire.request, wire.response)
    }
}
