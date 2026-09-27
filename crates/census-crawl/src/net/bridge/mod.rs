mod lane;
mod wire;

#[cfg(test)]
mod tests;

pub use self::lane::{validate_origin, BrowserLane};
pub use self::wire::{
    Action, BrowserCapture, BrowserError, BrowserFailure, BrowserOutcome, BrowserResponse,
    RankingPageObservation, RankingsCapture, RequestSpec, SearchBody, Verdict,
};

pub(super) const SESSION_OBJECT: &str = "BrowserSession";
pub(super) const SESSION_KEY: &str = "profile-0";

pub(super) const ADMITTED_BROWSER_ORIGINS: &[&str] = &["www.athletic.net"];

#[allow(clippy::panic)]
const _: fn() = || match SESSION_KEY {
    "profile-0" => {}
    other => panic!("browser lane key must be profile-0, got: {other}"),
};
