//! The Athletic.net acquisition transport: one persistent headed profile, one tab pool, and the CDP
//! request/response capture every source read goes through.
//!
//! # What this crate owns, and what a caller must not re-derive
//!
//! The transport performs one attempt and reports what it saw. Retrying belongs to the invocation
//! that asked (ADR-002), which is why nothing here has an attempt budget, a delay ladder, or a
//! sleep. What the crate does own is the classification of that attempt, and a caller that
//! classifies the same bytes a second time is reading them differently than the browser already
//! did:
//!
//! * **A challenge is classified here.** [`challenge::cf_header_challenge`] reads the
//!   `cf-mitigated` header; [`challenge::html_body_challenge`] reads the leading 128 KiB of an HTML
//!   body for the conjunctive marker sets that separate a Cloudflare interstitial from a public
//!   page which merely loads the platform script. A challenged fetch latches
//!   [`BrowserState::Challenged`], revokes the profile gate, and rejects every pending request with
//!   [`BrowserError::HumanRequired`].
//! * **Retry timing is classified here.** [`retry::retry_after_now`] reads `Retry-After` against
//!   the injected wall clock and fails closed on an unreadable clock, an ambiguous pair of headers,
//!   or an instant past [`retry::MAX_RETRY_DELAY`], so a caller never parks a lane on an unusable
//!   instant. A `429` is worth another invocation only by that reading of the header.
//! * **[`BrowserError`] is the failure vocabulary, [`Verdict`] is what a caller acts on.** The
//!   lifecycle maps the durable profile state onto it: a profile that is challenged or awaiting a
//!   human answers `HumanRequired`, and anything else that cannot serve traffic answers
//!   `Unavailable`. [`Verdict::of`] places every one of those reasons once, so a pipeline report and
//!   a census access condition cannot disagree about whether to ask again.
//! * **[`BrowserOutcome`] is what an attempt returns.** `BrowserManager::fetch` hands back either a
//!   [`BrowserCapture`] - the response together with the challenge verdict, the `Retry-After`
//!   reading and the capture instant - or a [`BrowserFailure`] with its verdict. Both encode on the
//!   wire the census mirrors, so a reader on the other side of that boundary reads a classification
//!   instead of performing one.
//!
//! # Rules a caller follows
//!
//! * Consume the classification above instead of scanning a captured body for markers of your own:
//!   it travels with the capture as `BrowserCapture::challenge`. A caller that scans again can
//!   disagree with the gate the transport already revoked.
//! * `BrowserError::HumanRequired` and `BrowserState::HumanRequired` are data, not obstacles: the
//!   profile needs a human step before it serves traffic. Stop the source, report the state, and do
//!   not work around it.
//! * Never drive one `profile_dir` from two managers. `pool::prepare_profile` validates the
//!   directory - a real directory, mode 0700, not accessible by other users - but does not lock it,
//!   so a second manager is not refused: it is a corruption path, not a second lane.
//! * Never rewrite the identity of a request. [`request::RequestSpec`] is what the transport
//!   fetched, and `semantic_url` is what a receipt cites; changing either leaves a capture that
//!   cannot be reconciled with its own evidence.
//!
//! No header spoofing, no login or captcha handling, no challenge solving, no proxy rotation: this
//! transport reads the site as itself, and a challenge is reported rather than circumvented.
//!
//! The `SourceResource` vocabulary stays in the pipeline crate - [`request::endpoint`] deliberately
//! takes an origin and a path - so a second acquirer (the census) can build the same spec, and cite
//! the same capture, without depending on the pipeline's source types.

use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use thiserror::Error;
use url::Url;

mod actor;
pub mod challenge;
pub mod clock;
pub mod drain;
mod lifecycle;
pub(crate) mod navigation;
mod ops;
mod pool;
pub mod protocol;
pub mod request;
mod response_wire;
pub mod retry;
pub(crate) mod transport;
pub use lifecycle::BrowserManager;
pub(crate) mod gate;

#[derive(Clone, Debug)]
pub struct BrowserSettings {
    pub cdp_endpoint: Option<Url>,
    pub executable: PathBuf,
    pub profile_dir: PathBuf,
    pub source_origin: Url,
    pub tabs: usize,
    pub request_timeout: Duration,
    pub challenge_wait: Duration,
    pub headed: bool,
}

impl BrowserSettings {
    pub fn validate(&self) -> anyhow::Result<()> {
        if !(1..=8).contains(&self.tabs) {
            anyhow::bail!("browser tab count must be in 1..=8");
        }
        if !self.executable.is_absolute() || !self.profile_dir.is_absolute() {
            anyhow::bail!("browser executable and profile paths must be absolute");
        }
        if !matches!(self.source_origin.scheme(), "http" | "https")
            || self.source_origin.host_str().is_none()
        {
            anyhow::bail!("browser source origin must be an HTTP URL with a host");
        }
        if self.request_timeout.is_zero() || self.challenge_wait.is_zero() {
            anyhow::bail!("browser timeouts must be positive");
        }
        if let Some(ref endpoint) = self.cdp_endpoint {
            validate_cdp_endpoint(endpoint)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserState {
    Ready,
    Challenged,
    CoolingDown,
    HumanRequired,
    Restarting,
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowserStatus {
    pub state: BrowserState,
    pub active_requests: usize,
    pub tabs: usize,
    pub cooldown_ms: u64,
}

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
    #[serde(with = "response_wire::status")]
    pub status: StatusCode,
    #[serde(with = "response_wire::headers")]
    pub headers: HeaderMap,
    #[serde(with = "response_wire::body")]
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
    /// [`BrowserState::Challenged`]: the profile needs a human step before it serves traffic again.
    pub challenge: bool,
    /// What `Retry-After` asked for, by this crate's arithmetic. `None` when the reply asked for no
    /// delay, or asked in a way that fails closed: an unusable header, an ambiguous pair, an instant
    /// past [`retry::MAX_RETRY_DELAY`], or an unreadable clock.
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

pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

fn validate_cdp_endpoint(url: &Url) -> anyhow::Result<()> {
    if url.scheme() != "http" && url.scheme() != "https" {
        anyhow::bail!("cdp endpoint must use http or https scheme");
    }
    let host = url
        .host()
        .ok_or_else(|| anyhow::anyhow!("cdp endpoint must have a host"))?;
    match host {
        url::Host::Domain("localhost") | url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST) => {}
        url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST) => {}
        _ => anyhow::bail!("cdp endpoint must resolve to loopback only"),
    }
    if url.port().is_none() {
        anyhow::bail!("cdp endpoint must have an explicit port");
    }
    if url.path() != "/" {
        anyhow::bail!("cdp endpoint path must be root (/)");
    }
    if !url.username().is_empty() || url.password().is_some() {
        anyhow::bail!("cdp endpoint must have no credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("cdp endpoint must have no query or fragment");
    }
    Ok(())
}
