//! The browser lane's wire contract, mirrored from `athleticnet-browser`'s request vocabulary.
//!
//! The census may not depend on that crate: it links the browser engine, and linking it would put the
//! ability to open a second manager on the one headed profile inside this crate's tree. What the two
//! sides share instead is the committed fixtures under `fixtures/wire/`, read by both crates' tests.
//! These types are the census's reading of them, and the fixture tests prove the reading is exact.
//!
//! Only the `fetch` action is mirrored. It is the only one the census sends: the pipeline's own
//! `rankings` action is built and consumed entirely on its side of the lane. The answer direction is
//! mirrored whole, including the two shapes the census reads nothing out of (`rankings`), because a
//! mirror that accepted any JSON there would stop being a drift alarm.

use serde::{Deserialize, Serialize};

/// One browser-lane request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    /// The address the browser navigates to.
    pub url: String,
    /// The identity a receipt cites. Equal to `url` when the two are one address.
    pub semantic_url: String,
    /// What the browser does with that address.
    pub action: Action,
}

/// What the browser does with the address it is handed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum Action {
    /// Fetch the address and capture the response.
    Fetch {
        /// A JSON body for endpoints that take one; the census's own requests carry none.
        body: Option<SearchBody>,
    },
}

/// The body the search endpoint takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchBody {
    pub q: String,
    pub fq: String,
    pub start: u32,
}

/// One captured response, mirrored from the crate's own `response_wire` codecs.
///
/// `status`, `headers` and `body` have no representation of their own on the wire, so the crate
/// carries them as a `u16`, ordered name/value pairs and base64. The mirror reads the same three
/// shapes; nothing here re-encodes them, because the census never writes an answer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResponse {
    /// The status as its `u16`.
    pub status: u16,
    /// Ordered name/value pairs: a repeated name keeps every value it carried (`set-cookie` is the
    /// one that does in practice). Names arrive lowercased, the way `HeaderName` keeps them.
    pub headers: Vec<(String, String)>,
    /// The body, base64, as the capture carries it.
    pub body: String,
    /// The rankings observation, when the capture came from the pipeline's rankings lane. The census
    /// reads nothing here; the field is mirrored so a capture that carries one still decodes.
    pub rankings: Option<RankingPageObservation>,
}

/// The pipeline's own rankings observation, mirrored for the reason [`BrowserResponse::rankings`]
/// states.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RankingPageObservation {
    pub capture: RankingsCapture,
    pub request_method: String,
    pub request_url: String,
    pub request_body: Option<String>,
    pub next_page: Option<u32>,
}

/// Which rankings page a capture came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RankingsCapture {
    Navigation,
    Results,
}

/// The typed reasons a request can stop without a capture.
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
    /// The reason's own name on the wire.
    ///
    /// The crate's `Display` text is its own and is not mirrored: what both sides share is the
    /// spelling the fixture pins, so a refusal an operator reads cannot drift from the contract.
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

/// What a failure means for a later invocation, as the transport itself classified it.
///
/// Read from the wire rather than re-derived: the two ways a lane used to drift — re-scanning a body
/// for a challenge, or mapping an error onto "retryable" of its own — have no shape here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The profile needs a person before it serves traffic again.
    HumanRequired,
    /// Another invocation is worth making after this outcome.
    Retryable,
    /// Another invocation is not worth making: terminal for the caller that saw it.
    Terminal,
}

/// A request the transport stopped without a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserFailure {
    pub error: BrowserError,
    pub verdict: Verdict,
}

/// One capture and the three readings that travel with it.
///
/// The verdict is not a field: `challenge` is the verdict for a capture, and the transport took it
/// from the body markers and the reply's own headers, so a reader that has the flag has proved the
/// body decoded rather than merely the status.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCapture {
    pub response: BrowserResponse,
    /// The body is a challenge page, or the reply carried a challenge header.
    pub challenge: bool,
    /// What `Retry-After` asked for. `None` is a reading, not a zero: an unusable header, an
    /// ambiguous pair or an unreadable clock all land here, and a receipt records the absence
    /// instead of substituting an instant.
    pub retry_after_ms: Option<u64>,
    /// When the capture completed, by the transport's clock. `None` reads the same way.
    pub fetched_at_ms: Option<u64>,
}

/// The transport's whole answer for one request: a capture with its verdict, or the typed failure it
/// stopped on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum BrowserOutcome {
    Captured(BrowserCapture),
    Failed(BrowserFailure),
}
