//! What one attempt returns, and the classification that travels with it.
//!
//! [`BrowserOutcome`] is the only type the transport hands back: a [`BrowserCapture`] - the captured
//! [`BrowserResponse`] together with the challenge verdict, the `Retry-After` reading and the
//! capture instant, all taken once when the capture completed - or a [`BrowserFailure`] carrying
//! both a [`BrowserError`] and the [`Verdict`] derived from it. Both encode on the wire the census
//! mirrors, so a reader on the other side of that boundary reads a classification instead of
//! performing one.
//!
//! Nothing here re-reads the bytes it classified: the challenge verdict and the retry timing are
//! fields of the capture, and [`Verdict::of`] is the single place a [`BrowserError`] is placed.

use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One captured response.
///
/// The type crosses a deployment boundary, so it is `Serialize`/`Deserialize` on the wire both
/// readers share; `response_wire` carries the fields that have no representation of their own. The
/// verdict and the timing are deliberately **not** fields here: they are taken once when the
/// capture completes and travel with it in [`BrowserCapture`], so that no reader has to reach a
/// verdict of its own from the same bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResponse {
    #[serde(with = "crate::response_wire::status")]
    pub status: StatusCode,
    #[serde(with = "crate::response_wire::headers")]
    pub headers: HeaderMap,
    #[serde(with = "crate::response_wire::body")]
    pub body: Vec<u8>,
    pub rankings: Option<crate::protocol::RankingPageObservation>,
}

/// The typed reasons a request can stop without a capture.
///
/// The reasons travel on the wire with the failure that carries them, so a reader classifies an
/// outcome from the outcome itself rather than from a lookup table it keeps beside this enum.
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

/// What the transport decided about one attempt.
///
/// This is data rather than a rule each caller re-writes, because more than one reader now acts on
/// a single attempt - the pipeline's source lane and the census - and a second reading of the same
/// bytes can disagree with the gate the transport has already applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The profile needs a human step before it serves traffic again.
    HumanRequired,
    /// Another invocation is worth making after this outcome.
    Retryable,
    /// Another invocation is not worth making: terminal for the caller that saw it.
    Terminal,
}

impl Verdict {
    /// Classify one failure - the single place this decision is written down.
    ///
    /// Every variant is listed rather than folded into a catch-all, so a new `BrowserError` cannot
    /// reach a caller unclassified: it fails to compile here until it is placed.
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

    /// Whether a later invocation is worth making.
    pub fn retryable(self) -> bool {
        matches!(self, Self::Retryable)
    }

    /// Whether the profile needs a human step before it serves traffic again.
    pub fn human_required(self) -> bool {
        matches!(self, Self::HumanRequired)
    }
}

/// A captured response together with the verdict and the timing the transport took for it.
///
/// The verdict, the `Retry-After` reading and the timestamp are taken once, when the capture
/// completes, so the profile gate, the cooldown and the receipt all read one computation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCapture {
    pub response: BrowserResponse,
    /// The body is a challenge page, or the reply carried `cf-mitigated: challenge`. A challenged
    /// capture is not the source's answer, and the transport has already latched
    /// [`crate::BrowserState::Challenged`]: the profile needs a human step before it serves traffic again.
    pub challenge: bool,
    /// What `Retry-After` asked for, by this crate's arithmetic. `None` when the reply asked for no
    /// delay, or asked in a way that fails closed: an unusable header, an ambiguous pair, an instant
    /// past [`crate::retry::MAX_RETRY_DELAY`], or an unreadable clock.
    pub retry_after_ms: Option<u64>,
    /// When the capture completed, by this crate's wall clock. `None` when the injected clock could
    /// not be read - a receipt records it as absent, it never substitutes an instant.
    pub fetched_at_ms: Option<u64>,
}

impl BrowserCapture {
    /// Capture one response, taking every value here: the challenge verdict, the `Retry-After`
    /// arithmetic and the wall-clock reading. One site decides all three, which is what keeps a
    /// caller's receipt and the transport's own gate from disagreeing.
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

/// A request the transport stopped without a capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserFailure {
    pub error: BrowserError,
    pub verdict: Verdict,
}

/// The transport's whole answer for one request: a capture with its verdict, or the typed failure it
/// stopped on.
///
/// This is the only type the transport hands back, so a caller cannot receive a capture without its
/// verdict or a failure without its classification. The two ways a lane used to drift - re-scanning
/// a body for a challenge, or mapping an error onto "retryable" of its own - have no shape here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum BrowserOutcome {
    /// A capture, boxed: a failure is two bytes and a capture is around 256, so the indirection
    /// keeps every queue entry, reply channel and join result small. Serde sees through the `Box`,
    /// so the wire - and the committed fixture - is unchanged.
    Captured(Box<BrowserCapture>),
    Failed(BrowserFailure),
}

impl BrowserOutcome {
    /// Classify one completed capture.
    pub fn captured(response: BrowserResponse, clock: &dyn crate::clock::Clock) -> Self {
        Self::Captured(Box::new(BrowserCapture::classify(response, clock)))
    }

    /// The outcome for a request that never reached the transport: a closed gate, a drain, a dead
    /// channel. The verdict is derived here, so those paths cannot report "retryable" by hand.
    pub fn failed(error: BrowserError) -> Self {
        Self::Failed(BrowserFailure {
            verdict: Verdict::of(error),
            error,
        })
    }

    /// The captured response, when there was one.
    pub fn response(&self) -> Option<&BrowserResponse> {
        match self {
            Self::Captured(capture) => Some(&capture.response),
            Self::Failed(_) => None,
        }
    }

    /// The verdict: the capture's challenge state, or the failure's classification.
    pub fn verdict(&self) -> Verdict {
        match self {
            Self::Captured(capture) if capture.challenge => Verdict::HumanRequired,
            Self::Captured(_) => Verdict::Terminal,
            Self::Failed(failure) => failure.verdict,
        }
    }
}
