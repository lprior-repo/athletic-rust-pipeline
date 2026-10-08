use serde::{Deserialize, Serialize};

use crate::net::RepresentationHeaders;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    pub url: String,
    pub semantic_url: String,
    pub action: Action,
    pub headers: RepresentationHeaders,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum Action {
    Fetch { body: Option<SearchBody> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchBody {
    pub q: String,
    pub fq: String,
    pub start: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResponse {
    pub status: u16,
    #[serde(default)]
    pub response_url: Option<Box<str>>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub rankings: Option<RankingPageObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RankingPageObservation {
    pub capture: RankingsCapture,
    pub request_method: String,
    pub request_url: String,
    pub request_body: Option<String>,
    pub next_page: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RankingsCapture {
    Navigation,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserError {
    Transport,
    Timeout,
    PayloadLimit,
    Redirect,
    Unavailable,
    HumanRequired,
    Protocol,
    Shutdown,
    TaskPanicked,
}

impl BrowserError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transport => "transport",
            Self::Timeout => "timeout",
            Self::PayloadLimit => "payload_limit",
            Self::Redirect => "redirect",
            Self::Unavailable => "unavailable",
            Self::HumanRequired => "human_required",
            Self::Protocol => "protocol",
            Self::Shutdown => "shutdown",
            Self::TaskPanicked => "task_panicked",
        }
    }
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    HumanRequired,
    Retryable,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserFailure {
    pub error: BrowserError,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCapture {
    pub response: BrowserResponse,
    pub challenge: bool,
    pub retry_after_ms: Option<u64>,
    pub fetched_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum BrowserOutcome {
    Captured(BrowserCapture),
    Failed(BrowserFailure),
}
