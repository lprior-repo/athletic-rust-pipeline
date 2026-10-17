use std::time::Duration;

use super::{ModelError, ModelOptions, ModelResponseFormat};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelOptionsWire {
    endpoint: String,
    model: String,
    timeout: Duration,
    max_tokens: u32,
    response_format: ModelResponseFormat,
}

impl TryFrom<ModelOptionsWire> for ModelOptions {
    type Error = ModelError;

    fn try_from(value: ModelOptionsWire) -> Result<Self, Self::Error> {
        Ok(Self::local_owned(value.endpoint, value.model)?
            .with_timeout(value.timeout)
            .with_max_tokens(value.max_tokens)
            .with_response_format(value.response_format))
    }
}
