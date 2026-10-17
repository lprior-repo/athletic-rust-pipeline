use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelEndpointFailure {
    InvalidUrl,
    HttpRequired,
    QueryOrFragment,
    ControlCharacters,
    MissingHost,
    IpRequired,
    LoopbackRequired,
    ExplicitPortRequired,
    NonZeroPortRequired,
    UserInfo,
    DotSegments,
    EncodedPath,
    RootPathRequired,
    ModelTooLong,
    EmptyModel,
}

impl std::fmt::Display for ModelEndpointFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(f, "invalid URL"),
            Self::HttpRequired => write!(f, "only http scheme is allowed"),
            Self::QueryOrFragment => write!(f, "query strings and fragments not allowed"),
            Self::ControlCharacters => write!(f, "control characters not allowed"),
            Self::MissingHost => write!(f, "endpoint must include a host"),
            Self::IpRequired => write!(f, "only IP addresses are allowed"),
            Self::LoopbackRequired => write!(f, "only loopback addresses are allowed"),
            Self::ExplicitPortRequired => write!(f, "port must be explicit"),
            Self::NonZeroPortRequired => write!(f, "port must be between 1 and 65535"),
            Self::UserInfo => write!(f, "userinfo not allowed"),
            Self::DotSegments => write!(f, "dot-segments in path not allowed"),
            Self::EncodedPath => write!(f, "percent-encoded path segments not allowed"),
            Self::RootPathRequired => write!(f, "only root path is allowed"),
            Self::ModelTooLong => write!(f, "model name exceeds maximum length"),
            Self::EmptyModel => write!(f, "model name must not be empty"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelContentFailure {
    NotVerdictBatch,
    ConfidenceRange,
    InvalidUtf8,
    InvalidJson,
}

impl std::fmt::Display for ModelContentFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotVerdictBatch => write!(f, "not a verdict batch"),
            Self::ConfidenceRange => write!(f, "confidence exceeds 100"),
            Self::InvalidUtf8 => write!(f, "response body is not valid UTF-8"),
            Self::InvalidJson => write!(f, "response is not valid JSON"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelError {
    InvalidEndpoint { reason: ModelEndpointFailure },
    RequestTimeout,
    RequestFailed,
    Status { status: u16 },
    Empty,
    Content { reason: ModelContentFailure },
    RequestTooLarge { bytes: usize },
    ResponseTooLarge { bytes: usize },
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEndpoint { reason } => write!(f, "invalid endpoint: {reason}"),
            Self::RequestTimeout => write!(f, "model request timed out"),
            Self::RequestFailed => write!(f, "model request failed"),
            Self::Status { status } => write!(f, "model returned status {status}"),
            Self::Empty => write!(f, "model returned no message content"),
            Self::Content { reason } => write!(f, "model content is not a verdict batch: {reason}"),
            Self::RequestTooLarge { bytes } => {
                write!(f, "request exceeds maximum size: {bytes} bytes")
            }
            Self::ResponseTooLarge { bytes } => {
                write!(f, "response exceeds maximum size: {bytes} bytes")
            }
        }
    }
}

impl std::error::Error for ModelError {}
