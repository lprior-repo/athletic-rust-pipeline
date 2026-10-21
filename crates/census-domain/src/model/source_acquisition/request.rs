use super::scalar::{bounded, url};
use super::{AcquisitionOccurrenceRefV1, CaptureDomainError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Method {
    #[serde(rename = "GET")]
    Get,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Body {
    Empty,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransportRecipe {
    DirectHttpOneShotV1,
    ReqwestNormalizedObservedHttpV1,
}
mod normalized;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RequestWire")]
pub struct RequestRepresentationV1 {
    method: Method,
    original_url: String,
    body: Body,
    representation_headers: Vec<(String, String)>,
    transport_recipe: TransportRecipe,
    effective_user_agent: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    method: Method,
    original_url: String,
    body: Body,
    representation_headers: Vec<(String, String)>,
    transport_recipe: TransportRecipe,
    effective_user_agent: String,
}
impl RequestRepresentationV1 {
    pub fn get(
        original_url: &str,
        representation_headers: Vec<(String, String)>,
        effective_user_agent: &str,
    ) -> Result<Self, CaptureDomainError> {
        url(original_url)?;
        bounded(effective_user_agent, "effective user agent", 1024)?;
        if !effective_user_agent
            .bytes()
            .all(|b| matches!(b, b' '..=b'~'))
        {
            return Err(CaptureDomainError::Invalid {
                field: "effective user agent",
            });
        }
        headers(&representation_headers, 8)?;
        if representation_headers.iter().any(|(name, _)| {
            !matches!(
                name.as_str(),
                "accept" | "accept-language" | "anettokens" | "user-agent"
            )
        }) {
            return Err(CaptureDomainError::Invalid {
                field: "representation headers",
            });
        }
        Ok(Self {
            method: Method::Get,
            original_url: original_url.to_string(),
            body: Body::Empty,
            representation_headers,
            transport_recipe: TransportRecipe::DirectHttpOneShotV1,
            effective_user_agent: effective_user_agent.to_string(),
        })
    }
    pub fn original_url(&self) -> &str {
        &self.original_url
    }
    pub fn representation_headers(&self) -> &[(String, String)] {
        &self.representation_headers
    }
    pub fn effective_user_agent(&self) -> &str {
        &self.effective_user_agent
    }
    pub fn observed_http_required(&self) -> bool {
        self.transport_recipe == TransportRecipe::ReqwestNormalizedObservedHttpV1
    }
}
impl TryFrom<RequestWire> for RequestRepresentationV1 {
    type Error = CaptureDomainError;
    fn try_from(w: RequestWire) -> Result<Self, Self::Error> {
        let (Method::Get, Body::Empty) = (w.method, w.body);
        match w.transport_recipe {
            TransportRecipe::DirectHttpOneShotV1 => Self::get(
                &w.original_url,
                w.representation_headers,
                &w.effective_user_agent,
            ),
            TransportRecipe::ReqwestNormalizedObservedHttpV1 => Self::normalized_get(
                &w.original_url,
                w.representation_headers,
                &w.effective_user_agent,
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionalRequestV1 {
    Unconditional,
    Pinned {
        original: AcquisitionOccurrenceRefV1,
        etag: Option<String>,
        last_modified: Option<String>,
    },
}
impl ConditionalRequestV1 {
    pub fn validate(&self) -> Result<(), CaptureDomainError> {
        if let Self::Pinned {
            etag,
            last_modified,
            ..
        } = self
        {
            if etag.is_none() && last_modified.is_none() {
                return Err(CaptureDomainError::Invalid {
                    field: "conditional validators",
                });
            }
            for value in [etag, last_modified].into_iter().flatten() {
                bounded(value, "conditional validator", 1024)?;
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DispatchWire")]
pub struct DispatchRequestV1 {
    target_url: String,
    conditional: ConditionalRequestV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_http: Option<Box<super::ObservedHttpRequestV1>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispatchWire {
    target_url: String,
    conditional: ConditionalRequestV1,
    observed_http: Option<super::ObservedHttpRequestV1>,
}
impl DispatchRequestV1 {
    pub fn new(
        target_url: &str,
        conditional: ConditionalRequestV1,
    ) -> Result<Self, CaptureDomainError> {
        url(target_url)?;
        conditional.validate()?;
        Ok(Self {
            target_url: target_url.to_string(),
            conditional,
            observed_http: None,
        })
    }
    pub fn target_url(&self) -> &str {
        &self.target_url
    }
    pub fn conditional(&self) -> &ConditionalRequestV1 {
        &self.conditional
    }
    pub fn new_observed(
        target_url: &str,
        conditional: ConditionalRequestV1,
        observed_http: super::ObservedHttpRequestV1,
    ) -> Result<Self, CaptureDomainError> {
        let mut result = Self::new(target_url, conditional)?;
        if observed_http.target_url() != target_url {
            return Err(CaptureDomainError::RequestTargetMismatch);
        }
        result.observed_http = Some(Box::new(observed_http));
        Ok(result)
    }
    pub fn observed_http(&self) -> Option<&super::ObservedHttpRequestV1> {
        self.observed_http.as_deref()
    }
}
impl TryFrom<DispatchWire> for DispatchRequestV1 {
    type Error = CaptureDomainError;
    fn try_from(w: DispatchWire) -> Result<Self, Self::Error> {
        match w.observed_http {
            Some(observed) => Self::new_observed(&w.target_url, w.conditional, observed),
            None => Self::new(&w.target_url, w.conditional),
        }
    }
}

pub(super) fn headers(values: &[(String, String)], max: usize) -> Result<(), CaptureDomainError> {
    if values.len() > max {
        return Err(CaptureDomainError::Bound {
            field: "headers",
            max,
        });
    }
    let mut previous: Option<&str> = None;
    for (name, value) in values {
        bounded(name, "header name", 64)?;
        if !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || previous.is_some_and(|old| old >= name.as_str())
        {
            return Err(CaptureDomainError::Invalid {
                field: "header order/name",
            });
        }
        if value.len() > 1024 || value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
            return Err(CaptureDomainError::Bound {
                field: "header value",
                max: 1024,
            });
        }
        previous = Some(name);
    }
    Ok(())
}
