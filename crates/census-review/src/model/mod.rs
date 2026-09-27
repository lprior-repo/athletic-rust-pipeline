use std::time::Duration;

use census_domain::model::{ReviewPacket, VerdictBatch};
use serde_json::Value;

mod endpoint;
mod request;
mod response;

pub use request::build_request_body;
use response::message_content;

use crate::model::response::parse_batch;

pub(super) const MAX_TOKENS_CAP: u32 = 8192;
const MAX_MODEL_LEN: usize = 256;
const REQUEST_CAP: usize = 1_048_576;
const RESPONSE_CAP: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOptions {
    endpoint: endpoint::ValidatedEndpoint,
    model: String,
    timeout: Duration,
    max_tokens: u32,
}

impl ModelOptions {
    pub fn local(raw: &str, model: &str) -> Result<Self, ModelError> {
        let endpoint = endpoint::ValidatedEndpoint::new(raw)?;

        if model.is_empty() {
            return Err(ModelError::InvalidEndpoint {
                reason: "model name must not be empty",
            });
        }
        if model.len() > MAX_MODEL_LEN {
            return Err(ModelError::InvalidEndpoint {
                reason: "model name exceeds maximum length",
            });
        }

        Ok(Self {
            endpoint,
            model: model.to_string(),
            timeout: Duration::from_secs(180),
            max_tokens: 1_536,
        })
    }

    pub fn completions_url(&self) -> String {
        self.endpoint.completions_url()
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("invalid endpoint: {reason}")]
    InvalidEndpoint { reason: &'static str },
    #[error("model request timed out")]
    RequestTimeout,
    #[error("model request failed")]
    RequestFailed,
    #[error("model returned status {status}")]
    Status { status: u16 },
    #[error("model returned no message content")]
    Empty,
    #[error("model content is not a verdict batch: {reason}")]
    Content { reason: &'static str },
    #[error("request exceeds maximum size: {bytes} bytes")]
    RequestTooLarge { bytes: usize },
    #[error("response exceeds maximum size: {bytes} bytes")]
    ResponseTooLarge { bytes: usize },
}

impl From<reqwest::Error> for ModelError {
    fn from(_err: reqwest::Error) -> Self {
        if _err.is_timeout() {
            ModelError::RequestTimeout
        } else {
            ModelError::RequestFailed
        }
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
            .timeout(options.timeout)
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
            .await?;
        let status = response.status().as_u16();
        let content_length = response.content_length().unwrap_or(0);
        let declared_cap = u64::try_from(RESPONSE_CAP).unwrap_or(u64::MAX);
        if content_length > declared_cap {
            return Err(ModelError::ResponseTooLarge {
                bytes: usize::try_from(content_length).unwrap_or(RESPONSE_CAP),
            });
        }
        let mut buf = Vec::new();
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(err) => return Err(ModelError::from(err)),
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
            reason: "response body is not valid UTF-8",
        })?;

        let parsed: Value = serde_json::from_str(&text).map_err(|_| ModelError::Content {
            reason: "response is not valid JSON",
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
