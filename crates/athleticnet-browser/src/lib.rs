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

#![forbid(unsafe_code)]

use std::time::Duration;

mod actor;
pub mod challenge;
pub mod clock;
pub mod drain;
mod lifecycle;
pub(crate) mod navigation;
mod ops;
mod outcome;
mod pool;
mod profile;
pub mod protocol;
pub mod request;
mod response_wire;
pub mod retry;
pub(crate) mod transport;
pub use lifecycle::BrowserManager;
pub use outcome::{
    BrowserCapture, BrowserError, BrowserFailure, BrowserOutcome, BrowserResponse, Verdict,
};
pub use profile::{BrowserSettings, BrowserState, BrowserStatus};
pub(crate) mod gate;

pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
