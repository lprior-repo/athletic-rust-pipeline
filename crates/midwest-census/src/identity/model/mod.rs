//! Talking to a local model server, and reading its answer as if the model were hostile.
//!
//! The lane's model is a small local server that produces JSON from a grammar, not an authority. Two
//! things are therefore fixed here rather than left to the prompt:
//!
//! * The request constrains the *shape* of the answer (`response_format` with a JSON schema), so a
//!   malformed batch is a server or prompt problem rather than a parse failure to be guessed at.
//! * The response is read for the one field that carries content and handed back as text: parsing,
//!   filtering and clamping happen in [`VerdictBatch::sanitize`], where they are testable, not in an
//!   HTTP client.
//!
//! Reasoning models spend their output budget thinking before they answer, so the request disables
//! thinking through the chat template and the client refuses a batch whose content is empty — the
//! caller then sees "the model produced nothing" instead of a mysterious parse error.
//!
//! `request` builds the body, `response` reads the answer back, and this file holds the client, the
//! options it was built with, and the error both halves return.

use std::time::Duration;

use census_domain::model::{ReviewPacket, VerdictBatch};
use serde_json::Value;

mod request;
mod response;

pub use request::build_request_body;
use response::{message_content, parse_batch, truncate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOptions {
    /// Base URL of an OpenAI-compatible server, e.g. `http://127.0.0.1:52080`.
    pub endpoint: String,
    /// The model name to ask for.
    pub model: String,
    /// Per-request budget: a local model that has not answered in this long is not going to.
    pub timeout: Duration,
    /// Output budget. A batch of a handful of verdicts fits well inside this.
    pub max_tokens: u32,
}

impl ModelOptions {
    /// Options for a local server at `endpoint`.
    pub fn local(endpoint: &str, model: &str) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
            timeout: Duration::from_secs(180),
            max_tokens: 1_536,
        }
    }

    /// The completions URL for this endpoint.
    pub fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.endpoint)
    }
}

/// Why a request or an answer could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("model request to {url} failed: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("model server answered {status} for {url}: {body}")]
    Status {
        url: String,
        status: u16,
        body: String,
    },
    #[error("model server answered {status} for {url} but no message content was readable")]
    Empty { url: String, status: u16 },
    #[error("model content was not a verdict batch: {source} (content: {content})")]
    Content {
        content: String,
        #[source]
        source: serde_json::Error,
    },
}

/// An OpenAI-compatible client for one local server.
#[derive(Debug, Clone)]
pub struct ModelClient {
    http: reqwest::Client,
    options: ModelOptions,
}

impl ModelClient {
    /// Build a client for `options`.
    pub fn new(options: ModelOptions) -> Result<Self, ModelError> {
        let http = reqwest::Client::builder()
            .timeout(options.timeout)
            .build()
            .map_err(|source| ModelError::Request {
                url: options.endpoint.clone(),
                source,
            })?;
        Ok(Self { http, options })
    }

    /// The options this client was built with.
    pub fn options(&self) -> &ModelOptions {
        &self.options
    }

    /// Ask the model about one packet and hand back the batch it returned, unsanitized.
    ///
    /// The caller sanitizes: this method's contract is only "the model answered with something shaped
    /// like a batch".
    pub async fn adjudicate(&self, packet: &ReviewPacket) -> Result<VerdictBatch, ModelError> {
        let url = self.options.completions_url();
        let body = build_request_body(packet, &self.options);
        let response = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|source| ModelError::Request {
                url: url.clone(),
                source,
            })?;
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(ModelError::Status {
                url,
                status,
                body: truncate(&text, 400),
            });
        }
        let parsed: Value = serde_json::from_str(&text).map_err(|source| ModelError::Content {
            content: truncate(&text, 400),
            source,
        })?;
        let content = message_content(&parsed).ok_or(ModelError::Empty { url, status })?;
        parse_batch(&content)
    }
}

#[cfg(test)]
#[path = "../model_tests.rs"]
mod tests;
