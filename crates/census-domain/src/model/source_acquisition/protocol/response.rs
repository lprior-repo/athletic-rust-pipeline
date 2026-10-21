use super::super::scalar::{bounded, url};
use super::super::{CaptureDomainError, ObservedHttpResponseV1};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
mod observed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ResponseWire")]
pub struct ResponseObservationV1 {
    status: u16,
    response_url: String,
    observed_at: String,
    headers: Vec<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_http: Option<Box<ObservedHttpResponseV1>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseWire {
    status: u16,
    response_url: String,
    observed_at: String,
    headers: Vec<(String, String)>,
    observed_http: Option<ObservedHttpResponseV1>,
}
impl ResponseObservationV1 {
    pub fn new(
        status: u16,
        response_url: &str,
        observed_at: &str,
        headers: Vec<(String, String)>,
    ) -> Result<Self, CaptureDomainError> {
        if !(100..=599).contains(&status) {
            return Err(CaptureDomainError::Invalid { field: "status" });
        }
        url(response_url)?;
        bounded(observed_at, "observation time", 64)?;
        let year = chrono::DateTime::parse_from_rfc3339(observed_at)
            .map(|date| date.year())
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(observed_at, "%Y-%m-%d").map(|date| date.year())
            })
            .map_err(|_| CaptureDomainError::Invalid {
                field: "observation time",
            })?;
        if !(1900..=2100).contains(&year) {
            return Err(CaptureDomainError::Invalid {
                field: "observation time",
            });
        }
        super::super::request::headers(&headers, 32)?;
        Ok(Self {
            status,
            response_url: response_url.to_string(),
            observed_at: observed_at.to_string(),
            headers,
            observed_http: None,
        })
    }
    pub fn status(&self) -> u16 {
        self.status
    }
    pub fn response_url(&self) -> &str {
        &self.response_url
    }
    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
    pub fn is_for_dispatch(&self, request: &super::super::DispatchRequestV1) -> bool {
        match (url(&self.response_url), url(request.target_url())) {
            (Ok(actual), Ok(expected)) => actual == expected,
            _ => false,
        }
    }
    pub fn redirects_to(&self, request: &super::super::DispatchRequestV1) -> bool {
        if !self.is_redirect() {
            return false;
        }
        let Some((_, location)) = self.headers.iter().find(|(name, _)| name == "location") else {
            return false;
        };
        match (
            url(&self.response_url)
                .ok()
                .and_then(|base| base.join(location).ok()),
            url(request.target_url()).ok(),
        ) {
            (Some(actual), Some(expected)) => actual == expected,
            _ => false,
        }
    }
    pub fn is_redirect(&self) -> bool {
        matches!(self.status, 301 | 302 | 303 | 307 | 308)
    }
}
impl TryFrom<ResponseWire> for ResponseObservationV1 {
    type Error = CaptureDomainError;
    fn try_from(w: ResponseWire) -> Result<Self, Self::Error> {
        match w.observed_http {
            Some(observed) => Self::new_observed(
                w.status,
                &w.response_url,
                &w.observed_at,
                w.headers,
                observed,
            ),
            None => Self::new(w.status, &w.response_url, &w.observed_at, w.headers),
        }
    }
}
