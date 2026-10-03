use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResponse {
    #[serde(with = "crate::response_wire::status")]
    pub status: StatusCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_url: Option<String>,
    #[serde(with = "crate::response_wire::headers")]
    pub headers: HeaderMap,
    #[serde(with = "crate::response_wire::body")]
    pub body: Vec<u8>,
    pub rankings: Option<crate::protocol::RankingPageObservation>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserError {
    #[error("browser transport failed")]
    Transport,
    #[error("browser request timed out")]
    Timeout,
    #[error("browser response exceeds payload limit")]
    PayloadLimit,
    #[error("browser redirect rejected")]
    Redirect,
    #[error("browser is unavailable")]
    Unavailable,
    #[error("human verification is required")]
    HumanRequired,
    #[error("browser protocol returned an invalid response")]
    Protocol,
    #[error("browser is shutting down")]
    Shutdown,
    #[error("browser worker task panicked")]
    TaskPanicked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    HumanRequired,
    Retryable,
    Terminal,
}

impl Verdict {
    pub fn of(error: BrowserError) -> Self {
        match error {
            BrowserError::Transport | BrowserError::Timeout => Self::Retryable,
            BrowserError::HumanRequired => Self::HumanRequired,
            BrowserError::PayloadLimit
            | BrowserError::Redirect
            | BrowserError::Unavailable
            | BrowserError::Protocol
            | BrowserError::Shutdown
            | BrowserError::TaskPanicked => Self::Terminal,
        }
    }

    pub fn retryable(self) -> bool {
        matches!(self, Self::Retryable)
    }

    pub fn human_required(self) -> bool {
        matches!(self, Self::HumanRequired)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCapture {
    pub response: BrowserResponse,
    pub challenge: bool,
    pub retry_after_ms: Option<u64>,
    pub fetched_at_ms: Option<u64>,
}

impl BrowserCapture {
    pub fn classify(response: BrowserResponse, clock: &dyn crate::clock::Clock) -> Self {
        let challenge = crate::challenge::response_challenge(&response);
        Self {
            retry_after_ms: crate::retry::retry_after_now(clock, &response.headers)
                .ok()
                .and_then(|delay| u64::try_from(delay.as_millis()).ok())
                .filter(|millis| *millis > 0),
            fetched_at_ms: clock.now_unix_ms().ok(),
            challenge,
            response,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserFailure {
    pub error: BrowserError,
    pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum BrowserOutcome {
    Captured(Box<BrowserCapture>),
    Failed(BrowserFailure),
}

impl BrowserOutcome {
    pub fn captured(response: BrowserResponse, clock: &dyn crate::clock::Clock) -> Self {
        Self::Captured(Box::new(BrowserCapture::classify(response, clock)))
    }

    pub fn failed(error: BrowserError) -> Self {
        Self::Failed(BrowserFailure {
            verdict: Verdict::of(error),
            error,
        })
    }

    pub fn response(&self) -> Option<&BrowserResponse> {
        match self {
            Self::Captured(capture) => Some(&capture.response),
            Self::Failed(_) => None,
        }
    }

    pub fn verdict(&self) -> Verdict {
        match self {
            Self::Captured(capture) if capture.challenge => Verdict::HumanRequired,
            Self::Captured(_) => Verdict::Terminal,
            Self::Failed(failure) => failure.verdict,
        }
    }
}
