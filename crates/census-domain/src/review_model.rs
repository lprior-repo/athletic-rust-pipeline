use std::time::Duration;

use endpoint::ValidatedEndpoint;
use wire::ModelOptionsWire;

mod endpoint;
mod error;
mod wire;

pub use error::{ModelContentFailure, ModelEndpointFailure, ModelError};

const MAX_MODEL_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResponseFormat {
    JsonSchema,
    PromptJson,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "ModelOptionsWire")]
pub struct ModelOptions {
    endpoint: ValidatedEndpoint,
    model: String,
    timeout: Duration,
    max_tokens: u32,
    response_format: ModelResponseFormat,
}

impl ModelOptions {
    pub fn local(raw: &str, model: &str) -> Result<Self, ModelError> {
        let endpoint = ValidatedEndpoint::new(raw)?;
        validate_model_name(model)?;

        Ok(Self::new(endpoint, model.to_string()))
    }

    fn local_owned(raw: String, model: String) -> Result<Self, ModelError> {
        let endpoint = ValidatedEndpoint::from_owned(raw)?;
        validate_model_name(&model)?;
        Ok(Self::new(endpoint, model))
    }

    fn new(endpoint: ValidatedEndpoint, model: String) -> Self {
        Self {
            endpoint,
            model,
            timeout: Duration::from_secs(180),
            max_tokens: 1_536,
            response_format: ModelResponseFormat::JsonSchema,
        }
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

    pub fn response_format(&self) -> ModelResponseFormat {
        self.response_format
    }

    pub fn with_response_format(mut self, response_format: ModelResponseFormat) -> Self {
        self.response_format = response_format;
        self
    }
}

fn validate_model_name(model: &str) -> Result<(), ModelError> {
    if model.len() > MAX_MODEL_LEN {
        return Err(ModelError::InvalidEndpoint {
            reason: ModelEndpointFailure::ModelTooLong,
        });
    }
    if model.trim().is_empty() {
        return Err(ModelError::InvalidEndpoint {
            reason: ModelEndpointFailure::EmptyModel,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "review_model/tests.rs"]
mod tests;
