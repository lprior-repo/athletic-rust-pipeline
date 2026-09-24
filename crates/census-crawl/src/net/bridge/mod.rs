//! The browser lane, census side: the request this crate posts, the answer it reads back, and the
//! evidence it mints from it.
//!
//! Athletic.net is acquired through the headed persistent-profile browser lane (`AGENTS.md`, source
//! policy), and exactly one process owns that profile — the pipeline's browser manager, behind the
//! root's `BrowserSession` object. The census therefore never launches a browser: [`BrowserLane`]
//! posts a request spec to that object and mints its own [`crate::net::FetchOutcome`] from the
//! response, so the evidence it records keeps the shape every other adapter already produces.
//!
//! What lives here is the wire half: `wire` mirrors both directions of `athleticnet-browser`'s
//! vocabulary, and the fixtures under `fixtures/wire/` are what keeps the two readings one contract —
//! the spec this module posts and the evidence it mints from the answer are each pinned by one.

mod lane;
mod wire;

#[cfg(test)]
mod tests;

pub use self::lane::{validate_origin, BrowserLane};
pub use self::wire::{
    Action, BrowserCapture, BrowserError, BrowserFailure, BrowserOutcome, BrowserResponse,
    RankingPageObservation, RankingsCapture, RequestSpec, SearchBody, Verdict,
};

/// The pipeline's object for the one headed profile. The name is deployment vocabulary rather than
/// this crate's: it is what the pipeline registers, and the census addresses it by name because the
/// two crates share no type that could carry it.
pub(super) const SESSION_OBJECT: &str = "BrowserSession";
/// The Restate object key for the one headed profile. The browser engine is configured to serve
/// exactly this profile; the pipeline registers the same key.
pub(super) const SESSION_KEY: &str = "profile-0";

/// Origins admitted to the browser transport.
///
/// Derived from the registry: every descriptor whose `transport` is `Browser` contributes its
/// `admission.origin` here.  A URL whose origin does not appear in this set is refused at the
/// lane boundary before it leaves this process.
pub(super) const ADMITTED_BROWSER_ORIGINS: &[&str] = &["www.athletic.net"];

/// Compile-time check that the session key is profile-0, the single profile the browser engine
/// serves.  If this fires the deployment is misconfigured.
// SESSION_KEY must be "profile-0" for correct lane routing (checked at runtime in init)
const _: () = { let _ = SESSION_KEY; };
