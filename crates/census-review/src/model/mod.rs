use census_domain::model::{ReviewPacket, VerdictBatch};
use census_domain::review_model::{ModelContentFailure, ModelError, ModelOptions};
use serde_json::Value;

mod request;
mod response;

pub use request::build_request_body;
use response::message_content;

use crate::model::response::parse_batch;

pub(super) const MAX_TOKENS_CAP: u32 = 8192;
const REQUEST_CAP: usize = 1_048_576;
const RESPONSE_CAP: usize = 262_144;

fn classify_transport_error(error: reqwest::Error) -> ModelError {
    if error.is_timeout() {
        ModelError::RequestTimeout
    } else {
        ModelError::RequestFailed
    }
}

#[derive(Debug, Clone)]
pub struct ModelClient {
    http: reqwest::Client,
    options: ModelOptions,
}

impl ModelClient {
    pub fn new(options: ModelOptions) -> Result<Self, ModelError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(options.timeout())
            .build()
            .map_err(|_| ModelError::RequestFailed)?;
        Ok(Self { http, options })
    }

    pub fn options(&self) -> &ModelOptions {
        &self.options
    }

    pub async fn adjudicate(&self, packet: &ReviewPacket) -> Result<VerdictBatch, ModelError> {
        let url = self.options.completions_url();
        let body = build_request_body(packet, &self.options)?;
        let mut response = self
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(classify_transport_error)?;
        let status = response.status().as_u16();
        let content_length = response.content_length().map_or(0, |value| value);
        let declared_cap = u64::try_from(RESPONSE_CAP).map_or(u64::MAX, |value| value);
        if content_length > declared_cap {
            return Err(ModelError::ResponseTooLarge {
                bytes: usize::try_from(content_length).map_or(RESPONSE_CAP, |value| value),
            });
        }
        let mut buf = Vec::new();
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(err) => return Err(classify_transport_error(err)),
            };
            let grown = buf.len().saturating_add(chunk.len());
            if grown > RESPONSE_CAP {
                return Err(ModelError::ResponseTooLarge { bytes: grown });
            }
            buf.extend_from_slice(&chunk);
        }

        if !(200..300).contains(&status) {
            return Err(ModelError::Status { status });
        }

        let text = String::from_utf8(buf).map_err(|_| ModelError::Content {
            reason: ModelContentFailure::InvalidUtf8,
        })?;

        let parsed: Value = serde_json::from_str(&text).map_err(|_| ModelError::Content {
            reason: ModelContentFailure::InvalidJson,
        })?;
        let content = message_content(&parsed).ok_or(ModelError::Empty)?;
        parse_batch(&content)
    }
}

#[cfg(test)]
#[path = "../model_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../model_transport_tests.rs"]
mod transport_tests;
